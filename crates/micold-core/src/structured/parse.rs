//! One line in, one event out (research R9).
//!
//! The line is read as a JSON `Value` first, so a line that is JSON is never lost to a field the
//! model does not expect: the `type` (and `subtype`, for `system`) picks a typed shape, and a line
//! that does not fit its shape is kept whole as [`StructuredEvent::Unrecognised`] (FR-012). No
//! shape denies unknown fields (FR-015).

use super::event::{ContentBlock, Delta, SequencedEvent, StructuredEvent, Usage};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Turn one line of the CLI's stdout into its event. Never fails (FR-014).
pub fn parse_line(line: &str) -> StructuredEvent {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return StructuredEvent::Unreadable {
            text: line.to_string(),
        };
    };
    typed(value, line).unwrap_or_else(|| StructuredEvent::Unrecognised {
        raw: line.to_string(),
    })
}

/// The typed event of a JSON line, or `None` when its type is unknown or its shape does not fit.
fn typed(value: Value, raw: &str) -> Option<StructuredEvent> {
    let raw = raw.to_string();
    match (value["type"].as_str()?, value["subtype"].as_str()) {
        ("system", Some("init")) => {
            let init: InitLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::Init {
                session_id: init.session_id,
                model: init.model,
                cwd: init.cwd,
                tools: init.tools,
                raw,
            })
        }
        ("assistant", _) => {
            let line: MessageLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::Assistant {
                message_id: line.message.id,
                blocks: line.message.content.into_blocks(),
                parent_tool_use_id: line.parent_tool_use_id,
                raw,
            })
        }
        ("user", _) => {
            let line: MessageLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::User {
                blocks: line.message.content.into_blocks(),
                parent_tool_use_id: line.parent_tool_use_id,
                raw,
            })
        }
        ("result", _) => {
            let line: ResultLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::Result {
                subtype: line.subtype,
                is_error: line.is_error,
                duration_ms: line.duration_ms,
                num_turns: line.num_turns,
                total_cost_usd: line.total_cost_usd,
                usage: line.usage,
                result: line.result,
                session_id: line.session_id,
                raw,
            })
        }
        ("stream_event", _) => {
            let line: StreamLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::StreamDelta {
                event_type: line.event.kind,
                index: line.event.index,
                delta: line.event.delta.map(delta_of),
                parent_tool_use_id: line.parent_tool_use_id,
                raw,
            })
        }
        _ => None,
    }
}

#[derive(Deserialize)]
struct InitLine {
    session_id: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    tools: Vec<String>,
}

#[derive(Deserialize)]
struct MessageLine {
    message: WireMessage,
    #[serde(default)]
    parent_tool_use_id: Option<String>,
}

#[derive(Deserialize)]
struct WireMessage {
    #[serde(default)]
    id: Option<String>,
    content: WireContent,
}

/// A message's content: the CLI writes a block list, and a plain string for some user messages.
#[derive(Deserialize)]
#[serde(untagged)]
enum WireContent {
    Text(String),
    Blocks(Vec<Value>),
}

impl WireContent {
    fn into_blocks(self) -> Vec<ContentBlock> {
        match self {
            Self::Text(text) => vec![ContentBlock::Text { text }],
            Self::Blocks(blocks) => blocks.into_iter().map(block_of).collect(),
        }
    }
}

/// One content block, or [`ContentBlock::Other`] with its JSON text when the model does not know
/// its type or its shape does not fit (FR-011).
fn block_of(block: Value) -> ContentBlock {
    let text = |key: &str| block[key].as_str().map(str::to_string);
    let typed = match block["type"].as_str() {
        Some("text") => text("text").map(|text| ContentBlock::Text { text }),
        Some("thinking") => text("thinking").map(|thinking| ContentBlock::Thinking { thinking }),
        Some("tool_use") => match (text("id"), text("name")) {
            (Some(id), Some(name)) => Some(ContentBlock::ToolUse {
                id,
                name,
                input: block.get("input").map(Value::to_string).unwrap_or_default(),
            }),
            _ => None,
        },
        Some("tool_result") => text("tool_use_id").map(|tool_use_id| ContentBlock::ToolResult {
            tool_use_id,
            content: match block.get("content") {
                Some(Value::String(content)) => content.clone(),
                Some(Value::Null) | None => String::new(),
                Some(content) => content.to_string(),
            },
            is_error: block["is_error"].as_bool().unwrap_or(false),
        }),
        _ => None,
    };
    typed.unwrap_or_else(|| ContentBlock::Other {
        raw: block.to_string(),
    })
}

#[derive(Deserialize)]
struct ResultLine {
    subtype: String,
    is_error: bool,
    #[serde(default)]
    duration_ms: Option<u64>,
    #[serde(default)]
    num_turns: Option<u64>,
    #[serde(default)]
    total_cost_usd: Option<f64>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Deserialize)]
struct StreamLine {
    event: WireStreamEvent,
    #[serde(default)]
    parent_tool_use_id: Option<String>,
}

#[derive(Deserialize)]
struct WireStreamEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    index: Option<u64>,
    #[serde(default)]
    delta: Option<Value>,
}

/// A partial-message delta, or [`Delta::Other`] with its JSON text when it is of another type.
fn delta_of(delta: Value) -> Delta {
    let text = |key: &str| delta[key].as_str().map(str::to_string);
    let typed = match delta["type"].as_str() {
        Some("text_delta") => text("text").map(|text| Delta::Text { text }),
        Some("thinking_delta") => text("thinking").map(|thinking| Delta::Thinking { thinking }),
        Some("input_json_delta") => {
            text("partial_json").map(|partial_json| Delta::InputJson { partial_json })
        }
        _ => None,
    };
    typed.unwrap_or_else(|| Delta::Other {
        raw: delta.to_string(),
    })
}

/// Each tool call's `seq`, in order, with the `seq` of the first event carrying its result, if
/// one does (`ToolUse.id == ToolResult.tool_use_id`, SC-001).
pub fn pair_tool_calls(events: &[SequencedEvent]) -> Vec<(u64, Option<u64>)> {
    let mut pairs: Vec<(u64, Option<u64>)> = Vec::new();
    let mut open: HashMap<&str, usize> = HashMap::new();
    for sequenced in events {
        let blocks = match &sequenced.event {
            StructuredEvent::Assistant { blocks, .. } | StructuredEvent::User { blocks, .. } => {
                blocks
            }
            _ => continue,
        };
        for block in blocks {
            match block {
                ContentBlock::ToolUse { id, .. } => {
                    open.insert(id, pairs.len());
                    pairs.push((sequenced.seq, None));
                }
                ContentBlock::ToolResult { tool_use_id, .. } => {
                    if let Some(at) = open.remove(tool_use_id.as_str()) {
                        pairs[at].1 = Some(sequenced.seq);
                    }
                }
                _ => {}
            }
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequenced(seq: u64, line: &str) -> SequencedEvent {
        SequencedEvent {
            seq,
            event: parse_line(line),
        }
    }

    #[test]
    fn a_tool_call_without_a_result_is_paired_with_nothing() {
        let events = [
            sequenced(
                0,
                r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"a","name":"Read","input":{}}]}}"#,
            ),
            sequenced(
                1,
                r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"other","content":"x"}]}}"#,
            ),
        ];
        assert_eq!(
            pair_tool_calls(&events),
            vec![(0, None)],
            "a result answers only the call it names (SC-001)"
        );
    }

    #[test]
    fn a_block_list_tool_result_is_kept_as_json_text_and_a_string_message_is_text() {
        let tool_result = parse_line(
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"a","content":[{"type":"text","text":"hi"}],"is_error":true}]}}"#,
        );
        match tool_result {
            StructuredEvent::User { blocks, .. } => match blocks.as_slice() {
                [ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                }] => {
                    assert_eq!(tool_use_id, "a");
                    assert!(is_error, "the result keeps its error flag (FR-011)");
                    assert_eq!(
                        serde_json::from_str::<Value>(content).expect("JSON text"),
                        serde_json::json!([{"type": "text", "text": "hi"}]),
                        "a block-list result is kept as its JSON text (FR-011)"
                    );
                }
                other => panic!("one tool result, got {other:?}"),
            },
            other => panic!("a user event, got {other:?}"),
        }
        let line = r#"{"type":"user","message":{"role":"user","content":"plain"}}"#;
        assert!(
            matches!(parse_line(line), StructuredEvent::User { blocks, .. }
                if blocks == vec![ContentBlock::Text { text: "plain".to_string() }]),
            "a user message given as a string is one text block"
        );
    }
}
