//! T032 (feature 939, FR-012, FR-006, FR-004, FR-024): what each entry's header, status and footer
//! say, and the rows the view draws, read off the pure view model.

mod support;

use iced::Size;
use micold_client::features::structured_transcript::State;
use micold_client::icons::Icon;
use micold_client::ui::transcript::{
    footer_view, rows, tool_header, tool_status, BlockCache, BlockRenderers, Row, Status, Tone,
    NO_OUTPUT, RAW_LABEL,
};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{Outcome, ToolCard, ToolStatus, Transcript, TurnFooter};
use micold_core::structured::{SequencedEvent, StructuredEvent};
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

#[test]
fn a_raw_row_carries_the_raw_event_label_before_its_text() {
    let session = SessionId::new();
    let mut state = State::default();
    state.apply(session, 0, &[unreadable(0, "{not json")], false);
    let cache = BlockCache::default();
    let (painted, _) = painted_text_settled_at(
        view_of(&state, session, &BlockRenderers::PLAIN, &cache),
        &mut renderer(),
        Size::new(600.0, 400.0),
    );
    let painted: Vec<String> = painted.into_iter().map(|p| p.content).collect();
    assert_eq!(painted, vec![RAW_LABEL, "{not json"]);
}
