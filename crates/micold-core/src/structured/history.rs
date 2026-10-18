//! A structured session's event history (data-model *EventHistory*, research R5, FR-018).
//!
//! Kept by the daemon in memory, per session, outside the live process, so a stop and start
//! within one daemon keeps it (FR-021a). Bounded by both an event count and the retained line
//! text: a count alone does not bound multi-MiB tool results, bytes alone let thousands of tiny
//! deltas pile up. Eviction drops the oldest events and counts them in `omitted`.

use std::collections::VecDeque;

use super::{SequencedEvent, StructuredEvent};

/// The most events a history retains.
pub const MAX_EVENTS: usize = 5_000;
/// The most line text, in bytes, a history retains.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;

/// The bounded, sequenced history of one structured session.
#[derive(Debug)]
pub struct EventHistory {
    events: VecDeque<SequencedEvent>,
    next_seq: u64,
    bytes: usize,
    omitted: u64,
    max_events: usize,
    max_bytes: usize,
}

impl Default for EventHistory {
    fn default() -> Self {
        Self::with_bounds(MAX_EVENTS, MAX_BYTES)
    }
}

impl EventHistory {
    /// An empty history with the default bounds.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty history bounded by `max_events` events and `max_bytes` bytes of line text.
    pub fn with_bounds(max_events: usize, max_bytes: usize) -> Self {
        Self {
            events: VecDeque::new(),
            next_seq: 0,
            bytes: 0,
            omitted: 0,
            max_events,
            max_bytes,
        }
    }

    /// Append `event` under the next `seq`, evict the oldest events while over a bound, and
    /// return the sequenced event.
    pub fn append(&mut self, event: StructuredEvent) -> SequencedEvent {
        let sequenced = SequencedEvent {
            seq: self.next_seq,
            event,
        };
        self.next_seq += 1;
        self.bytes += sequenced.event.text_len();
        self.events.push_back(sequenced.clone());
        while self.events.len() > self.max_events || self.bytes > self.max_bytes {
            let Some(old) = self.events.pop_front() else {
                break;
            };
            self.bytes -= old.event.text_len();
            self.omitted += 1;
        }
        sequenced
    }

    /// How many events were evicted, and the retained events, oldest first.
    pub fn snapshot(&self) -> (u64, Vec<SequencedEvent>) {
        (self.omitted, self.events.iter().cloned().collect())
    }

    /// The retained line text, in bytes.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// How many events were evicted.
    pub fn omitted(&self) -> u64 {
        self.omitted
    }

    /// How many events are retained.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether no event is retained.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> StructuredEvent {
        StructuredEvent::Unreadable {
            text: text.to_owned(),
        }
    }

    fn seqs(history: &EventHistory) -> Vec<u64> {
        history.snapshot().1.iter().map(|e| e.seq).collect()
    }

    #[test]
    fn append_assigns_seq_from_zero_strictly_increasing() {
        let mut history = EventHistory::new();
        let first = history.append(line("a"));
        let second = history.append(line("b"));
        let third = history.append(StructuredEvent::TurnEndedWithoutResult);
        assert_eq!((first.seq, second.seq, third.seq), (0, 1, 2));
        assert_eq!(seqs(&history), vec![0, 1, 2]);
        assert_eq!(history.snapshot().1[1].event, line("b"));
    }

    #[test]
    fn the_oldest_events_are_evicted_past_the_event_bound_and_counted() {
        let mut history = EventHistory::with_bounds(3, usize::MAX);
        for text in ["a", "b", "c", "d", "e"] {
            history.append(line(text));
        }
        let (omitted, events) = history.snapshot();
        assert_eq!(omitted, 2);
        assert_eq!(
            events.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
        assert_eq!(events[0].event, line("c"));
    }

    #[test]
    fn the_oldest_events_are_evicted_past_the_byte_bound_and_counted() {
        let mut history = EventHistory::with_bounds(usize::MAX, 10);
        history.append(line("aaaa"));
        history.append(line("bbbb"));
        assert_eq!((history.bytes(), history.omitted()), (8, 0));
        history.append(line("cccc"));
        assert_eq!((history.bytes(), history.omitted()), (8, 1));
        assert_eq!(seqs(&history), vec![1, 2]);
    }

    #[test]
    fn bytes_track_the_retained_line_text_of_every_variant() {
        let mut history = EventHistory::new();
        history.append(line("12345"));
        history.append(StructuredEvent::Unrecognised {
            raw: "{\"x\":1}".to_owned(),
        });
        history.append(StructuredEvent::TurnEndedWithoutResult);
        assert_eq!(history.bytes(), 5 + 7);
    }

    #[test]
    fn an_event_larger_than_the_byte_bound_evicts_everything_including_itself() {
        let mut history = EventHistory::with_bounds(usize::MAX, 4);
        history.append(line("ab"));
        history.append(line("too long"));
        assert_eq!(
            (history.len(), history.bytes(), history.omitted()),
            (0, 0, 2)
        );
        assert_eq!(history.append(line("c")).seq, 2);
    }

    #[test]
    fn the_default_bounds_are_five_thousand_events_and_64_mib() {
        assert_eq!(MAX_EVENTS, 5_000);
        assert_eq!(MAX_BYTES, 64 * 1024 * 1024);
        let mut history = EventHistory::new();
        for _ in 0..MAX_EVENTS + 1 {
            history.append(line(""));
        }
        assert_eq!((history.len(), history.omitted()), (MAX_EVENTS, 1));
    }
}
