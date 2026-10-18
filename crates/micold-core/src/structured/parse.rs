//! One line in, one event out (research R9).

use super::event::{SequencedEvent, StructuredEvent};

/// Turn one line of the CLI's stdout into its event. Never fails (FR-014).
pub fn parse_line(line: &str) -> StructuredEvent {
    StructuredEvent::Unreadable {
        text: line.to_string(),
    }
}

/// Each tool call's `seq`, with the `seq` of the event carrying its result, if one does.
pub fn pair_tool_calls(_events: &[SequencedEvent]) -> Vec<(u64, Option<u64>)> {
    Vec::new()
}
