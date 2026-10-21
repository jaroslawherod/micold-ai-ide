//! Feature 939 — the transcript model over #938's recorded fixtures and synthetic events
//! (contracts/transcript-model.md: input rule, event → entry rules, guarantee G5).

use micold_core::structured::transcript::{
    input_summary, EntryKey, EntryKind, Outcome, RawReason, ResultPart, ToolStatus, Transcript,
};
use micold_core::structured::{
    decode_events, encode_events, ContentBlock, SequencedEvent, StructuredEvent, MAX_EVENTS,
};
use serde_json::json;
use std::collections::HashSet;

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{
    assistant_block, assistant_blocks, block_delta, block_start, events_of, input_json_delta,
    lines_of, message_start, sequenced, stream_event, stream_marker, text_delta, thinking_block,
    thinking_delta, tool_result, tool_use, turn_result, user_sent, user_text, with_message_id,
    with_parent, without_stream_events, PROMPT_RECORDED, RECORDED,
};

/// A transcript built from `events` in one history message.
fn built(events: &[SequencedEvent]) -> Transcript {
    let mut transcript = Transcript::new();
    transcript.apply(0, events, false);
    transcript
}

/// The fixture's events without its partial-message lines, numbered from `0`.
fn finished_events_of(name: &str) -> Vec<SequencedEvent> {
    events_of(&without_stream_events(&lines_of(name)))
}

fn kinds(transcript: &Transcript) -> Vec<EntryKind> {
    transcript.entries().map(|e| e.kind().clone()).collect()
}

/// The entries that are not raw rows: the conversation the fixture holds.
fn conversation(transcript: &Transcript) -> Vec<EntryKind> {
    kinds(transcript)
        .into_iter()
        .filter(|k| !matches!(k, EntryKind::Raw { .. }))
        .collect()
}

fn text(text: &str) -> ContentBlock {
    ContentBlock::Text {
        text: text.to_string(),
    }
}

fn one(seq: u64, event: StructuredEvent) -> [SequencedEvent; 1] {
    [SequencedEvent { seq, event }]
}

fn card_of<'a>(
    transcript: &'a Transcript,
    id: &str,
) -> &'a micold_core::structured::transcript::ToolCard {
    match transcript
        .entry(&EntryKey::Tool(id.to_string()))
        .map(|e| e.kind())
    {
        Some(EntryKind::ToolCall(card)) => card,
        other => panic!("tool use {id} has a card, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------
// T004 — replace-or-continue (#938 wire.md), emptiness, the omitted notice, lookup.

#[test]
fn an_empty_history_message_is_a_fresh_empty_transcript() {
    let mut transcript = built(&sequenced([user_text("hi")]));
    transcript.apply(0, &[], false);
    assert!(transcript.is_empty(), "no event is held (FR-006)");
    assert_eq!(transcript.entries().count(), 0);
    assert_eq!(transcript.omitted_notice(), None);
}

#[test]
fn a_history_message_continuing_the_history_in_progress_appends() {
    let events = sequenced([
        user_text("one"),
        assistant_block(text("two")),
        user_text("three"),
    ]);
    let mut transcript = Transcript::new();
    transcript.apply(4, &events[..2], false);
    transcript.apply(0, &events[2..], false);
    assert_eq!(kinds(&transcript), kinds(&built(&events)));
    assert_eq!(transcript.rebuilds(), 0, "a continuation is not a rebuild");
    assert_eq!(
        transcript.omitted_notice(),
        Some(4),
        "the first message's omitted stays"
    );
}

#[test]
fn a_history_message_that_does_not_continue_rebuilds() {
    let events = sequenced([user_text("one"), assistant_block(text("two"))]);
    let mut transcript = built(&events);
    transcript.apply(0, &events, false);
    assert_eq!(
        kinds(&transcript),
        kinds(&built(&events)),
        "the same history again is shown once"
    );
    assert_eq!(
        transcript.rebuilds(),
        1,
        "FR-021: a history that does not continue is rebuilt"
    );

    // A history after a live event is a fresh snapshot even when its seq continues (wire.md).
    let mut transcript = built(&events[..1]);
    transcript.apply(0, &events[1..], true);
    transcript.apply(0, &one(2, user_text("three")), false);
    assert_eq!(
        kinds(&transcript),
        vec![EntryKind::UserMessage {
            text: "three".to_string(),
            attachments: vec![],
        }],
        "a history after a live event replaces the entries"
    );
    assert_eq!(
        transcript.rebuilds(),
        1,
        "FR-021: a history after a live event is rebuilt"
    );
}

#[test]
fn a_live_message_appends() {
    let events = sequenced([user_text("one"), assistant_block(text("two"))]);
    let mut transcript = built(&events[..1]);
    transcript.apply(0, &events[1..], true);
    assert_eq!(
        kinds(&transcript),
        kinds(&built(&events)),
        "a live message appends to the entries"
    );
    assert_eq!(
        transcript.rebuilds(),
        0,
        "FR-021: a live append is not a rebuild"
    );
}

#[test]
fn a_transcript_with_no_event_is_empty() {
    assert!(Transcript::new().is_empty(), "nothing yet (FR-006)");
}

#[test]
fn omitted_events_are_not_held() {
    let mut transcript = Transcript::new();
    transcript.apply(3, &[], false);
    assert!(transcript.is_empty(), "omitted events are not held");
}

#[test]
fn a_transcript_with_an_event_is_not_empty() {
    assert!(
        !built(&sequenced([user_text("hi")])).is_empty(),
        "a held event makes the transcript non-empty"
    );
}

#[test]
fn a_complete_history_has_no_omitted_notice() {
    assert_eq!(
        built(&sequenced([user_text("hi")])).omitted_notice(),
        None,
        "a history with nothing omitted carries no notice (FR-004)"
    );
}

#[test]
fn the_omitted_notice_carries_the_services_omitted_count() {
    let mut transcript = Transcript::new();
    transcript.apply(
        7,
        &[SequencedEvent {
            seq: 7,
            event: user_text("hi"),
        }],
        false,
    );
    assert_eq!(transcript.omitted_notice(), Some(7), "FR-004");
}

#[test]
fn an_entry_is_found_by_its_key() {
    let transcript = built(&sequenced([
        user_text("hi"),
        tool_use("toolu_1", "Bash", r#"{"command":"ls"}"#),
    ]));
    assert_eq!(
        transcript
            .entry(&EntryKey::At { seq: 0, block: 0 })
            .map(|e| e.kind()),
        Some(&EntryKind::UserMessage {
            text: "hi".to_string(),
            attachments: vec![],
        }),
        "an entry is found by its positional key"
    );
}

#[test]
fn a_tool_card_is_found_by_its_tool_use_id() {
    let transcript = built(&sequenced([
        user_text("hi"),
        tool_use("toolu_1", "Bash", r#"{"command":"ls"}"#),
    ]));
    assert_eq!(
        card_of(&transcript, "toolu_1").name,
        "Bash",
        "a tool card is found by its tool-use id"
    );
}

#[test]
fn a_key_with_no_entry_finds_nothing() {
    let transcript = built(&sequenced([
        user_text("hi"),
        tool_use("toolu_1", "Bash", r#"{"command":"ls"}"#),
    ]));
    assert!(
        transcript
            .entry(&EntryKey::At { seq: 1, block: 0 })
            .is_none(),
        "a positional key whose event made no entry at that block finds nothing"
    );
}

// ---------------------------------------------------------------------------------------
// T007 — US1 at the model: session start, messages, text, footers, raw rows, notices.

#[test]
fn a_text_turn_is_session_start_reply_and_a_success_footer() {
    let transcript = built(&finished_events_of("text-turn"));
    let entries = conversation(&transcript);
    assert_eq!(entries.len(), 3, "{entries:?}");
    match &entries[0] {
        EntryKind::SessionStart { model, cwd, .. } => {
            // The recording's own model: the init line of fixtures/stream-json/text-turn.jsonl.
            assert_eq!(
                model.as_deref(),
                Some("claude-sonnet-5-5"),
                "the session start names the recording's model"
            );
            assert!(cwd.is_some(), "the session start names the folder");
        }
        other => panic!("the transcript starts with the session, got {other:?}"),
    }
    match &entries[1] {
        EntryKind::AssistantText { text, streaming } => {
            assert!(
                text.contains("blue"),
                "the reply is the recording's answer, which names a colour: {text}"
            );
            assert!(!streaming, "a finished turn's reply is not streaming");
        }
        other => panic!("then the reply, got {other:?}"),
    }
    match &entries[2] {
        EntryKind::Footer(footer) => {
            // 6674 ms and 0.0371549 USD are the recording's: the result line of text-turn.jsonl.
            assert_eq!(footer.outcome, Outcome::Success, "a success result");
            assert_eq!(
                footer.duration_ms,
                Some(6674),
                "the footer carries the recording's duration"
            );
            assert_eq!(
                footer.cost_usd,
                Some(0.0371549),
                "the footer carries the recording's cost"
            );
            assert_eq!(footer.num_turns, Some(1), "one turn in the recording");
            assert_eq!(footer.error_text, None, "a success has no error text");
        }
        other => panic!("then the footer, got {other:?}"),
    }
    assert!(
        !entries
            .iter()
            .any(|k| matches!(k, EntryKind::UserMessage { .. })),
        "the recording echoes no user message"
    );
}

#[test]
fn a_user_text_event_is_a_user_message_in_its_place() {
    let transcript = built(&sequenced([
        assistant_block(text("before")),
        user_text("hello"),
        assistant_block(text("after")),
    ]));
    assert_eq!(
        kinds(&transcript)[1],
        EntryKind::UserMessage {
            text: "hello".to_string(),
            attachments: vec![],
        }
    );
}

#[test]
fn an_error_turn_footer_carries_its_error_text() {
    let transcript = built(&finished_events_of("error-result"));
    let footer = transcript
        .entries()
        .find_map(|e| match e.kind() {
            EntryKind::Footer(footer) => Some(footer.clone()),
            _ => None,
        })
        .expect("the turn has a footer");
    assert_eq!(footer.outcome, Outcome::Error);
    assert_eq!(
        footer.error_text.as_deref(),
        Some("error_max_turns"),
        "no result text: the subtype"
    );

    let transcript = built(&sequenced([turn_result(
        true,
        "error_during_execution",
        Some("it broke"),
    )]));
    match &kinds(&transcript)[0] {
        EntryKind::Footer(footer) => {
            assert_eq!(footer.error_text.as_deref(), Some("it broke"), "FR-005")
        }
        other => panic!("a footer, got {other:?}"),
    }
}

#[test]
fn every_unmodelled_line_and_block_is_a_raw_row_in_its_place() {
    let lines = lines_of("tolerance");
    let transcript = built(&events_of(&lines));
    for (seq, reason) in [
        (1, RawReason::Unrecognised),
        (3, RawReason::Unrecognised),
        (4, RawReason::Unrecognised),
        (6, RawReason::Unreadable),
    ] {
        assert_eq!(
            transcript
                .entry(&EntryKey::At { seq, block: 0 })
                .map(|e| e.kind()),
            Some(&EntryKind::Raw {
                text: lines[seq as usize].clone(),
                reason
            }),
            "line {seq} is a raw row with its text (FR-003)"
        );
    }
    assert_eq!(
        transcript
            .entry(&EntryKey::At { seq: 5, block: 0 })
            .map(|e| e.kind()),
        Some(&EntryKind::AssistantText {
            text: "before".to_string(),
            streaming: false
        })
    );
    match transcript
        .entry(&EntryKey::At { seq: 5, block: 1 })
        .map(|e| e.kind())
    {
        Some(EntryKind::Raw {
            text,
            reason: RawReason::UnknownBlock,
        }) => assert!(text.contains("hologram"), "{text}"),
        other => panic!("an unknown block is a raw row, got {other:?}"),
    }
}

#[test]
fn a_turn_ended_without_result_stands_in_place_of_its_footer() {
    let transcript = built(&sequenced([
        user_text("go"),
        assistant_block(text("partial")),
        StructuredEvent::TurnEndedWithoutResult,
    ]));
    assert_eq!(
        kinds(&transcript).last(),
        Some(&EntryKind::TurnEndedWithoutResult),
        "US1 scenario 5"
    );
}

#[test]
fn an_oversized_event_is_a_notice_in_its_place() {
    let transcript = built(&sequenced([
        user_text("go"),
        StructuredEvent::OversizedOnWire {
            original_bytes: 9_000_000,
        },
        assistant_block(text("after")),
    ]));
    assert_eq!(
        kinds(&transcript)[1],
        EntryKind::Oversized {
            original_bytes: 9_000_000
        },
        "FR-004"
    );
}

#[test]
fn entries_are_in_opening_order_and_keyed_by_their_place() {
    for name in RECORDED.iter().chain(&["tolerance"]) {
        let events = finished_events_of(name);
        let transcript = built(&events);
        let mut last = 0;
        for entry in transcript.entries() {
            assert!(entry.opened_at() >= last, "{name}: in opening order");
            last = entry.opened_at();
            match entry.key() {
                EntryKey::At { seq, .. } => assert_eq!(*seq, entry.opened_at(), "{name}"),
                EntryKey::Tool(_) => assert!(
                    matches!(entry.kind(), EntryKind::ToolCall(_)),
                    "{name}: only a card is keyed by its tool use"
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// T012 — US2 at the model: cards, pairing, statuses, thinking, summaries, sub-agents (G5).

#[test]
fn a_tool_turn_is_one_card_per_tool_use_holding_its_result() {
    let events = finished_events_of("tool-turn");
    let transcript = built(&events);
    let uses: Vec<(u64, String, String, String)> = events
        .iter()
        .flat_map(|e| match &e.event {
            StructuredEvent::Assistant { blocks, .. } => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((e.seq, id.clone(), name.clone(), input.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();
    assert!(!uses.is_empty(), "the fixture holds a tool use");
    let cards = transcript
        .entries()
        .filter(|e| matches!(e.kind(), EntryKind::ToolCall(_)))
        .count();
    assert_eq!(cards, uses.len(), "one card per tool use (FR-007)");
    for (seq, id, name, input) in uses {
        let entry = transcript
            .entry(&EntryKey::Tool(id.clone()))
            .expect("the card is keyed by its tool use");
        assert_eq!(entry.opened_at(), seq, "where the tool use occurred");
        let card = card_of(&transcript, &id);
        assert_eq!(card.name, name);
        assert_eq!(card.status, ToolStatus::Done, "US2 scenario 1");
        assert_eq!(card.summary, input_summary(&name, &input));
        let result = card.result.as_ref().expect("the card holds its result");
        assert!(!result.is_error);
        assert!(
            result
                .parts
                .iter()
                .any(|p| matches!(p, ResultPart::Text(t) if t.contains("42"))),
            "{result:?}"
        );
    }
    assert!(
        !kinds(&transcript)
            .iter()
            .any(|k| matches!(k, EntryKind::OrphanResult { .. })),
        "the result is not an entry of its own"
    );
}

#[test]
fn an_error_result_fails_its_card() {
    let transcript = built(&sequenced([
        tool_use("toolu_1", "Bash", r#"{"command":"false"}"#),
        tool_result("toolu_1", "exit 1", true),
    ]));
    assert_eq!(card_of(&transcript, "toolu_1").status, ToolStatus::Failed);
}

#[test]
fn a_result_turns_its_running_card_done_in_place() {
    for (is_error, status) in [(false, ToolStatus::Done), (true, ToolStatus::Failed)] {
        let mut transcript = built(&sequenced([
            user_text("go"),
            tool_use("toolu_1", "Read", r#"{"file_path":"/a"}"#),
        ]));
        assert_eq!(card_of(&transcript, "toolu_1").status, ToolStatus::Running);
        let before: Vec<_> = transcript.entries().cloned().collect();
        transcript.apply(0, &one(2, tool_result("toolu_1", "text", is_error)), true);
        let after: Vec<_> = transcript.entries().cloned().collect();
        assert_eq!(after.len(), before.len(), "no new entry (FR-007)");
        assert_eq!(after[0], before[0], "the other entry is untouched");
        assert!(after[1].revision() > before[1].revision(), "FR-008");
        assert_eq!(after[1].key(), before[1].key());
        assert_eq!(card_of(&transcript, "toolu_1").status, status);
    }
}

#[test]
fn a_turn_end_turns_its_running_cards_to_no_result_and_a_late_result_still_lands() {
    for end in [
        turn_result(false, "success", None),
        StructuredEvent::TurnEndedWithoutResult,
    ] {
        let mut transcript = built(&sequenced([
            tool_use("toolu_1", "Bash", r#"{"command":"sleep 9"}"#),
            tool_use("toolu_2", "Bash", r#"{"command":"ls"}"#),
            tool_result("toolu_2", "a", false),
            end,
        ]));
        assert_eq!(card_of(&transcript, "toolu_1").status, ToolStatus::NoResult);
        assert_eq!(card_of(&transcript, "toolu_2").status, ToolStatus::Done);
        transcript.apply(0, &one(4, tool_result("toolu_1", "late", false)), true);
        assert_eq!(
            card_of(&transcript, "toolu_1").status,
            ToolStatus::Done,
            "US2 scenario 6"
        );
    }
}

#[test]
fn a_result_with_no_card_is_an_orphan_result() {
    let transcript = built(&sequenced([tool_result("toolu_gone", "text", true)]));
    assert_eq!(
        kinds(&transcript),
        vec![EntryKind::OrphanResult {
            tool_use_id: "toolu_gone".to_string(),
            parts: vec![ResultPart::Text("text".to_string())],
            is_error: true,
        }]
    );
}

#[test]
fn a_thinking_block_is_a_thinking_entry() {
    let transcript = built(&sequenced([thinking_block("hmm")]));
    assert_eq!(
        kinds(&transcript),
        vec![EntryKind::Thinking {
            text: "hmm".to_string(),
            streaming: false
        }],
        "US2 scenario 5"
    );
}

/// A main session that ran a sub-agent under `toolu_task`, and a sub-agent message naming a tool
/// use the transcript does not hold.
fn with_subagent() -> Vec<SequencedEvent> {
    sequenced([
        user_text("go"),
        tool_use("toolu_task", "Task", r#"{"description":"look around"}"#),
        with_parent(user_text("sub prompt"), "toolu_task"),
        with_parent(assistant_block(text("sub reply")), "toolu_task"),
        with_parent(assistant_block(text("elsewhere")), "toolu_other"),
        tool_result("toolu_task", "found it", false),
        with_parent(assistant_block(text("late elsewhere")), "toolu_other"),
        turn_result(false, "success", None),
    ])
}

#[test]
fn sub_agent_messages_are_counted_not_shown() {
    let transcript = built(&with_subagent());
    let card = card_of(&transcript, "toolu_task");
    assert_eq!(card.subagent_messages, 2, "FR-024");
    assert_eq!(card.status, ToolStatus::Done);
    let notices: Vec<_> = transcript
        .entries()
        .filter(|e| matches!(e.kind(), EntryKind::SubagentNotice { .. }))
        .collect();
    assert_eq!(notices.len(), 1, "one notice for every orphan message");
    assert_eq!(notices[0].kind(), &EntryKind::SubagentNotice { count: 2 });
    assert_eq!(
        notices[0].key(),
        &EntryKey::At { seq: 4, block: 0 },
        "in the place of the first"
    );
    assert!(
        !kinds(&transcript).iter().any(|k| matches!(
            k,
            EntryKind::AssistantText { text, .. } | EntryKind::UserMessage { text, .. }
                if text.contains("sub") || text.contains("elsewhere")
        )),
        "no sub-agent text in the main transcript"
    );
}

#[test]
fn removing_sub_agent_events_changes_only_the_counts() {
    let events = with_subagent();
    let main_only: Vec<SequencedEvent> = events
        .iter()
        .filter(|e| {
            !matches!(
                &e.event,
                StructuredEvent::Assistant {
                    parent_tool_use_id: Some(_),
                    ..
                } | StructuredEvent::User {
                    parent_tool_use_id: Some(_),
                    ..
                }
            )
        })
        .cloned()
        .collect();
    let without_counts = |transcript: &Transcript| -> Vec<(EntryKey, EntryKind)> {
        transcript
            .entries()
            .filter(|e| !matches!(e.kind(), EntryKind::SubagentNotice { .. }))
            .map(|e| {
                let mut kind = e.kind().clone();
                if let EntryKind::ToolCall(card) = &mut kind {
                    card.subagent_messages = 0;
                }
                (e.key().clone(), kind)
            })
            .collect()
    };
    assert_eq!(
        without_counts(&built(&events)),
        without_counts(&built(&main_only)),
        "G5"
    );
}

#[test]
fn sub_agent_events_stay_in_the_history_in_order() {
    let transcript = built(&with_subagent());
    let seqs: Vec<u64> = transcript
        .subagent_events("toolu_task")
        .map(|e| e.seq)
        .collect();
    assert_eq!(seqs, vec![2, 3]);
    let seqs: Vec<u64> = transcript
        .subagent_events("toolu_other")
        .map(|e| e.seq)
        .collect();
    assert_eq!(seqs, vec![4, 6]);
}

#[test]
fn a_nested_transcript_shows_its_sub_agent_as_top_level() {
    let mut nested = Transcript::nested("toolu_task");
    nested.apply(0, &with_subagent(), false);
    assert_eq!(
        kinds(&nested),
        vec![
            EntryKind::UserMessage {
                text: "sub prompt".to_string(),
                attachments: vec![],
            },
            EntryKind::AssistantText {
                text: "sub reply".to_string(),
                streaming: false
            },
        ],
        "the sub-agent's messages, and nothing of the main session or other sub-agents"
    );
}

#[test]
fn a_message_of_several_blocks_is_one_entry_per_block() {
    let transcript = built(&sequenced([assistant_blocks(vec![
        ContentBlock::Thinking {
            thinking: "plan".to_string(),
        },
        text("answer"),
        ContentBlock::ToolUse {
            id: "toolu_1".to_string(),
            name: "Grep".to_string(),
            input: r#"{"pattern":"fn main"}"#.to_string(),
        },
    ])]));
    let keys: Vec<EntryKey> = transcript.entries().map(|e| e.key().clone()).collect();
    assert_eq!(
        keys,
        vec![
            EntryKey::At { seq: 0, block: 0 },
            EntryKey::At { seq: 0, block: 1 },
            EntryKey::Tool("toolu_1".to_string()),
        ]
    );
    assert_eq!(card_of(&transcript, "toolu_1").summary, "fn main");
}

// ---------------------------------------------------------------------------------------
// T016 — US3 at the model: streamed entries grow in place, the finished message supersedes
// them (G2, SC-002), and every recorded event is accounted for (G1, SC-001).

/// Applies `events` one live message each.
fn replayed(events: &[SequencedEvent]) -> Transcript {
    let mut transcript = Transcript::new();
    for event in events {
        transcript.apply(0, std::slice::from_ref(event), true);
    }
    transcript
}

/// `kind` with its `streaming` flags cleared.
fn settled(kind: &EntryKind) -> EntryKind {
    let mut kind = kind.clone();
    match &mut kind {
        EntryKind::AssistantText { streaming, .. } | EntryKind::Thinking { streaming, .. } => {
            *streaming = false;
        }
        EntryKind::ToolCall(card) => card.streaming = false,
        _ => {}
    }
    kind
}

fn snapshot(transcript: &Transcript) -> Vec<(EntryKey, u64, EntryKind)> {
    transcript
        .entries()
        .map(|e| (e.key().clone(), e.revision(), e.kind().clone()))
        .collect()
}

/// The last entry, which a streamed event grows.
fn last(transcript: &Transcript) -> (EntryKey, u64, EntryKind) {
    snapshot(transcript).pop().expect("an entry")
}

#[test]
fn replaying_a_recorded_fixture_one_event_at_a_time_changes_only_the_streamed_entry() {
    for name in RECORDED {
        let events = events_of(&lines_of(name));
        let mut transcript = Transcript::new();
        for event in &events {
            let before = snapshot(&transcript);
            transcript.apply(0, std::slice::from_ref(event), true);
            let after = snapshot(&transcript);
            assert!(
                after.len() >= before.len(),
                "{name} seq {}: no entry leaves",
                event.seq
            );
            let changed: Vec<usize> = (0..before.len())
                .filter(|&i| before[i] != after[i])
                .collect();
            assert!(
                changed.len() <= 1,
                "{name} seq {}: only the touched entry changes (FR-016), changed {changed:?}",
                event.seq
            );
            for i in changed {
                assert_eq!(before[i].0, after[i].0, "{name}: an entry keeps its key");
                assert!(after[i].1 > before[i].1, "{name}: its revision rises");
            }
        }
        let streamed: Vec<EntryKind> = transcript.entries().map(|e| settled(e.kind())).collect();
        let finished: Vec<EntryKind> = built(&finished_events_of(name))
            .entries()
            .map(|e| settled(e.kind()))
            .collect();
        // A raw row that arrives while a block streams sits after that block's entry, which
        // opened at its first delta, but before it when only the finished message is held
        // (the 944 recordings' `thinking_tokens`): the conversation and the raw rows are each
        // identical, in order; only how the two interleave may differ.
        let is_raw = |k: &EntryKind| matches!(k, EntryKind::Raw { .. });
        let split = |all: &[EntryKind]| -> (Vec<EntryKind>, Vec<EntryKind>) {
            all.iter().cloned().partition(|k| !is_raw(k))
        };
        assert_eq!(
            split(&streamed),
            split(&finished),
            "{name}: SC-002, no text duplicated or lost"
        );
        assert!(
            !transcript.entries().any(|e| settled(e.kind()) != *e.kind()),
            "{name}: nothing is still streaming once the turn ended"
        );
        assert_eq!(
            snapshot(&transcript)
                .into_iter()
                .map(|(key, _, kind)| (key, kind))
                .collect::<Vec<_>>(),
            snapshot(&built(&events))
                .into_iter()
                .map(|(key, _, kind)| (key, kind))
                .collect::<Vec<_>>(),
            "{name}: one event at a time builds what one message builds"
        );
    }
}

/// The recordings in which no unmodelled event arrives while a block streams: #938's five.
const NO_RAW_MID_STREAM: [&str; 5] = [
    "text-turn",
    "tool-turn",
    "partial-deltas",
    "error-result",
    "resume-init",
];

#[test]
fn a_recording_with_no_raw_row_mid_stream_streams_into_its_turn_without_deltas_exactly() {
    // SC-002 (the partial-message fixture among them): with no raw row arriving mid-stream, even
    // the order of every entry is the same.
    let settled_kinds = |transcript: Transcript| -> Vec<EntryKind> {
        transcript.entries().map(|e| settled(e.kind())).collect()
    };
    for name in NO_RAW_MID_STREAM {
        assert_eq!(
            settled_kinds(built(&events_of(&lines_of(name)))),
            settled_kinds(built(&finished_events_of(name))),
            "{name}: SC-002, the streamed turn is the finished turn, entry for entry"
        );
    }
}

#[test]
fn each_text_delta_grows_the_same_reply_in_place() {
    let events = events_of(&lines_of("partial-deltas"));
    let start = events
        .iter()
        .position(|e| {
            matches!(&e.event, StructuredEvent::StreamDelta { event_type, .. }
                if event_type == "content_block_start")
        })
        .expect("the fixture streams a block");
    let mut transcript = replayed(&events[..=start]);
    let (key, mut revision, kind) = last(&transcript);
    assert_eq!(
        kind,
        EntryKind::AssistantText {
            text: String::new(),
            streaming: true
        },
        "the reply appears at its block start"
    );
    let count = transcript.entries().len();
    let mut text = String::new();
    for event in &events[start + 1..] {
        let StructuredEvent::StreamDelta {
            delta: Some(micold_core::structured::Delta::Text { text: delta }),
            ..
        } = &event.event
        else {
            break;
        };
        text.push_str(delta);
        transcript.apply(0, std::slice::from_ref(event), true);
        assert_eq!(
            transcript.entries().len(),
            count,
            "no entry added (US3 scenario 1)"
        );
        let (now_key, now_revision, now_kind) = last(&transcript);
        assert_eq!(now_key, key);
        assert!(now_revision > revision);
        revision = now_revision;
        assert_eq!(
            now_kind,
            EntryKind::AssistantText {
                text: text.clone(),
                streaming: true
            },
            "FR-014"
        );
    }
    assert!(text.len() > 20, "the fixture streamed its reply: {text:?}");
}

#[test]
fn thinking_and_tool_input_deltas_grow_entries_created_at_block_start() {
    let events = sequenced([
        message_start("msg_1"),
        block_start(0, json!({ "type": "thinking", "thinking": "" })),
        thinking_delta(0, "pl"),
        thinking_delta(0, "an"),
        block_delta(0, json!({ "type": "signature_delta", "signature": "c2ln" })),
        block_start(
            1,
            json!({ "type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {} }),
        ),
        input_json_delta(1, r#"{"comm"#),
        input_json_delta(1, r#"and": "ls -la"}"#),
    ]);
    let thinking_key = EntryKey::At { seq: 1, block: 0 };
    let thinking = |t: &Transcript| t.entry(&thinking_key).map(|e| e.kind().clone());

    let transcript = replayed(&events[..2]);
    assert_eq!(
        thinking(&transcript),
        Some(EntryKind::Thinking {
            text: String::new(),
            streaming: true
        }),
        "the thinking card appears at once (US3 scenario 3)"
    );
    let transcript = replayed(&events[..5]);
    assert_eq!(
        thinking(&transcript),
        Some(EntryKind::Thinking {
            text: "plan".to_string(),
            streaming: true
        })
    );
    assert_eq!(
        transcript.entries().len(),
        1,
        "a signature delta is absorbed"
    );

    let transcript = replayed(&events[..6]);
    let card = card_of(&transcript, "toolu_1");
    assert_eq!(
        (
            card.name.as_str(),
            card.input.as_str(),
            card.status,
            card.streaming
        ),
        ("Bash", "", ToolStatus::Running, true),
        "the tool card appears at its block start"
    );
    let transcript = replayed(&events[..7]);
    assert_eq!(card_of(&transcript, "toolu_1").input, r#"{"comm"#);
    assert_eq!(
        card_of(&transcript, "toolu_1").summary,
        input_summary("Bash", r#"{"comm"#),
        "the summary follows the partial input"
    );
    let mut transcript = replayed(&events);
    let card = card_of(&transcript, "toolu_1");
    assert_eq!(card.input, r#"{"command": "ls -la"}"#);
    assert_eq!(card.summary, "ls -la");
    assert_eq!(transcript.entries().len(), 2);

    let finished = sequenced([
        thinking_block("plan"),
        tool_use("toolu_1", "Bash", r#"{"command":"ls -la"}"#),
    ]);
    for (offset, event) in finished.into_iter().enumerate() {
        transcript.apply(
            0,
            &one(8 + offset as u64, with_message_id(event.event, "msg_1")),
            true,
        );
    }
    assert_eq!(
        transcript.entries().len(),
        2,
        "superseded in place, not appended"
    );
    assert_eq!(
        thinking(&transcript),
        Some(EntryKind::Thinking {
            text: "plan".to_string(),
            streaming: false
        })
    );
    let card = card_of(&transcript, "toolu_1");
    assert_eq!(
        (card.input.as_str(), card.streaming, card.status),
        (r#"{"command":"ls -la"}"#, false, ToolStatus::Running)
    );
}

#[test]
fn the_finished_message_replaces_the_streamed_text_once_in_place() {
    let mut events = sequenced([
        user_text("hi"),
        message_start("msg_1"),
        block_start(0, json!({ "type": "text", "text": "" })),
        text_delta(0, "Hel"),
        text_delta(0, "lo"),
        with_message_id(assistant_block(text("Hello.")), "msg_1"),
    ]);
    let transcript = replayed(&events);
    let entries = snapshot(&transcript);
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(
        entries[1].0,
        EntryKey::At { seq: 2, block: 0 },
        "in the same place"
    );
    assert_eq!(
        entries[1].2,
        EntryKind::AssistantText {
            text: "Hello.".to_string(),
            streaming: false
        },
        "the finished text, once (US3 scenario 2, FR-015)"
    );

    // The rest of the message is absorbed; the stream is closed.
    events.extend(
        [
            stream_marker("content_block_stop"),
            stream_marker("message_delta"),
            stream_marker("ping"),
            stream_marker("message_stop"),
        ]
        .into_iter()
        .zip(6u64..)
        .map(|(event, seq)| SequencedEvent { seq, event }),
    );
    let closed = replayed(&events);
    assert_eq!(snapshot(&closed), entries, "absorbed: nothing changes");

    // A finished message with no stream open is a new entry.
    let mut later = closed.clone();
    later.apply(0, &one(10, assistant_block(text("More."))), true);
    assert_eq!(later.entries().len(), 3);

    // A finished message of another message id does not supersede the stream.
    let other = replayed(&sequenced([
        message_start("msg_1"),
        block_start(0, json!({ "type": "text", "text": "" })),
        text_delta(0, "part"),
        with_message_id(assistant_block(text("Other.")), "msg_2"),
    ]));
    assert_eq!(
        kinds(&other),
        vec![
            EntryKind::AssistantText {
                text: "part".to_string(),
                streaming: true
            },
            EntryKind::AssistantText {
                text: "Other.".to_string(),
                streaming: false
            },
        ]
    );

    // An unknown id on either side supersedes.
    let unknown = replayed(&sequenced([
        message_start("msg_1"),
        block_start(0, json!({ "type": "text", "text": "" })),
        text_delta(0, "part"),
        assistant_block(text("Whole.")),
    ]));
    assert_eq!(
        kinds(&unknown),
        vec![EntryKind::AssistantText {
            text: "Whole.".to_string(),
            streaming: false
        }]
    );
}

#[test]
fn deltas_never_finished_keep_their_partial_text() {
    let streamed = || {
        vec![
            message_start("msg_1"),
            block_start(0, json!({ "type": "text", "text": "" })),
            text_delta(0, "half a th"),
        ]
    };
    let partial = EntryKind::AssistantText {
        text: "half a th".to_string(),
        streaming: false,
    };

    let mut events = streamed();
    events.push(StructuredEvent::TurnEndedWithoutResult);
    events.push(assistant_block(text("after")));
    let transcript = replayed(&sequenced(events));
    assert_eq!(
        kinds(&transcript),
        vec![
            partial.clone(),
            EntryKind::TurnEndedWithoutResult,
            EntryKind::AssistantText {
                text: "after".to_string(),
                streaming: false
            },
        ],
        "the turn's end closes the stream; a later message does not supersede"
    );

    let mut events = streamed();
    events.push(message_start("msg_2"));
    let transcript = replayed(&sequenced(events));
    assert_eq!(
        kinds(&transcript),
        vec![partial],
        "a new message closes the open one"
    );
}

#[test]
fn an_unknown_stream_event_or_delta_is_a_raw_row() {
    let unknown_event = stream_event(json!({ "type": "message_rewind", "index": 0 }));
    let unknown_delta = block_delta(0, json!({ "type": "citations_delta", "citation": {} }));
    let raw_of = |event: &StructuredEvent| match event {
        StructuredEvent::StreamDelta { raw, .. } => raw.clone(),
        other => panic!("a stream event, got {other:?}"),
    };
    let expected = vec![
        EntryKind::AssistantText {
            text: String::new(),
            streaming: true,
        },
        EntryKind::Raw {
            text: raw_of(&unknown_event),
            reason: RawReason::UnknownStreamEvent,
        },
        EntryKind::Raw {
            text: raw_of(&unknown_delta),
            reason: RawReason::UnknownStreamEvent,
        },
    ];
    let transcript = replayed(&sequenced([
        message_start("msg_1"),
        block_start(0, json!({ "type": "text", "text": "" })),
        unknown_event,
        unknown_delta,
    ]));
    assert_eq!(kinds(&transcript), expected, "FR-003");
}

#[test]
fn a_sub_agents_deltas_are_absorbed_not_counted() {
    let transcript = replayed(&sequenced([
        tool_use("toolu_task", "Task", r#"{"description":"look"}"#),
        with_parent(message_start("msg_sub"), "toolu_task"),
        with_parent(
            block_start(0, json!({ "type": "text", "text": "" })),
            "toolu_task",
        ),
        with_parent(text_delta(0, "sub"), "toolu_task"),
        with_parent(stream_marker("message_stop"), "toolu_task"),
        with_parent(stream_marker("brand_new"), "toolu_other"),
    ]));
    assert_eq!(transcript.entries().len(), 1, "{:?}", kinds(&transcript));
    assert_eq!(card_of(&transcript, "toolu_task").subagent_messages, 0);
}

/// The parent tool use a sub-agent's event names (FR-024).
fn subagent_parent(event: &StructuredEvent) -> Option<&str> {
    match event {
        StructuredEvent::Assistant {
            parent_tool_use_id, ..
        }
        | StructuredEvent::User {
            parent_tool_use_id, ..
        }
        | StructuredEvent::StreamDelta {
            parent_tool_use_id, ..
        } => parent_tool_use_id.as_deref(),
        _ => None,
    }
}

/// Whether an entry was opened by the event `seq`.
fn opened_by(transcript: &Transcript, seq: u64) -> bool {
    transcript.entries().any(|e| e.opened_at() == seq)
}

#[test]
fn every_event_of_every_recorded_fixture_is_accounted_for() {
    const ABSORBED: [&str; 5] = [
        "message_start",
        "content_block_stop",
        "message_delta",
        "message_stop",
        "ping",
    ];
    for name in RECORDED {
        let events = events_of(&lines_of(name));
        let transcript = built(&events);
        let all = kinds(&transcript);
        let mut uses = HashSet::new();
        let mut subagent_messages = 0;
        for SequencedEvent { seq, event } in &events {
            let at = format!("{name} seq {seq}");
            if let Some(parent) = subagent_parent(event) {
                // FR-024: a sub-agent's event opens no entry of its own; its messages are
                // counted on the card it names, or on the one notice when none is held.
                assert!(
                    transcript.entries().all(|e| e.opened_at() != *seq
                        || matches!(e.kind(), EntryKind::SubagentNotice { .. })),
                    "{at}: a sub-agent event of {parent} is not shown"
                );
                if matches!(
                    event,
                    StructuredEvent::Assistant { .. } | StructuredEvent::User { .. }
                ) {
                    subagent_messages += 1;
                }
                continue;
            }
            match event {
                StructuredEvent::StreamDelta {
                    event_type, delta, ..
                } => match event_type.as_str() {
                    "content_block_start" => {
                        assert!(opened_by(&transcript, *seq), "{at}: its streamed entry");
                    }
                    "content_block_delta" => match delta {
                        // A thinking block's signature carries no text: absorbed.
                        Some(micold_core::structured::Delta::Other { raw })
                            if raw.contains("\"signature_delta\"") =>
                        {
                            assert!(!opened_by(&transcript, *seq), "{at}: a signature, absorbed")
                        }
                        Some(micold_core::structured::Delta::Other { .. }) | None => {
                            assert!(opened_by(&transcript, *seq), "{at}: a raw row")
                        }
                        Some(_) => assert!(
                            !opened_by(&transcript, *seq),
                            "{at}: a typed delta, grown into its entry"
                        ),
                    },
                    other => assert!(
                        ABSORBED.contains(&other) || opened_by(&transcript, *seq),
                        "{at}: {other} absorbed or a raw row"
                    ),
                },
                StructuredEvent::Assistant { blocks, .. } => {
                    for block in blocks {
                        match block {
                            ContentBlock::Text { text } => assert!(
                                all.contains(&EntryKind::AssistantText {
                                    text: text.clone(),
                                    streaming: false
                                }),
                                "{at}: its text"
                            ),
                            ContentBlock::Thinking { thinking } => assert!(
                                all.contains(&EntryKind::Thinking {
                                    text: thinking.clone(),
                                    streaming: false
                                }),
                                "{at}: its thinking"
                            ),
                            ContentBlock::ToolUse { id, input, .. } => {
                                assert_eq!(&card_of(&transcript, id).input, input, "{at}");
                                uses.insert(id.clone());
                            }
                            _ => assert!(opened_by(&transcript, *seq), "{at}: a raw row"),
                        }
                    }
                }
                StructuredEvent::User { blocks, .. } => {
                    for block in blocks {
                        match block {
                            ContentBlock::ToolResult { tool_use_id, .. } => assert!(
                                card_of(&transcript, tool_use_id).result.is_some(),
                                "{at}: its card holds it"
                            ),
                            _ => assert!(opened_by(&transcript, *seq), "{at}: its entry"),
                        }
                    }
                }
                StructuredEvent::Status { .. } | StructuredEvent::ControlResponse { .. } => {
                    assert!(!opened_by(&transcript, *seq), "{at}: absorbed (#942)")
                }
                _ => assert!(opened_by(&transcript, *seq), "{at}: its entry"),
            }
        }
        let cards = all
            .iter()
            .filter(|k| matches!(k, EntryKind::ToolCall(_)))
            .count();
        assert_eq!(cards, uses.len(), "{name}: one card per tool use");
        let counted: u64 = all
            .iter()
            .map(|k| match k {
                EntryKind::ToolCall(card) => card.subagent_messages,
                EntryKind::SubagentNotice { count } => *count,
                _ => 0,
            })
            .sum();
        assert_eq!(
            counted, subagent_messages,
            "{name}: every sub-agent message counted once (FR-024)"
        );
    }
}

// ---------------------------------------------------------------------------------------
// T017 — the bound (G4, FR-020) and the rebuild (G3, FR-013, FR-021).

/// `count` user messages numbered from `from`.
fn fillers(from: u64, count: usize) -> Vec<SequencedEvent> {
    (from..)
        .take(count)
        .map(|seq| SequencedEvent {
            seq,
            event: user_text("filler"),
        })
        .collect()
}

#[test]
fn past_the_bound_the_oldest_events_and_their_entries_leave_the_front() {
    let mut events = sequenced([
        assistant_blocks(vec![
            text("first"),
            ContentBlock::ToolUse {
                id: "toolu_old".to_string(),
                name: "Bash".to_string(),
                input: r#"{"command":"ls"}"#.to_string(),
            },
        ]),
        with_parent(assistant_block(text("sub")), "toolu_gone"),
        user_text("kept"),
    ]);
    events.extend(fillers(3, MAX_EVENTS - 2));
    let mut transcript = Transcript::new();
    transcript.apply(0, &events[..MAX_EVENTS], false);
    assert_eq!(transcript.held(), MAX_EVENTS);
    assert_eq!(transcript.omitted_notice(), None);

    transcript.apply(0, &events[MAX_EVENTS..], true);
    assert_eq!(transcript.held(), MAX_EVENTS, "G4");
    assert_eq!(
        transcript.omitted_notice(),
        Some(1),
        "the dropped event is counted"
    );
    assert!(
        transcript
            .entry(&EntryKey::Tool("toolu_old".to_string()))
            .is_none(),
        "the card of a dropped tool use leaves"
    );
    assert_eq!(
        transcript.entries().next().map(|e| e.key().clone()),
        Some(EntryKey::At { seq: 1, block: 0 }),
        "every entry the dropped event opened left the front"
    );

    // Two more: the notice and the user message leave, and the counts follow.
    let next = MAX_EVENTS as u64 + 1;
    transcript.apply(0, &fillers(next, 2), true);
    assert_eq!(transcript.held(), MAX_EVENTS);
    assert_eq!(transcript.omitted_notice(), Some(3));
    assert_eq!(transcript.entries().next().map(|e| e.opened_at()), Some(3));
    assert_eq!(transcript.entries().len(), MAX_EVENTS);

    // A late result of the dropped tool use is a result without its call, not an error.
    let late = next + 2;
    transcript.apply(0, &one(late, tool_result("toolu_old", "done", false)), true);
    assert!(matches!(
        transcript.entries().last().map(|e| e.kind()),
        Some(EntryKind::OrphanResult { tool_use_id, is_error: false, .. }) if tool_use_id == "toolu_old"
    ));
    // A sub-agent message after its notice left opens a new one in its place.
    transcript.apply(
        0,
        &one(
            late + 1,
            with_parent(assistant_block(text("sub")), "toolu_gone"),
        ),
        true,
    );
    assert_eq!(
        transcript
            .entries()
            .last()
            .map(|e| (e.key().clone(), e.kind().clone())),
        Some((
            EntryKey::At {
                seq: late + 1,
                block: 0
            },
            EntryKind::SubagentNotice { count: 1 }
        ))
    );
}

#[test]
fn the_omitted_notice_sums_the_services_omitted_and_the_dropped_events() {
    let mut transcript = Transcript::new();
    transcript.apply(4, &fillers(4, MAX_EVENTS + 3), false);
    assert_eq!(transcript.held(), MAX_EVENTS);
    assert_eq!(transcript.omitted_notice(), Some(4 + 3), "FR-004");
}

#[test]
fn rebuilding_from_the_same_history_keeps_every_key_once() {
    for name in RECORDED {
        let events = events_of(&lines_of(name));
        let keys = |t: &Transcript| t.entries().map(|e| e.key().clone()).collect::<Vec<_>>();
        let mut transcript = replayed(&events);
        let before = keys(&transcript);
        assert_eq!(
            before.iter().collect::<HashSet<_>>().len(),
            before.len(),
            "{name}: no key twice"
        );
        transcript.apply(0, &events, false);
        assert_eq!(
            transcript.rebuilds(),
            1,
            "{name}: FR-021 a history that does not continue is rebuilt once"
        );
        assert_eq!(keys(&transcript), before, "{name}: G3, FR-021");
    }
}

#[test]
fn a_streamed_block_superseded_by_another_kind_stops_streaming_when_the_stream_closes() {
    let transcript = replayed(&sequenced([
        message_start("msg_1"),
        block_start(0, json!({ "type": "thinking", "thinking": "" })),
        thinking_delta(0, "hm"),
        with_message_id(assistant_block(text("Answer.")), "msg_1"),
        stream_marker("message_stop"),
    ]));
    assert_eq!(
        kinds(&transcript),
        vec![
            EntryKind::Thinking {
                text: "hm".to_string(),
                streaming: false
            },
            EntryKind::AssistantText {
                text: "Answer.".to_string(),
                streaming: false
            },
        ]
    );
}

// #942 T007 — the CLI's mode and control replies are absorbed: they change the session, not the
// conversation, so they make no entry and no raw row.

fn is_mode_or_control(event: &StructuredEvent) -> bool {
    matches!(
        event,
        StructuredEvent::Status { .. } | StructuredEvent::ControlResponse { .. }
    )
}

#[test]
fn status_and_control_response_events_make_no_entry() {
    for name in PROMPT_RECORDED {
        let events = events_of(&lines_of(name));
        let absorbed: Vec<SequencedEvent> = events
            .iter()
            .filter(|e| is_mode_or_control(&e.event))
            .cloned()
            .collect();
        assert!(!absorbed.is_empty(), "{name} has status or control lines");
        assert_eq!(
            kinds(&built(&absorbed)),
            vec![],
            "{name}: status and control_response lines are absorbed, no entry"
        );
    }
}

#[test]
fn status_and_control_response_events_leave_the_transcript_unchanged() {
    for name in PROMPT_RECORDED {
        let events = events_of(&lines_of(name));
        let whole = built(&events);
        let without = built(
            &events
                .iter()
                .filter(|e| !is_mode_or_control(&e.event))
                .cloned()
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            kinds(&whole),
            kinds(&without),
            "{name}: the transcript is the same with or without them (no raw row)"
        );
    }
}

#[test]
fn a_saved_history_with_mode_and_control_lines_restores_to_the_same_transcript() {
    for name in ["prompt-mode-change", "prompt-plan-approval"] {
        let events = events_of(&lines_of(name));
        let (omitted, restored) =
            decode_events(&encode_events(0, &events)).expect("the saved history decodes");
        assert_eq!(omitted, 0);
        assert_eq!(
            restored,
            events.iter().map(|e| e.event.clone()).collect::<Vec<_>>(),
            "{name}: each line parses again to the same event"
        );
        let restored: Vec<SequencedEvent> = restored
            .into_iter()
            .enumerate()
            .map(|(seq, event)| SequencedEvent {
                seq: seq as u64,
                event,
            })
            .collect();
        assert_eq!(kinds(&built(&restored)), kinds(&built(&events)), "{name}");
    }
}

// -- Feature 941 (T013): the user's own messages, their pictures, and interrupted turns ----------

fn note(seq: u64, event: StructuredEvent) -> SequencedEvent {
    SequencedEvent { seq, event }
}

/// `interrupt-turn.jsonl`, with the daemon's notes around it: `UserSent` first, and, when
/// `asked`, the `InterruptAsked` note right after the first text delta, as the daemon writes it.
fn interrupted_turn(asked: bool) -> Vec<SequencedEvent> {
    let mut events = vec![user_sent("tell a long story", &[])];
    let mut noted = !asked;
    for event in events_of(&lines_of("interrupt-turn")) {
        let first_delta = matches!(
            &event.event,
            StructuredEvent::StreamDelta { delta: Some(_), .. }
        );
        events.push(event.event);
        if first_delta && !noted {
            events.push(StructuredEvent::InterruptAsked);
            noted = true;
        }
    }
    sequenced(events)
}

#[test]
fn a_sent_message_is_a_user_message_with_its_pictures_names() {
    let transcript = built(&sequenced([user_sent("look", &["a.png", "b.gif"])]));
    assert_eq!(
        kinds(&transcript),
        vec![EntryKind::UserMessage {
            text: "look".into(),
            attachments: vec!["a.png".into(), "b.gif".into()],
        }]
    );
}

#[test]
fn an_interrupted_turn_closes_with_the_notice_and_no_footer_and_the_cli_text_is_absorbed() {
    for asked in [true, false] {
        let transcript = built(&interrupted_turn(asked));
        let kinds = kinds(&transcript);
        assert_eq!(
            kinds.last(),
            Some(&EntryKind::Interrupted),
            "asked {asked}: {kinds:?}"
        );
        assert!(
            !kinds.iter().any(|k| matches!(k, EntryKind::Footer(_))),
            "asked {asked}"
        );
        let user_texts: Vec<_> = kinds
            .iter()
            .filter_map(|k| match k {
                EntryKind::UserMessage { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(user_texts, ["tell a long story"], "asked {asked}");
    }
}

#[test]
fn an_interrupt_racing_a_successful_end_still_closes_interrupted() {
    let transcript = built(&sequenced([
        user_sent("hi", &[]),
        assistant_block(text("Hello")),
        StructuredEvent::InterruptAsked,
        turn_result(false, "success", Some("Hello")),
    ]));
    assert_eq!(kinds(&transcript).last(), Some(&EntryKind::Interrupted));
}

#[test]
fn a_turn_with_neither_mark_keeps_its_footer_and_a_late_note_marks_nothing() {
    let transcript = built(&sequenced([
        user_sent("hi", &[]),
        assistant_block(text("Hello")),
        turn_result(false, "success", Some("Hello")),
        StructuredEvent::InterruptAsked,
        user_sent("next", &[]),
        assistant_block(text("Again")),
        turn_result(false, "success", Some("Again")),
    ]));
    let footers = kinds(&transcript)
        .into_iter()
        .filter(|k| matches!(k, EntryKind::Footer(_)))
        .count();
    assert_eq!(footers, 2);
    assert!(!kinds(&transcript).contains(&EntryKind::Interrupted));
}

#[test]
fn the_history_is_the_sent_texts_oldest_first_never_the_absorbed_interrupt_text() {
    let mut events = interrupted_turn(true);
    let next = events.len() as u64;
    events.push(note(next, user_sent("second", &["a.png"])));
    events.push(note(next + 1, turn_result(false, "success", None)));
    let transcript = built(&events);
    assert_eq!(
        transcript.user_messages().collect::<Vec<_>>(),
        ["tell a long story", "second"]
    );

    // Restored from a saved history, the notes are still there.
    let (_, restored) =
        decode_events(&encode_events(0, &events)).expect("the saved history decodes");
    let restored = built(&sequenced(restored));
    assert_eq!(
        restored.user_messages().collect::<Vec<_>>(),
        ["tell a long story", "second"]
    );
    assert!(kinds(&restored).contains(&EntryKind::Interrupted));
}
