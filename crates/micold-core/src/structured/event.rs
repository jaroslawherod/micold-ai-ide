//! The structured event model (data-model.md *StructuredEvent*, FR-011).
//!
//! Every CLI-sourced variant keeps the original line in `raw`, so nothing is lost where the model
//! is narrower than the CLI (FR-011, FR-015). Typed fields are `Option` where the CLI may omit
//! them. Externally tagged (serde's default), because the wire is postcard, which cannot carry an
//! internally tagged enum.

use crate::session::SessionKind;
use serde::{Deserialize, Serialize};

/// One line the CLI wrote, typed; or a note the daemon adds (FR-014).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StructuredEvent {
    /// `{"type":"system","subtype":"init"}`: the session the CLI runs.
    Init {
        session_id: String,
        model: Option<String>,
        cwd: Option<String>,
        tools: Vec<String>,
        /// `permissionMode`: the mode the CLI started in (#942 FR-016).
        permission_mode: Option<String>,
        /// `slash_commands`: every command name the CLI offers (#943 R1); absent → empty.
        slash_commands: Vec<String>,
        /// `skills`: the names among them that are skills (#943 R1); absent → empty.
        skills: Vec<String>,
        /// `terminal_slash_commands`: the names the CLI marks terminal-only (#943 R2).
        terminal_slash_commands: Vec<String>,
        raw: String,
    },
    /// `{"type":"assistant"}`: one assistant message and its blocks.
    Assistant {
        message_id: Option<String>,
        blocks: Vec<ContentBlock>,
        parent_tool_use_id: Option<String>,
        /// `message.model`: the model that answered; `"<synthetic>"` for an answer the CLI made
        /// itself, such as a built-in command's output (#943 R4).
        model: Option<String>,
        /// The top-level `error`, such as `"authentication_failed"` (#943 R9).
        error: Option<String>,
        raw: String,
    },
    /// `{"type":"user"}`: a user message; tool results arrive here.
    User {
        blocks: Vec<ContentBlock>,
        parent_tool_use_id: Option<String>,
        /// `isSynthetic`: the CLI wrote the message itself, as the summary after a compaction
        /// (#943 R5).
        is_synthetic: bool,
        raw: String,
    },
    /// `{"type":"result"}`: the turn ended.
    Result {
        subtype: String,
        is_error: bool,
        duration_ms: Option<u64>,
        num_turns: Option<u64>,
        total_cost_usd: Option<f64>,
        usage: Option<Usage>,
        result: Option<String>,
        session_id: Option<String>,
        raw: String,
    },
    /// `{"type":"stream_event"}`: a partial-message event (`content_block_delta`, ...).
    StreamDelta {
        event_type: String,
        index: Option<u64>,
        delta: Option<Delta>,
        parent_tool_use_id: Option<String>,
        raw: String,
    },
    /// `{"type":"system","subtype":"status"}`: the CLI's state changed; `permissionMode` when the
    /// change is a mode change (#942 FR-016, research P-3, P-9).
    Status {
        permission_mode: Option<String>,
        raw: String,
    },
    /// `{"type":"control_response"}`: the CLI's answer to the control request `request_id` the
    /// daemon wrote to its stdin (#942 FR-017).
    ControlResponse {
        request_id: String,
        outcome: ControlOutcome,
        raw: String,
    },
    /// `{"type":"system","subtype":"compact_boundary"}`: the conversation was compacted here; its
    /// `compact_metadata` (#943 R5).
    CompactBoundary {
        trigger: Option<String>,
        pre_tokens: Option<u64>,
        post_tokens: Option<u64>,
        raw: String,
    },
    /// `{"type":"conversation_reset"}`: the conversation started over (`/clear`, #943 R5).
    ConversationReset {
        trigger: Option<String>,
        raw: String,
    },
    /// Valid JSON of a type the model does not know, or a known type that does not fit (FR-012).
    Unrecognised { raw: String },
    /// A line that is not JSON (FR-013).
    Unreadable { text: String },
    /// Daemon note: the process ended in the middle of a turn.
    TurnEndedWithoutResult,
    /// Wire only: stands in for an event too large for one frame (research R6).
    OversizedOnWire { original_bytes: u64 },
    /// Daemon note (feature 941): a user message was written to the CLI's stdin. `attachments`
    /// names its pictures. Appended, so files saved before it decode unchanged.
    UserSent {
        text: String,
        attachments: Vec<String>,
    },
    /// Daemon note (feature 941): an interrupt request was written for the open turn.
    InterruptAsked,
    /// Daemon note: the commands and models the CLI's `initialize` answer reported at a spawn
    /// (#943 R1). Never produced by [`parse_line`](super::parse_line).
    CommandsReported {
        commands: Vec<ReportedCommand>,
        models: Vec<ReportedModel>,
    },
    /// Daemon note: the CLI accepted a model change; `model` is the name it resolved (#943 R7).
    ModelChanged { model: String },
    /// Daemon note: the CLI refused a deferred model change, with its reason (#943 R7).
    ModelRefused { model: String, message: String },
    /// Daemon note: the session switched kind here (#943 R8).
    KindSwitched { to: SessionKind },
}

/// `PartialEq` is total here: the only float, `total_cost_usd`, comes from JSON, which has no NaN,
/// so the wire messages carrying events can stay `Eq` as every `DaemonMsg` is.
impl Eq for StructuredEvent {}

impl StructuredEvent {
    /// The length of the line text the event keeps (`raw`, or an unreadable line's `text`); `0`
    /// for the daemon's notes but `UserSent`, which counts its text. What a history's byte bound counts (research R5).
    pub fn text_len(&self) -> usize {
        match self {
            Self::Init { raw, .. }
            | Self::Assistant { raw, .. }
            | Self::User { raw, .. }
            | Self::Result { raw, .. }
            | Self::StreamDelta { raw, .. }
            | Self::Status { raw, .. }
            | Self::ControlResponse { raw, .. }
            | Self::CompactBoundary { raw, .. }
            | Self::ConversationReset { raw, .. }
            | Self::Unrecognised { raw } => raw.len(),
            Self::Unreadable { text } | Self::UserSent { text, .. } => text.len(),
            Self::TurnEndedWithoutResult
            | Self::OversizedOnWire { .. }
            | Self::InterruptAsked
            | Self::CommandsReported { .. }
            | Self::ModelChanged { .. }
            | Self::ModelRefused { .. }
            | Self::KindSwitched { .. } => 0,
        }
    }
}

/// One command of the CLI's `initialize` answer (#943 R1, contracts/claude-commands.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedCommand {
    pub name: String,
    pub description: Option<String>,
    /// `argumentHint`; `None` when the CLI sent none or an empty one.
    pub argument_hint: Option<String>,
    pub aliases: Vec<String>,
    /// `builtin: true`: one of the CLI's own commands.
    pub builtin: bool,
}

/// One model of the CLI's `initialize` answer (#943 R7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedModel {
    /// What `set_model` and `--model` take (`default`, `haiku`, …).
    pub value: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

/// How the CLI answered a control request (contracts/claude-permission.md *Mode change*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlOutcome {
    /// `subtype: success`; the `response` object's JSON text (`null` when it sent none), as JSON
    /// text because the wire is postcard, which cannot carry a `serde_json::Value`.
    Success(String),
    /// `subtype: error`: the CLI's `error` text and its `error_code`
    /// (`bypass_not_launched`, research P-9).
    Error {
        message: String,
        code: Option<String>,
    },
}

/// One block inside a message (FR-011).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    /// A tool call; `input` is its JSON text.
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    /// A tool's answer to the call `tool_use_id`; `content` is its text, or the JSON text of a
    /// block list.
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    /// A block of a type the model does not know, kept whole as JSON text.
    Other {
        raw: String,
    },
}

/// Token counts of a turn, as far as the CLI reports them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

/// The `delta` of a partial-message event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Delta {
    /// `text_delta`: more answer text.
    Text { text: String },
    /// `thinking_delta`: more thinking text.
    Thinking { thinking: String },
    /// `input_json_delta`: more of a tool call's input.
    InputJson { partial_json: String },
    /// Any other delta, kept whole as JSON text.
    Other { raw: String },
}

/// An event and its place in the session's history (data-model *SequencedEvent*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequencedEvent {
    pub seq: u64,
    pub event: StructuredEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sent_message_counts_its_text_and_an_interrupt_note_counts_nothing() {
        let sent = StructuredEvent::UserSent {
            text: "héllo".into(),
            attachments: vec!["a-very-long-picture-name.png".into()],
        };
        assert_eq!(sent.text_len(), "héllo".len());
        assert_eq!(StructuredEvent::InterruptAsked.text_len(), 0);
    }
}
