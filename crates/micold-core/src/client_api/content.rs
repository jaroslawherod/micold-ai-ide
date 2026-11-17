//! A structured session's history as [`ContentEntry`] rows for `read_session` (FR-002): prose,
//! tool calls and results, and turn outcomes. Thinking, deltas and control traffic are left out;
//! nothing here carries terminal bytes (FR-006).

use super::messages::{ContentEntry, EntryKind, Role};
use crate::structured::{ContentBlock, SequencedEvent, StructuredEvent};

/// The entries of `events`, numbered from 1.
pub fn entries(events: &[SequencedEvent]) -> Vec<ContentEntry> {
    let mut out = Vec::new();
    let mut push = |role, kind, text: String| {
        let seq = out.len() as u64 + 1;
        out.push(ContentEntry::new(seq, role, kind, text));
    };
    for sequenced in events {
        match &sequenced.event {
            StructuredEvent::UserSent { text, .. } => {
                push(Role::User, EntryKind::Text, text.clone())
            }
            StructuredEvent::User { blocks, .. } => {
                for (kind, text) in blocks.iter().filter_map(block) {
                    let role = if kind == EntryKind::Text {
                        Role::User
                    } else {
                        Role::System
                    };
                    push(role, kind, text);
                }
            }
            StructuredEvent::Assistant { blocks, .. } => {
                for (kind, text) in blocks.iter().filter_map(block) {
                    push(Role::Assistant, kind, text);
                }
            }
            StructuredEvent::Result {
                subtype, result, ..
            } => push(
                Role::System,
                EntryKind::Status,
                result.clone().unwrap_or_else(|| subtype.clone()),
            ),
            StructuredEvent::Unreadable { text } => {
                push(Role::System, EntryKind::Status, text.clone())
            }
            _ => {}
        }
    }
    out
}

/// The entry kind and text of a content block worth showing.
fn block(block: &ContentBlock) -> Option<(EntryKind, String)> {
    match block {
        ContentBlock::Text { text } => Some((EntryKind::Text, text.clone())),
        ContentBlock::ToolUse { name, input, .. } => {
            Some((EntryKind::ToolCall, format!("{name} {input}")))
        }
        ContentBlock::ToolResult { content, .. } => Some((EntryKind::ToolResult, content.clone())),
        ContentBlock::Thinking { .. } | ContentBlock::Other { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seq: u64, event: StructuredEvent) -> SequencedEvent {
        SequencedEvent { seq, event }
    }

    #[test]
    fn prose_tools_and_outcomes_become_entries_in_order() {
        let events = [
            at(
                1,
                StructuredEvent::UserSent {
                    text: "hi".into(),
                    attachments: vec![],
                },
            ),
            at(
                2,
                StructuredEvent::Assistant {
                    message_id: None,
                    blocks: vec![
                        ContentBlock::Thinking {
                            thinking: "hm".into(),
                        },
                        ContentBlock::Text {
                            text: "hello".into(),
                        },
                        ContentBlock::ToolUse {
                            id: "t".into(),
                            name: "Read".into(),
                            input: "{}".into(),
                        },
                    ],
                    parent_tool_use_id: None,
                    model: None,
                    error: None,
                    raw: String::new(),
                },
            ),
            at(3, StructuredEvent::InterruptAsked),
            at(
                4,
                StructuredEvent::Unreadable {
                    text: "line".into(),
                },
            ),
        ];
        let got: Vec<_> = entries(&events)
            .into_iter()
            .map(|e| (e.seq, e.role, e.kind, e.text))
            .collect();
        assert_eq!(
            got,
            [
                (1, Role::User, EntryKind::Text, "hi".to_owned()),
                (2, Role::Assistant, EntryKind::Text, "hello".to_owned()),
                (
                    3,
                    Role::Assistant,
                    EntryKind::ToolCall,
                    "Read {}".to_owned()
                ),
                (4, Role::System, EntryKind::Status, "line".to_owned()),
            ]
        );
    }
}
