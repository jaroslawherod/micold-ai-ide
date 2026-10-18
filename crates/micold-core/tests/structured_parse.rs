//! Feature 938 T008 — the parser over recorded `claude` fixtures (FR-011–FR-015, FR-024; SC-001,
//! SC-002). Fixtures and how they were recorded: `tests/fixtures/stream-json/README.md`.

use micold_core::structured::{
    pair_tool_calls, parse_line, ContentBlock, Delta, LineSplitter, SequencedEvent,
    StructuredEvent,
};

const RECORDED: [&str; 5] = [
    "text-turn",
    "tool-turn",
    "partial-deltas",
    "error-result",
    "resume-init",
];

fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/stream-json/{name}.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} reads: {e}"))
}

/// The fixture cut by the splitter: every line, the trailing one from `finish()` included.
fn lines_of(name: &str) -> Vec<String> {
    let mut splitter = LineSplitter::default();
    let mut lines = splitter.push(&fixture_bytes(name));
    lines.extend(splitter.finish());
    lines
}

fn events_of(lines: &[String]) -> Vec<SequencedEvent> {
    lines
        .iter()
        .zip(0u64..)
        .map(|(line, seq)| SequencedEvent {
            seq,
            event: parse_line(line),
        })
        .collect()
}

/// Whether the line's `type` (and `subtype`, for `system`) is one the model types (R9).
fn is_known_type(line: &str) -> bool {
    let value: serde_json::Value = serde_json::from_str(line).expect("a recorded line is JSON");
    match value["type"].as_str() {
        Some("assistant" | "user" | "result" | "stream_event") => true,
        Some("system") => value["subtype"].as_str() == Some("init"),
        _ => false,
    }
}

fn raw_of(event: &StructuredEvent) -> Option<&str> {
    match event {
        StructuredEvent::Init { raw, .. }
        | StructuredEvent::Assistant { raw, .. }
        | StructuredEvent::User { raw, .. }
        | StructuredEvent::Result { raw, .. }
        | StructuredEvent::StreamDelta { raw, .. }
        | StructuredEvent::Unrecognised { raw } => Some(raw),
        StructuredEvent::Unreadable { text } => Some(text),
        StructuredEvent::TurnEndedWithoutResult | StructuredEvent::OversizedOnWire { .. } => None,
    }
}

fn session_id_of_init(events: &[SequencedEvent]) -> String {
    events
        .iter()
        .find_map(|e| match &e.event {
            StructuredEvent::Init { session_id, .. } => Some(session_id.clone()),
            _ => None,
        })
        .expect("the fixture has an init event")
}

fn last_result(events: &[SequencedEvent]) -> &StructuredEvent {
    events
        .iter()
        .rev()
        .map(|e| &e.event)
        .find(|e| matches!(e, StructuredEvent::Result { .. }))
        .expect("the fixture has a result event")
}

#[test]
fn every_recorded_line_is_one_event_in_order_and_every_known_type_is_typed() {
    for name in RECORDED {
        let lines = lines_of(name);
        let events = events_of(&lines);
        assert!(!lines.is_empty(), "{name} has lines");
        assert_eq!(events.len(), lines.len(), "{name}: one event per line (FR-014)");
        for (line, event) in lines.iter().zip(&events) {
            assert_eq!(
                raw_of(&event.event),
                Some(line.as_str()),
                "{name}: each event keeps its own line, in the CLI's order (FR-011, FR-014)"
            );
            assert!(
                !matches!(event.event, StructuredEvent::Unreadable { .. }),
                "{name}: a recorded line is JSON, so it is never unreadable: {line}"
            );
            if is_known_type(line) {
                assert!(
                    !matches!(event.event, StructuredEvent::Unrecognised { .. }),
                    "{name}: a known event type is recognised as that type (SC-001): {line}"
                );
            }
        }
    }
}

#[test]
fn a_text_turn_starts_with_init_and_ends_with_a_successful_result() {
    let events = events_of(&lines_of("text-turn"));
    let init = events
        .iter()
        .map(|e| &e.event)
        .find(|e| !matches!(e, StructuredEvent::Unrecognised { .. }))
        .expect("the turn has a typed event");
    match init {
        StructuredEvent::Init {
            session_id,
            model,
            cwd,
            ..
        } => {
            assert!(!session_id.is_empty(), "init names the session");
            assert!(
                model.as_deref().is_some_and(|m| !m.is_empty()),
                "init names the model"
            );
            assert!(
                cwd.as_deref().is_some_and(|c| !c.is_empty()),
                "init names the cwd"
            );
        }
        other => panic!("the first typed event of a turn is its init, got {other:?}"),
    }
    assert!(
        matches!(
            events.last().map(|e| &e.event),
            Some(StructuredEvent::Result { .. })
        ),
        "the turn ends with its result"
    );
    match last_result(&events) {
        StructuredEvent::Result {
            is_error,
            duration_ms,
            total_cost_usd,
            usage,
            ..
        } => {
            assert!(!is_error, "a plain answer is a success");
            assert!(duration_ms.is_some(), "the result carries its duration (FR-011)");
            assert!(total_cost_usd.is_some(), "the result carries its cost (FR-011)");
            assert!(usage.is_some(), "the result carries its usage (FR-011)");
        }
        _ => unreachable!(),
    }
}

#[test]
fn a_tool_turn_pairs_every_tool_call_with_its_result() {
    let events = events_of(&lines_of("tool-turn"));
    let tool_use = events.iter().find_map(|e| match &e.event {
        StructuredEvent::Assistant { blocks, .. } => blocks.iter().find_map(|b| match b {
            ContentBlock::ToolUse { id, name, input } => Some((id, name, input)),
            _ => None,
        }),
        _ => None,
    });
    let (id, name, input) = tool_use.expect("the turn has a tool call");
    assert!(!id.is_empty(), "a tool call has its identity (FR-011)");
    assert_eq!(name, "Read", "the tool call names its tool (FR-011)");
    assert!(
        input.contains("notes.txt"),
        "the tool call keeps its input as JSON text: {input}"
    );

    let pairs = pair_tool_calls(&events);
    assert!(!pairs.is_empty(), "the tool call is paired");
    for (use_seq, result_seq) in pairs {
        let result_seq = result_seq.expect("every tool call has its result (SC-001)");
        assert!(result_seq > use_seq, "a result follows its call");
        assert!(
            matches!(
                events[result_seq as usize].event,
                StructuredEvent::User { .. }
            ),
            "tool results arrive in user events"
        );
    }
}

#[test]
fn partial_deltas_are_stream_deltas_with_text() {
    let events = events_of(&lines_of("partial-deltas"));
    let text_deltas = events
        .iter()
        .filter(|e| {
            matches!(
                &e.event,
                StructuredEvent::StreamDelta {
                    event_type,
                    delta: Some(Delta::Text { .. }),
                    ..
                } if event_type == "content_block_delta"
            )
        })
        .count();
    assert!(
        text_deltas > 1,
        "a longer answer streams as several text deltas (FR-011)"
    );
}

#[test]
fn an_error_result_is_an_error() {
    let events = events_of(&lines_of("error-result"));
    match last_result(&events) {
        StructuredEvent::Result {
            is_error, subtype, ..
        } => {
            assert!(is_error, "the turn ended in an error result");
            assert!(subtype.starts_with("error"), "its subtype says so: {subtype}");
        }
        _ => unreachable!(),
    }
}

#[test]
fn a_resumed_session_reports_the_same_session_id() {
    let first = events_of(&lines_of("text-turn"));
    let resumed = events_of(&lines_of("resume-init"));
    assert_eq!(
        session_id_of_init(&resumed),
        session_id_of_init(&first),
        "a resumed conversation continues the same session"
    );
}

#[test]
fn the_tolerance_fixture_keeps_every_line() {
    let lines = lines_of("tolerance");
    assert_eq!(lines.len(), 9, "nine lines, the unterminated last one included");
    let events: Vec<StructuredEvent> = lines.iter().map(|l| parse_line(l)).collect();

    assert!(matches!(events[0], StructuredEvent::Init { .. }), "a real init line");
    for (index, why) in [
        (1, "an unknown type (FR-012)"),
        (3, "an assistant line with no message (FR-012)"),
        (4, "an unknown system subtype (FR-012)"),
    ] {
        assert_eq!(
            events[index],
            StructuredEvent::Unrecognised {
                raw: lines[index].clone()
            },
            "{why} is kept whole as unrecognised"
        );
    }
    match &events[2] {
        StructuredEvent::Assistant { blocks, .. } => assert_eq!(
            blocks,
            &vec![ContentBlock::Text {
                text: "hello".to_string()
            }],
            "extra fields do not stop a known type being typed (FR-015)"
        ),
        other => panic!("an assistant line with extra fields is typed, got {other:?}"),
    }
    match &events[5] {
        StructuredEvent::Assistant { blocks, .. } => {
            assert_eq!(blocks.len(), 2, "both blocks are kept");
            match &blocks[1] {
                ContentBlock::Other { raw } => assert!(
                    raw.contains("hologram") && raw.contains("cube"),
                    "the unknown block keeps its full content (FR-011): {raw}"
                ),
                other => panic!("an unknown block is kept as Other, got {other:?}"),
            }
        }
        other => panic!("a message with an unknown block is typed, got {other:?}"),
    }
    assert_eq!(
        events[6],
        StructuredEvent::Unreadable {
            text: "not json at all".to_string()
        },
        "a line that is not JSON is unreadable, with its text (FR-013)"
    );
    assert!(
        matches!(events[7], StructuredEvent::Result { .. }),
        "a CRLF-terminated result is typed: {:?}",
        events[7]
    );
    assert!(
        matches!(events[8], StructuredEvent::Result { .. }),
        "the unterminated last line is one event: {:?}",
        events[8]
    );
    for (line, event) in lines.iter().zip(&events) {
        assert_eq!(
            raw_of(event),
            Some(line.as_str()),
            "every event keeps its line (FR-011)"
        );
    }
}
