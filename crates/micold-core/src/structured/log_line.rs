//! One plain line per structured event, for the read-only log pane (feature 938, FR-022a).
//!
//! Render-free. A line never contains a newline: multi-line text is flattened, so the pane is one
//! row per event. #939 replaces the pane; this stays the fallback text of an event.

use super::{ContentBlock, Delta, SequencedEvent, StructuredEvent};

/// The longest text one line carries from an event; the rest is cut with an ellipsis.
const MAX_TEXT_CHARS: usize = 400;

/// The line for `event`: its sequence number, then a short summary of what it is.
pub fn log_line(event: &SequencedEvent) -> String {
    format!("#{} {}", event.seq, summary(&event.event))
}

/// The line that stands for `n` earlier events the history no longer holds.
pub fn omitted_line(n: u64) -> String {
    let noun = if n == 1 { "event" } else { "events" };
    format!("… {n} earlier {noun} omitted")
}

fn summary(event: &StructuredEvent) -> String {
    match event {
        StructuredEvent::Init { model, .. } => match model {
            Some(model) => format!("init: model {}", flat(model)),
            None => "init".to_owned(),
        },
        StructuredEvent::Assistant { blocks, .. } => blocks_summary("assistant", blocks),
        StructuredEvent::User { blocks, .. } => blocks_summary("user", blocks),
        StructuredEvent::Result {
            subtype,
            is_error,
            duration_ms,
            ..
        } => {
            let outcome = if *is_error { "error" } else { "ok" };
            match duration_ms {
                Some(ms) => format!("result: {} {outcome} in {ms} ms", flat(subtype)),
                None => format!("result: {} {outcome}", flat(subtype)),
            }
        }
        StructuredEvent::StreamDelta {
            event_type, delta, ..
        } => {
            let kind = match delta {
                Some(Delta::Text { .. }) => " text_delta",
                Some(Delta::Thinking { .. }) => " thinking_delta",
                Some(Delta::InputJson { .. }) => " input_json_delta",
                Some(Delta::Other { .. }) => " other delta",
                None => "",
            };
            format!("stream: {}{kind}", flat(event_type))
        }
        StructuredEvent::Unrecognised { raw } => match type_of(raw) {
            Some(t) => format!("unrecognised event of type {}", flat(&t)),
            None => "unrecognised event".to_owned(),
        },
        StructuredEvent::Unreadable { text } => format!("unreadable line: {}", cut(&flat(text))),
        StructuredEvent::TurnEndedWithoutResult => "turn ended without a result".to_owned(),
        StructuredEvent::OversizedOnWire { original_bytes } => {
            format!("event too large to show ({original_bytes} bytes)")
        }
    }
}

fn blocks_summary(who: &str, blocks: &[ContentBlock]) -> String {
    if blocks.is_empty() {
        return who.to_owned();
    }
    let parts: Vec<String> = blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => format!("text: {}", cut(&flat(text))),
            ContentBlock::Thinking { thinking } => format!("thinking: {}", cut(&flat(thinking))),
            ContentBlock::ToolUse { name, .. } => format!("tool call {}", flat(name)),
            ContentBlock::ToolResult {
                tool_use_id,
                is_error,
                ..
            } => {
                let error = if *is_error { " (error)" } else { "" };
                format!("tool result for {}{error}", flat(tool_use_id))
            }
            ContentBlock::Other { raw } => match type_of(raw) {
                Some(t) => format!("block of type {}", flat(&t)),
                None => "block".to_owned(),
            },
        })
        .collect();
    format!("{who}: {}", parts.join("; "))
}

/// The `type` of a JSON object's text, when it has one.
fn type_of(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    value.get("type")?.as_str().map(str::to_owned)
}

/// `text` on one line: every run of whitespace, newlines included, becomes one space.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cut(text: &str) -> String {
    if text.chars().count() <= MAX_TEXT_CHARS {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(MAX_TEXT_CHARS).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(event: StructuredEvent) -> SequencedEvent {
        SequencedEvent { seq: 7, event }
    }

    fn assistant(blocks: Vec<ContentBlock>) -> StructuredEvent {
        StructuredEvent::Assistant {
            message_id: None,
            blocks,
            parent_tool_use_id: None,
            raw: String::new(),
        }
    }

    #[test]
    fn init_names_the_model() {
        let line = log_line(&seq(StructuredEvent::Init {
            session_id: "s".into(),
            model: Some("opus".into()),
            cwd: None,
            tools: vec![],
            raw: String::new(),
        }));
        assert!(line.contains("init") && line.contains("opus"), "{line}");
    }

    #[test]
    fn assistant_blocks_show_text_thinking_and_tool_name() {
        let line = log_line(&seq(assistant(vec![
            ContentBlock::Text {
                text: "hello".into(),
            },
            ContentBlock::Thinking {
                thinking: "hmm".into(),
            },
            ContentBlock::ToolUse {
                id: "t1".into(),
                name: "Bash".into(),
                input: "{}".into(),
            },
        ])));
        for want in ["hello", "hmm", "Bash"] {
            assert!(line.contains(want), "{line}");
        }
    }

    #[test]
    fn tool_result_names_call_id_and_error_flag() {
        let result = |is_error| {
            log_line(&seq(StructuredEvent::User {
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "toolu_9".into(),
                    content: "x".into(),
                    is_error,
                }],
                parent_tool_use_id: None,
                raw: String::new(),
            }))
        };
        assert!(result(false).contains("toolu_9"));
        assert!(!result(false).contains("error"));
        assert!(result(true).contains("error"));
    }

    #[test]
    fn result_gives_outcome_and_duration() {
        let line = log_line(&seq(StructuredEvent::Result {
            subtype: "success".into(),
            is_error: false,
            duration_ms: Some(1234),
            num_turns: None,
            total_cost_usd: None,
            usage: None,
            result: None,
            session_id: None,
            raw: String::new(),
        }));
        assert!(line.contains("success") && line.contains("1234"), "{line}");
    }

    #[test]
    fn stream_delta_names_its_type() {
        let line = log_line(&seq(StructuredEvent::StreamDelta {
            event_type: "content_block_delta".into(),
            index: Some(0),
            delta: Some(Delta::Text { text: "x".into() }),
            parent_tool_use_id: None,
            raw: String::new(),
        }));
        assert!(line.contains("content_block_delta") && line.contains("text_delta"));
    }

    #[test]
    fn unrecognised_names_its_type_when_it_has_one() {
        let with = log_line(&seq(StructuredEvent::Unrecognised {
            raw: r#"{"type":"weird"}"#.into(),
        }));
        assert!(with.contains("weird"), "{with}");
        let without = log_line(&seq(StructuredEvent::Unrecognised { raw: "[1]".into() }));
        assert!(without.contains("unrecognised"), "{without}");
    }

    #[test]
    fn notes_and_unreadable_lines() {
        let unreadable = log_line(&seq(StructuredEvent::Unreadable {
            text: "not json".into(),
        }));
        assert!(unreadable.contains("not json"));
        assert!(log_line(&seq(StructuredEvent::TurnEndedWithoutResult)).contains("without"));
        let big = log_line(&seq(StructuredEvent::OversizedOnWire {
            original_bytes: 99999,
        }));
        assert!(big.contains("99999"));
    }

    #[test]
    fn no_line_contains_a_newline() {
        let lines = [
            log_line(&seq(assistant(vec![ContentBlock::Text {
                text: "a\nb\r\nc".into(),
            }]))),
            log_line(&seq(StructuredEvent::Unreadable {
                text: "x\ny".into(),
            })),
        ];
        for line in lines {
            assert!(!line.contains('\n') && !line.contains('\r'), "{line:?}");
        }
    }

    #[test]
    fn omitted_line_names_the_count() {
        assert!(omitted_line(12).contains("12"));
        assert!(!omitted_line(1).contains("events"));
    }
}
