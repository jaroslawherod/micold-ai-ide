//! One line in, one event out (research R9).
//!
//! The line is read as a JSON `Value` first, so a line that is JSON is never lost to a field the
//! model does not expect: the `type` (and `subtype`, for `system`) picks a typed shape, and a line
//! that does not fit its shape is kept whole as [`StructuredEvent::Unrecognised`] (FR-012). No
//! shape denies unknown fields (FR-015).

use super::event::{ContentBlock, ControlOutcome, Delta, SequencedEvent, StructuredEvent, Usage};
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
fn typed(value: Value, line: &str) -> Option<StructuredEvent> {
    let raw = || line.to_string();
    match (value["type"].as_str()?, value["subtype"].as_str()) {
        ("system", Some("init")) => {
            let slash_commands = names(&value["slash_commands"]);
            let skills = names(&value["skills"]);
            let terminal_slash_commands = names(&value["terminal_slash_commands"]);
            let init: InitLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::Init {
                session_id: init.session_id,
                model: init.model,
                cwd: init.cwd,
                tools: init.tools.into_iter().flatten().map(tool_name).collect(),
                permission_mode: init.permission_mode,
                slash_commands,
                skills,
                terminal_slash_commands,
                raw: raw(),
            })
        }
        ("system", Some("compact_boundary")) => {
            let metadata = &value["compact_metadata"];
            Some(StructuredEvent::CompactBoundary {
                trigger: metadata["trigger"].as_str().map(str::to_string),
                pre_tokens: metadata["pre_tokens"].as_u64(),
                post_tokens: metadata["post_tokens"].as_u64(),
                raw: raw(),
            })
        }
        ("conversation_reset", _) => Some(StructuredEvent::ConversationReset {
            trigger: value["trigger"].as_str().map(str::to_string),
            raw: raw(),
        }),
        ("system", Some("status")) => Some(StructuredEvent::Status {
            permission_mode: value["permissionMode"].as_str().map(str::to_string),
            raw: raw(),
        }),
        ("control_response", _) => {
            let response = &value["response"];
            let request_id = response["request_id"].as_str()?.to_string();
            let text = |key: &str| response[key].as_str().map(str::to_string);
            let outcome = match response["subtype"].as_str()? {
                "success" => ControlOutcome::Success(response["response"].to_string()),
                "error" => ControlOutcome::Error {
                    message: text("error").unwrap_or_default(),
                    code: text("error_code"),
                },
                _ => return None,
            };
            Some(StructuredEvent::ControlResponse {
                request_id,
                outcome,
                raw: raw(),
            })
        }
        ("assistant", _) => {
            let model = value["message"]["model"].as_str().map(str::to_string);
            let error = value["error"].as_str().map(str::to_string);
            let line: MessageLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::Assistant {
                message_id: line.message.id,
                blocks: line.message.content.into_blocks(),
                parent_tool_use_id: line.parent_tool_use_id,
                model,
                error,
                raw: raw(),
            })
        }
        ("user", _) => {
            let is_synthetic = value["isSynthetic"].as_bool().unwrap_or(false);
            let line: MessageLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::User {
                blocks: line.message.content.into_blocks(),
                parent_tool_use_id: line.parent_tool_use_id,
                is_synthetic,
                raw: raw(),
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
                raw: raw(),
            })
        }
        ("stream_event", _) => {
            let line: StreamLine = serde_json::from_value(value).ok()?;
            Some(StructuredEvent::StreamDelta {
                event_type: line.event.kind,
                index: line.event.index,
                delta: line.event.delta.map(delta_of),
                parent_tool_use_id: line.parent_tool_use_id,
                raw: raw(),
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
    /// Any JSON, so an unexpected entry or a `null` list costs the tools, not the init.
    #[serde(default)]
    tools: Option<Vec<Value>>,
    #[serde(default, rename = "permissionMode")]
    permission_mode: Option<String>,
}

/// The string entries of a JSON list; anything else (absent, `null`, not a list) is empty and a
/// non-string entry is dropped (#943 R1).
fn names(list: &Value) -> Vec<String> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|name| name.as_str().map(str::to_string))
        .collect()
}

/// A tool's name, or the JSON text of an entry that is not a string.
fn tool_name(tool: Value) -> String {
    match tool {
        Value::String(name) => name,
        other => other.to_string(),
    }
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
                // Always JSON text: `null` when the CLI sent no input.
                input: block.get("input").unwrap_or(&Value::Null).to_string(),
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

    #[test]
    fn odd_tools_and_a_missing_tool_input_do_not_cost_the_event() {
        let init = parse_line(
            r#"{"type":"system","subtype":"init","session_id":"s","tools":["Read",{"mcp":"x"}]}"#,
        );
        assert!(
            matches!(&init, StructuredEvent::Init { tools, .. }
                if tools == &vec!["Read".to_string(), r#"{"mcp":"x"}"#.to_string()]),
            "a tool entry that is not a name is kept as JSON text: {init:?}"
        );
        let no_list =
            parse_line(r#"{"type":"system","subtype":"init","session_id":"s","tools":null}"#);
        assert!(
            matches!(&no_list, StructuredEvent::Init { tools, .. } if tools.is_empty()),
            "a null tool list still gives the init its session id: {no_list:?}"
        );
        let call = parse_line(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"a","name":"Now"}]}}"#,
        );
        assert!(
            matches!(&call, StructuredEvent::Assistant { blocks, .. }
                if matches!(blocks.as_slice(), [ContentBlock::ToolUse { input, .. }] if input == "null")),
            "a call without input has the JSON text `null`: {call:?}"
        );
    }
}
