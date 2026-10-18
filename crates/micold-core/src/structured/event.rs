//! The structured event model (data-model.md *StructuredEvent*, FR-011).
//!
//! Every CLI-sourced variant keeps the original line in `raw`, so nothing is lost where the model
//! is narrower than the CLI (FR-011, FR-015). Typed fields are `Option` where the CLI may omit
//! them. Externally tagged (serde's default), because the wire is postcard, which cannot carry an
//! internally tagged enum.

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
        raw: String,
    },
    /// `{"type":"assistant"}`: one assistant message and its blocks.
    Assistant {
        message_id: Option<String>,
        blocks: Vec<ContentBlock>,
        parent_tool_use_id: Option<String>,
        raw: String,
    },
    /// `{"type":"user"}`: a user message; tool results arrive here.
    User {
        blocks: Vec<ContentBlock>,
        parent_tool_use_id: Option<String>,
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
    /// Valid JSON of a type the model does not know, or a known type that does not fit (FR-012).
    Unrecognised { raw: String },
    /// A line that is not JSON (FR-013).
    Unreadable { text: String },
    /// Daemon note: the process ended in the middle of a turn.
    TurnEndedWithoutResult,
    /// Wire only: stands in for an event too large for one frame (research R6).
    OversizedOnWire { original_bytes: u64 },
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequencedEvent {
    pub seq: u64,
    pub event: StructuredEvent,
}
