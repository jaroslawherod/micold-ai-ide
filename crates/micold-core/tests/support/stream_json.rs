//! Feature 939 T002 — the stream-json fixture helpers shared by `structured_parse.rs` and
//! `structured_transcript.rs` (each `#[path]`-includes this file), and a builder of synthetic
//! events for cases the recordings do not hold.

#![allow(dead_code)]

use micold_core::structured::{
    parse_line, ContentBlock, LineSplitter, SequencedEvent, StructuredEvent,
};
use serde_json::{json, Value};

/// The fixtures recorded from a real `claude` (`tests/fixtures/stream-json/README.md`); the last
/// four are feature 944's, with sub-agents and background tasks (944 research R11).
pub const RECORDED: [&str; 9] = [
    "text-turn",
    "tool-turn",
    "partial-deltas",
    "error-result",
    "resume-init",
    "subagent-turn",
    "nested-subagent",
    "background-command",
    "background-subagent",
];

/// The fixtures feature 941 recorded: an interrupted turn (P-4) and a message with a picture (P-5).
pub const RECORDED_941: [&str; 2] = ["interrupt-turn", "image-turn"];

/// #943's recordings: commands, model changes, an authentication failure and a resume after
/// `/clear` (`tests/fixtures/stream-json/README.md`, *Commands, models and authentication*).
pub const COMMAND_RECORDED: [&str; 11] = [
    "command-initialize",
    "command-skill",
    "command-custom",
    "command-compact",
    "command-clear",
    "command-builtin-output",
    "command-unknown",
    "model-change",
    "model-refused",
    "auth-failure",
    "resume-after-clear",
];

/// The stdout of the permission-prompt recordings (#942,
/// `tests/fixtures/permission-prompt/README.md`).
pub const PROMPT_RECORDED: [&str; 6] = [
    "prompt-tool-request",
    "prompt-always-allow",
    "prompt-plan-approval",
    "prompt-question-multi",
    "prompt-mode-change",
    "prompt-mode-change-bypass",
];

pub fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/stream-json/{name}.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} reads: {e}"))
}

/// The fixture cut by the splitter: every line, the trailing one from `finish()` included.
pub fn lines_of(name: &str) -> Vec<String> {
    let mut splitter = LineSplitter::default();
    let mut lines = splitter.push(&fixture_bytes(name));
    lines.extend(splitter.finish());
    lines
}

/// The lines parsed and numbered from `0`, as a history holds them.
pub fn events_of(lines: &[String]) -> Vec<SequencedEvent> {
    sequenced(lines.iter().map(|line| parse_line(line)))
}

/// The lines without the partial-message (`"type":"stream_event"`) ones: the same session as a
/// CLI run without `--include-partial-messages` would have written it.
pub fn without_stream_events(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| !line.contains(r#""type":"stream_event""#))
        .cloned()
        .collect()
}

/// `events` numbered from `0` in order.
pub fn sequenced(events: impl IntoIterator<Item = StructuredEvent>) -> Vec<SequencedEvent> {
    events
        .into_iter()
        .zip(0u64..)
        .map(|(event, seq)| SequencedEvent { seq, event })
        .collect()
}

// ---------------------------------------------------------------------------------------
// Synthetic events. Each keeps a `raw` line of the CLI's shape, so code that reads `raw` (a tool
// result's content shape) sees what it would see from a real line.

fn message_line(kind: &str, content: Value, parent: Option<&str>) -> String {
    json!({
        "type": kind,
        "message": { "role": kind, "content": content },
        "parent_tool_use_id": parent,
        "session_id": "<SESSION_ID>",
    })
    .to_string()
}

fn block_json(block: &ContentBlock) -> Value {
    match block {
        ContentBlock::Text { text } => json!({ "type": "text", "text": text }),
        ContentBlock::Thinking { thinking } => json!({ "type": "thinking", "thinking": thinking }),
        ContentBlock::ToolUse { id, name, input } => json!({
            "type": "tool_use",
            "id": id,
            "name": name,
            "input": serde_json::from_str::<Value>(input).unwrap_or(Value::Null),
        }),
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": content,
            "is_error": is_error,
        }),
        ContentBlock::Other { raw } => serde_json::from_str(raw).unwrap_or(Value::Null),
    }
}

/// A user message of plain text, as the CLI echoes one.
/// The daemon's note of a sent user message (feature 941).
pub fn user_sent(text: &str, attachments: &[&str]) -> StructuredEvent {
    StructuredEvent::UserSent {
        text: text.to_string(),
        attachments: attachments.iter().map(|name| name.to_string()).collect(),
    }
}

pub fn user_text(text: &str) -> StructuredEvent {
    let blocks = vec![ContentBlock::Text {
        text: text.to_string(),
    }];
    StructuredEvent::User {
        is_synthetic: false,
        raw: message_line("user", json!(text), None),
        blocks,
        parent_tool_use_id: None,
    }
}

/// A finished assistant message holding `block`.
pub fn assistant_block(block: ContentBlock) -> StructuredEvent {
    assistant_blocks(vec![block])
}

/// A finished assistant message holding `blocks`, in order.
pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> StructuredEvent {
    let content: Vec<Value> = blocks.iter().map(block_json).collect();
    StructuredEvent::Assistant {
        error: None,
        model: None,
        message_id: None,
        raw: message_line("assistant", Value::Array(content), None),
        blocks,
        parent_tool_use_id: None,
    }
}

/// A finished assistant message holding one thinking block.
pub fn thinking_block(thinking: &str) -> StructuredEvent {
    assistant_block(ContentBlock::Thinking {
        thinking: thinking.to_string(),
    })
}

/// A finished assistant message holding one tool use.
pub fn tool_use(id: &str, name: &str, input: &str) -> StructuredEvent {
    assistant_block(ContentBlock::ToolUse {
        id: id.to_string(),
        name: name.to_string(),
        input: input.to_string(),
    })
}

/// A user message holding the result of tool use `id`, its content a string.
pub fn tool_result(id: &str, content: &str, is_error: bool) -> StructuredEvent {
    tool_result_json(id, json!(content), is_error)
}

/// A user message holding the result of tool use `id`, its content any JSON (a block list).
pub fn tool_result_json(id: &str, content: Value, is_error: bool) -> StructuredEvent {
    let block = ContentBlock::ToolResult {
        tool_use_id: id.to_string(),
        content: match &content {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        },
        is_error,
    };
    let raw = message_line(
        "user",
        json!([{
            "type": "tool_result",
            "tool_use_id": id,
            "content": content,
            "is_error": is_error,
        }]),
        None,
    );
    StructuredEvent::User {
        is_synthetic: false,
        blocks: vec![block],
        parent_tool_use_id: None,
        raw,
    }
}

/// A turn's result: a success, or an error of `subtype` carrying `result` text when given.
pub fn turn_result(is_error: bool, subtype: &str, result: Option<&str>) -> StructuredEvent {
    StructuredEvent::Result {
        subtype: subtype.to_string(),
        is_error,
        duration_ms: Some(1_000),
        num_turns: Some(1),
        total_cost_usd: Some(0.01),
        usage: None,
        result: result.map(str::to_string),
        session_id: None,
        raw: json!({ "type": "result", "subtype": subtype, "is_error": is_error }).to_string(),
    }
}

/// `event` as a sub-agent's: its `parent_tool_use_id` set to `parent`. Other events unchanged.
pub fn with_parent(event: StructuredEvent, parent: &str) -> StructuredEvent {
    let reparent = |raw: &str| {
        let mut value: Value = serde_json::from_str(raw).expect("a synthetic line is JSON");
        value["parent_tool_use_id"] = json!(parent);
        value.to_string()
    };
    match event {
        StructuredEvent::Assistant {
            message_id,
            blocks,
            raw,
            ..
        } => StructuredEvent::Assistant {
            error: None,
            model: None,
            message_id,
            blocks,
            raw: reparent(&raw),
            parent_tool_use_id: Some(parent.to_string()),
        },
        StructuredEvent::User { blocks, raw, .. } => StructuredEvent::User {
            is_synthetic: false,
            blocks,
            raw: reparent(&raw),
            parent_tool_use_id: Some(parent.to_string()),
        },
        StructuredEvent::StreamDelta {
            event_type,
            index,
            delta,
            raw,
            ..
        } => StructuredEvent::StreamDelta {
            event_type,
            index,
            delta,
            raw: reparent(&raw),
            parent_tool_use_id: Some(parent.to_string()),
        },
        other => other,
    }
}

/// `event` (an `Assistant`) as a part of the message `id`. Other events unchanged.
pub fn with_message_id(event: StructuredEvent, id: &str) -> StructuredEvent {
    match event {
        StructuredEvent::Assistant {
            blocks,
            raw,
            parent_tool_use_id,
            ..
        } => {
            let mut value: Value = serde_json::from_str(&raw).expect("a synthetic line is JSON");
            value["message"]["id"] = json!(id);
            StructuredEvent::Assistant {
                error: None,
                model: None,
                message_id: Some(id.to_string()),
                blocks,
                raw: value.to_string(),
                parent_tool_use_id,
            }
        }
        other => other,
    }
}

/// A partial-message line of the CLI's shape around `event`, its `stream_event`'s `event`
/// object, parsed as the service parses it.
pub fn stream_event(event: Value) -> StructuredEvent {
    parse_line(
        &json!({
            "type": "stream_event",
            "event": event,
            "parent_tool_use_id": null,
            "session_id": "<SESSION_ID>",
        })
        .to_string(),
    )
}

/// `message_start` of the message `id`.
pub fn message_start(id: &str) -> StructuredEvent {
    stream_event(json!({
        "type": "message_start",
        "message": { "id": id, "type": "message", "role": "assistant", "content": [] },
    }))
}

/// `content_block_start` of block `index`, `block` its empty content block.
pub fn block_start(index: u64, block: Value) -> StructuredEvent {
    stream_event(json!({ "type": "content_block_start", "index": index, "content_block": block }))
}

/// `content_block_delta` of block `index` carrying `delta`.
pub fn block_delta(index: u64, delta: Value) -> StructuredEvent {
    stream_event(json!({ "type": "content_block_delta", "index": index, "delta": delta }))
}

pub fn text_delta(index: u64, text: &str) -> StructuredEvent {
    block_delta(index, json!({ "type": "text_delta", "text": text }))
}

pub fn thinking_delta(index: u64, thinking: &str) -> StructuredEvent {
    block_delta(
        index,
        json!({ "type": "thinking_delta", "thinking": thinking }),
    )
}

pub fn input_json_delta(index: u64, partial_json: &str) -> StructuredEvent {
    block_delta(
        index,
        json!({ "type": "input_json_delta", "partial_json": partial_json }),
    )
}

/// A partial-message event carrying only its type (`message_stop`, `ping`, ...).
pub fn stream_marker(kind: &str) -> StructuredEvent {
    stream_event(json!({ "type": kind }))
}
