//! The change log that makes resume exact (FR-003, data-model.md *ChangeLog*, research R5): the
//! last [`CAPACITY`] changes of one epoch, each with a sequence number one above the last.

use std::collections::VecDeque;

use super::messages::{Change, ChangeKind, ResyncReason};

/// How many changes the log holds.
pub const CAPACITY: usize = 1024;

/// The feed's changes of one epoch.
#[derive(Debug, Clone)]
pub struct ChangeLog {
    epoch: u64,
    next_seq: u64,
    capacity: usize,
    ring: VecDeque<Change>,
}

/// What a resume from a position gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Since {
    /// Every change after the position, in order (maybe none).
    Changes(Vec<Change>),
    /// The changes are not held: the client reloads a snapshot.
    ResyncNeeded(ResyncReason),
}

impl ChangeLog {
    /// An empty log of `epoch`, holding [`CAPACITY`] changes.
    pub fn new(epoch: u64) -> Self {
        Self::with_capacity(epoch, CAPACITY)
    }

    /// An empty log of `epoch`, holding `capacity` (at least 1) changes.
    pub fn with_capacity(epoch: u64, capacity: usize) -> Self {
        Self {
            epoch,
            next_seq: 1,
            capacity: capacity.max(1),
            ring: VecDeque::new(),
        }
    }

    /// The epoch.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The last sequence number given out (0 before the first).
    pub fn last_seq(&self) -> u64 {
        self.next_seq - 1
    }

    /// Number `kinds` in order and keep them, dropping the oldest beyond the capacity; returns the
    /// numbered changes.
    pub fn append(&mut self, kinds: Vec<ChangeKind>) -> Vec<Change> {
        let changes: Vec<Change> = kinds
            .into_iter()
            .map(|kind| {
                let seq = self.next_seq;
                self.next_seq += 1;
                Change { seq, kind }
            })
            .collect();
        for change in &changes {
            if self.ring.len() == self.capacity {
                self.ring.pop_front();
            }
            self.ring.push_back(change.clone());
        }
        changes
    }

    /// Every change after `seq` of `epoch`, or why they cannot be served.
    pub fn since(&self, epoch: u64, seq: u64) -> Since {
        if epoch != self.epoch {
            return Since::ResyncNeeded(ResyncReason::EpochChanged);
        }
        let last = self.last_seq();
        if seq > last {
            return Since::ResyncNeeded(ResyncReason::Ahead);
        }
        if seq == last {
            return Since::Changes(Vec::new());
        }
        let oldest = self.ring.front().map_or(self.next_seq, |c| c.seq);
        if seq + 1 < oldest {
            return Since::ResyncNeeded(ResyncReason::TooOld);
        }
        Since::Changes(self.ring.iter().filter(|c| c.seq > seq).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_api::messages::{ChangeKind, DaemonStatus};

    fn kind(n: u64) -> ChangeKind {
        ChangeKind::DaemonStatus {
            daemon_id: format!("d{n}"),
            status: DaemonStatus::Online,
        }
    }

    fn log_of(n: u64, capacity: usize) -> ChangeLog {
        let mut log = ChangeLog::with_capacity(7, capacity);
        log.append((1..=n).map(kind).collect());
        log
    }

    fn seqs(since: Since) -> Vec<u64> {
        match since {
            Since::Changes(changes) => changes.iter().map(|c| c.seq).collect(),
            other => panic!("expected changes, got {other:?}"),
        }
    }

    #[test]
    fn seq_increases_by_one_from_one() {
        let mut log = ChangeLog::new(1);
        let first = log.append(vec![kind(1), kind(2)]);
        let second = log.append(vec![kind(3)]);
        let all: Vec<u64> = first.iter().chain(&second).map(|c| c.seq).collect();
        assert_eq!(all, [1, 2, 3]);
        assert_eq!(log.last_seq(), 3);
    }

    #[test]
    fn another_epoch_needs_a_resync() {
        assert_eq!(
            log_of(3, 8).since(8, 1),
            Since::ResyncNeeded(ResyncReason::EpochChanged)
        );
    }

    #[test]
    fn changes_after_n_are_served_in_order() {
        assert_eq!(seqs(log_of(5, 8).since(7, 2)), [3, 4, 5]);
    }

    #[test]
    fn n_plus_one_at_the_oldest_held_is_servable() {
        // Capacity 3 of 5: 3, 4, 5 held.
        assert_eq!(seqs(log_of(5, 3).since(7, 2)), [3, 4, 5]);
    }

    #[test]
    fn n_plus_one_below_the_oldest_held_needs_a_resync() {
        assert_eq!(
            log_of(5, 3).since(7, 1),
            Since::ResyncNeeded(ResyncReason::TooOld)
        );
    }

    #[test]
    fn n_at_the_last_is_an_empty_batch() {
        assert_eq!(seqs(log_of(5, 3).since(7, 5)), Vec::<u64>::new());
    }

    #[test]
    fn n_beyond_the_last_needs_a_resync() {
        assert_eq!(
            log_of(5, 3).since(7, 6),
            Since::ResyncNeeded(ResyncReason::Ahead)
        );
    }

    #[test]
    fn zero_on_an_empty_log_is_an_empty_batch() {
        assert_eq!(seqs(ChangeLog::new(7).since(7, 0)), Vec::<u64>::new());
    }

    #[test]
    fn zero_after_the_first_change_dropped_needs_a_resync() {
        assert_eq!(
            log_of(2, 1).since(7, 0),
            Since::ResyncNeeded(ResyncReason::TooOld)
        );
    }
}
