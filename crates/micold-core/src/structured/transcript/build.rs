//! One event → the entries it opens or changes (contracts/transcript-model.md *Event → entry
//! rules*).

use super::entry::{
    EntryKey, EntryKind, Outcome, RawReason, ToolCard, ToolResultView, ToolStatus, TurnFooter,
};
use super::{parent_of, result::result_parts, stream, summary::input_summary, Scope, Transcript};
use crate::structured::{ContentBlock, SequencedEvent, StructuredEvent};

/// How an event stands to the transcript's scope (research R7).
enum Role<'a> {
    /// The transcript's own event.
    TopLevel,
    /// A sub-agent's event, naming this parent tool use.
    Subagent(&'a str),
    /// Another scope's event: ignored.
    Foreign,
}

fn role<'a>(scope: &Scope, event: &'a StructuredEvent) -> Role<'a> {
    match (scope, parent_of(event)) {
        (Scope::Main, None | Some(None)) => Role::TopLevel,
        (Scope::Main, Some(Some(parent))) => Role::Subagent(parent),
        (Scope::Subagent(own), Some(Some(parent))) if parent == own => Role::TopLevel,
        (Scope::Subagent(_), _) => Role::Foreign,
    }
}

/// Applies one held event to the entries.
pub(super) fn push(transcript: &mut Transcript, sequenced: &SequencedEvent) {
    let seq = sequenced.seq;
    let event = &sequenced.event;
    match role(&transcript.scope, event) {
        Role::Foreign => {}
        Role::Subagent(parent) => {
            // A sub-agent's message is counted, its deltas are not (FR-024).
            if matches!(
                event,
                StructuredEvent::Assistant { .. } | StructuredEvent::User { .. }
            ) {
                count_subagent_message(transcript, seq, parent);
            }
        }
        Role::TopLevel => push_own(transcript, seq, event),
    }
}

fn push_own(transcript: &mut Transcript, seq: u64, event: &StructuredEvent) {
    let at = |block: u32| EntryKey::At { seq, block };
    match event {
        StructuredEvent::Init {
            session_id,
            model,
            cwd,
            ..
        } => {
            transcript.open(
                at(0),
                seq,
                EntryKind::SessionStart {
                    model: model.clone(),
                    cwd: cwd.clone(),
                    resumed_session: session_id.clone(),
                },
            );
        }
        StructuredEvent::Assistant {
            blocks,
            raw,
            message_id,
            ..
        } => {
            transcript.in_turn = true;
            push_blocks(
                transcript,
                seq,
                blocks,
                raw,
                Speaker::Assistant(message_id.as_deref()),
            );
        }
        StructuredEvent::User { blocks, raw, .. } => {
            transcript.in_turn = true;
            if is_interrupted_text(blocks) {
                // The CLI's own mark of an interrupt: absorbed, it marks the turn (FR-014).
                transcript.interrupted = true;
            } else {
                push_blocks(transcript, seq, blocks, raw, Speaker::User);
            }
        }
        StructuredEvent::Result {
            subtype,
            is_error,
            duration_ms,
            num_turns,
            total_cost_usd,
            result,
            ..
        } => {
            let footer = TurnFooter {
                outcome: if *is_error {
                    Outcome::Error
                } else {
                    Outcome::Success
                },
                duration_ms: *duration_ms,
                cost_usd: *total_cost_usd,
                num_turns: *num_turns,
                error_text: is_error.then(|| result.clone().unwrap_or_else(|| subtype.clone())),
            };
            stream::close(transcript);
            let kind = if transcript.interrupted {
                EntryKind::Interrupted
            } else {
                EntryKind::Footer(footer)
            };
            transcript.open(at(0), seq, kind);
            end_turn(transcript);
        }
        StructuredEvent::TurnEndedWithoutResult => {
            stream::close(transcript);
            transcript.open(at(0), seq, EntryKind::TurnEndedWithoutResult);
            end_turn(transcript);
        }
        StructuredEvent::StreamDelta { .. } => {
            transcript.in_turn = true;
            stream::push(transcript, seq, event);
        }
        StructuredEvent::UserSent { text, attachments } => {
            transcript.in_turn = true;
            let kind = EntryKind::UserMessage {
                text: text.clone(),
                attachments: attachments.clone(),
            };
            transcript.open(at(0), seq, kind);
        }
        // Marks the open turn; one written after the turn's end marks nothing (R4).
        StructuredEvent::InterruptAsked => {
            transcript.interrupted |= transcript.in_turn;
        }
        StructuredEvent::OversizedOnWire { original_bytes } => {
            transcript.open(
                at(0),
                seq,
                EntryKind::Oversized {
                    original_bytes: *original_bytes,
                },
            );
        }
        // The CLI's mode and control replies change the session, not the conversation (#942).
        // #943's compaction and clear lines and the daemon's command, model and kind notes get
        // their entries in its M2; until then they make none.
        StructuredEvent::Status { .. }
        | StructuredEvent::ControlResponse { .. }
        | StructuredEvent::CompactBoundary { .. }
        | StructuredEvent::ConversationReset { .. }
        | StructuredEvent::CommandsReported { .. }
        | StructuredEvent::ModelChanged { .. }
        | StructuredEvent::ModelRefused { .. }
        | StructuredEvent::KindSwitched { .. } => {}
        StructuredEvent::Unrecognised { raw } => {
            transcript.open(at(0), seq, raw_row(raw, RawReason::Unrecognised));
        }
        StructuredEvent::Unreadable { text } => {
            transcript.open(at(0), seq, raw_row(text, RawReason::Unreadable));
        }
    }
}

/// The CLI's user line for an interrupted turn (P-4): one text block,
/// `[Request interrupted by user]` (or a longer form of it, such as `… for tool use]`).
fn is_interrupted_text(blocks: &[ContentBlock]) -> bool {
    matches!(blocks, [ContentBlock::Text { text }]
        if text.starts_with("[Request interrupted by user") && text.ends_with(']'))
}

fn raw_row(text: &str, reason: RawReason) -> EntryKind {
    EntryKind::Raw {
        text: text.to_string(),
        reason,
    }
}

#[derive(Clone, Copy)]
enum Speaker<'a> {
    User,
    /// The message's id, when known.
    Assistant(Option<&'a str>),
}

/// One entry per block, keyed by the block's position, except a tool result, which goes into its
/// card. An assistant block takes the place of the open stream's next streamed block, when it has
/// one (research R3).
fn push_blocks(
    transcript: &mut Transcript,
    seq: u64,
    blocks: &[ContentBlock],
    raw: &str,
    speaker: Speaker,
) {
    for (index, block) in blocks.iter().enumerate() {
        let key = EntryKey::At {
            seq,
            block: u32::try_from(index).unwrap_or(u32::MAX),
        };
        let streamed = match speaker {
            Speaker::Assistant(message_id) if !matches!(block, ContentBlock::ToolResult { .. }) => {
                stream::supersede(transcript, message_id)
            }
            _ => None,
        };
        match block {
            ContentBlock::Text { text } => {
                let text = text.clone();
                let kind = match speaker {
                    Speaker::User => EntryKind::UserMessage {
                        text,
                        attachments: Vec::new(),
                    },
                    Speaker::Assistant(_) => EntryKind::AssistantText {
                        text,
                        streaming: false,
                    },
                };
                open_or_replace(transcript, streamed, key, seq, kind);
            }
            ContentBlock::Thinking { thinking } => {
                let kind = EntryKind::Thinking {
                    text: thinking.clone(),
                    streaming: false,
                };
                open_or_replace(transcript, streamed, key, seq, kind);
            }
            ContentBlock::ToolUse { id, name, input } => {
                push_tool_use(transcript, seq, id, name, input);
            }
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                let parts = result_parts(raw, index, content);
                push_tool_result(transcript, key, seq, tool_use_id, parts, *is_error);
            }
            ContentBlock::Other { raw } => {
                transcript.open(key, seq, raw_row(raw, RawReason::UnknownBlock));
            }
        }
    }
}

/// Puts `kind` in the place of the streamed entry `streamed` when that entry is of its kind,
/// keeping its key and place (FR-015); else opens it as a new entry.
fn open_or_replace(
    transcript: &mut Transcript,
    streamed: Option<super::entry::EntryId>,
    key: EntryKey,
    seq: u64,
    kind: EntryKind,
) {
    let same_kind = streamed
        .and_then(|id| transcript.get(id))
        .is_some_and(|e| std::mem::discriminant(e.kind()) == std::mem::discriminant(&kind));
    match streamed {
        Some(id) if same_kind => transcript.change(id, |old| *old = kind),
        _ => {
            transcript.open(key, seq, kind);
        }
    }
}

/// Opens the card of tool use `id`, or completes the one already held (FR-007).
fn push_tool_use(transcript: &mut Transcript, seq: u64, id: &str, name: &str, input: &str) {
    let summary = input_summary(name, input);
    if let Some(&card) = transcript.tools.get(id) {
        transcript.change(card, |kind| {
            if let EntryKind::ToolCall(card) = kind {
                card.name = name.to_string();
                card.input = input.to_string();
                card.summary = summary;
                card.streaming = false;
            }
        });
        return;
    }
    let card = ToolCard {
        id: id.to_string(),
        name: name.to_string(),
        input: input.to_string(),
        summary,
        status: ToolStatus::Running,
        result: None,
        subagent_messages: 0,
        streaming: false,
    };
    let entry = transcript.open(
        EntryKey::Tool(id.to_string()),
        seq,
        EntryKind::ToolCall(card),
    );
    transcript.tools.insert(id.to_string(), entry);
    transcript.turn_cards.push(entry);
}

/// Fills the card of `tool_use_id` in place (FR-008), or shows the result without its call.
fn push_tool_result(
    transcript: &mut Transcript,
    key: EntryKey,
    seq: u64,
    tool_use_id: &str,
    parts: Vec<super::entry::ResultPart>,
    is_error: bool,
) {
    match transcript.tools.get(tool_use_id) {
        Some(&card) => transcript.change(card, |kind| {
            if let EntryKind::ToolCall(card) = kind {
                card.status = if is_error {
                    ToolStatus::Failed
                } else {
                    ToolStatus::Done
                };
                card.result = Some(ToolResultView { parts, is_error });
            }
        }),
        None => {
            transcript.open(
                key,
                seq,
                EntryKind::OrphanResult {
                    tool_use_id: tool_use_id.to_string(),
                    parts,
                    is_error,
                },
            );
        }
    }
}

/// The turn ended: its cards still running will get no result.
fn end_turn(transcript: &mut Transcript) {
    transcript.in_turn = false;
    transcript.interrupted = false;
    for card in std::mem::take(&mut transcript.turn_cards) {
        let running = matches!(
            transcript.get(card).map(|e| e.kind()),
            Some(EntryKind::ToolCall(ToolCard {
                status: ToolStatus::Running,
                ..
            }))
        );
        if running {
            transcript.change(card, |kind| {
                if let EntryKind::ToolCall(card) = kind {
                    card.status = ToolStatus::NoResult;
                }
            });
        }
    }
}

/// One more sub-agent message: on the card it names, else on the transcript's single notice,
/// opened in the place of the first (FR-024).
fn count_subagent_message(transcript: &mut Transcript, seq: u64, parent: &str) {
    if let Some(&card) = transcript.tools.get(parent) {
        transcript.change(card, |kind| {
            if let EntryKind::ToolCall(card) = kind {
                card.subagent_messages += 1;
            }
        });
        return;
    }
    match transcript.subagent_notice {
        Some(notice) => transcript.change(notice, |kind| {
            if let EntryKind::SubagentNotice { count } = kind {
                *count += 1;
            }
        }),
        None => {
            let notice = transcript.open(
                EntryKey::At { seq, block: 0 },
                seq,
                EntryKind::SubagentNotice { count: 1 },
            );
            transcript.subagent_notice = Some(notice);
        }
    }
}
