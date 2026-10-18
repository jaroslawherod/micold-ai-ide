//! The daemon connection actor (client side of the render/drive switch, task T041).
//!
//! A single long-lived iced [`Subscription`] owns the socket to the daemon: it `connect_or_spawn`s
//! (auto-starting the daemon on a cold start), performs the strict handshake, and then runs a
//! bidirectional pump — forwarding the App's outgoing [`ClientMsg`]s to the daemon and turning
//! every incoming control message / grid frame into an app [`Message`]. The App drives sessions by
//! sending through the [`Outbox`] handed to it on connect; it never owns a PTY or the socket itself.
//!
//! Keeping the connection in a subscription (not scattered `Task`s) means exactly one socket, one
//! ordered inbound stream, and one place a disconnect is observed — the single-source-of-truth rule
//! the whole re-architecture rests on.

use crate::features::connection::Msg as ConnectionMsg;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::Subscription;

use micold_core::connect::{connect_at, connect_or_spawn, Connected, Credentials, SPAWN_TIMEOUT};
use micold_core::daemons::{DaemonId, DaemonRegistry, DaemonRuntime, DaemonState, DaemonStates};
use micold_core::endpoint::{self, DialAddress};
use micold_core::kube::settings::KubeSettings;
use micold_core::kube::start::DriftChoice;
use micold_core::protocol::codec::{CodecError, Frame};
use micold_core::protocol::keepalive::{self, Keepalive, KeepaliveAction};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::provision::ai_cli::AiCliTable;
use micold_core::provision::keys::DaemonKeys;
use micold_core::provision::AttemptLedger;
use micold_core::provision::{
    run as ssh_run, CancelToken, Egress, EventSink, Ownership, ProvisionEvent, ProvisionTarget,
    Step, StepBounds,
};
use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::SystemRunner;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::{AiCliSelection, NetworkPosture};
use micold_core::ssh::argv::{current_build_id, DEFAULT_CONNECT_TIMEOUT_SECS};
use micold_core::ssh::argv::{PinnedReach, Reach};
use micold_core::ssh::provision::ProvisionOptions;
use micold_core::ssh::reconnect::Backoff;
use micold_core::ssh::remote_path;
use micold_core::ssh::{ClientPlatform, ReasonContext, SshHost, SshTarget};

use crate::app::Message;

/// The build string this client announces in the handshake (diagnostics only).
const CLIENT_BUILD: &str = concat!("micold-ai-ide/", env!("CARGO_PKG_VERSION"));

/// Where the daemon runs, and what to present to it (feature 027).
///
/// Handed in rather than read here. The shell is the single place that chooses a real settings
/// store (FR-017/FR-018), and this subscription is not the shell — so it takes the *answer*, not
/// the means of finding it. That constraint is checked by
/// `tests/no_concrete_implementations.rs`, and it caught the first version of this file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Placement {
    /// Which placement the user selected.
    pub kind: PlacementKind,
    /// The state directory the sandbox token lives in.
    pub state_dir: std::path::PathBuf,
}

/// What the client presents to a sandboxed daemon.
///
/// The token is **read from the file** the client wrote and the runtime mounted, rather than
/// remembered in memory, so a sandbox started by a previous run of this client is still reachable
/// by this one. A missing token file is not fixed up here: the handshake refuses, which is the
/// honest outcome, and the remedy is to restart the sandbox so a new token is issued.
fn sandbox_credentials(token_path: &Path) -> Credentials {
    let token = micold_core::protocol::auth::Token::read_from(token_path).ok();
    Credentials {
        auth_token: token.map(|t| micold_core::protocol::messages::PresentedToken::new(t.as_str())),
        require_fingerprint_match: false,
    }
}

/// The one `ssh` runner of this process, chosen by the shell (`Capabilities`' sibling: this module
/// names no concrete implementation, `tests/no_concrete_implementations.rs`). A process that never
/// installs one (a test, the showcase) reports an SSH daemon as unreachable instead of spawning.
static SSH_HOST: std::sync::OnceLock<std::sync::Arc<dyn SshHost>> = std::sync::OnceLock::new();

/// Install the runner SSH daemons are provisioned and dialled with (feature 489). The first call
/// wins; later ones are ignored.
pub fn install_ssh_host(host: std::sync::Arc<dyn SshHost>) {
    let _ = SSH_HOST.set(host);
}

/// The runner installed by [`install_ssh_host`], when the process chose one.
pub fn ssh_host() -> Option<std::sync::Arc<dyn SshHost>> {
    SSH_HOST.get().cloned()
}

/// The host's IANA time zone, which a container daemon the bridge starts is told as `TZ`.
static HOST_TIME_ZONE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

/// Install the host's time zone (the shell reads it; this module reads no host files). The first
/// call wins.
pub fn install_time_zone(zone: Option<String>) {
    let _ = HOST_TIME_ZONE.set(zone);
}

/// How many messages may queue in each direction before backpressure. Input is small and rare
/// relative to grid frames; a few hundred slots is ample without unbounded growth.
const CHANNEL_CAPACITY: usize = 256;

/// A cloneable handle the App uses to send [`ClientMsg`]s to the daemon. The channel is **unbounded**
/// so a stamped `SessionInput` is never dropped under backpressure — input is the lossless, ordered
/// log the whole feature rests on (G2); dropping a frame here would be an unrecoverable gap because
/// the [`crate::input::SessionInputStamper`] has already consumed its serial. (Control messages are
/// small and rare, so the unbounded channel cannot grow meaningfully from them.)
///
/// `PartialEq`/`Eq` compare *identity* (a shared `Arc` token) so [`Message`] stays `Eq` — clones of
/// one handle are equal (they feed the same connection); handles from different connections differ.
#[derive(Debug, Clone)]
pub struct Outbox {
    tx: mpsc::UnboundedSender<ClientMsg>,
    /// Identity token: cloned (pointer-shared) with the handle, so `Arc::ptr_eq` distinguishes
    /// connections without needing `Sender` to be `Eq` (it is not).
    id: std::sync::Arc<()>,
}

impl Outbox {
    /// `pub` (not private) so the binary's own tests (`main.rs`'s `update_inner` tests, T100) can
    /// build a real `Outbox` over a manually-created channel and assert what gets sent, without
    /// needing a live connection — the binary is a separate crate from this library, so
    /// `pub(crate)` would not reach it.
    pub fn new(tx: mpsc::UnboundedSender<ClientMsg>) -> Self {
        Self {
            tx,
            id: std::sync::Arc::new(()),
        }
    }

    /// Queue a message to the daemon (never blocks the UI thread; a closed channel is ignored).
    pub fn send(&self, msg: ClientMsg) {
        let _ = self.tx.unbounded_send(msg);
    }
}

impl PartialEq for Outbox {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.id, &other.id)
    }
}
impl Eq for Outbox {}

/// What identifies one daemon's actor: the daemon and the settings its connection is made from.
///
/// The fingerprint is the runtime alone. A rename and an `auto_start` change are not in it, so
/// they restart nothing; a changed container name, port or profile is, so editing a daemon's
/// runtime restarts that daemon's actor and no other (contract: Actor).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActorKey {
    /// The daemon.
    pub id: DaemonId,
    /// Its runtime settings, serialised (the registry's own file form), so any edit shows.
    pub fingerprint: String,
}

/// How an actor reaches its daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dial {
    /// Connect to the host daemon, spawning it on a cold start.
    Host,
    /// Dial a container's published loopback port with this daemon's own token. Never spawns.
    Container {
        /// The port from the daemon's own entry, not the default sandbox port.
        port: u16,
        /// Where this daemon's token lives on the host.
        token_path: PathBuf,
    },
    /// Provision and bridge to a daemon on a remote host with the user's own `ssh` (feature 489).
    /// One try per start while it has never connected: that failure is reported and not retried
    /// until the user starts it again. Once connected, a lost link is retried with a growing delay
    /// until it returns or the user stops the attempts.
    Ssh {
        /// The host and connection settings.
        target: SshTarget,
    },
    /// Provision and bridge to a container daemon over the container's own SSH server, published
    /// on loopback, with the app's own keys (feature 806). A running released-image container is
    /// [`Dial::Container`] instead.
    ContainerSsh {
        /// The port the container's SSH server is published on (the daemon's own entry).
        port: u16,
        /// The runtime the login name is asked of.
        runtime: micold_core::sandbox::runtime::RuntimeKind,
        /// The container's name.
        container_name: String,
        /// The daemon's key directory on the host.
        keys_dir: PathBuf,
        /// Whether the container may connect out: with none, installers go through the relay.
        network: micold_core::sandbox::NetworkPosture,
        /// The image reference the container was created from, which the daemon reports in a
        /// refusal (`MICOLD_IMAGE_REFERENCE`).
        image: String,
        /// Whether the daemon is kept running with nobody connected (`MICOLD_IDLE_STOP=off`).
        survive_logout: bool,
        /// The AI CLIs this daemon offers once installed (FR-010).
        ai_clis: AiCliSelection,
    },
    /// Start a pod in a cluster and bridge to it (feature 686). One try per start, like
    /// [`Dial::Ssh`].
    Kubernetes {
        /// The daemon's name, which its object names derive from.
        name: String,
        /// The daemon's settings.
        settings: KubeDial,
        /// What to do with a pod that runs with other settings: the user's answer to the drift
        /// prompt, `Ask` until there is one. Part of the identity, so an answer restarts the actor.
        drift: DriftDial,
    },
}

/// A [`DriftChoice`] as part of an actor's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriftDial(pub DriftChoice);

impl std::hash::Hash for DriftDial {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(&self.0).hash(state);
    }
}

/// A Kubernetes daemon's settings as part of an actor's identity: `KubeSettings` is not `Hash`, so
/// its serialised form is hashed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KubeDial(pub Box<KubeSettings>);

impl std::hash::Hash for KubeDial {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        serde_json::to_string(&self.0)
            .unwrap_or_default()
            .hash(state);
    }
}

/// Everything one actor needs, and its subscription identity (it is hashed whole). A change to any
/// field restarts that actor only.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActorSpec {
    /// The daemon and runtime fingerprint.
    pub key: ActorKey,
    /// How to reach it.
    pub dial: Dial,
    /// Bumped when the user reconnects, so the actor restarts with the first delay of its backoff
    /// and a daemon is never dialled by two actors at once.
    pub epoch: u32,
    /// The restart continues an established connection, so a failed first dial is retried.
    /// Changes only together with `epoch`.
    pub resumed: bool,
}

/// The key of every daemon this build can run, one per entry. An `Unsupported` runtime has none.
pub fn actor_keys(registry: &DaemonRegistry) -> Vec<ActorKey> {
    registry
        .entries()
        .iter()
        .filter(|e| e.runtime.is_startable())
        .map(|e| ActorKey {
            id: e.id,
            fingerprint: serde_json::to_string(&identity_of(&e.runtime)).unwrap_or_default(),
        })
        .collect()
}

/// The runtime as it counts toward an actor's identity: a container's migration notice is for the
/// row, so clearing it after a start must not restart the actor that just connected.
fn identity_of(runtime: &DaemonRuntime) -> DaemonRuntime {
    match runtime {
        DaemonRuntime::Container(c) => {
            DaemonRuntime::Container(micold_core::daemons::ContainerSettings {
                migration: None,
                ..c.clone()
            })
        }
        other => other.clone(),
    }
}

/// The actors that should be running: one per startable entry the client should be dialling, so a
/// stopped daemon has none and is neither spawned nor reconnected until it is started.
///
/// `state_dir` and `legacy_default` locate each container daemon's token
/// ([`micold_core::daemons::DaemonEntry::token_path`]).
pub fn actor_specs(
    registry: &DaemonRegistry,
    states: &DaemonStates,
    state_dir: &Path,
    legacy_default: Option<DaemonId>,
) -> Vec<ActorSpec> {
    actor_specs_answered(
        registry,
        states,
        state_dir,
        legacy_default,
        &BTreeMap::new(),
    )
}

/// [`actor_specs`] with the answers the user gave to the drift prompts (feature 686): a Kubernetes
/// daemon in `drift` is dialled with that choice, the others with `Ask`.
pub fn actor_specs_answered(
    registry: &DaemonRegistry,
    states: &DaemonStates,
    state_dir: &Path,
    legacy_default: Option<DaemonId>,
    drift: &BTreeMap<DaemonId, DriftChoice>,
) -> Vec<ActorSpec> {
    let keys = actor_keys(registry);
    registry
        .entries()
        .iter()
        .filter(|e| states.should_dial(e.id))
        // An SSH daemon makes one try per start: once unreachable it is not dialled until the user
        // starts it again, which moves it to `Starting` and puts its actor back.
        .filter(|e| {
            !(matches!(
                e.runtime,
                DaemonRuntime::Ssh(_) | DaemonRuntime::Kubernetes(_)
            ) && matches!(states.state(e.id), Some(DaemonState::Unreachable { .. })))
        })
        .filter_map(|entry| {
            let key = keys.iter().find(|k| k.id == entry.id)?.clone();
            let dial = match &entry.runtime {
                DaemonRuntime::Host => Dial::Host,
                DaemonRuntime::Container(c) if states.is_legacy(entry.id) => Dial::Container {
                    port: c.port,
                    token_path: entry.token_path(state_dir, legacy_default)?,
                },
                DaemonRuntime::Container(c) => Dial::ContainerSsh {
                    port: c.port,
                    runtime: c.profile.runtime,
                    container_name: c.container_name.clone(),
                    keys_dir: entry.keys_dir(state_dir, legacy_default)?,
                    network: c.profile.network,
                    image: c.profile.image.reference().to_string(),
                    survive_logout: c.profile.survive_logout,
                    ai_clis: c.profile.ai_clis.clone(),
                },
                DaemonRuntime::Ssh(s) => Dial::Ssh {
                    target: s.target.clone(),
                },
                DaemonRuntime::Kubernetes(k) => Dial::Kubernetes {
                    name: entry.name.as_str().to_string(),
                    settings: KubeDial(Box::new(k.clone())),
                    drift: DriftDial(drift.get(&entry.id).copied().unwrap_or(DriftChoice::Ask)),
                },
                _ => return None,
            };
            Some(ActorSpec {
                key,
                dial,
                epoch: states.epoch(entry.id),
                resumed: states.resumed(entry.id),
            })
        })
        .collect()
}

/// The connection subscriptions: one actor per daemon, each with its own socket, [`Outbox`],
/// failure streak and reconnect backoff, every message tagged with its daemon
/// ([`Message::daemon`]). Add it to the app's subscription set.
///
/// `run_with` rather than `run`: the builder has to be a plain `fn` for its identity to be stable,
/// and `run_with` identifies each subscription by the *data* ([`ActorSpec`]) plus the function
/// pointer. So an edited daemon restarts its own actor only; a rename changes no spec.
pub fn connections(
    registry: &DaemonRegistry,
    states: &DaemonStates,
    state_dir: &Path,
    legacy_default: Option<DaemonId>,
    drift: &BTreeMap<DaemonId, DriftChoice>,
) -> Subscription<Message> {
    Subscription::batch(
        actor_specs_answered(registry, states, state_dir, legacy_default, drift)
            .into_iter()
            .map(|spec| Subscription::run_with(spec, |spec| actor(spec.clone()))),
    )
}

/// How long a local daemon's actor waits after a lost connection before trying again. Short enough
/// that a restarted (or freed) daemon is picked up promptly; long enough not to spin on a hard
/// failure. The connection banner is what the user sees during the gap (FR-027); the reconnect
/// itself is automatic (FR-028 — the fresh `Welcome` catalog is the resync). A remote daemon's
/// actor paces itself with [`Backoff`] instead (feature 489, FR-007).
const RECONNECT_BACKOFF: std::time::Duration = std::time::Duration::from_secs(1);

/// Consecutive failed connects, so a single transient one never reaches the user (feature 028,
/// FR-016, lifecycle contract §4.15).
///
/// The idle stop opened a gap that did not exist while the daemon ran forever: come back to the
/// machine at the thirty-minute mark and the first connect can land on a daemon that is unwinding,
/// or on nothing at all. The client closes that gap itself — the reconnect loop backs off one
/// [`RECONNECT_BACKOFF`] and, on the host placement, `connect_or_spawn` starts a fresh daemon — so
/// a first failure is a fact about timing, not a fact the user can act on.
///
/// What the user still sees during the gap is the reconnecting banner (`Disconnected`), which is
/// true. What they are spared is the error notification, which says the service could not be reached
/// when it is about to be. A second consecutive failure is a different claim — the gap outlived the
/// remedy — and that one is reported.
///
/// One state, not a timestamp: the streak is only ever read between two attempts a backoff apart, so
/// the count *is* the elapsed time in the only unit that matters here.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ConnectFailures {
    consecutive: u32,
}

/// What the attempts of one actor so far have shown. Owned by the reconnect loop rather than by
/// one attempt: the whole point is what the *previous* attempt did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LinkLife {
    /// The streak of failed connects (feature 028).
    failures: ConnectFailures,
    /// A handshake has succeeded since this actor started: a failed attempt now is a lost link
    /// being retried, not a start that failed (feature 489, FR-008).
    established: bool,
    /// The latest attempt reached `Connected`: the next delay starts again from the first.
    connected: bool,
}

impl ConnectFailures {
    /// Record a failed connect; returns whether it should reach the user.
    fn record_failure(&mut self) -> bool {
        self.consecutive += 1;
        self.consecutive > 1
    }

    /// Record a connect that worked, so the *next* isolated gap is absorbed too.
    ///
    /// Without this the counter would measure the client's uptime rather than the daemon's absence:
    /// a window open all day would treat a bad moment in the afternoon as a continuation of one that
    /// morning and report it instantly.
    fn record_success(&mut self) {
        self.consecutive = 0;
    }
}

/// One inbound event of the bidirectional pump, unifying the source streams so a single loop can
/// service all of them without a `select!` (which would need fused, pinned futures).
enum Io {
    /// A message the App wants sent to the daemon.
    Outgoing(ClientMsg),
    /// A frame that arrived from the daemon.
    Incoming(Result<Frame<DaemonMsg>, CodecError>),
    /// The keepalive check timer fired (FR-026).
    Tick,
}

/// Why the pump loop ended — the outer loop uses this to decide between reconnecting and giving up.
enum PumpEnd {
    /// The connection dropped (EOF, decode error, or a half-open link the keepalive reaped). The
    /// outer loop backs off and reconnects.
    Disconnected,
    /// The iced side is gone; stop the whole subscription.
    AppGone,
    /// A one-try bring-up failed and was reported; nothing is retried until the user starts it.
    Ended,
}

fn actor(spec: ActorSpec) -> impl Stream<Item = Message> {
    let id = spec.key.id;
    // iced 0.14 takes an `AsyncFnOnce` here (0.13 took an `FnOnce` returning a future), so the
    // sender's type no longer falls out of inference and has to be named.
    iced::stream::channel(
        CHANNEL_CAPACITY,
        move |mut output: mpsc::Sender<Message>| async move {
            // Resolve the host endpoint once; it does not change over the app's life. A container
            // daemon dials its own port and needs none.
            let endpoint = match spec.dial {
                Dial::Host => match endpoint::resolve() {
                    Ok(e) => Some(e),
                    Err(err) => {
                        let _ = output
                            .send(Message::daemon(
                                id,
                                ConnectionMsg::ConnectFailed(err.to_string()),
                            ))
                            .await;
                        return;
                    }
                },
                Dial::Container { .. }
                | Dial::Ssh { .. }
                | Dial::ContainerSsh { .. }
                | Dial::Kubernetes { .. } => None,
            };

            // Reconnect loop: connect, pump until the link drops, surface it, back off, repeat. A
            // half-open connection is caught by the keepalive inside `pump`, so the client never sits
            // forever presenting stale content as live (FR-026/027).
            let remote = matches!(spec.dial, Dial::Ssh { .. } | Dial::Kubernetes { .. });
            let (spec_ref, endpoint_ref) = (&spec, endpoint.as_ref());
            let mut loop_output = output.clone();
            reconnect_loop(
                id,
                &mut loop_output,
                remote,
                spec.resumed,
                |mut life| {
                    let mut output = output.clone();
                    async move {
                        let end =
                            connect_and_pump(id, spec_ref, endpoint_ref, &mut output, &mut life)
                                .await;
                        (end, life)
                    }
                },
                tokio::time::sleep,
            )
            .await;
        },
    )
}

/// Run `attempt` until the app is gone or a one-try bring-up has failed: after each lost link or
/// failed attempt, tell the app and wait before the next. A remote daemon waits 1, 2, 4, 8, then 15
/// seconds, starting over after a connection that held (feature 489, FR-007); a local daemon waits
/// [`RECONNECT_BACKOFF`] every time. `sleep` is the clock, so a test needs no real delay.
async fn reconnect_loop<A, Fut, S, Wait>(
    id: DaemonId,
    output: &mut mpsc::Sender<Message>,
    remote: bool,
    resumed: bool,
    mut attempt: A,
    sleep: S,
) where
    A: FnMut(LinkLife) -> Fut,
    Fut: std::future::Future<Output = (PumpEnd, LinkLife)>,
    S: Fn(std::time::Duration) -> Wait,
    Wait: std::future::Future<Output = ()>,
{
    let mut life = LinkLife {
        established: resumed,
        ..LinkLife::default()
    };
    let mut backoff = Backoff::new();
    loop {
        life.connected = false;
        let (end, after) = attempt(life).await;
        life = after;
        match end {
            PumpEnd::AppGone | PumpEnd::Ended => return,
            PumpEnd::Disconnected => {
                if life.connected {
                    backoff.reset();
                }
                if output
                    .send(Message::daemon(id, ConnectionMsg::Disconnected))
                    .await
                    .is_err()
                {
                    return;
                }
                let delay = if remote {
                    backoff.next_delay()
                } else {
                    RECONNECT_BACKOFF
                };
                sleep(delay).await;
            }
        }
    }
}

/// Connect (spawning the daemon on a cold start), announce `DaemonConnected`, then pump both
/// directions with a keepalive until the connection ends. A connect failure is surfaced as
/// `DaemonConnectFailed` and reported as a disconnect so the outer loop retries.
async fn connect_and_pump(
    id: DaemonId,
    spec: &ActorSpec,
    endpoint: Option<&endpoint::Endpoint>,
    output: &mut mpsc::Sender<Message>,
    life: &mut LinkLife,
) -> PumpEnd {
    // The relay of a Kubernetes daemon lives as long as its connection.
    let mut held_relay: Option<crate::kube::relay::Relay> = None;
    let attempt = match (&spec.dial, endpoint) {
        // Unchanged: auto-spawn a detached host process and poll until it accepts. Only the host
        // entry ever reaches here; a container daemon is never spawned by the client.
        (Dial::Host, Some(endpoint)) => {
            connect_or_spawn(endpoint, CLIENT_BUILD, SPAWN_TIMEOUT).await
        }
        (Dial::Host, None) => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no host endpoint",
        )),
        (Dial::Ssh { target }, _) => {
            let sink = provisioning_sink(id, output);
            match ssh_attempt(SSH_HOST.get().map(|h| &**h), target, &sink).await {
                Ok(c) => Ok(c),
                // A link that was up and is down is retried; a start that never connected is
                // reported once and ends the actor.
                Err(reason) if life.established => {
                    return report(id, output, ConnectionMsg::ConnectFailed(reason)).await;
                }
                Err(reason) => {
                    return report(id, output, ConnectionMsg::StartFailed(reason))
                        .await
                        .ended();
                }
            }
        }
        (
            Dial::ContainerSsh {
                port,
                runtime,
                container_name,
                keys_dir,
                network,
                image,
                survive_logout,
                ai_clis,
            },
            _,
        ) => {
            let sink = provisioning_sink(id, output);
            let target = ContainerReach {
                port: *port,
                runtime: *runtime,
                container_name,
                keys_dir,
                network: *network,
                image,
                survive_logout: *survive_logout,
                ai_clis,
                time_zone: HOST_TIME_ZONE.get().and_then(|z| z.as_deref()),
            };
            match container_attempt(
                SSH_HOST.get().map(|h| &**h),
                &target,
                &sink,
                !life.established,
            )
            .await
            {
                Ok(c) => Ok(c),
                Err(reason) if life.established => {
                    return report(id, output, ConnectionMsg::ConnectFailed(reason)).await;
                }
                Err(reason) => {
                    return report(id, output, ConnectionMsg::StartFailed(reason))
                        .await
                        .ended();
                }
            }
        }
        (
            Dial::Kubernetes {
                name,
                settings,
                drift,
            },
            _,
        ) => {
            let Some(env) = KUBE_ENV.get() else {
                return report(
                    id,
                    output,
                    ConnectionMsg::StartFailed("this build cannot reach a cluster".to_string()),
                )
                .await
                .ended();
            };
            let sink = provisioning_sink(id, output);
            // A lost link is reconnected to the running pod, after a look at whether it is still
            // there: a drift found then is kept.
            // A daemon that never connected is on its first start, which creates the pod if it
            // is missing: that is the user's Start, not a re-dial.
            let tried = if life.established {
                kube_reconnect(env, name, &settings.0, &sink).await
            } else {
                kube_attempt(env, name, &settings.0, drift.0, &sink)
                    .await
                    .map_err(KubeReconnectError::Attempt)
            };
            match tried {
                Ok((c, relay)) => {
                    held_relay = Some(relay);
                    Ok(c)
                }
                // The pod is gone: the daemon stops with the reason and nothing is recreated until
                // the user starts it (FR-011).
                Err(KubeReconnectError::Gone(reason)) => {
                    return report(id, output, ConnectionMsg::PodGone(reason))
                        .await
                        .ended();
                }
                // The lookup could not tell: keep trying with the back-off.
                Err(KubeReconnectError::Lookup(reason)) => {
                    return report(id, output, ConnectionMsg::ConnectFailed(reason)).await;
                }
                // A first start that finds the pod changed asks what to do; the answer restarts
                // this actor with the choice (a lost link is retried, not asked about).
                Err(KubeReconnectError::Attempt(KubeAttemptError::Drift { fields }))
                    if !life.established =>
                {
                    return report(id, output, ConnectionMsg::KubeDrift(fields))
                        .await
                        .ended();
                }
                Err(KubeReconnectError::Attempt(reason)) if life.established => {
                    return report(id, output, ConnectionMsg::ConnectFailed(reason.to_string()))
                        .await;
                }
                Err(reason) => {
                    let reason = reason.to_string();
                    return report(id, output, ConnectionMsg::StartFailed(reason))
                        .await
                        .ended();
                }
            }
        }
        // The sandbox is brought up by `shell::sandbox`, which the app drives so the user can watch
        // the image being acquired. All this loop does is dial it — and if nothing is listening,
        // say so. It does **not** fall back to a host process: that is FR-035, and this is the one
        // place it would be easiest to lose.
        (Dial::Container { port, token_path }, _) => {
            let address = DialAddress::Loopback { port: *port };
            let credentials = sandbox_credentials(token_path);
            match connect_at(&address, CLIENT_BUILD, &credentials).await {
                Ok(Some(c)) => Ok(c),
                Ok(None) => Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    format!("no sandboxed daemon is listening on {}", address.describe()),
                )),
                Err(e) => Err(e),
            }
        }
    };
    // Dropping the relay ends the pod's forward: it lives until this function returns.
    let _relay_lives_until_return = &held_relay;

    let connected = match attempt {
        Ok(c) => c,
        Err(err) => {
            return report_transient_connect_failure(
                id,
                output,
                err.to_string(),
                &mut life.failures,
            )
            .await;
        }
    };
    // We reached *a* daemon. A refusal below is still a connection, and a connection is what the
    // streak counts — the mismatch banners are its own state and say something specific.
    life.failures.record_success();
    let (conn, welcome) = match connected {
        Connected::Ready(conn, welcome) => (conn, welcome),
        // A contract mismatch is its own recoverable state (US6, FR-021/022): surface both versions
        // so the app can show the "restart service" action, not a generic connect error.
        Connected::Refused(micold_core::protocol::messages::RefusalReason::VersionMismatch {
            client,
            daemon,
            daemon_build,
            ..
        }) => {
            let sent = output
                .send(Message::daemon(
                    id,
                    ConnectionMsg::VersionMismatch {
                        client,
                        daemon,
                        daemon_build,
                    },
                ))
                .await
                .is_ok();
            return if sent {
                PumpEnd::Disconnected
            } else {
                PumpEnd::AppGone
            };
        }
        // Same contract, different package version — most releases don't touch the wire schema, so
        // this is the common shape a `.deb` upgrade takes (US6, FR-022a, BUG-002). Its own
        // recoverable state, distinct from a contract mismatch: nothing is actually incompatible,
        // only stale.
        Connected::Refused(micold_core::protocol::messages::RefusalReason::BuildMismatch {
            client_build,
            daemon_build,
        }) => {
            let sent = output
                .send(Message::daemon(
                    id,
                    ConnectionMsg::BuildMismatch {
                        client_build,
                        daemon_build,
                    },
                ))
                .await
                .is_ok();
            return if sent {
                PumpEnd::Disconnected
            } else {
                PumpEnd::AppGone
            };
        }
        // The one refusal that exists purely for the development loop (FR-024d, R8). It reached the
        // user as `StaleDevImage { client_fingerprint: "…", … }` through the catch-all below, which
        // names the tag only as debug noise and the remedy not at all — and the person seeing it is
        // by definition mid-rebuild, with a daemon that will now misbehave in ways that look like
        // bugs in the code they just wrote. Reason *and* remedy, in the words the fix is spelled in.
        Connected::Refused(micold_core::protocol::messages::RefusalReason::StaleDevImage {
            client_fingerprint,
            daemon_fingerprint,
            image,
        }) => {
            return report_refusal(
                id,
                output,
                stale_dev_image_advice(&image, &daemon_fingerprint, &client_fingerprint),
            )
            .await;
        }
        Connected::Refused(reason) => {
            return report_refusal(
                id,
                output,
                format!("daemon refused the connection: {reason:?}"),
            )
            .await;
        }
    };

    // A daemon on a remote host knows its own paths only; the client keys everything by identities
    // that name the daemon too. The translation is this one seam (feature 489, research R5).
    let remote = matches!(spec.dial, Dial::Ssh { .. } | Dial::Kubernetes { .. });
    let welcome_catalog = if remote {
        remote_path::wrap_catalog(welcome.catalog, id)
    } else {
        welcome.catalog
    };
    // Hand the App a fresh Outbox and the welcome state (the resync on every (re)connect, FR-028).
    let (mut sink, incoming) = conn.split();
    let (tx, rx) = mpsc::unbounded::<ClientMsg>();
    if output
        .send(Message::daemon(
            id,
            ConnectionMsg::Connected {
                outbox: Outbox::new(tx),
                catalog: welcome_catalog,
                settings: welcome.settings,
                plan_usage: welcome.plan_usage,
            },
        ))
        .await
        .is_err()
    {
        return PumpEnd::AppGone;
    }
    life.established = true;
    life.connected = true;

    // Keepalive: poll on a fixed cadence; any inbound frame proves life, and the deadline turns a
    // silent half-open link into an explicit disconnect within the SC-011 budget (FR-026).
    let mut keepalive = Keepalive::new(Instant::now());
    let ticks = iced::futures::stream::unfold((), |()| async {
        tokio::time::sleep(keepalive::CHECK_INTERVAL).await;
        Some((Io::Tick, ()))
    });
    let mut events = iced::futures::stream::select(
        iced::futures::stream::select(rx.map(Io::Outgoing), incoming.map(Io::Incoming)),
        ticks.boxed(),
    );

    while let Some(io) = events.next().await {
        match io {
            Io::Outgoing(msg) => {
                let msg = if remote {
                    remote_path::outgoing(msg, id)
                } else {
                    msg
                };
                if sink.send(Frame::Control(msg)).await.is_err() {
                    return PumpEnd::Disconnected; // socket closed under us
                }
            }
            Io::Incoming(Ok(frame)) => {
                keepalive.on_daemon_frame(Instant::now());
                let msg = match frame {
                    Frame::Control(dm) => {
                        let dm = if remote {
                            remote_path::incoming(dm, id)
                        } else {
                            dm
                        };
                        Message::daemon(id, ConnectionMsg::Event(dm))
                    }
                    Frame::Grid(frame) => Message::daemon(id, ConnectionMsg::GridFrame(frame)),
                };
                if output.send(msg).await.is_err() {
                    return PumpEnd::AppGone;
                }
            }
            Io::Incoming(Err(_)) => return PumpEnd::Disconnected, // decode error / EOF
            Io::Tick => match keepalive.poll(Instant::now()) {
                KeepaliveAction::SendPing => {
                    // Nonce is diagnostic only; the deadline is reset by *any* inbound frame, so we
                    // don't need to correlate the matching Pong.
                    if sink
                        .send(Frame::Control(ClientMsg::Ping { nonce: 0 }))
                        .await
                        .is_err()
                    {
                        return PumpEnd::Disconnected;
                    }
                }
                KeepaliveAction::Expired => return PumpEnd::Disconnected, // half-open (FR-026)
                KeepaliveAction::Idle => {}
            },
        }
    }

    PumpEnd::Disconnected
}

/// What to tell someone whose `:dev` image is behind their working tree (FR-024d, research R8).
///
/// Separated from the pump so the thing FR-024d actually requires — that the tag and the rebuild
/// command both appear — is checkable without a live connection.
fn stale_dev_image_advice(
    image: &str,
    daemon_fingerprint: &str,
    client_fingerprint: &str,
) -> String {
    format!(
        "the sandbox is running `{image}`, built from a different working tree than this client \
         (image {daemon_fingerprint}, client {client_fingerprint}). Rebuild it with \
         `mise run image`, then restart the sandbox."
    )
}

/// A connect that never reached a daemon, reported only once it has outlived one reconnect backoff
/// (feature 028, FR-016, §4.15 — see [`ConnectFailures`]).
///
/// The first one is absorbed: the caller still gets `Disconnected`, so the reconnecting banner goes
/// up and the loop retries a second later, but no error notification is raised for a gap the client
/// is about to close itself.
///
/// Separate from [`report_refusal`], which the refusal paths use, and deliberately so: a
/// refusal means a daemon answered and said no — a version or build mismatch the user has to act on.
/// Absorbing one of those would delay a banner that is already correct, for a condition that will
/// not resolve itself.
async fn report_transient_connect_failure(
    id: DaemonId,
    output: &mut mpsc::Sender<Message>,
    reason: String,
    failures: &mut ConnectFailures,
) -> PumpEnd {
    if !failures.record_failure() {
        // Nothing is reported and nothing is logged from here: `log_line` belongs to the binary,
        // not this module, and the next attempt one backoff later either succeeds — in which case
        // there was nothing to tell — or reports this same reason itself.
        return PumpEnd::Disconnected;
    }
    report_connect_failure(id, output, reason).await
}

/// Report a connect failure to the app and map it to a disconnect (so the outer loop retries). If the
/// app is gone, that surfaces as `AppGone` instead.
async fn report_connect_failure(
    id: DaemonId,
    output: &mut mpsc::Sender<Message>,
    reason: String,
) -> PumpEnd {
    report(id, output, ConnectionMsg::ConnectFailed(reason)).await
}

/// Report a daemon's refusal, as [`report_connect_failure`] does a failure — but as its own
/// message, so the app does not mistake a daemon that said no for one not listening yet (#370).
async fn report_refusal(
    id: DaemonId,
    output: &mut mpsc::Sender<Message>,
    reason: String,
) -> PumpEnd {
    report(id, output, ConnectionMsg::Refused(reason)).await
}

impl PumpEnd {
    /// A reported one-try failure ends the actor, unless the app is already gone.
    fn ended(self) -> PumpEnd {
        match self {
            PumpEnd::AppGone => PumpEnd::AppGone,
            _ => PumpEnd::Ended,
        }
    }
}

/// An event sink that publishes each start step and the copy's progress as daemon state. A full
/// channel drops an event: the next one carries a larger number or the next step.
fn provisioning_sink(id: DaemonId, output: &mpsc::Sender<Message>) -> EventSink {
    let sender = std::sync::Mutex::new(output.clone());
    std::sync::Arc::new(move |event| {
        if let Ok(mut sender) = sender.lock() {
            // A failed install is said once, so it is not dropped for a full channel the way a
            // progress event is: it is retried for a moment first (feature 806, FR-012).
            let keep = matches!(event, ProvisionEvent::Warned(_));
            let mut message = Message::daemon(id, ConnectionMsg::Provisioning(event));
            for _ in 0..if keep { 100 } else { 1 } {
                match sender.try_send(message) {
                    Err(e) if e.is_full() => {
                        message = e.into_inner();
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    _ => break,
                }
            }
        }
    })
}

/// One bring-up of an SSH daemon through the shared provisioning sequence: provision, bridge and
/// handshake, with a failure already in the user's words (FR-008, FR-018). `None` is a process
/// that installed no runner.
/// What an attempt on `target` needs. The connect watchdog: no attempt hangs the client. The
/// user's `ConnectTimeout` replaces the default when their SSH configuration sets one (FR-009).
async fn ssh_options(host: &dyn SshHost, target: &SshTarget) -> ProvisionOptions {
    let platform = ClientPlatform::current();
    let timeout_secs = micold_core::ssh::config_hosts::resolve(host, target, platform)
        .await
        .map_or(DEFAULT_CONNECT_TIMEOUT_SECS, |e| e.timeout_secs());
    ProvisionOptions {
        platform,
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs,
        build_id: current_build_id(),
        client_build: CLIENT_BUILD.to_string(),
    }
}

/// What the mismatched daemon on `target` says of its sessions (feature 489, FR-011). Anything but
/// a clear answer is `Unknown`.
pub async fn remote_offer(target: SshTarget) -> micold_core::ssh::ReplaceOffer {
    let Some(host) = ssh_host() else {
        return micold_core::ssh::ReplaceOffer::Unknown;
    };
    let opts = ssh_options(&*host, &target).await;
    micold_core::ssh::status::fetch_offer(&*host, &target, &opts).await
}

/// End the mismatched daemon on `target` when it has no running session; the caller reconnects,
/// which starts this build's daemon in its place. `Err` says why it was left running.
pub async fn replace_remote(target: SshTarget) -> Result<(), micold_core::ssh::ReplaceOffer> {
    let Some(host) = ssh_host() else {
        return Err(micold_core::ssh::ReplaceOffer::Unknown);
    };
    let opts = ssh_options(&*host, &target).await;
    micold_core::ssh::status::stop_if_idle(&*host, &target, &opts).await
}

async fn ssh_attempt(
    host: Option<&dyn SshHost>,
    target: &SshTarget,
    sink: &EventSink,
) -> Result<Connected, String> {
    let Some(host) = host else {
        return Err("this build cannot run `ssh`".to_string());
    };
    let platform = ClientPlatform::current();
    let opts = ssh_options(host, target).await;
    let timeout_secs = opts.timeout_secs;
    let provisioned = ssh_run(
        host,
        &ProvisionTarget::user_host(
            target,
            std::time::Duration::from_secs(u64::from(timeout_secs)),
        ),
        &opts,
        &CancelToken::new(),
        sink,
    )
    .await;
    provisioned.map(|p| p.connected).map_err(|failure| {
        failure.reason(&ReasonContext {
            host: target.host.clone(),
            port: target.port.unwrap_or(22),
            timeout_secs,
            platform,
        })
    })
}

/// What one container bring-up needs beyond the shared sequence.
struct ContainerReach<'a> {
    port: u16,
    runtime: micold_core::sandbox::runtime::RuntimeKind,
    container_name: &'a str,
    keys_dir: &'a Path,
    network: micold_core::sandbox::NetworkPosture,
    image: &'a str,
    survive_logout: bool,
    ai_clis: &'a AiCliSelection,
    time_zone: Option<&'a str>,
}

/// The environment of a daemon the bridge starts in a container. sshd builds a session's
/// environment afresh, so the container's own `-e` values (set at `create`, which only the daemon
/// the entrypoint starts inherits) ride the remote command: the image's data home, the image
/// reference a refusal names, the host's time zone and the keep-running opt-in.
fn container_env(reach: &ContainerReach<'_>) -> Vec<(String, String)> {
    let mut env = vec![(
        "XDG_DATA_HOME".to_string(),
        micold_core::ssh::argv::CONTAINER_DATA_HOME.to_string(),
    )];
    if !reach.image.is_empty() {
        env.push((
            "MICOLD_IMAGE_REFERENCE".to_string(),
            reach.image.to_string(),
        ));
    }
    if let Some(zone) = reach.time_zone {
        env.push(("TZ".to_string(), zone.to_string()));
    }
    if let Some(value) = micold_core::sandbox::argv::idle_stop_value(reach.survive_logout) {
        env.push((
            micold_core::spawn::IDLE_STOP_ENV.to_string(),
            value.to_string(),
        ));
    }
    env
}

/// The sequence's target for a container: pinned loopback reach with the daemon's own keys, the
/// pinned AI CLIs installed through a registry-only relay, and the container's step bounds.
fn container_target(
    login: String,
    reach: &ContainerReach<'_>,
    keys: &DaemonKeys,
) -> ProvisionTarget {
    let table = AiCliTable::pinned();
    ProvisionTarget {
        reach: Reach::Pinned(PinnedReach {
            host: "127.0.0.1".to_string(),
            port: reach.port,
            user: login,
            key: keys.client_key(),
            known_hosts: keys.known_hosts(),
        }),
        owned: Ownership::AppOwned {
            ai_clis: reach.ai_clis.clone(),
            // A container that may connect out installs directly; one that may not goes through a
            // relay that allows only the package registries.
            egress: match reach.network {
                NetworkPosture::NoOutbound => Egress::Relay {
                    allow: table.allow(),
                },
                NetworkPosture::Outbound => Egress::Direct,
            },
            table,
            state_dir: PathBuf::from(CONTAINER_STATE_DIR),
        },
        env: container_env(reach),
        bounds: StepBounds::container(),
    }
}

/// The state directory inside a container (the basic image's contract).
const CONTAINER_STATE_DIR: &str = "/var/lib/micold-ai-ide";

/// One bring-up of a container daemon through the shared provisioning sequence: wait for the
/// container, ask it its login name, pin its host key, then provision, bridge and handshake.
///
/// `roll_back` is a first start: when it fails, is cancelled or is dropped after the daemon was
/// started, the container whose daemon it started is stopped (FR-020). A re-dial of a link that was
/// up leaves the container alone.
async fn container_attempt(
    host: Option<&dyn SshHost>,
    reach: &ContainerReach<'_>,
    sink: &EventSink,
    roll_back: bool,
) -> Result<Connected, String> {
    let Some(host) = host else {
        return Err("this build cannot run `ssh`".to_string());
    };
    let bounds = StepBounds::container();
    // The wait below is the runtime pulling the image and creating the container, which can take
    // minutes: say so in the row instead of leaving it on a bare "starting" (feature 806).
    sink(ProvisionEvent::Entered(Step::Acquire));
    let login = micold_core::provision::login::wait_for_login(
        host,
        reach.runtime.program(),
        reach.container_name,
        bounds.acquire + bounds.create_container,
    )
    .await
    .map_err(|reason| {
        format!(
            "the container {} is not ready: {reason}",
            reach.container_name
        )
    })?;
    let platform = ClientPlatform::current();
    let keys = DaemonKeys::ensure(reach.keys_dir, platform, host)
        .await
        .map_err(|f| {
            f.reason(&ReasonContext {
                host: "127.0.0.1".to_string(),
                port: reach.port,
                timeout_secs: DEFAULT_CONNECT_TIMEOUT_SECS,
                platform,
            })
        })?;
    keys.write_known_hosts(reach.port)
        .map_err(|e| format!("could not pin the container's host key: {e}"))?;
    let timeout_secs = DEFAULT_CONNECT_TIMEOUT_SECS;
    let opts = ProvisionOptions {
        platform,
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs,
        build_id: current_build_id(),
        client_build: CLIENT_BUILD.to_string(),
    };
    // Dropping this future is the cancel (the actor is aborted); the token is for the steps' own
    // guards and is never shared.
    let cancel = CancelToken::new();
    let (runtime, name) = (reach.runtime, reach.container_name.to_string());
    // A daemon found running was not started by this attempt: it is never stopped (FR-020).
    let roll_back = roll_back
        && !micold_core::provision::login::daemon_running(host, runtime.program(), &name).await;
    let mut undo = StartRollback::new(roll_back.then(|| {
        Box::new(move |ledger: AttemptLedger| {
            let _ =
                micold_core::provision::rollback(&ledger, &CliRuntime::new(runtime, SystemRunner));
        }) as Box<dyn FnOnce(AttemptLedger) + Send>
    }));
    let recording = undo.recording_sink(sink, &name);
    let result = ssh_run(
        host,
        &container_target(login, reach, &keys),
        &opts,
        &cancel,
        &recording,
    )
    .await;
    match result {
        Ok(p) => {
            undo.disarm();
            Ok(p.connected)
        }
        Err(failure) => Err(failure.reason(&ReasonContext {
            host: "127.0.0.1".to_string(),
            port: reach.port,
            timeout_secs,
            platform,
        })),
    }
}

/// Stops the container whose daemon an attempt started when the attempt ends any way but
/// connected, including when its future is dropped by a cancel (feature 806, FR-019, FR-020).
struct StartRollback {
    ledger: std::sync::Arc<std::sync::Mutex<AttemptLedger>>,
    undo: Option<Box<dyn FnOnce(AttemptLedger) + Send>>,
}

impl StartRollback {
    /// `None` for an attempt that must leave the container alone.
    fn new(undo: Option<Box<dyn FnOnce(AttemptLedger) + Send>>) -> Self {
        Self {
            ledger: std::sync::Arc::default(),
            undo,
        }
    }

    /// `sink`, also noting in the ledger that the daemon of container `name` is being started.
    fn recording_sink(&self, sink: &EventSink, name: &str) -> EventSink {
        let (ledger, inner, name) = (self.ledger.clone(), sink.clone(), name.to_string());
        std::sync::Arc::new(move |event| {
            if matches!(event, ProvisionEvent::Entered(Step::Start)) {
                ledger
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .record_daemon_started(micold_core::sandbox::runtime::ContainerId(
                        name.clone(),
                    ));
            }
            inner(event);
        })
    }

    /// The attempt connected: nothing to undo.
    fn disarm(&mut self) {
        self.undo = None;
    }
}

impl Drop for StartRollback {
    fn drop(&mut self) {
        let Some(undo) = self.undo.take() else { return };
        let ledger = self
            .ledger
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if !ledger.is_empty() {
            // A runtime command blocks: keep it off the async worker that dropped the attempt.
            std::thread::spawn(move || undo(ledger));
        }
    }
}

/// What a Kubernetes daemon's start needs from the shell (this module names no concrete
/// implementation, `tests/no_concrete_implementations.rs`).
pub struct KubeEnv {
    /// Connects to the cluster of a kubeconfig context.
    pub connect: crate::kube::KubeConnector,
    /// Makes the pod's host key and this client's key.
    pub keygen: std::sync::Arc<dyn micold_core::kube::access::KeyGen>,
    /// Runs `ssh` and `scp` toward the relay.
    pub host: std::sync::Arc<dyn SshHost>,
    /// The directory holding every Kubernetes daemon's key directory.
    pub keys_root: PathBuf,
    /// This machine's home directory, where the allowed credentials' source files are.
    pub home: PathBuf,
    /// The cancel token of each Kubernetes daemon's start under way, by daemon name.
    pub cancels: std::sync::Mutex<BTreeMap<String, CancelToken>>,
}

impl KubeEnv {
    /// An environment with no start under way.
    pub fn new(
        connect: crate::kube::KubeConnector,
        keygen: std::sync::Arc<dyn micold_core::kube::access::KeyGen>,
        host: std::sync::Arc<dyn SshHost>,
        keys_root: PathBuf,
        home: PathBuf,
    ) -> Self {
        Self {
            connect,
            keygen,
            host,
            keys_root,
            home,
            cancels: std::sync::Mutex::default(),
        }
    }
}

/// Stop the start of the Kubernetes daemon `name` that is under way: it ends cancelled and is
/// rolled back. Whether there was a start to stop.
pub fn cancel_kube_start(env: &KubeEnv, name: &str) -> bool {
    let cancels = env.cancels.lock().unwrap_or_else(|e| e.into_inner());
    cancels.get(name).map(CancelToken::cancel).is_some()
}

/// Stop the start of the Kubernetes daemon `name` on the environment the shell installed.
pub fn cancel_installed_kube_start(name: &str) -> bool {
    KUBE_ENV
        .get()
        .is_some_and(|env| cancel_kube_start(env, name))
}

/// Removes a start's cancel token when the attempt ends, however it ends.
struct Registered<'a> {
    env: &'a KubeEnv,
    name: &'a str,
}

impl<'a> Registered<'a> {
    fn new(env: &'a KubeEnv, name: &'a str, token: &CancelToken) -> Self {
        env.cancels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.to_string(), token.clone());
        Self { env, name }
    }
}

impl Drop for Registered<'_> {
    fn drop(&mut self) {
        self.env
            .cancels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(self.name);
    }
}

/// Why one bring-up of a Kubernetes daemon did not connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KubeAttemptError {
    /// The pod runs with other settings than the daemon's, and the start was not told what to do
    /// about it ([`DriftChoice::Ask`]): the fields that differ. The attempt is not rolled back.
    Drift {
        /// The settings that differ, in the user's words.
        fields: Vec<String>,
    },
    /// The start failed, with the reason in the user's words; it is rolled back.
    Failed(String),
}

impl From<String> for KubeAttemptError {
    fn from(reason: String) -> Self {
        Self::Failed(reason)
    }
}

impl From<&str> for KubeAttemptError {
    fn from(reason: &str) -> Self {
        Self::Failed(reason.to_string())
    }
}

impl std::fmt::Display for KubeAttemptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Drift { fields } => write!(
                f,
                "the pod runs with other settings ({}): recreate it or keep it",
                fields.join(", ")
            ),
            Self::Failed(reason) => f.write_str(reason),
        }
    }
}

static KUBE_ENV: std::sync::OnceLock<KubeEnv> = std::sync::OnceLock::new();

/// Install what Kubernetes daemons are started with. The first call wins.
pub fn install_kube(env: KubeEnv) {
    let _ = KUBE_ENV.set(env);
}

/// Why a re-dial of a Kubernetes daemon did not connect (feature 686, FR-011).
#[derive(Debug)]
pub enum KubeReconnectError {
    /// The pod is gone (deleted, evicted, node lost); the text is the reason the row shows.
    Gone(String),
    /// Looking the pod up failed (a 5xx, a timeout): neither gone nor reachable.
    Lookup(String),
    /// The pod is there but the bring-up failed.
    Attempt(KubeAttemptError),
}

impl std::fmt::Display for KubeReconnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gone(reason) | Self::Lookup(reason) => f.write_str(reason),
            Self::Attempt(e) => e.fmt(f),
        }
    }
}

/// Re-dial a Kubernetes daemon whose link was lost, or that the user reconnects: look the pod up
/// first, and when it is gone report why instead of creating one. A pod that runs is adopted, so
/// no second pod and no duplicate session appear; a drift is kept.
pub async fn kube_reconnect(
    env: &KubeEnv,
    name: &str,
    settings: &KubeSettings,
    sink: &EventSink,
) -> Result<(Connected, crate::kube::relay::Relay), KubeReconnectError> {
    use micold_core::kube::classify::PodGone;
    let (context, namespace) = (&settings.context.0, &settings.namespace.0);
    let names = micold_core::kube::names::ResourceNames::new(context, namespace, name);
    let lookup = |e: String| KubeReconnectError::Lookup(format!("cannot look up the pod: {e}"));
    let cluster = (env.connect)(context.clone())
        .await
        .map_err(|f| lookup(f.to_string()))?;
    let status = cluster
        .pod_status(namespace, &names.pod())
        .await
        .map_err(|f| lookup(f.message))?;
    // The cluster port has no node lookup: a lost node shows as the pod's phase `Unknown`.
    if let Some(gone) = PodGone::classify(status.as_ref(), true) {
        return Err(KubeReconnectError::Gone(gone.to_string()));
    }
    kube_attempt(env, name, settings, DriftChoice::Keep, sink)
        .await
        .map_err(KubeReconnectError::Attempt)
}

/// The account the pod's SSH server logs this client in as: the basic image's passwd entry for
/// the pod's uid (`packaging/basic/Containerfile`). sshd refuses any other name.
pub const POD_SSH_USER: &str = "micold";

/// One bring-up of a Kubernetes daemon: the start sequence up to a running pod, the relay to its
/// SSH port with its host key pinned, then the shared copy, install and connect sequence over it.
/// The relay returned must outlive the connection. A failure is the reason in the user's words and
/// the attempt is already rolled back; a drift is [`KubeAttemptError::Drift`] unless `drift` says
/// what to do about it. [`cancel_kube_start`] stops the attempt while it runs.
pub async fn kube_attempt(
    env: &KubeEnv,
    name: &str,
    settings: &KubeSettings,
    drift: DriftChoice,
    sink: &EventSink,
) -> Result<(Connected, crate::kube::relay::Relay), KubeAttemptError> {
    use micold_core::kube::start::{
        connect_pod, pod_target, start, ConnectOutcome, ConnectRequest, StartOutcome, StartPhase,
        StartRequest,
    };
    let fail =
        |f: micold_core::kube::classify::KubeFailure| KubeAttemptError::Failed(f.to_string());
    let image = settings.pod_image();
    let (context, namespace) = (&settings.context.0, &settings.namespace.0);
    let names = micold_core::kube::names::ResourceNames::new(context, namespace, name);
    let keys = crate::kube::keys::KubeKeys::new(&env.keys_root, names.key().as_str());
    let client = keys
        .load_or_create(&*env.keygen, || uuid::Uuid::new_v4().to_string())
        .map_err(|e| format!("cannot write the SSH key files: {e}"))?;
    if client.public_key.is_empty() {
        let _ = keys.delete();
        return Err("cannot make an SSH key: is `ssh-keygen` installed?".into());
    }
    let cancel = CancelToken::new();
    let _registered = Registered::new(env, name, &cancel);
    let cluster = (env.connect)(context.clone()).await.map_err(fail)?;
    let read = |path: &Path| std::fs::read_to_string(path);
    let request = StartRequest {
        cluster: &*cluster,
        keys: &*env.keygen,
        server: "",
        settings,
        names: &names,
        image: &image,
        client: &client,
        drift,
        home: &env.home,
        hosts: &[],
        read: &read,
    };
    let started = match start(&request, &mut |_| {}).await {
        StartOutcome::Ready(started) => started,
        StartOutcome::Drift { fields } => {
            return Err(KubeAttemptError::Drift {
                fields: fields.iter().map(|f| f.to_string()).collect(),
            })
        }
        StartOutcome::Failed(failure) => return Err(fail(failure)),
    };
    let relay = crate::kube::relay::Relay::open(
        cluster.clone(),
        namespace.clone(),
        started.pod.clone(),
        micold_core::kube::manifest::SSH_PORT,
    )
    .await
    .map_err(|e| format!("cannot open the local relay to the pod: {e}"))?;
    let pinned = async {
        let secret = cluster
            .get(
                namespace,
                micold_core::kube::ObjectKind::Secret,
                &names.access_secret(),
            )
            .await
            .map_err(|e| format!("cannot read the pod's host key: {e:?}"))?
            .and_then(|view| micold_core::kube::access::AccessSecret::parse(&view.data))
            .ok_or("the pod's access secret has no host key")?;
        keys.write_known_hosts(relay.addr().port(), &secret.host_key.public_line)
            .map_err(|e| format!("cannot pin the pod's host key: {e}"))
    }
    .await;
    if let Err(reason) = pinned {
        // The pod is rolled back like any failed connect.
        micold_core::kube::start::rollback(&request, &started.attempt).await;
        return Err(reason.into());
    }
    let target = pod_target(
        micold_core::ssh::argv::PinnedReach {
            host: "127.0.0.1".to_string(),
            port: relay.addr().port(),
            user: POD_SSH_USER.to_string(),
            key: keys.private_key(),
            known_hosts: keys.known_hosts(),
        },
        &settings.providers,
    );
    let opts = ProvisionOptions {
        platform: ClientPlatform::current(),
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs: DEFAULT_CONNECT_TIMEOUT_SECS,
        build_id: current_build_id(),
        client_build: CLIENT_BUILD.to_string(),
    };
    let connect = ConnectRequest {
        host: &*env.host,
        target: &target,
        opts: &opts,
        cancel: &cancel,
    };
    let forward = |event: &micold_core::provision::ProvisionEvent| sink(event.clone());
    // The row's activity "Installing <cli>" is the log's first line of the install.
    let clis = settings
        .providers
        .iter()
        .map(|cli| cli.provider().display_name())
        .collect::<Vec<_>>()
        .join(", ");
    let mut on_phase = |phase: StartPhase| {
        if phase == StartPhase::Install {
            sink(micold_core::provision::ProvisionEvent::InstallLine(
                format!("Installing {clis}"),
            ));
        }
    };
    match connect_pod(&request, &started, &connect, &mut on_phase, &mut {
        forward
    })
    .await
    {
        ConnectOutcome::Ready(done) => Ok((done.connected, relay)),
        ConnectOutcome::Failed(failure) => Err(fail(failure)),
    }
}

async fn report(id: DaemonId, output: &mut mpsc::Sender<Message>, msg: ConnectionMsg) -> PumpEnd {
    if output.send(Message::daemon(id, msg)).await.is_err() {
        PumpEnd::AppGone
    } else {
        PumpEnd::Disconnected
    }
}

#[cfg(test)]
mod tests {
    use super::{
        actor_specs, reconnect_loop, ssh_attempt, stale_dev_image_advice, ConnectFailures, Dial,
        LinkLife, PumpEnd, RECONNECT_BACKOFF,
    };
    use iced::futures::channel::mpsc;
    use micold_core::daemons::{
        DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime, DaemonState, DaemonStates,
    };
    use micold_core::ssh::{
        Artefacts, SshCommand, SshOutput, SshRunner, SshSettings, SshStdio, SshTarget,
    };
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn the_pod_login_is_the_basic_images_entry_for_the_pods_uid() {
        let containerfile = include_str!("../../../packaging/basic/Containerfile");
        let entry = format!(
            "{}:x:{}:",
            super::POD_SSH_USER,
            micold_core::kube::manifest::IMAGE_USER_UID
        );
        assert!(
            containerfile.contains(&entry),
            "the basic image has no passwd entry {entry}..."
        );
    }

    /// A runner standing in for `ssh`: it cannot be started, as on a machine with no OpenSSH.
    struct NoSsh;

    impl SshRunner for NoSsh {
        fn run(
            &self,
            _: SshCommand,
            _: Vec<u8>,
            _: Option<std::time::Duration>,
            _: Option<micold_core::ssh::ProvisionProgress>,
        ) -> iced::futures::future::BoxFuture<'_, std::io::Result<SshOutput>> {
            Box::pin(async { Err(std::io::ErrorKind::NotFound.into()) })
        }

        fn open(&self, _: SshCommand) -> std::io::Result<SshStdio> {
            Err(std::io::ErrorKind::NotFound.into())
        }
    }

    impl Artefacts for NoSsh {
        fn own(&self) -> std::io::Result<Vec<u8>> {
            Ok(Vec::new())
        }
        fn bundled(&self, _: &str, _: &str) -> Option<Vec<u8>> {
            None
        }
    }

    fn ssh_registry() -> (DaemonRegistry, micold_core::daemons::DaemonId) {
        let mut registry = DaemonRegistry::default();
        let target = SshTarget::new("build-box", Some("me"), None, None).unwrap();
        let id = registry
            .add(
                "Build box",
                DaemonRuntime::Ssh(SshSettings { target }),
                false,
            )
            .unwrap();
        (registry, id)
    }

    /// T133: a started Kubernetes daemon is dialled with `Dial::Kubernetes`; once unreachable it
    /// is not, until started again.
    #[test]
    fn a_started_kubernetes_daemon_gets_a_kubernetes_spec() {
        let mut registry = DaemonRegistry::default();
        let settings = micold_core::kube::settings::KubeSettings::new("kind-dev", "dev", &[]);
        let id = registry
            .add("Cluster", DaemonRuntime::Kubernetes(settings), false)
            .unwrap();
        let dir = std::path::Path::new("/nonexistent");
        let mut states = DaemonStates::default();
        states.insert(id, DaemonState::Starting);
        let specs = actor_specs(&registry, &states, dir, None);
        assert!(matches!(
            specs.as_slice(),
            [s] if matches!(&s.dial, Dial::Kubernetes { name, settings, .. }
                if name == "Cluster" && settings.0.context.0 == "kind-dev" && settings.0.namespace.0 == "dev")
        ));
        states.insert(id, DaemonState::Unreachable { reason: "x".into() });
        assert!(actor_specs(&registry, &states, dir, None).is_empty());
    }

    /// The answer to the drift prompt is part of the actor's identity, so giving it restarts the
    /// daemon's actor with the choice (feature 686, contract §3).
    #[test]
    fn a_drift_answer_changes_the_kubernetes_actor_and_defaults_to_asking() {
        use micold_core::kube::start::DriftChoice;
        let mut registry = DaemonRegistry::default();
        let settings = micold_core::kube::settings::KubeSettings::new("kind-dev", "dev", &[]);
        let id = registry
            .add("Cluster", DaemonRuntime::Kubernetes(settings), false)
            .unwrap();
        let mut states = DaemonStates::default();
        states.insert(id, DaemonState::Starting);
        let dir = std::path::Path::new("/nonexistent");
        let asked = actor_specs(&registry, &states, dir, None);
        let answer = std::collections::BTreeMap::from([(id, DriftChoice::Recreate)]);
        let answered = super::actor_specs_answered(&registry, &states, dir, None, &answer);
        assert_ne!(asked[0], answered[0]);
        assert!(
            matches!(&asked[0].dial, Dial::Kubernetes { drift, .. } if drift.0 == DriftChoice::Ask)
        );
        assert!(
            matches!(&answered[0].dial, Dial::Kubernetes { drift, .. } if drift.0 == DriftChoice::Recreate)
        );
    }

    /// T011: a started SSH daemon is dialled with `Dial::Ssh`; once it is unreachable it is not,
    /// until started again.
    #[test]
    fn an_ssh_daemon_is_dialled_when_starting_and_not_when_unreachable() {
        let (registry, id) = ssh_registry();
        let dir = std::path::Path::new("/nonexistent");
        let mut states = DaemonStates::default();
        states.insert(id, DaemonState::Starting);
        let specs = actor_specs(&registry, &states, dir, None);
        assert!(
            matches!(specs.as_slice(), [s] if matches!(&s.dial, Dial::Ssh { target } if target.host == "build-box"))
        );

        states.insert(
            id,
            DaemonState::Unreachable {
                reason: "no route".into(),
            },
        );
        assert!(actor_specs(&registry, &states, dir, None).is_empty());
    }

    use super::{container_target, AttemptLedger, ContainerReach, EventSink, StartRollback};
    use micold_core::provision::keys::DaemonKeys;
    use micold_core::provision::{Egress, Ownership, StepBounds};
    use micold_core::ssh::argv::Reach;
    use std::path::PathBuf;

    fn container_registry() -> (DaemonRegistry, DaemonId) {
        let mut registry = DaemonRegistry::default();
        let id = registry
            .add(
                "Box",
                DaemonRuntime::Container(micold_core::daemons::ContainerSettings {
                    container_name: "box-ctr".into(),
                    port: 7001,
                    profile: Default::default(),
                    migration: None,
                }),
                false,
            )
            .unwrap();
        (registry, id)
    }

    /// T053: a container daemon is dialled over its SSH server with its own keys directory; a
    /// running released-image container (legacy) keeps the token channel (FR-017).
    #[test]
    fn a_container_is_dialled_over_ssh_and_a_legacy_one_with_its_token() {
        let (registry, id) = container_registry();
        let base = std::path::Path::new("/state");
        let mut states = DaemonStates::default();
        states.insert(id, DaemonState::Starting);
        let specs = actor_specs(&registry, &states, base, None);
        assert!(
            matches!(specs.as_slice(), [s] if matches!(&s.dial,
                Dial::ContainerSsh { port: 7001, container_name, keys_dir, .. }
                    if container_name == "box-ctr"
                        && keys_dir == &base.join("daemons").join("box-ctr").join("ssh"))),
            "{specs:?}"
        );

        states.set_legacy(id, true);
        let specs = actor_specs(&registry, &states, base, None);
        assert!(
            matches!(specs.as_slice(), [s] if matches!(&s.dial, Dial::Container { port: 7001, .. })),
            "{specs:?}"
        );
    }

    /// T053: the container's target is pinned loopback reach with the daemon's own keys, installs
    /// through a registry-only relay and uses the container's bounds.
    #[test]
    fn the_container_target_is_pinned_app_owned_and_relayed() {
        let keys = DaemonKeys {
            dir: PathBuf::from("/state/ssh"),
        };
        let reach = ContainerReach {
            port: 7001,
            runtime: Default::default(),
            container_name: "box-ctr",
            keys_dir: &keys.dir,
            network: Default::default(),
            image: "ghcr.io/o/img:1",
            survive_logout: true,
            ai_clis: &micold_core::sandbox::AiCliSelection::new_daemon(),
            time_zone: Some("Europe/Berlin"),
        };
        let target = container_target("node".into(), &reach, &keys);
        let Reach::Pinned(pinned) = &target.reach else {
            panic!("a container is reached pinned: {target:?}");
        };
        assert_eq!(
            (pinned.host.as_str(), pinned.port, pinned.user.as_str()),
            ("127.0.0.1", 7001, "node")
        );
        assert_eq!(pinned.key, keys.client_key());
        assert_eq!(pinned.known_hosts, keys.known_hosts());
        assert!(matches!(
            &target.owned,
            Ownership::AppOwned { egress: Egress::Relay { allow }, .. } if !allow.is_empty()
        ));
        assert_eq!(target.bounds, StepBounds::container());
        assert!(
            matches!(&target.owned, Ownership::AppOwned { ai_clis, .. } if *ai_clis == micold_core::sandbox::AiCliSelection::new_daemon()),
            "the stored selection, not every CLI, is installed and offered: {:?}",
            target.owned
        );
        // sshd builds a session's environment afresh, so the daemon is told the image's data home,
        // where provisioning wrote the catalogue and the AI CLIs.
        // The container's other `-e` values ride along too, so this daemon matches the entrypoint's.
        let get = |k: &str| {
            target
                .env
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("XDG_DATA_HOME"), Some("/var/lib"));
        assert_eq!(get("MICOLD_IMAGE_REFERENCE"), Some("ghcr.io/o/img:1"));
        assert_eq!(get("MICOLD_IDLE_STOP"), Some("off"));
        assert_eq!(get("TZ"), Some("Europe/Berlin"));
        let quiet_reach = ContainerReach {
            survive_logout: false,
            image: "",
            time_zone: None,
            ..reach
        };
        let plain = container_target("node".into(), &quiet_reach, &keys).env;
        assert!(plain
            .iter()
            .all(|(n, _)| n != "MICOLD_IDLE_STOP" && n != "MICOLD_IMAGE_REFERENCE" && n != "TZ"));
        let open = ContainerReach {
            network: micold_core::sandbox::NetworkPosture::Outbound,
            ..reach
        };
        assert!(matches!(
            container_target("node".into(), &open, &keys).owned,
            Ownership::AppOwned {
                egress: Egress::Direct,
                ..
            }
        ));
    }

    /// Clearing a container's migration notice after its start must not change the actor's identity.
    #[test]
    fn clearing_the_migration_notice_keeps_the_actors_identity() {
        use micold_core::daemons::{DaemonRuntime, MigrationNotice};
        let runtime = |migration| {
            let mut c = match micold_core::daemons::DaemonRuntime::Container(
                serde_json::from_str(r#"{"container_name":"c","port":7001}"#).unwrap(),
            ) {
                DaemonRuntime::Container(c) => c,
                _ => unreachable!(),
            };
            c.migration = migration;
            DaemonRuntime::Container(c)
        };
        assert_eq!(
            serde_json::to_string(&super::identity_of(&runtime(Some(
                MigrationNotice::MovedOffReleasedImage
            ))))
            .unwrap(),
            serde_json::to_string(&super::identity_of(&runtime(None))).unwrap()
        );
    }

    fn quiet() -> micold_core::provision::EventSink {
        std::sync::Arc::new(|_| {})
    }

    /// The row says "getting the image" while the actor waits for the container, which is the long
    /// part of a first start (feature 806, T045).
    #[tokio::test]
    async fn a_container_start_announces_getting_the_image_before_it_waits() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = events.clone();
        let sink: micold_core::provision::EventSink =
            std::sync::Arc::new(move |e| seen.lock().unwrap().push(e));
        let dir = tempfile::tempdir().unwrap();
        let reach = ContainerReach {
            port: 7001,
            runtime: micold_core::sandbox::runtime::RuntimeKind::Docker,
            container_name: "box-ctr",
            keys_dir: dir.path(),
            network: micold_core::sandbox::NetworkPosture::Outbound,
            image: "img",
            survive_logout: false,
            ai_clis: &micold_core::sandbox::AiCliSelection::new_daemon(),
            time_zone: None,
        };
        // `NoSsh` never finds the container, so the attempt is still waiting when the timeout ends it.
        let waited = tokio::time::timeout(
            std::time::Duration::from_millis(300),
            super::container_attempt(Some(&NoSsh), &reach, &sink, true),
        )
        .await;
        assert!(waited.is_err(), "the container never answers");
        assert_eq!(
            events.lock().unwrap().first(),
            Some(&micold_core::provision::ProvisionEvent::Entered(
                micold_core::provision::Step::Acquire
            ))
        );
    }

    /// T011: a bring-up that cannot start `ssh` is a failure in words, with no retry loop behind
    /// it (the actor ends on it).
    #[tokio::test]
    async fn a_missing_ssh_is_reported_in_words_not_retried() {
        let target = SshTarget::new("build-box", None, None, None).unwrap();
        let err = match ssh_attempt(Some(&NoSsh), &target, &quiet()).await {
            Ok(_) => panic!("no ssh cannot connect"),
            Err(e) => e,
        };
        assert!(err.to_lowercase().contains("ssh"), "{err}");
        let none = ssh_attempt(None, &target, &quiet())
            .await
            .err()
            .expect("no runner");
        assert!(none.contains("ssh"), "{none}");
    }

    /// S-6 against the real retry rather than a copy of it: an unattended bring-up after a failed
    /// one must not arrive on the next connection tick, or the whole budget is spent in the seconds
    /// it takes the reconnect loop to come round a few times. The first may go at once — the
    /// service is missing *now*.
    #[test]
    fn unattended_bring_ups_after_the_first_wait_longer_than_a_reconnect() {
        let delays = micold_core::sandbox::lifecycle::UNATTENDED_BRING_UP_DELAYS;
        assert!(
            delays.len() >= 2,
            "spacing needs at least two attempts to be between: {delays:?}"
        );
        for (i, delay) in delays.iter().enumerate().skip(1) {
            assert!(
                *delay > RECONNECT_BACKOFF,
                "attempt {} waits {delay:?}, no longer than the {RECONNECT_BACKOFF:?} reconnect (S-6)",
                i + 1
            );
        }
    }

    /// §4.15 (feature 028) — one transient connect failure must not reach the user.
    ///
    /// The idle stop creates a gap that did not exist before: come back to the machine at the
    /// thirty-minute mark and the client's next connect can land on a daemon that is unwinding, or
    /// on nothing at all. The client closes that gap itself within one `RECONNECT_BACKOFF`, so
    /// putting a banner up for it would be telling the user about a problem that is over before
    /// they finish reading it — and a banner that cries wolf is a banner ignored when the daemon
    /// really is gone.
    #[test]
    fn a_single_transient_connect_failure_does_not_raise_the_banner() {
        let mut failures = ConnectFailures::default();
        assert!(
            !failures.record_failure(),
            "the first failure is absorbed by the reconnect backoff"
        );
    }

    /// …and a *second* consecutive failure does raise it. The rule is "absorb one", not "stay
    /// quiet": a daemon that is genuinely gone must still reach the user, one backoff later.
    #[test]
    fn a_second_consecutive_failure_raises_the_banner() {
        let mut failures = ConnectFailures::default();
        assert!(!failures.record_failure());
        assert!(
            failures.record_failure(),
            "a failure that persists past one backoff is not transient"
        );
        assert!(
            failures.record_failure(),
            "and it keeps being reported for as long as it lasts"
        );
    }

    /// A success resets the streak, so the *next* isolated gap is absorbed too.
    ///
    /// Without this, a long-running client that had two bad moments hours apart would treat the
    /// second one as a continuation of the first and raise the banner instantly — the counter would
    /// be measuring the client's uptime rather than the daemon's absence.
    #[test]
    fn a_successful_connect_resets_the_streak() {
        let mut failures = ConnectFailures::default();
        assert!(!failures.record_failure());
        failures.record_success();
        assert!(
            !failures.record_failure(),
            "an isolated gap after a healthy connection is still an isolated gap"
        );
    }

    /// FR-024d asks for the tag **and** the rebuild command. Before this, the refusal reached the
    /// user as a `{:?}` dump of `StaleDevImage`, which carried the tag as debug noise and no remedy
    /// at all — and its whole audience is someone mid-rebuild, about to read a stale daemon's
    /// behaviour as a bug in the code they just wrote.
    #[test]
    fn the_stale_image_advice_names_the_tag_and_the_rebuild_command() {
        let advice = stale_dev_image_advice("micold-daemon:dev", "aaaa1111", "bbbb2222");
        assert!(advice.contains("micold-daemon:dev"), "{advice}");
        assert!(advice.contains("mise run image"), "{advice}");
        // Both fingerprints, so "which side is stale" is answerable from the message alone.
        assert!(
            advice.contains("aaaa1111") && advice.contains("bbbb2222"),
            "{advice}"
        );
    }

    /// One scripted attempt of the stand-in transport.
    #[derive(Clone, Copy)]
    enum Step {
        /// Connects, then the link drops.
        Held,
        /// Fails to connect.
        Failed,
        /// A one-try bring-up that failed and ended the actor.
        Ended,
    }

    /// Run the reconnect loop over `steps` (then the app goes away) with a clock that does not
    /// wait. Returns the delays it asked for, the attempts made and the disconnects it reported.
    async fn run_script(remote: bool, steps: &[Step]) -> (Vec<u64>, usize, usize) {
        let script = Arc::new(Mutex::new(
            steps
                .iter()
                .copied()
                .collect::<std::collections::VecDeque<_>>(),
        ));
        let delays = Arc::new(Mutex::new(Vec::new()));
        let attempts = Arc::new(Mutex::new(0usize));
        let (mut tx, mut rx) = mpsc::channel(64);
        reconnect_loop(
            DaemonId(1),
            &mut tx,
            remote,
            false,
            |mut life: LinkLife| {
                let (script, attempts) = (script.clone(), attempts.clone());
                async move {
                    *attempts.lock().unwrap() += 1;
                    let step = script.lock().unwrap().pop_front();
                    match step {
                        Some(Step::Held) => {
                            life.established = true;
                            life.connected = true;
                            (PumpEnd::Disconnected, life)
                        }
                        Some(Step::Failed) => (PumpEnd::Disconnected, life),
                        Some(Step::Ended) => (PumpEnd::Ended, life),
                        None => (PumpEnd::AppGone, life),
                    }
                }
            },
            |delay: Duration| {
                delays.lock().unwrap().push(delay.as_secs());
                async {}
            },
        )
        .await;
        drop(tx);
        let mut reported = 0;
        while rx.try_recv().is_ok() {
            reported += 1;
        }
        let delays = delays.lock().unwrap().clone();
        let attempts = *attempts.lock().unwrap();
        (delays, attempts, reported)
    }

    /// A reconnect pressed during an outage restarts the actor; it must treat a failed first dial
    /// as a lost link being retried, not as a failed start (which would end the attempts).
    #[tokio::test]
    async fn a_resumed_actor_starts_as_established() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (mut tx, _rx) = mpsc::channel(8);
        reconnect_loop(
            DaemonId(1),
            &mut tx,
            true,
            true,
            |life: LinkLife| {
                seen.lock().unwrap().push(life.established);
                async move { (PumpEnd::AppGone, life) }
            },
            |_| async {},
        )
        .await;
        assert_eq!(*seen.lock().unwrap(), [true]);
    }

    /// T041: a remote daemon is retried after 1, 2, 4, 8, then 15 s; a connection that held starts
    /// the sequence over; every lost attempt is reported.
    #[tokio::test]
    async fn a_remote_actor_backs_off_and_starts_over_after_a_connection() {
        use Step::{Failed, Held};
        let steps = [
            Held, Failed, Failed, Failed, Failed, Failed, Failed, Held, Failed,
        ];
        let (delays, attempts, reported) = run_script(true, &steps).await;
        assert_eq!(delays, [1, 2, 4, 8, 15, 15, 15, 1, 2]);
        assert_eq!(attempts, steps.len() + 1, "the last attempt ends the loop");
        assert_eq!(reported, delays.len(), "each lost attempt reaches the app");
    }

    /// T041: a local daemon keeps its fixed pause.
    #[tokio::test]
    async fn a_local_actor_keeps_the_fixed_pause() {
        use Step::{Failed, Held};
        let (delays, ..) = run_script(false, &[Held, Failed, Failed, Failed]).await;
        assert_eq!(delays, [1, 1, 1, 1]);
    }

    /// T041: a start that never connected is reported once and the actor ends: no second connect.
    #[tokio::test]
    async fn a_failed_start_ends_the_actor_without_another_attempt() {
        let (delays, attempts, reported) =
            run_script(true, &[Step::Ended, Step::Held, Step::Held]).await;
        assert_eq!((delays.len(), attempts, reported), (0, 1, 0));
    }

    /// T041: the user's Reconnect gives the actor a new identity (one restart, one actor), stopping
    /// the attempts removes it, and neither leaves two actors for one daemon.
    #[test]
    fn a_reconnect_replaces_the_actor_and_giving_up_removes_it() {
        let (registry, id) = ssh_registry();
        let dir = std::path::Path::new("/nonexistent");
        let mut states = DaemonStates::default();
        states.insert(id, DaemonState::Starting);
        states.apply(id, DaemonEvent::Connected);
        states.apply(id, DaemonEvent::Lost("eof".into()));
        let before = actor_specs(&registry, &states, dir, None);
        assert_eq!(before.len(), 1, "the attempts run in one actor");

        states.apply(id, DaemonEvent::Reconnect);
        let after = actor_specs(&registry, &states, dir, None);
        assert_eq!(after.len(), 1);
        assert_ne!(after[0], before[0], "a new epoch restarts the one actor");
        assert!(
            !before[0].resumed && after[0].resumed,
            "the restart continues an established link"
        );
        assert_eq!(after[0].key, before[0].key);

        states.apply(id, DaemonEvent::GiveUp);
        assert!(actor_specs(&registry, &states, dir, None).is_empty());
        states.apply(id, DaemonEvent::Reconnect);
        assert_eq!(actor_specs(&registry, &states, dir, None).len(), 1);
    }

    /// T051 (FR-013): a remote daemon in every state but connected changes nothing of a connected
    /// local daemon: its state, its outbox handle and the actor that serves it.
    #[test]
    fn a_remote_in_any_state_leaves_a_connected_local_daemon_untouched() {
        use crate::links::DaemonLinks;
        let dir = std::path::Path::new("/nonexistent");
        let (mut registry, remote) = ssh_registry();
        let local = registry.add("Local", DaemonRuntime::Host, true).unwrap();
        let outbox = crate::daemon::Outbox::new(mpsc::unbounded().0);
        let refused = DaemonEvent::Refused {
            client: "1.0".into(),
            daemon: "2.0".into(),
        };
        let remote_paths: [Vec<DaemonEvent>; 6] = [
            vec![],
            vec![DaemonEvent::Connected, DaemonEvent::Lost("eof".into())],
            vec![DaemonEvent::StartFailed("no route".into())],
            vec![refused],
            vec![
                DaemonEvent::Connected,
                DaemonEvent::Lost("eof".into()),
                DaemonEvent::GiveUp,
            ],
            vec![DaemonEvent::Stop],
        ];
        let mut baseline = DaemonLinks::default();
        baseline.insert(local, DaemonState::Starting);
        assert!(baseline.connected(local, outbox.clone()));
        let local_spec = |links: &DaemonLinks| {
            actor_specs(&registry, links.states(), dir, None)
                .into_iter()
                .find(|s| s.key.id == local)
        };
        let expected = local_spec(&baseline).expect("the local daemon has an actor");

        for events in remote_paths {
            let mut links = baseline.clone();
            links.insert(remote, DaemonState::Starting);
            for event in events {
                links.apply(remote, event);
            }
            assert_ne!(links.state(remote), Some(&DaemonState::Connected));
            assert_eq!(links.state(local), Some(&DaemonState::Connected));
            assert_eq!(links.outbox(local), Some(&outbox), "the same handle");
            assert_eq!(local_spec(&links).as_ref(), Some(&expected));
        }
    }

    /// Runs `body` on a rollback armed to report the ledger it undoes.
    fn rolled_back(body: impl FnOnce(&mut StartRollback)) -> Option<AttemptLedger> {
        let (tx, rx) = std::sync::mpsc::channel();
        let undo = Box::new(move |ledger| tx.send(ledger).unwrap())
            as Box<dyn FnOnce(AttemptLedger) + Send>;
        let mut rollback = StartRollback::new(Some(undo));
        body(&mut rollback);
        drop(rollback);
        rx.recv_timeout(std::time::Duration::from_millis(300)).ok()
    }

    fn entered_start(rollback: &StartRollback) {
        let sink: EventSink = std::sync::Arc::new(|_| {});
        rollback.recording_sink(&sink, "ctr")(micold_core::provision::ProvisionEvent::Entered(
            micold_core::provision::Step::Start,
        ));
    }

    #[test]
    fn an_attempt_that_started_the_daemon_stops_that_container_when_it_ends() {
        let ledger = rolled_back(|r| entered_start(r)).expect("rolled back");
        assert_eq!(
            ledger.started_daemon,
            Some(micold_core::sandbox::runtime::ContainerId("ctr".into()))
        );
    }

    #[test]
    fn an_attempt_that_never_reached_the_start_or_connected_undoes_nothing() {
        assert_eq!(rolled_back(|_| {}), None, "daemon never started");
        assert_eq!(
            rolled_back(|r| {
                entered_start(r);
                r.disarm();
            }),
            None,
            "connected"
        );
    }
}
