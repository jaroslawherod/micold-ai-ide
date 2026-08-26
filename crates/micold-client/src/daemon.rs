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

use std::time::Instant;

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::Subscription;

use micold_core::connect::{connect_at, connect_or_spawn, Connected, Credentials, SPAWN_TIMEOUT};
use micold_core::endpoint::{self, DialAddress};
use micold_core::protocol::codec::{CodecError, Frame};
use micold_core::protocol::keepalive::{self, Keepalive, KeepaliveAction};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::sandbox::placement::PlacementKind;

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
    /// Whether a build-fingerprint mismatch refuses the connection — true only for a locally built
    /// image, whose daemon came from this same working tree (research R8).
    pub strict_fingerprint: bool,
}

/// What the client presents to a sandboxed daemon.
///
/// The token is **read from the file** the client wrote and the runtime mounted, rather than
/// remembered in memory, so a sandbox started by a previous run of this client is still reachable
/// by this one. A missing token file is not fixed up here: the handshake refuses, which is the
/// honest outcome, and the remedy is to restart the sandbox so a new token is issued.
fn sandbox_credentials(placement: &Placement) -> Credentials {
    let token = micold_core::protocol::auth::Token::read_from(
        &micold_core::protocol::auth::host_token_path(&placement.state_dir),
    )
    .ok();
    Credentials {
        auth_token: token.map(|t| micold_core::protocol::messages::PresentedToken::new(t.as_str())),
        require_fingerprint_match: placement.strict_fingerprint,
    }
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

/// The connection subscription. Add it to the app's subscription set; iced keeps one instance alive
/// for the app's lifetime.
///
/// `run_with` rather than `run`: feature 027 gave this subscription a parameter, and the builder
/// still has to be a plain `fn` for its identity to be stable — a capturing closure would make iced
/// restart it every frame. `run_with` identifies the subscription by the *data* plus the function
/// pointer, which is exactly right here: the placement is read once at boot and does not change
/// while the app runs, so the identity is as stable as it was before.
pub fn connection(placement: Placement) -> Subscription<Message> {
    Subscription::run_with(placement, |p| actor(p.clone()))
}

/// How long to wait after a lost connection before trying to re-establish it. Short enough that a
/// restarted (or freed) daemon is picked up promptly; long enough not to spin on a hard failure. The
/// connection banner is what the user sees during the gap (FR-027); the reconnect itself is automatic
/// (FR-028 — the fresh `Welcome` catalog is the resync).
const RECONNECT_BACKOFF: std::time::Duration = std::time::Duration::from_secs(1);

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
}

fn actor(placement: Placement) -> impl Stream<Item = Message> {
    // iced 0.14 takes an `AsyncFnOnce` here (0.13 took an `FnOnce` returning a future), so the
    // sender's type no longer falls out of inference and has to be named.
    iced::stream::channel(
        CHANNEL_CAPACITY,
        |mut output: mpsc::Sender<Message>| async move {
            // Resolve the endpoint once; it does not change over the app's life.
            let endpoint = match endpoint::resolve() {
                Ok(e) => e,
                Err(err) => {
                    let _ = output
                        .send(Message::DaemonConnectFailed(err.to_string()))
                        .await;
                    return;
                }
            };

            // Reconnect loop: connect, pump until the link drops, surface it, back off, repeat. A
            // half-open connection is caught by the keepalive inside `pump`, so the client never sits
            // forever presenting stale content as live (FR-026/027).
            loop {
                match connect_and_pump(&placement, &endpoint, &mut output).await {
                    PumpEnd::AppGone => return,
                    PumpEnd::Disconnected => {
                        if output.send(Message::DaemonDisconnected).await.is_err() {
                            return;
                        }
                        tokio::time::sleep(RECONNECT_BACKOFF).await;
                    }
                }
            }
        },
    )
}

/// Connect (spawning the daemon on a cold start), announce `DaemonConnected`, then pump both
/// directions with a keepalive until the connection ends. A connect failure is surfaced as
/// `DaemonConnectFailed` and reported as a disconnect so the outer loop retries.
async fn connect_and_pump(
    placement: &Placement,
    endpoint: &endpoint::Endpoint,
    output: &mut mpsc::Sender<Message>,
) -> PumpEnd {
    let attempt = match placement.kind {
        // Unchanged: auto-spawn a detached host process and poll until it accepts.
        PlacementKind::HostProcess => connect_or_spawn(endpoint, CLIENT_BUILD, SPAWN_TIMEOUT).await,
        // The sandbox is brought up by `shell::sandbox`, which the app drives so the user can watch
        // the image being acquired. All this loop does is dial it — and if nothing is listening,
        // say so. It does **not** fall back to a host process: that is FR-035, and this is the one
        // place it would be easiest to lose.
        PlacementKind::LocalSandbox => {
            let address = DialAddress::Loopback {
                port: micold_core::endpoint::DEFAULT_SANDBOX_PORT,
            };
            let credentials = sandbox_credentials(placement);
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

    let connected = match attempt {
        Ok(c) => c,
        Err(err) => {
            return report_connect_failure(output, err.to_string()).await;
        }
    };
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
                .send(Message::DaemonVersionMismatch {
                    client,
                    daemon,
                    daemon_build,
                })
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
                .send(Message::DaemonBuildMismatch {
                    client_build,
                    daemon_build,
                })
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
            return report_connect_failure(
                output,
                stale_dev_image_advice(&image, &daemon_fingerprint, &client_fingerprint),
            )
            .await;
        }
        Connected::Refused(reason) => {
            return report_connect_failure(
                output,
                format!("daemon refused the connection: {reason:?}"),
            )
            .await;
        }
    };

    // Hand the App a fresh Outbox and the welcome state (the resync on every (re)connect, FR-028).
    let (mut sink, incoming) = conn.split();
    let (tx, rx) = mpsc::unbounded::<ClientMsg>();
    if output
        .send(Message::DaemonConnected {
            outbox: Outbox::new(tx),
            catalog: welcome.catalog,
            settings: welcome.settings,
        })
        .await
        .is_err()
    {
        return PumpEnd::AppGone;
    }

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
                if sink.send(Frame::Control(msg)).await.is_err() {
                    return PumpEnd::Disconnected; // socket closed under us
                }
            }
            Io::Incoming(Ok(frame)) => {
                keepalive.on_daemon_frame(Instant::now());
                let msg = match frame {
                    Frame::Control(dm) => Message::DaemonEvent(dm),
                    Frame::Grid(frame) => Message::DaemonGridFrame(frame),
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

/// Report a connect failure to the app and map it to a disconnect (so the outer loop retries). If the
/// app is gone, that surfaces as `AppGone` instead.
async fn report_connect_failure(output: &mut mpsc::Sender<Message>, reason: String) -> PumpEnd {
    if output
        .send(Message::DaemonConnectFailed(reason))
        .await
        .is_err()
    {
        PumpEnd::AppGone
    } else {
        PumpEnd::Disconnected
    }
}

#[cfg(test)]
mod tests {
    use super::stale_dev_image_advice;

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
        assert!(advice.contains("aaaa1111") && advice.contains("bbbb2222"), "{advice}");
    }
}
