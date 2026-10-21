//! The replace-or-continue rule of a `StructuredEvents` message (#938 wire.md), shared by the
//! transcript (#939) and the session insights (#944) so both hold the same events (research R1).

use super::SequencedEvent;

/// What a receiver of `StructuredEvents` messages remembers to decide whether the next message
/// replaces what it holds or continues it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Continuity {
    /// One past the last `seq` received; `0` before any.
    next_seq: u64,
    /// Whether the last message was a live one.
    last_live: bool,
    /// Whether anything is held: an event, or a count of omitted ones.
    holds_anything: bool,
}

impl Continuity {
    /// Takes one message and says whether it **replaces** what is held (`true`) or appends to it.
    /// A `live: false` message replaces unless it continues the history in progress (the last
    /// message was not live, something is held, and its first `seq` is the next one); an empty
    /// one replaces too. A `live: true` message appends. Call it before appending the events.
    pub(crate) fn take(&mut self, omitted: u64, events: &[SequencedEvent], live: bool) -> bool {
        let continues = events.first().is_some_and(|e| e.seq == self.next_seq);
        let replaces = !live && !(continues && !self.last_live && self.holds_anything);
        if replaces {
            *self = Self {
                holds_anything: omitted > 0,
                ..Self::default()
            };
        }
        self.last_live = live;
        if let Some(last) = events.last() {
            self.next_seq = last.seq + 1;
            self.holds_anything = true;
        }
        replaces
    }

    /// One past the last `seq` received.
    pub(crate) fn next_seq(&self) -> u64 {
        self.next_seq
    }
}

#[cfg(test)]
mod tests {
    use super::Continuity;
    use crate::structured::{SequencedEvent, StructuredEvent};

    fn seqs(seqs: std::ops::Range<u64>) -> Vec<SequencedEvent> {
        seqs.map(|seq| SequencedEvent {
            seq,
            event: StructuredEvent::TurnEndedWithoutResult,
        })
        .collect()
    }

    #[test]
    fn an_empty_history_message_replaces() {
        let mut sync = Continuity::default();
        sync.take(0, &seqs(0..3), false);
        assert!(sync.take(0, &[], false));
        assert_eq!(sync.next_seq(), 0);
    }

    #[test]
    fn a_history_message_continuing_a_held_history_continues() {
        let mut sync = Continuity::default();
        assert!(sync.take(0, &seqs(0..3), false));
        assert!(!sync.take(0, &seqs(3..5), false));
        assert_eq!(sync.next_seq(), 5);
    }

    #[test]
    fn a_history_message_continuing_only_an_omitted_count_continues() {
        let mut sync = Continuity::default();
        sync.take(7, &[], false);
        assert!(!sync.take(7, &seqs(0..2), false));
    }

    #[test]
    fn a_history_message_that_does_not_continue_replaces() {
        let mut sync = Continuity::default();
        sync.take(0, &seqs(0..3), false);
        assert!(sync.take(0, &seqs(4..6), false), "a gap");
        assert!(sync.take(0, &seqs(0..2), false), "a restart from 0");
        sync.take(0, &seqs(2..3), true);
        assert!(
            sync.take(0, &seqs(3..4), false),
            "after a live message a history message is a new history"
        );
    }

    #[test]
    fn a_history_message_after_an_empty_history_replaces() {
        let mut sync = Continuity::default();
        sync.take(0, &[], false);
        assert!(sync.take(0, &seqs(0..1), false));
    }

    #[test]
    fn a_live_message_continues() {
        let mut sync = Continuity::default();
        sync.take(0, &seqs(0..3), false);
        assert!(!sync.take(0, &seqs(3..4), true));
        assert!(!sync.take(0, &seqs(9..10), true), "even past a gap");
        assert!(!Continuity::default().take(0, &seqs(0..1), true));
    }

    #[test]
    fn next_seq_follows_the_last_event() {
        let mut sync = Continuity::default();
        sync.take(0, &seqs(0..3), false);
        sync.take(0, &[], true);
        assert_eq!(sync.next_seq(), 3);
        sync.take(0, &seqs(3..8), true);
        assert_eq!(sync.next_seq(), 8);
    }
}
