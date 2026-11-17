//! The change feed of `/api/ws` (FR-003, research R5, R8). Daemons push no change events, so one
//! poller lists every second (and at once when a daemon links or leaves), diffs the list against
//! the last one and appends the differences to the [`ChangeLog`], which every subscriber reads
//! from. It runs only while someone subscribes; its snapshot and log outlive a stop, so the first
//! pass after a restart appends what changed meanwhile and a resume across the gap stays exact.
//! The epoch is random per process: a resume against another orchestrator process resyncs.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use micold_core::client_api::diff::diff;
use micold_core::client_api::log::{ChangeLog, Since};
use micold_core::client_api::messages::{Change, Position, ResyncReason, Snapshot};
use tokio::sync::broadcast;

use super::directory::LinkDirectory;

/// How often the poller lists.
pub const POLL: Duration = Duration::from_secs(1);

/// The batches a subscriber may fall behind before it reads the log instead.
pub const BACKLOG: usize = 256;

/// The feed.
pub struct Feed {
    directory: Arc<dyn LinkDirectory>,
    poll: Duration,
    inner: Mutex<Inner>,
    /// Held for a whole pass, so passes finish in the order they list and an older list never
    /// replaces a newer snapshot.
    passing: tokio::sync::Mutex<()>,
    batches: broadcast::Sender<Vec<Change>>,
}

struct Inner {
    log: ChangeLog,
    snapshot: Option<Snapshot>,
    subscribers: usize,
    polling: bool,
}

/// A subscriber's place in the feed: new batches arrive on `batches`. Dropping it unsubscribes.
pub struct Subscription {
    feed: Arc<Feed>,
    /// Every batch appended after the subscription began.
    pub batches: broadcast::Receiver<Vec<Change>>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.feed.lock().subscribers -= 1;
    }
}

/// The answer to a subscribe.
pub enum Subscribed {
    /// A fresh snapshot; changes after its `seq` follow.
    Snapshot(Subscription, Snapshot),
    /// The changes after the asked position (maybe none); newer ones follow.
    Resumed(Subscription, Vec<Change>),
    /// The asked position cannot be served.
    ResyncNeeded {
        /// The feed's last seq.
        seq: u64,
        /// Why.
        reason: ResyncReason,
    },
}

/// A random epoch that a JavaScript number holds exactly (53 bits), never 0.
fn random_epoch() -> u64 {
    (uuid::Uuid::new_v4().as_u64_pair().0 >> 11).max(1)
}

impl Feed {
    /// A feed over `directory`, polling every `poll` while subscribed; a subscriber may fall
    /// `backlog` batches behind before it reads the log.
    pub fn new(directory: Arc<dyn LinkDirectory>, poll: Duration, backlog: usize) -> Arc<Self> {
        Arc::new(Self {
            directory,
            poll,
            inner: Mutex::new(Inner {
                log: ChangeLog::new(random_epoch()),
                snapshot: None,
                subscribers: 0,
                polling: false,
            }),
            passing: tokio::sync::Mutex::new(()),
            batches: broadcast::channel(backlog.max(1)).0,
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The feed's epoch.
    pub fn epoch(&self) -> u64 {
        self.lock().log.epoch()
    }

    /// The feed's last seq.
    pub fn last_seq(&self) -> u64 {
        self.lock().log.last_seq()
    }

    /// Every change after `seq` of the current epoch (for a subscriber that fell behind).
    pub fn since(&self, seq: u64) -> Since {
        let inner = self.lock();
        inner.log.since(inner.log.epoch(), seq)
    }

    /// Subscribe from `since`, or from a fresh snapshot.
    pub async fn subscribe(self: &Arc<Self>, since: Option<Position>) -> Subscribed {
        // A stopped poller's snapshot may be stale: list first.
        let stale = {
            let inner = self.lock();
            inner.snapshot.is_none() || !inner.polling
        };
        if since.is_none() && stale {
            self.pass().await;
        }
        let mut inner = self.lock();
        let resumed = match since {
            None => None,
            Some(position) => match inner.log.since(position.epoch, position.seq) {
                Since::Changes(changes) => Some(changes),
                Since::ResyncNeeded(reason) => {
                    return Subscribed::ResyncNeeded {
                        seq: inner.log.last_seq(),
                        reason,
                    }
                }
            },
        };
        // Under the lock that appends: the receiver sees exactly the batches after this position.
        let subscription = Subscription {
            feed: Arc::clone(self),
            batches: self.batches.subscribe(),
        };
        inner.subscribers += 1;
        if !inner.polling {
            inner.polling = true;
            tokio::spawn(Arc::clone(self).poll_while_subscribed());
        }
        match resumed {
            Some(changes) => Subscribed::Resumed(subscription, changes),
            None => {
                let mut snapshot = inner.snapshot.clone().unwrap_or_default();
                snapshot.epoch = inner.log.epoch();
                snapshot.seq = inner.log.last_seq();
                Subscribed::Snapshot(subscription, snapshot)
            }
        }
    }

    /// List once, and append and broadcast what changed since the last list.
    async fn pass(&self) {
        let _passing = self.passing.lock().await;
        let new = self.directory.list().await;
        let mut inner = self.lock();
        if let Some(old) = &inner.snapshot {
            let kinds = diff(old, &new);
            if !kinds.is_empty() {
                let changes = inner.log.append(kinds);
                // No receiver is not an error: the log keeps the changes for a resume.
                let _ = self.batches.send(changes);
            }
        }
        inner.snapshot = Some(new);
    }

    async fn poll_while_subscribed(self: Arc<Self>) {
        loop {
            {
                let mut inner = self.lock();
                if inner.subscribers == 0 {
                    inner.polling = false;
                    return;
                }
            }
            self.pass().await;
            tokio::select! {
                () = tokio::time::sleep(self.poll) => {}
                () = self.directory.changed() => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use futures_util::future::{BoxFuture, FutureExt};
    use micold_core::client_api::messages::{ContentEntry, Daemon, DaemonStatus};

    use super::super::directory::ApiError;
    use super::*;

    /// Lists `old` slowly the first time and `new` at once after.
    #[derive(Default)]
    struct SlowFirst(AtomicUsize);

    fn snapshot(id: &str) -> Snapshot {
        Snapshot {
            daemons: vec![Daemon {
                id: id.to_owned(),
                name: id.to_owned(),
                status: DaemonStatus::Online,
            }],
            ..Snapshot::default()
        }
    }

    impl LinkDirectory for SlowFirst {
        fn list(&self) -> BoxFuture<'_, Snapshot> {
            let first = self.0.fetch_add(1, Ordering::SeqCst) == 0;
            async move {
                if first {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    snapshot("old")
                } else {
                    snapshot("new")
                }
            }
            .boxed()
        }

        fn read_session<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
            _: &'a str,
        ) -> BoxFuture<'a, Result<Vec<ContentEntry>, ApiError>> {
            async { Ok(Vec::new()) }.boxed()
        }

        fn changed(&self) -> BoxFuture<'_, ()> {
            std::future::pending().boxed()
        }
    }

    #[tokio::test]
    async fn concurrent_passes_never_replace_a_newer_list_with_an_older_one() {
        let feed = Feed::new(Arc::new(SlowFirst::default()), POLL, BACKLOG);
        tokio::join!(feed.pass(), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            feed.pass().await;
        });
        let snapshot = feed.lock().snapshot.clone().unwrap();
        assert_eq!(snapshot.daemons[0].id, "new");
    }
}
