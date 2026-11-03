//! T032 (feature 939, FR-012, FR-006, FR-004, FR-024): what each entry's header, status and footer
//! say, and the rows the view draws, read off the pure view model.

use crate::support;

use iced::Size;
use micold_client::features::structured_transcript::State;
use micold_client::icons::Icon;
use micold_client::ui::transcript::{
    footer_view, rows, tool_header, tool_status, BlockCache, BlockRenderers, Row, Status, Tone,
    NO_OUTPUT,
};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{Outcome, ToolCard, ToolStatus, Transcript, TurnFooter};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent};
use support::layout::{painted_text_settled_at, renderer};
use support::transcript::view_of;

fn card(status: ToolStatus, subagent_messages: u64) -> ToolCard {
    ToolCard {
        id: "toolu_1".into(),
        name: "Bash".into(),
        input: r#"{"command":"cargo test"}"#.into(),
        summary: "cargo test".into(),
        status,
        result: None,
        subagent_messages,
        streaming: false,
    }
}

fn footer(outcome: Outcome) -> TurnFooter {
    TurnFooter {
        outcome,
        duration_ms: Some(6_674),
        cost_usd: Some(0.0371549),
        num_turns: Some(1),
        error_text: (outcome == Outcome::Error).then(|| "error_max_turns".to_owned()),
    }
}

const FAILED: Status = Status {
    icon: Icon::ToolFailed,
    label: "Failed",
    tone: Tone::Error,
};

#[test]
fn a_failed_card_says_failed_with_the_failed_icon_in_the_error_role() {
    let header = tool_header(&card(ToolStatus::Failed, 0));
    assert_eq!(header.status, FAILED);
    assert!(header.failed, "a failed card is drawn failed (its outline)");
}

#[test]
fn an_error_footer_says_failed_with_the_failed_icon_in_the_error_role() {
    let view = footer_view(&footer(Outcome::Error));
    assert_eq!(view.status, FAILED);
    assert_eq!(view.error_text.as_deref(), Some("error_max_turns"));
}

#[test]
fn a_success_footer_is_not_marked_failed_and_shows_its_figures() {
    let view = footer_view(&footer(Outcome::Success));
    assert_eq!(view.status.tone, Tone::Muted);
    assert_ne!(view.status.icon, Icon::ToolFailed);
    assert_eq!(
        view.figures,
        vec!["6.7 s", "session total $0.0372", "1 turn"],
        "the cost is the CLI session's running total, labelled so"
    );
}

#[test]
fn every_other_status_has_its_own_icon_and_label_and_no_error_role() {
    for (status, icon, label) in [
        (ToolStatus::Running, Icon::ToolRunning, "Running"),
        (ToolStatus::Done, Icon::ToolDone, "Done"),
        (ToolStatus::NoResult, Icon::ToolNoResult, "No result"),
    ] {
        assert_eq!(
            tool_status(status),
            Status {
                icon,
                label,
                tone: Tone::Muted
            },
            "{status:?}"
        );
        assert!(!tool_header(&card(status, 0)).failed);
    }
}

#[test]
fn a_card_shows_its_sub_agent_count_while_above_zero() {
    assert_eq!(tool_header(&card(ToolStatus::Running, 0)).subagents, None);
    assert_eq!(
        tool_header(&card(ToolStatus::Running, 1))
            .subagents
            .as_deref(),
        Some("1 sub-agent message")
    );
    assert_eq!(
        tool_header(&card(ToolStatus::Done, 3)).subagents.as_deref(),
        Some("3 sub-agent messages")
    );
}

#[test]
fn an_empty_transcript_shows_no_output_yet() {
    assert_eq!(rows(None), vec![Row::Empty]);
    assert_eq!(rows(Some(&Transcript::new())), vec![Row::Empty]);

    let session = SessionId::new();
    let mut state = State::default();
    state.apply(session, 0, &[], false);
    let cache = BlockCache::default();
    let (painted, _) = painted_text_settled_at(
        view_of(&state, session, &BlockRenderers::PLAIN, &cache),
        &mut renderer(),
        Size::new(600.0, 400.0),
    );
    let painted: Vec<String> = painted.into_iter().map(|p| p.content).collect();
    assert_eq!(painted, vec![NO_OUTPUT]);
}

fn unreadable(seq: u64, line: &str) -> SequencedEvent {
    SequencedEvent {
        seq,
        event: StructuredEvent::Unreadable { text: line.into() },
    }
}

#[test]
fn the_omitted_notice_is_first_when_events_are_omitted() {
    let mut t = Transcript::new();
    t.apply(40, &[unreadable(40, "not json")], false);
    let shown = rows(Some(&t));
    assert_eq!(shown[0], Row::Omitted(40));
    assert!(matches!(shown[1], Row::Entry(_)));
    assert_eq!(shown.len(), 2);

    let mut none_omitted = Transcript::new();
    none_omitted.apply(0, &[unreadable(0, "not json")], false);
    assert!(matches!(rows(Some(&none_omitted))[0], Row::Entry(_)));
}

fn init(seq: u64) -> SequencedEvent {
    SequencedEvent {
        seq,
        event: StructuredEvent::Init {
            session_id: "s".into(),
            model: Some("opus".into()),
            cwd: Some("/work/tree".into()),
            tools: vec![],
            permission_mode: None,
            slash_commands: vec![],
            skills: vec![],
            terminal_slash_commands: vec![],
            raw: "{}".into(),
        },
    }
}

fn result(seq: u64, is_error: bool) -> SequencedEvent {
    SequencedEvent {
        seq,
        event: StructuredEvent::Result {
            subtype: if is_error {
                "error_max_turns"
            } else {
                "success"
            }
            .into(),
            is_error,
            duration_ms: Some(6_674),
            num_turns: Some(2),
            total_cost_usd: Some(0.5),
            usage: None,
            result: is_error.then(|| "ran out of turns".to_owned()),
            session_id: None,
            raw: "{}".into(),
        },
    }
}

fn orphan(seq: u64, is_error: bool) -> SequencedEvent {
    SequencedEvent {
        seq,
        event: StructuredEvent::User {
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "toolu_gone".into(),
                content: "ORPHAN OUTPUT".into(),
                is_error,
            }],
            parent_tool_use_id: None,
            is_synthetic: false,
            raw: "{}".into(),
        },
    }
}

/// What the view of `state` paints, in order.
fn painted_of(state: &State, session: SessionId) -> Vec<String> {
    let cache = BlockCache::default();
    let (painted, _) = painted_text_settled_at(
        view_of(state, session, &BlockRenderers::PLAIN, &cache),
        &mut renderer(),
        Size::new(600.0, 600.0),
    );
    // The card's chevron is an icon glyph in the private-use area: not text.
    painted
        .into_iter()
        .map(|p| p.content)
        .filter(|c| !c.chars().all(|ch| ('\u{e000}'..='\u{f8ff}').contains(&ch)))
        .collect()
}

fn open_all(state: &mut State, session: SessionId) {
    let keys: Vec<_> = state
        .transcript(session)
        .expect("held")
        .entries()
        .map(|e| e.key().clone())
        .collect();
    for key in keys {
        state.toggle(session, key);
    }
}

fn state_with(events: &[SequencedEvent]) -> (State, SessionId) {
    let session = SessionId::new();
    let mut state = State::default();
    state.apply(session, 0, events, false);
    (state, session)
}

/// SC-001, US1 scenario 1: collapsed, a raw entry paints its summary title and none of its text.
#[test]
fn a_raw_entry_is_one_folded_summary_with_none_of_its_text() {
    let (state, session) = state_with(&[unreadable(0, "{not json")]);
    assert_eq!(painted_of(&state, session), vec!["Unreadable line"]);
}

/// FR-004: open, the raw detail is the entry text; tabs are the only change.
#[test]
fn an_open_raw_entry_shows_its_original_text() {
    let (mut state, session) = state_with(&[unreadable(0, "{not\tjson")]);
    open_all(&mut state, session);
    let painted = painted_of(&state, session);
    assert_eq!(painted, vec!["Unreadable line", "{not    json"]);
}

/// US1 scenario 3: the other three kinds are folded cards with their facts.
#[test]
fn session_start_footer_and_orphan_are_folded_summaries_with_their_facts() {
    let (state, session) = state_with(&[init(0), orphan(1, false), result(2, false)]);
    let painted = painted_of(&state, session).join("|");
    for want in [
        "Session started",
        "opus",
        "Result without a call",
        "Turn done",
        "6.7 s",
        "2 turns",
    ] {
        assert!(painted.contains(want), "{want:?} in {painted}");
    }
    for hidden in ["ORPHAN OUTPUT", "/work/tree", "session total"] {
        assert!(!painted.contains(hidden), "{hidden:?} is detail: {painted}");
    }
}

#[test]
fn opening_a_summary_shows_its_detail() {
    let (mut state, session) = state_with(&[init(0), orphan(1, false), result(2, true)]);
    open_all(&mut state, session);
    let painted = painted_of(&state, session).join("|");
    for want in [
        "/work/tree",
        "ORPHAN OUTPUT",
        "session total",
        "ran out of turns",
    ] {
        assert!(painted.contains(want), "{want:?} in {painted}");
    }
}

/// FR-013 groundwork: an error says so in words in its summary.
#[test]
fn error_footers_and_orphans_say_failed_or_error_in_their_title() {
    let (state, session) = state_with(&[orphan(0, true), result(1, true)]);
    let painted = painted_of(&state, session).join("|");
    assert!(painted.contains("error"), "{painted}");
    assert!(painted.contains("Turn failed"), "{painted}");
    assert!(
        !painted.contains("ran out of turns"),
        "collapsed: {painted}"
    );
}

#[test]
fn a_footer_arriving_later_adds_its_summary() {
    let (mut state, session) = state_with(&[init(0)]);
    assert!(!painted_of(&state, session).join("|").contains("Turn done"));
    state.apply(session, 0, &[result(1, false)], false);
    assert!(painted_of(&state, session).join("|").contains("Turn done"));
}

/// A very large raw detail does not make the card as tall as the text.
#[test]
fn a_very_large_raw_detail_is_height_bounded() {
    let big = (0..400)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (mut state, session) = state_with(&[unreadable(0, &big)]);
    open_all(&mut state, session);
    let cache = BlockCache::default();
    let mut mounted = support::transcript::Mounted::new(Size::new(600.0, 600.0));
    let (_, showing) = mounted.frame(
        view_of(&state, session, &BlockRenderers::PLAIN, &cache),
        vec![],
    );
    assert!(showing.content < 500.0, "{showing:?}");
}
