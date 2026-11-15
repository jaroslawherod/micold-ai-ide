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
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub mod dial;

use dial::Handshake;

/// What this daemon announces it can do.
const CAPABILITIES: &[&str] = &["sessions"];

/// The running link task and its status cell.
pub struct LinkHandle {
    status: watch::Receiver<LinkStatus>,
    task: JoinHandle<()>,
}

impl LinkHandle {
    /// Where the link stands now.
    pub fn status(&self) -> LinkStatus {
        self.status.borrow().clone()
    }

    /// A receiver that wakes on each status change.
    pub fn watch(&self) -> watch::Receiver<LinkStatus> {
        self.status.clone()
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
    let link = config.orchestrator_link.as_ref().filter(|l| l.enabled)?;
    // An empty socket is refused by `config check` (L1); a daemon started with one does not dial.
    if link
        .socket
        .as_ref()
        .is_some_and(|s| s.as_os_str().is_empty())
    {
        return None;
    }
    let endpoint = link_endpoint(config.daemon.kind, link.socket.as_deref()).ok()?;
    let announcement = announcement(config);
    let (tx, status) = watch::channel(LinkStatus::NotLinked);
    let task = tokio::spawn(run(DialAddress::Local(endpoint), announcement, tx));
    Some(LinkHandle { status, task })
}

/// Spawn the link task for the configuration the daemon started with, if it has one.
pub fn spawn_for(state: &crate::state::DaemonState) -> Option<LinkHandle> {
    spawn(&state.config_current()?)
}

fn announcement(config: &DaemonConfig) -> LinkAnnouncement {
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

/// Sets the status and logs the settled states once each.
struct Reporter {
    tx: watch::Sender<LinkStatus>,
    logged: LinkStatus,
}

impl Reporter {
    fn set(&mut self, status: LinkStatus, detail: &str) {
        self.tx.send_replace(status.clone());
        if status == self.logged || matches!(status, LinkStatus::Linking) {
            return;
        }
        match (&self.logged, &status) {
            (LinkStatus::Linked, LinkStatus::NotLinked) => {
                tracing::warn!("orchestrator link lost");
            }
            (_, LinkStatus::NotLinked) => {}
            (_, LinkStatus::Linked) => tracing::info!("orchestrator link up"),
            _ => tracing::warn!(state = status.name(), detail, "orchestrator link refused"),
        }
        self.logged = status;
    }
}

async fn run(address: DialAddress, announcement: LinkAnnouncement, tx: watch::Sender<LinkStatus>) {
    let mut policy = RetryPolicy::new();
    let mut report = Reporter {
        tx,
        logged: LinkStatus::NotLinked,
    };
    loop {
        let delay = match dial_address(&address).await {
            Ok(Some(stream)) => {
                report.set(LinkStatus::Linking, "");
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
