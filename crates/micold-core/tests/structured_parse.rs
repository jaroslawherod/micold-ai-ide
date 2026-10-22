//! Feature 938 T008 — the parser over recorded `claude` fixtures (FR-011–FR-015, FR-024; SC-001,
//! SC-002). Fixtures and how they were recorded: `tests/fixtures/stream-json/README.md`.

use micold_core::structured::{
    pair_tool_calls, parse_line, ContentBlock, ControlOutcome, Delta, SequencedEvent,
    StructuredEvent,
};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{events_of, lines_of, COMMAND_RECORDED, PROMPT_RECORDED, RECORDED, RECORDED_941};

/// Whether the line's `type` (and `subtype`, for `system`) is one the model types (R9).
fn is_known_type(line: &str) -> bool {
    let value: serde_json::Value = serde_json::from_str(line).expect("a recorded line is JSON");
    match value["type"].as_str() {
        Some("assistant" | "user" | "result" | "stream_event" | "control_response") => true,
        Some("conversation_reset") => true,
        Some("system") => matches!(
            value["subtype"].as_str(),
            Some("init" | "status" | "compact_boundary")
        ),
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
        | StructuredEvent::Status { raw, .. }
        | StructuredEvent::ControlResponse { raw, .. }
        | StructuredEvent::CompactBoundary { raw, .. }
        | StructuredEvent::ConversationReset { raw, .. }
        | StructuredEvent::Unrecognised { raw } => Some(raw),
        StructuredEvent::Unreadable { text } => Some(text),
        StructuredEvent::TurnEndedWithoutResult
        | StructuredEvent::OversizedOnWire { .. }
        | StructuredEvent::UserSent { .. }
        | StructuredEvent::InterruptAsked
        | StructuredEvent::CommandsReported { .. }
        | StructuredEvent::ModelChanged { .. }
        | StructuredEvent::ModelRefused { .. }
        | StructuredEvent::KindSwitched { .. } => None,
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
    for name in RECORDED
        .into_iter()
        .chain(PROMPT_RECORDED)
        .chain(RECORDED_941)
        .chain(COMMAND_RECORDED)
    {
        let lines = lines_of(name);
        let events = events_of(&lines);
        assert!(!lines.is_empty(), "{name} has lines");
        assert_eq!(
            events.len(),
            lines.len(),
            "{name}: one event per line (FR-014)"
        );
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
            assert!(
                duration_ms.is_some(),
                "the result carries its duration (FR-011)"
            );
            assert!(
                total_cost_usd.is_some(),
                "the result carries its cost (FR-011)"
            );
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
            assert!(
                subtype.starts_with("error"),
                "its subtype says so: {subtype}"
            );
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
    assert_eq!(
        lines.len(),
        9,
        "nine lines, the unterminated last one included"
    );
    let events: Vec<StructuredEvent> = lines.iter().map(|l| parse_line(l)).collect();

    assert!(
        matches!(events[0], StructuredEvent::Init { .. }),
        "a real init line"
    );
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

// #942 T005 — the CLI's permission mode and control replies, over the permission-prompt recordings
// (`tests/fixtures/permission-prompt/README.md`).

fn events_of_fixture(name: &str) -> Vec<StructuredEvent> {
    events_of(&lines_of(name))
        .into_iter()
        .map(|sequenced| sequenced.event)
        .collect()
}

fn init_mode(name: &str) -> Option<String> {
    events_of_fixture(name)
        .into_iter()
        .find_map(|event| match event {
            StructuredEvent::Init {
                permission_mode, ..
            } => Some(permission_mode),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} has an init"))
}

/// Every `permissionMode` the fixture's `status` lines report, in order.
fn status_modes(name: &str) -> Vec<String> {
    events_of_fixture(name)
        .into_iter()
        .filter_map(|event| match event {
            StructuredEvent::Status {
                permission_mode, ..
            } => permission_mode,
            _ => None,
        })
        .collect()
}

fn control_responses(name: &str) -> Vec<(String, ControlOutcome)> {
    events_of_fixture(name)
        .into_iter()
        .filter_map(|event| match event {
            StructuredEvent::ControlResponse {
                request_id,
                outcome,
                ..
            } => Some((request_id, outcome)),
            _ => None,
        })
        .collect()
}

#[test]
fn init_reports_the_mode_the_cli_was_launched_in() {
    assert_eq!(
        init_mode("prompt-tool-request").as_deref(),
        Some("default"),
        "init carries permissionMode (FR-016)"
    );
    assert_eq!(
        init_mode("prompt-plan-approval").as_deref(),
        Some("plan"),
        "a launch with --permission-mode plan reports plan"
    );
}

#[test]
fn a_status_line_reports_the_mode_it_changed_to() {
    assert_eq!(
        status_modes("prompt-plan-approval"),
        vec!["default".to_string()],
        "an approved ExitPlanMode moves the CLI to default by itself (P-3)"
    );
    assert_eq!(
        status_modes("prompt-mode-change"),
        vec!["acceptEdits".to_string()],
        "a successful set_permission_mode is followed by its status line (P-9)"
    );
    assert_eq!(
        status_modes("prompt-mode-change-bypass"),
        vec!["bypassPermissions".to_string()]
    );
    let plain = events_of_fixture("prompt-tool-request")
        .into_iter()
        .find(|event| matches!(event, StructuredEvent::Status { .. }))
        .expect("the CLI writes status lines without a mode too");
    assert!(
        matches!(
            plain,
            StructuredEvent::Status {
                permission_mode: None,
                ..
            }
        ),
        "a status line without permissionMode has none: {plain:?}"
    );
}

#[test]
fn a_control_response_is_typed_as_success_or_error_for_its_request() {
    let responses = control_responses("prompt-mode-change");
    assert_eq!(
        responses.len(),
        2,
        "one response per request: {responses:?}"
    );
    match &responses[0] {
        (id, ControlOutcome::Success(response)) => {
            assert_eq!(id, "micold-1");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(response).expect("JSON text"),
                serde_json::json!({"mode": "acceptEdits"}),
                "the success keeps the CLI's response object"
            );
        }
        other => panic!("micold-1 succeeds: {other:?}"),
    }
    match &responses[1] {
        (id, ControlOutcome::Error { message, code }) => {
            assert_eq!(id, "micold-2");
            assert!(
                message.contains("bypassPermissions"),
                "the error keeps the CLI's text: {message}"
            );
            assert_eq!(code.as_deref(), Some("bypass_not_launched"));
        }
        other => panic!("micold-2 fails: {other:?}"),
    }
    assert!(
        matches!(
            control_responses("prompt-mode-change-bypass").as_slice(),
            [(id, ControlOutcome::Success(_))] if id == "micold-1"
        ),
        "launched with --allow-dangerously-skip-permissions, bypass succeeds (P-10)"
    );
}

#[test]
fn a_control_response_that_does_not_fit_stays_unrecognised() {
    for line in [
        r#"{"type":"control_response","response":{"subtype":"success"}}"#,
        r#"{"type":"control_response","response":{"subtype":"other","request_id":"micold-1"}}"#,
        r#"{"type":"control_response"}"#,
    ] {
        assert!(
            matches!(parse_line(line), StructuredEvent::Unrecognised { .. }),
            "kept whole, never dropped (FR-012): {line}"
        );
    }
    assert!(
        matches!(
            parse_line(r#"{"type":"control_response","response":{"subtype":"error","request_id":"r"}}"#),
            StructuredEvent::ControlResponse {
                outcome: ControlOutcome::Error { ref message, code: None },
                ..
            } if message.is_empty()
        ),
        "an error without text or code is still an error"
    );
}

/// Feature 941 (T012): `UserSent` and `InterruptAsked` are the daemon's notes; no CLI line parses
/// as either.
#[test]
fn the_parser_never_makes_a_daemon_note_of_941() {
    for name in RECORDED
        .into_iter()
        .chain(PROMPT_RECORDED)
        .chain(RECORDED_941)
        .chain(COMMAND_RECORDED)
    {
        for event in events_of(&lines_of(name)) {
            assert!(
                !matches!(
                    event.event,
                    StructuredEvent::UserSent { .. } | StructuredEvent::InterruptAsked
                ),
                "{name}: {:?}",
                event.event
            );
        }
    }
}

/// Feature 941 (T012, P-4): the interrupted turn's lines are typed: the control response, the
/// partial assistant message, the CLI's interrupted user text and the `error_during_execution`
/// result.
#[test]
fn the_interrupt_fixture_is_typed_line_by_line() {
    let events: Vec<_> = events_of(&lines_of("interrupt-turn"))
        .into_iter()
        .map(|e| e.event)
        .collect();
    assert!(events.iter().any(|e| matches!(
        e,
        StructuredEvent::ControlResponse {
            outcome: ControlOutcome::Success(_),
            ..
        }
    )));
    assert!(events
        .iter()
        .any(|e| matches!(e, StructuredEvent::User { blocks, .. }
        if matches!(blocks.as_slice(), [ContentBlock::Text { text }]
            if text == "[Request interrupted by user]"))));
    assert!(
        matches!(last_result(&events_of(&lines_of("interrupt-turn"))),
        StructuredEvent::Result { subtype, is_error: true, .. } if subtype == "error_during_execution")
    );
    assert!(matches!(last_result(&events_of(&lines_of("image-turn"))),
        StructuredEvent::Result { subtype, is_error: false, .. } if subtype == "success"));
}

// #943 T004 — the command, model and authentication lines (data-model *Events*), over #943's
// recordings.

fn inits(name: &str) -> Vec<StructuredEvent> {
    events_of_fixture(name)
        .into_iter()
        .filter(|event| matches!(event, StructuredEvent::Init { .. }))
        .collect()
}

#[test]
fn init_carries_the_command_skill_and_terminal_only_lists() {
    let StructuredEvent::Init {
        slash_commands,
        skills,
        terminal_slash_commands,
        ..
    } = &inits("command-initialize")[0]
    else {
        unreachable!()
    };
    for name in [
        "greet", "hello", "compact", "clear", "context", "model", "mcp",
    ] {
        assert!(
            slash_commands.iter().any(|c| c == name),
            "{name} in slash_commands: {slash_commands:?}"
        );
    }
    assert!(skills.iter().any(|s| s == "greet"), "{skills:?}");
    assert!(
        !skills.iter().any(|s| s == "hello"),
        "a custom command is not a skill"
    );
    assert_eq!(
        terminal_slash_commands,
        &["doctor", "color", "focus", "reload-plugins"].map(String::from)
    );
}

#[test]
fn an_init_without_the_lists_has_them_empty() {
    match parse_line(r#"{"type":"system","subtype":"init","session_id":"s","slash_commands":null}"#)
    {
        StructuredEvent::Init {
            slash_commands,
            skills,
            terminal_slash_commands,
            ..
        } => {
            assert!(slash_commands.is_empty());
            assert!(skills.is_empty());
            assert!(terminal_slash_commands.is_empty());
        }
        other => panic!("an init, got {other:?}"),
    }
    match parse_line(r#"{"type":"system","subtype":"init","session_id":"s","skills":["a",7,"b"]}"#)
    {
        StructuredEvent::Init { skills, .. } => {
            assert_eq!(skills, ["a", "b"], "a non-string entry is dropped")
        }
        other => panic!("an init, got {other:?}"),
    }
}

fn assistants(name: &str) -> Vec<(Option<String>, Option<String>, Vec<ContentBlock>)> {
    events_of_fixture(name)
        .into_iter()
        .filter_map(|event| match event {
            StructuredEvent::Assistant {
                model,
                error,
                blocks,
                ..
            } => Some((model, error, blocks)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_built_in_commands_answer_is_a_synthetic_assistant_message() {
    let answers = assistants("command-builtin-output");
    assert_eq!(answers.len(), 1);
    let (model, error, blocks) = &answers[0];
    assert_eq!(model.as_deref(), Some("<synthetic>"));
    assert_eq!(error, &None);
    match &blocks[..] {
        [ContentBlock::Text { text }] => assert!(text.starts_with("## Context Usage"), "{text}"),
        other => panic!("one text block, got {other:?}"),
    }
    let (model, _, _) = &assistants("model-change")[0];
    assert_eq!(
        model.as_deref(),
        Some("claude-haiku-5-5"),
        "a model turn names its model"
    );
}

#[test]
fn an_authentication_failure_is_an_assistant_error() {
    let answers = assistants("auth-failure");
    assert_eq!(answers.len(), 1);
    let (model, error, blocks) = &answers[0];
    assert_eq!(error.as_deref(), Some("authentication_failed"));
    assert_eq!(model.as_deref(), Some("<synthetic>"));
    assert!(
        matches!(&blocks[..], [ContentBlock::Text { text }] if text.starts_with("Invalid API key")),
        "{blocks:?}"
    );
    assert!(matches!(
        last_result(&events_of(&lines_of("auth-failure"))),
        StructuredEvent::Result { is_error: true, .. }
    ));
}

#[test]
fn a_compaction_is_a_boundary_then_a_synthetic_summary() {
    let events = events_of_fixture("command-compact");
    let boundary = events
        .iter()
        .position(|e| matches!(e, StructuredEvent::CompactBoundary { .. }))
        .expect("a compact boundary");
    match &events[boundary] {
        StructuredEvent::CompactBoundary {
            trigger,
            pre_tokens,
            post_tokens,
            ..
        } => {
            assert_eq!(trigger.as_deref(), Some("manual"));
            assert_eq!(*pre_tokens, Some(37155));
            assert_eq!(*post_tokens, Some(6215));
        }
        _ => unreachable!(),
    }
    match &events[boundary + 1] {
        StructuredEvent::User {
            is_synthetic: true,
            blocks,
            ..
        } => assert!(
            matches!(&blocks[..], [ContentBlock::Text { text }] if text.starts_with("This session is being continued")),
            "{blocks:?}"
        ),
        other => panic!("the synthetic summary follows the boundary, got {other:?}"),
    }
    let synthetic = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                StructuredEvent::User {
                    is_synthetic: true,
                    ..
                }
            )
        })
        .count();
    assert_eq!(synthetic, 1, "only the summary is synthetic");
}

#[test]
fn a_clear_is_a_conversation_reset_and_the_next_init_has_a_new_session() {
    let events = events_of_fixture("command-clear");
    let reset = events
        .iter()
        .position(|e| matches!(e, StructuredEvent::ConversationReset { .. }))
        .expect("a conversation reset");
    assert!(matches!(
        &events[reset],
        StructuredEvent::ConversationReset { trigger, .. } if trigger.as_deref() == Some("clear")
    ));
    let session_of = |e: &StructuredEvent| match e {
        StructuredEvent::Init { session_id, .. } => Some(session_id.clone()),
        _ => None,
    };
    let before = events[..reset]
        .iter()
        .find_map(session_of)
        .expect("an init before");
    let after = events[reset..]
        .iter()
        .find_map(session_of)
        .expect("an init after");
    assert_eq!(before, "<SESSION_ID>");
    assert_ne!(after, before, "the init after a clear names a new session");
}

#[test]
fn the_new_lines_tolerate_missing_and_unknown_fields() {
    match parse_line(r#"{"type":"system","subtype":"compact_boundary","x":1}"#) {
        StructuredEvent::CompactBoundary {
            trigger: None,
            pre_tokens: None,
            post_tokens: None,
            ..
        } => {}
        other => panic!("a boundary with no metadata, got {other:?}"),
    }
    match parse_line(
        r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"auto","pre_tokens":"lots","post_tokens":5,"new":true}}"#,
    ) {
        StructuredEvent::CompactBoundary {
            trigger,
            pre_tokens,
            post_tokens,
            ..
        } => {
            assert_eq!(trigger.as_deref(), Some("auto"));
            assert_eq!(pre_tokens, None, "a count that is not a number is unknown");
            assert_eq!(post_tokens, Some(5));
        }
        other => panic!("a boundary, got {other:?}"),
    }
    assert!(matches!(
        parse_line(r#"{"type":"conversation_reset","unknown":[1]}"#),
        StructuredEvent::ConversationReset { trigger: None, .. }
    ));
    assert!(matches!(
        parse_line(r#"{"type":"user","isSynthetic":"yes","message":{"content":"x"}}"#),
        StructuredEvent::User {
            is_synthetic: false,
            ..
        }
    ));
    assert!(matches!(
        parse_line(r#"{"type":"assistant","error":7,"message":{"model":["m"],"content":[]}}"#),
        StructuredEvent::Assistant {
            model: None,
            error: None,
            ..
        }
    ));
}
