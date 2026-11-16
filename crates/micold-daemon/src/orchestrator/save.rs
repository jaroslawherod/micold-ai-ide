//! The orchestrator's one saving thread (FR-004). A link hands its snapshot over and goes on: the
//! write and its fsyncs run on this thread, never on a runtime worker, so a slow disk cannot hold
//! up accept or a link's heartbeat timers. Snapshots handed over while a write runs are coalesced:
//! only the newest is written next.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use micold_core::orchestrator::{RegistrySaver, RegistryStore, Snapshot, StoreError};

#[derive(Default)]
pub(super) struct Pending {
    /// The newest snapshot not yet written.
    pub(super) snapshot: Option<Snapshot>,
    /// Set by [`SaveWorker`]'s drop: nothing handed over later is written.
    closed: bool,
}

/// Where links hand their snapshots over.
#[derive(Default)]
pub(crate) struct SaveQueue {
    pending: Mutex<Pending>,
    wake: Condvar,
}

impl SaveQueue {
    pub(super) fn pending(&self) -> MutexGuard<'_, Pending> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Hand `snapshot` over, replacing an older one still waiting. Never blocks on the store.
    pub(crate) fn offer(&self, snapshot: Snapshot) {
        let mut pending = self.pending();
        if pending.closed {
            return;
        }
        if pending
            .snapshot
            .as_ref()
            .is_none_or(|waiting| waiting.seq < snapshot.seq)
        {
            pending.snapshot = Some(snapshot);
            self.wake.notify_one();
        }
    }

    /// The next snapshot to write, waiting for one; `None` once closed with nothing left.
    fn next(&self) -> Option<Snapshot> {
        let mut pending = self.pending();
        loop {
            if let Some(snapshot) = pending.snapshot.take() {
                return Some(snapshot);
            }
            if pending.closed {
                return None;
            }
            pending = self
                .wake
                .wait(pending)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// The saving thread. Dropping it writes what is still waiting, then joins the thread: once it is
/// gone, nothing writes the registry file any more.
pub(crate) struct SaveWorker {
    queue: Arc<SaveQueue>,
    thread: Option<JoinHandle<()>>,
}

impl SaveWorker {
    /// Start the thread writing to `store`, with the queue that feeds it.
    pub(crate) fn start(store: Arc<dyn RegistryStore>) -> std::io::Result<(Self, Arc<SaveQueue>)> {
        let queue = Arc::new(SaveQueue::default());
        let feed = Arc::clone(&queue);
        let saver = RegistrySaver::new(store);
        let thread = std::thread::Builder::new()
            .name("orchestrator-save".to_owned())
            .spawn(move || {
                while let Some(snapshot) = feed.next() {
                    // A failure is logged: the registry in memory stays right and the next
                    // change tries again.
                    match saver.save(&snapshot) {
                        Ok(_) => {}
                        // The JSON store's own text already says so.
                        Err(e @ StoreError::Save { .. }) => {
                            eprintln!("micold-daemon: orchestrator: {e}");
                        }
                        Err(e) => eprintln!(
                            "micold-daemon: orchestrator: a registry change could not be saved: {e}"
                        ),
                    }
                }
            })?;
        let worker = Self {
            queue: Arc::clone(&queue),
            thread: Some(thread),
        };
        Ok((worker, queue))
    }
}

impl Drop for SaveWorker {
    fn drop(&mut self) {
        let mut pending = self.queue.pending();
        pending.closed = true;
        self.queue.wake.notify_one();
        drop(pending);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
