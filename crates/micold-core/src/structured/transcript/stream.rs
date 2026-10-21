//! The assistant message being streamed (research R3, data-model.md *OpenStream*): its blocks grow
//! from their deltas, and the finished message's blocks take their places in order.

use super::entry::{Entry, EntryId, EntryKey, EntryKind, RawReason, ToolCard, ToolStatus};
use super::{summary::input_summary, Transcript};
use crate::structured::{Delta, StructuredEvent};
use serde_json::Value;

/// The message whose partial events are arriving.
#[derive(Debug, Clone)]
pub(super) struct OpenStream {
    /// The message's id, from `message_start`; a finished message supersedes this stream's
    /// blocks when its id matches or either is unknown.
    message_id: Option<String>,
    /// In `content_block_start` order.
    blocks: Vec<StreamedBlock>,
}

#[derive(Debug, Clone)]
struct StreamedBlock {
    index: u64,
    /// `None` for a block of a type the transcript does not stream: its finished block is shown.
    entry: Option<EntryId>,
    /// A finished block took its place: its deltas no longer apply.
    finished: bool,
}

/// Takes one top-level `StreamDelta` (contracts/transcript-model.md *Event → entry rules*).
pub(super) fn push(transcript: &mut Transcript, seq: u64, event: &StructuredEvent) {
    let StructuredEvent::StreamDelta {
        event_type,
        index,
        delta,
        raw,
        ..
    } = event
    else {
        return;
    };
    match (event_type.as_str(), delta) {
        ("message_start", _) => {
            close(transcript);
            let message_id = line_of(raw)["event"]["message"]["id"]
                .as_str()
                .map(str::to_string);
            transcript.stream = Some(OpenStream {
                message_id,
                blocks: Vec::new(),
            });
        }
        ("content_block_start", _) => start_block(transcript, seq, index.unwrap_or(0), raw),
        ("content_block_delta", Some(Delta::Other { raw: other })) => {
            let signature = line_of(other)["type"].as_str() == Some("signature_delta");
            if !signature {
                unknown(transcript, seq, raw);
            }
        }
        ("content_block_delta", Some(delta)) => grow(transcript, index.unwrap_or(0), delta),
        ("content_block_stop" | "message_delta" | "ping", _) => {}
        ("message_stop", _) => close(transcript),
        _ => unknown(transcript, seq, raw),
    }
}

fn line_of(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn unknown(transcript: &mut Transcript, seq: u64, raw: &str) {
    transcript.open(
        EntryKey::At { seq, block: 0 },
        seq,
        EntryKind::Raw {
            text: raw.to_string(),
            reason: RawReason::UnknownStreamEvent,
        },
    );
}

/// Creates the block's entry at once, from the start event's `content_block`.
fn start_block(transcript: &mut Transcript, seq: u64, index: u64, raw: &str) {
    let line = line_of(raw);
    let block = &line["event"]["content_block"];
    let text = |key: &str| block[key].as_str().unwrap_or_default().to_string();
    let key = EntryKey::At { seq, block: 0 };
    let entry = match (block["type"].as_str(), block["id"].as_str()) {
        (Some("text"), _) => Some(transcript.open(
            key,
            seq,
            EntryKind::AssistantText {
                text: text("text"),
                streaming: true,
            },
        )),
        (Some("thinking"), _) => Some(transcript.open(
            key,
            seq,
            EntryKind::Thinking {
                text: text("thinking"),
                streaming: true,
            },
        )),
        (Some("tool_use"), Some(id)) => Some(start_card(transcript, seq, id, &text("name"))),
        _ => None,
    };
    transcript
        .stream
        .get_or_insert_with(|| OpenStream {
            message_id: None,
            blocks: Vec::new(),
        })
        .blocks
        .push(StreamedBlock {
            index,
            entry,
            finished: false,
        });
}

/// A `Running` card whose input is still arriving.
fn start_card(transcript: &mut Transcript, seq: u64, id: &str, name: &str) -> EntryId {
    if let Some(&card) = transcript.tools.get(id) {
        return card;
    }
    let card = ToolCard {
        id: id.to_string(),
        name: name.to_string(),
        input: String::new(),
        summary: input_summary(name, ""),
        status: ToolStatus::Running,
        result: None,
        subagent_messages: 0,
        streaming: true,
    };
    let entry = transcript.open(
        EntryKey::Tool(id.to_string()),
        seq,
        EntryKind::ToolCall(card),
    );
    transcript.tools.insert(id.to_string(), entry);
    transcript.turn_cards.push(entry);
    entry
}

/// Appends a delta to the streamed entry of its `index`; a delta for no such entry, of another
/// kind than its entry, or adding nothing is absorbed.
fn grow(transcript: &mut Transcript, index: u64, delta: &Delta) {
    let Some(id) = transcript.stream.as_ref().and_then(|stream| {
        stream
            .blocks
            .iter()
            .rev()
            .find(|b| b.index == index && !b.finished)
            .and_then(|b| b.entry)
    }) else {
        return;
    };
    let more = match delta {
        Delta::Text { text } | Delta::Thinking { thinking: text } => text,
        Delta::InputJson { partial_json } => partial_json,
        Delta::Other { .. } => return,
    };
    let grows = match (transcript.get(id).map(Entry::kind), delta) {
        (Some(kind @ EntryKind::AssistantText { .. }), Delta::Text { .. })
        | (Some(kind @ EntryKind::Thinking { .. }), Delta::Thinking { .. })
        | (Some(kind @ EntryKind::ToolCall(_)), Delta::InputJson { .. }) => is_streaming(kind),
        _ => false,
    };
    if more.is_empty() || !grows {
        return;
    }
    transcript.change(id, |kind| match kind {
        EntryKind::AssistantText { text, .. } | EntryKind::Thinking { text, .. } => {
            text.push_str(more);
        }
        EntryKind::ToolCall(card) => {
            card.input.push_str(more);
            card.summary = input_summary(&card.name, &card.input);
        }
        _ => {}
    });
}
/// The streamed entry a finished top-level assistant block of message `message_id` takes the
/// place of: the open stream's next unfinished block, which is finished from now on. `None` when
/// no stream is open, it streams another message, every streamed block is finished, or that block
/// has no entry.
pub(super) fn supersede(transcript: &mut Transcript, message_id: Option<&str>) -> Option<EntryId> {
    let stream = transcript.stream.as_mut()?;
    let same = match (stream.message_id.as_deref(), message_id) {
        (Some(open), Some(finished)) => open == finished,
        _ => true,
    };
    if !same {
        return None;
    }
    let block = stream.blocks.iter_mut().find(|b| !b.finished)?;
    block.finished = true;
    block.entry
}

/// Closes the open stream: its blocks never finished keep their partial content, and every
/// block still streaming stops (one superseded by a finished block of another kind included).
pub(super) fn close(transcript: &mut Transcript) {
    let Some(stream) = transcript.stream.take() else {
        return;
    };
    for id in stream.blocks.iter().filter_map(|b| b.entry) {
        if transcript
            .get(id)
            .map(Entry::kind)
            .is_some_and(is_streaming)
        {
            transcript.change(id, |kind| {
                if let Some(streaming) = streaming_flag(kind) {
                    *streaming = false;
                }
            });
        }
    }
}

fn is_streaming(kind: &EntryKind) -> bool {
    match kind {
        EntryKind::AssistantText { streaming, .. }
        | EntryKind::Thinking { streaming, .. }
        | EntryKind::ToolCall(ToolCard { streaming, .. }) => *streaming,
        _ => false,
    }
}

fn streaming_flag(kind: &mut EntryKind) -> Option<&mut bool> {
    match kind {
        EntryKind::AssistantText { streaming, .. }
        | EntryKind::Thinking { streaming, .. }
        | EntryKind::ToolCall(ToolCard { streaming, .. }) => Some(streaming),
        _ => None,
    }
}
