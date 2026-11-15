//! The daemon's side of the orchestrator link (#1725): a background task that dials the
//! orchestrator, announces this daemon, answers its pings and dials again when the link is lost
//! or refused. It reads only the configuration it is given and writes only its own status cell:
//! it takes no `DaemonState` lock a session waits on, so an orchestrator outage never touches a
//! session (FR-008). A daemon with no `[orchestrator_link] enabled = true` spawns nothing.

use std::collections::BTreeSet;
use std::hash::{BuildHasher, Hasher};
use std::sync::Arc;

use micold_core::connect::dial_address;
use micold_core::daemon_config::DaemonConfig;
use micold_core::endpoint::{link_endpoint, DialAddress};
use micold_core::orchestrator::{jittered, LinkAnnouncement, LinkStatus, RetryPolicy};
use micold_core::protocol::version::PROTOCOL_VERSION;
use micold_core::ssh::status::LinkStatusReport;
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub mod dial;

use dial::Handshake;

/// What this daemon announces it can do.
const CAPABILITIES: &[&str] = &["sessions"];

/// What the status cell holds: the status, the orchestrator's sentence when it refused, when the
/// status was entered and how many dials reached the endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkState {
    /// Where the link stands.
    pub status: LinkStatus,
    /// The refusal's sentence (the orchestrator's message, or the refused `Hello`'s reason);
    /// empty for every other status.
    pub detail: String,
    /// When the link entered this status, in Unix seconds. A retry refused the same way again
    /// keeps the first refusal's time.
    pub since: u64,
    /// Dials that reached the endpoint, each counted when it begins.
    pub attempts: u64,
}

impl LinkState {
    /// The `link` object of `status.json` (FR-012).
    pub fn report(&self) -> LinkStatusReport {
        LinkStatusReport::new(&self.status, self.since, &self.detail)
    }
}

/// The running link task and its status cell.
pub struct LinkHandle {
    state: watch::Receiver<LinkState>,
    task: JoinHandle<()>,
}

impl LinkHandle {
    /// Where the link stands now.
    pub fn status(&self) -> LinkStatus {
        self.state.borrow().status.clone()
    }

    /// The whole status cell now.
    pub fn state(&self) -> LinkState {
        self.state.borrow().clone()
    }

    /// A receiver that wakes on each change of the cell.
    pub fn watch(&self) -> watch::Receiver<LinkState> {
        self.state.clone()
    }

    /// Whether the task has ended (it only does when aborted).
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// Stop dialling.
    pub fn abort(&self) {
        self.task.abort();
    }
}

/// Spawn the link task when `[orchestrator_link]` is enabled with a usable endpoint; otherwise
/// return before spawning, opening or logging anything (FR-010). Never awaited by the caller.
pub fn spawn(config: &Arc<DaemonConfig>) -> Option<LinkHandle> {
    spawn_as(config, announcement(config))
}

/// [`spawn`], announcing `announcement` instead of the one `config` gives (a test announces
/// another protocol version this way).
#[doc(hidden)]
pub fn spawn_as(config: &Arc<DaemonConfig>, announcement: LinkAnnouncement) -> Option<LinkHandle> {
    let link = config.orchestrator_link.as_ref().filter(|l| l.enabled)?;
    // An empty socket is refused by `config check` (L1); a daemon started with one does not dial.
    if link
        .socket
        .as_ref()
        .is_some_and(|s| s.as_os_str().is_empty())
    {
        return None;
    }
    let endpoint = link_endpoint(config.daemon.kind, link.socket.as_deref())
        .inspect_err(|e| {
            tracing::warn!(error = %e, "orchestrator link enabled but its endpoint cannot be resolved");
        })
        .ok()?;
    let reporter = Reporter::new();
    let state = reporter.tx.subscribe();
    let task = tokio::spawn(run(DialAddress::Local(endpoint), announcement, reporter));
    Some(LinkHandle { state, task })
}

/// Spawn the link task for the configuration the daemon started with, if it has one.
pub fn spawn_for(state: &crate::state::DaemonState) -> Option<LinkHandle> {
    spawn(&state.config_current()?)
}

/// What a daemon of `config` announces.
pub fn announcement(config: &DaemonConfig) -> LinkAnnouncement {
    LinkAnnouncement {
        format: micold_core::orchestrator::announce::ANNOUNCEMENT_FORMAT,
        id: config.wire_id(),
        kind: config.daemon.kind,
        name: config.daemon.name.clone(),
        protocol_version: PROTOCOL_VERSION,
        capabilities: CAPABILITIES
            .iter()
            .map(|c| (*c).to_owned())
            .collect::<BTreeSet<_>>(),
        tenant: None,
    }
}

/// A value in `0.0..1.0` for the jitter, from the standard library's per-process random state.
fn jitter_unit() -> f64 {
    let bits = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    (bits >> 11) as f64 / (1_u64 << 53) as f64
}

/// Now, in Unix seconds.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Sets the status cell and logs the settled states once each.
struct Reporter {
    tx: watch::Sender<LinkState>,
    /// The last settled (not `Linking`) state: a retry that ends the same way keeps its `since`
    /// and logs nothing.
    settled: LinkState,
}

impl Reporter {
    fn new() -> Self {
        let settled = LinkState {
            status: LinkStatus::NotLinked,
            detail: String::new(),
            since: unix_now(),
            attempts: 0,
        };
        let (tx, _) = watch::channel(settled.clone());
        Self { tx, settled }
    }

    /// A dial reached the endpoint.
    fn linking(&mut self) {
        self.settled.attempts += 1;
        self.tx.send_replace(LinkState {
            status: LinkStatus::Linking,
            detail: String::new(),
            since: unix_now(),
            attempts: self.settled.attempts,
        });
    }

    fn set(&mut self, status: LinkStatus, detail: &str) {
        let changed = status != self.settled.status || detail != self.settled.detail;
        if changed {
            let before = std::mem::replace(&mut self.settled.status, status.clone());
            self.settled.detail = detail.to_owned();
            self.settled.since = unix_now();
            Self::log(&before, &status, detail);
        }
        self.tx.send_replace(self.settled.clone());
    }

    /// One line per change of settled status (FR-012), none per retry.
    fn log(before: &LinkStatus, status: &LinkStatus, detail: &str) {
        if before == status {
            return;
        }
        match (before, status) {
            (LinkStatus::Linked, LinkStatus::NotLinked) => {
                tracing::warn!("orchestrator link lost");
            }
            // A refused daemon whose orchestrator then went away.
            (_, LinkStatus::NotLinked) => tracing::info!("orchestrator link not linked"),
            (_, LinkStatus::Linked) => tracing::info!("orchestrator link up"),
            _ => tracing::warn!(state = status.name(), detail, "orchestrator link refused"),
        }
    }
}

async fn run(address: DialAddress, announcement: LinkAnnouncement, mut report: Reporter) {
    let mut policy = RetryPolicy::new();
    loop {
        let delay = match dial_address(&address).await {
            Ok(Some(stream)) => {
                report.linking();
                match dial::attempt(stream, &announcement).await {
                    Ok(Handshake::Linked(framed)) => {
                        policy.on_linked();
                        report.set(LinkStatus::Linked, "");
                        let ended = dial::steady(framed).await;
                        if let Err(e) = &ended {
                            tracing::debug!(error = %e, "orchestrator link ended");
                        }
                        report.set(LinkStatus::NotLinked, "");
                        jittered(policy.after_loss(), jitter_unit())
                    }
                    Ok(Handshake::Refused { reason, message }) => {
                        report.set(LinkStatus::Refused(reason), &message);
                        jittered(policy.after_refusal(), jitter_unit())
                    }
                    Ok(Handshake::HelloRefused(detail)) => {
                        report.set(LinkStatus::HelloRefused(detail.clone()), &detail);
                        jittered(policy.after_refusal(), jitter_unit())
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "orchestrator dial failed");
                        report.set(LinkStatus::NotLinked, "");
                        policy.after_failure()
                    }
                }
            }
            // Nothing listens, or the dial failed: not an error to report.
            Ok(None) | Err(_) => {
                report.set(LinkStatus::NotLinked, "");
                policy.after_failure()
            }
        };
        tokio::time::sleep(delay).await;
    }
}
