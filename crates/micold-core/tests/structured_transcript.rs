//! Feature 939 — the transcript model over #938's recorded fixtures and synthetic events
//! (contracts/transcript-model.md: input rule, event → entry rules, guarantee G5).

use micold_core::structured::transcript::{
    input_summary, EntryKey, EntryKind, Outcome, RawReason, ResultPart, ToolStatus, Transcript,
};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{
    assistant_block, assistant_blocks, events_of, lines_of, sequenced, thinking_block, tool_result,
    tool_use, turn_result, user_text, with_parent, without_stream_events, RECORDED,
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
    assert_eq!(transcript.rebuilds(), 1);

    // A history after a live event is a fresh snapshot even when its seq continues (wire.md).
    let mut transcript = built(&events[..1]);
    transcript.apply(0, &events[1..], true);
    transcript.apply(0, &one(2, user_text("three")), false);
    assert_eq!(
        kinds(&transcript),
        vec![EntryKind::UserMessage {
            text: "three".to_string()
        }]
    );
    assert_eq!(transcript.rebuilds(), 1);
}

#[test]
fn a_live_message_appends() {
    let events = sequenced([user_text("one"), assistant_block(text("two"))]);
    let mut transcript = built(&events[..1]);
    transcript.apply(0, &events[1..], true);
    assert_eq!(kinds(&transcript), kinds(&built(&events)));
    assert_eq!(transcript.rebuilds(), 0);
}

#[test]
fn a_transcript_with_no_event_is_empty() {
    assert!(Transcript::new().is_empty(), "nothing yet (FR-006)");
    let mut transcript = Transcript::new();
    transcript.apply(3, &[], false);
    assert!(transcript.is_empty(), "omitted events are not held");
    assert!(!built(&sequenced([user_text("hi")])).is_empty());
}

#[test]
fn the_omitted_notice_carries_the_services_omitted_count() {
    assert_eq!(built(&sequenced([user_text("hi")])).omitted_notice(), None);
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
            text: "hi".to_string()
        })
    );
    assert_eq!(card_of(&transcript, "toolu_1").name, "Bash");
    assert!(transcript
        .entry(&EntryKey::At { seq: 1, block: 0 })
        .is_none());
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
            assert!(model.is_some(), "the session start names the model");
            assert!(cwd.is_some(), "the session start names the folder");
        }
        other => panic!("the transcript starts with the session, got {other:?}"),
    }
    match &entries[1] {
        EntryKind::AssistantText { text, streaming } => {
            assert!(text.contains("blue"), "{text}");
            assert!(!streaming);
        }
        other => panic!("then the reply, got {other:?}"),
    }
    match &entries[2] {
        EntryKind::Footer(footer) => {
            assert_eq!(footer.outcome, Outcome::Success);
            assert_eq!(footer.duration_ms, Some(6674));
            assert!(footer.cost_usd.is_some());
            assert_eq!(footer.num_turns, Some(1));
            assert_eq!(footer.error_text, None);
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
            text: "hello".to_string()
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
            EntryKind::AssistantText { text, .. } | EntryKind::UserMessage { text }
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
                text: "sub prompt".to_string()
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
