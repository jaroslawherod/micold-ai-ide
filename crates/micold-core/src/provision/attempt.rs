//! Cancelling one start attempt (feature 806, FR-019, data-model *AttemptLedger and CancelToken*).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
