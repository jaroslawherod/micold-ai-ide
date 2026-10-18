//! Cancelling one start attempt (feature 806, FR-019, data-model *AttemptLedger and CancelToken*).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

use crate::sandbox::runtime::{ContainerId, ContainerRuntime};

/// A flag a start attempt checks between steps and awaits beside every child it waits on. Cloning
/// shares the flag: the client keeps one end to cancel with, the sequence the other.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<(AtomicBool, Notify)>);

impl CancelToken {
    /// A token nobody has cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel: wake every waiter, now and for any that arrive later. Calling it again does nothing.
    pub fn cancel(&self) {
        self.0 .0.store(true, Ordering::SeqCst);
        self.0 .1.notify_waiters();
    }

    /// Whether [`CancelToken::cancel`] has been called.
    pub fn is_cancelled(&self) -> bool {
        self.0 .0.load(Ordering::SeqCst)
    }

    /// Resolves once the token is cancelled (at once when it already is).
    pub async fn cancelled(&self) {
        loop {
            let notified = self.0 .1.notified();
            tokio::pin!(notified);
            // Registered before the flag is read, so a `cancel` in between is not missed.
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

/// What one start attempt created, so a failed or cancelled attempt removes exactly that (FR-020,
/// data-model *AttemptLedger and CancelToken*).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttemptLedger {
    /// The container this attempt created. Never an adopted one.
    pub created_container: Option<ContainerId>,
    /// The container whose daemon this attempt started. Never a daemon found running.
    pub started_daemon: Option<ContainerId>,
}

impl AttemptLedger {
    /// An attempt that has created nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// This attempt created (or may have created) container `id`.
    pub fn record_created(&mut self, id: ContainerId) {
        self.created_container = Some(id);
    }

    /// This attempt started the daemon in container `id`.
    pub fn record_daemon_started(&mut self, id: ContainerId) {
        self.started_daemon = Some(id);
    }

    /// Whether the attempt made anything that [`rollback`] would undo.
    pub fn is_empty(&self) -> bool {
        self.created_container.is_none() && self.started_daemon.is_none()
    }
}

/// Undo what `ledger` records: remove a container the attempt created (which ends any daemon in
/// it), else stop the container whose daemon the attempt started. An adopted container and a daemon
/// found running are not in the ledger and are left alone; the host's state directory is never
/// touched. Both calls are idempotent, so a rollback after a half-made container is safe.
pub fn rollback<R: ContainerRuntime + ?Sized>(
    ledger: &AttemptLedger,
    runtime: &R,
) -> Result<(), crate::sandbox::runtime::RuntimeError> {
    match (&ledger.created_container, &ledger.started_daemon) {
        (Some(id), _) => runtime.remove(id),
        (None, Some(id)) => runtime.stop(id),
        (None, None) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::cli::CliRuntime;
    use crate::sandbox::exec::RecordingRunner;
    use crate::sandbox::runtime::RuntimeKind;
    use std::time::Duration;

    /// The runtime commands `rollback` issued, as `verb args…` lines.
    fn rolled_back(ledger: &AttemptLedger) -> Vec<String> {
        let runner = RecordingRunner::new();
        rollback(ledger, &CliRuntime::new(RuntimeKind::Docker, &runner)).unwrap();
        runner
            .calls()
            .iter()
            .map(|c| c.args_lossy().join(" "))
            .collect()
    }

    fn id(s: &str) -> ContainerId {
        ContainerId(s.to_string())
    }

    #[test]
    fn rollback_removes_a_container_created_in_this_attempt() {
        let mut ledger = AttemptLedger::new();
        ledger.record_created(id("c1"));
        assert_eq!(rolled_back(&ledger), ["rm -f c1"]);
    }

    #[test]
    fn rollback_stops_a_container_whose_daemon_this_attempt_started() {
        let mut ledger = AttemptLedger::new();
        ledger.record_daemon_started(id("c2"));
        assert_eq!(rolled_back(&ledger), ["stop c2"]);
    }

    #[test]
    fn rollback_removing_a_created_container_does_not_also_stop_it() {
        let mut ledger = AttemptLedger::new();
        ledger.record_created(id("c3"));
        ledger.record_daemon_started(id("c3"));
        assert_eq!(rolled_back(&ledger), ["rm -f c3"]);
    }

    #[test]
    fn rollback_of_an_adopted_container_and_a_found_daemon_touches_nothing() {
        let ledger = AttemptLedger::new();
        assert!(ledger.is_empty());
        assert!(rolled_back(&ledger).is_empty());
    }

    #[test]
    fn cancel_is_idempotent_and_visible() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
        assert!(token.clone().is_cancelled(), "clones share the flag");
    }

    #[tokio::test]
    async fn cancel_wakes_every_waiter() {
        let token = CancelToken::new();
        let waiters: Vec<_> = (0..3)
            .map(|_| {
                let t = token.clone();
                tokio::spawn(async move { t.cancelled().await })
            })
            .collect();
        tokio::time::sleep(Duration::from_millis(20)).await;
        token.cancel();
        for waiter in waiters {
            tokio::time::timeout(Duration::from_secs(2), waiter)
                .await
                .expect("woken")
                .unwrap();
        }
    }

    #[tokio::test]
    async fn a_waiter_after_the_cancel_returns_at_once() {
        let token = CancelToken::new();
        token.cancel();
        tokio::time::timeout(Duration::from_millis(100), token.cancelled())
            .await
            .expect("already cancelled");
    }
}
