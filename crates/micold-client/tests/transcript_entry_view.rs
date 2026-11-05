//! T032 (feature 939, FR-012, FR-006, FR-004, FR-024): what each entry's header, status and footer
//! say, and the rows the view draws, read off the pure view model.

use crate::support;

use crate::stream_json::{assistant_block, thinking_block, user_sent};
use iced::{Element, Size};
use micold_client::features::structured_transcript::{State, TranscriptMsg};
use micold_client::icons::Icon;
use micold_client::ui::transcript::RenderCtx;
use micold_client::ui::transcript::{
    footer_view, rows, tool_header, tool_status, BlockCache, BlockRenderers, Row, Status, Tone,
    NO_OUTPUT,
};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{Outcome, ToolCard, ToolStatus, Transcript, TurnFooter};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Rgb};
use micold_material::test_support::Mounted;
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
    assert_eq!(rows(None, true), vec![Row::Empty]);
    assert_eq!(rows(Some(&Transcript::new()), true), vec![Row::Empty]);

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
    let shown = rows(Some(&t), true);
    assert_eq!(shown[0], Row::Omitted(40));
    assert!(matches!(shown[1], Row::Entry(_)));
    assert_eq!(shown.len(), 2);

    let mut none_omitted = Transcript::new();
    none_omitted.apply(0, &[unreadable(0, "not json")], false);
    assert!(matches!(rows(Some(&none_omitted), true)[0], Row::Entry(_)));
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
    let painted = painted_of(&state, session);
    assert_eq!(
        painted.iter().filter(|t| *t == "Turn failed").count(),
        1,
        "{painted:?}"
    );
    assert_eq!(
        painted
            .iter()
            .filter(|t| *t == "Result without a call · error")
            .count(),
        1,
        "{painted:?}"
    );
    assert!(
        !painted.iter().any(|t| t.contains("ran out of turns")),
        "collapsed: {painted:?}"
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

// ---- #1664 M2: the "Show raw events" toggle ----

/// A mixed transcript: a raw line, a session start, an ok footer, an error footer, an ok orphan
/// and an error orphan. The first, second, third and fifth are hidden with the toggle off.
fn mixed() -> Vec<SequencedEvent> {
    vec![
        unreadable(0, "not json"),
        init(1),
        result(2, false),
        result(3, true),
        orphan(4, false),
        orphan(5, true),
    ]
}

/// What the entry rows of `t` are, as their kinds' debug text: stable across builds of the same
/// history, where an entry's revision is not.
fn entry_kinds(t: &Transcript, show_raw: bool) -> Vec<String> {
    rows(Some(t), show_raw)
        .into_iter()
        .map(|row| match row {
            Row::Entry(e) => format!("{:?}", e.kind()),
            other => format!("{other:?}"),
        })
        .collect()
}

/// SC-002, FR-013: with the toggle on every low-signal entry is a row; off, the summary lines plus
/// the hidden count are the same total and the error ones stay.
#[test]
fn rows_follow_the_toggle_and_the_hidden_count_completes_the_total() {
    let mut t = Transcript::new();
    t.apply(0, &mixed(), false);

    let on = rows(Some(&t), true);
    assert_eq!(on.len(), 6);
    assert!(on.iter().all(|r| matches!(r, Row::Entry(_))));

    let off = rows(Some(&t), false);
    assert_eq!(
        off[0],
        Row::Hidden(4),
        "the notice comes first (no omitted)"
    );
    let kept = off.iter().filter(|r| matches!(r, Row::Entry(_))).count();
    assert_eq!(kept, 2, "the error footer and the error orphan stay");
    assert_eq!(kept as u64 + 4, on.len() as u64, "SC-002");
    let order = |rows: &[Row]| -> Vec<u64> {
        rows.iter()
            .filter_map(|r| match r {
                Row::Entry(e) => Some(e.opened_at()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(order(&off), vec![3, 5], "in transcript order");
}

/// US2 scenario 4: the notice follows the omitted notice, and only exists while something is hidden.
#[test]
fn the_hidden_notice_follows_omitted_and_exists_only_above_zero() {
    let mut t = Transcript::new();
    t.apply(7, &[unreadable(7, "x"), init(8)], false);
    let off = rows(Some(&t), false);
    assert_eq!(off, vec![Row::Omitted(7), Row::Hidden(2)]);
    assert!(!rows(Some(&t), true)
        .iter()
        .any(|r| matches!(r, Row::Hidden(_))));

    let mut only_errors = Transcript::new();
    only_errors.apply(0, &[result(0, true)], false);
    assert!(!rows(Some(&only_errors), false)
        .iter()
        .any(|r| matches!(r, Row::Hidden(_))));
}

/// Edge case: a transcript of only hidden entries shows the notice, not "No output yet".
#[test]
fn a_transcript_of_only_hidden_entries_shows_the_notice_not_empty() {
    let (state, session) = state_with(&[unreadable(0, "x"), init(1)]);
    let off = rows(state.transcript(session), false);
    assert_eq!(off, vec![Row::Hidden(2)]);
    assert!(!off.contains(&Row::Empty));
}

/// SC-002 as drawn: off, no raw text and "N events hidden" with a "Show" button; on, the
/// summaries and no notice.
#[test]
fn the_view_paints_the_notice_off_and_the_summaries_on() {
    let (state, session) = state_with(&[unreadable(0, "RAW LINE"), init(1)]);
    let cache = BlockCache::default();
    let paint = |show_raw: bool| -> Vec<String> {
        let ctx = micold_client::ui::transcript::RenderCtx {
            show_raw,
            ..support::transcript::ctx(&cache)
        };
        let (painted, _) = painted_text_settled_at(
            micold_client::ui::transcript::view(&state, session, &BlockRenderers::PLAIN, ctx),
            &mut renderer(),
            Size::new(600.0, 600.0),
        );
        painted.into_iter().map(|p| p.content).collect()
    };
    let off = paint(false);
    assert!(off.iter().any(|t| t == "2 events hidden"), "{off:?}");
    assert!(off.iter().any(|t| t == "Show"), "{off:?}");
    assert!(!off.iter().any(|t| t.contains("RAW LINE")));
    let on = paint(true);
    assert!(!on.iter().any(|t| t.contains("hidden")), "{on:?}");
    assert!(on.iter().any(|t| t == "Session started"), "{on:?}");
}

/// FR-011, US3: the same history built live (one event at a time) and resumed (all at once)
/// gives the same rows, for both values of the toggle.
#[test]
fn live_and_resumed_transcripts_give_equal_rows() {
    let events = mixed();
    let mut resumed = Transcript::new();
    resumed.apply(0, &events, false);
    let mut live = Transcript::new();
    for event in &events {
        live.apply(0, std::slice::from_ref(event), true);
    }
    for show_raw in [false, true] {
        assert_eq!(
            entry_kinds(&live, show_raw),
            entry_kinds(&resumed, show_raw),
            "show_raw = {show_raw}"
        );
    }
}

/// Every distinct `ShowRawEvents` a click anywhere in the transcript view publishes.
fn raw_events_sent(state: &State, session: SessionId, show_raw: bool) -> Vec<bool> {
    let cache = BlockCache::default();
    let ctx = micold_client::ui::transcript::RenderCtx {
        show_raw,
        ..support::transcript::ctx(&cache)
    };
    let insights = micold_client::features::structured_insights::State::default();
    let view = micold_client::ui::transcript::view_with_insights(
        state,
        micold_client::ui::transcript::InsightsCtx {
            state: &insights,
            cli_running: false,
            mode: micold_core::session::PermissionMode::Default,
        },
        session,
        &BlockRenderers::PLAIN,
        ctx,
    );
    let mut seen = Vec::new();
    for m in support::layout::clicks_at(
        view,
        &renderer(),
        support::layout::grid(iced::Rectangle::with_size(support::layout::WINDOW), 3.0),
    ) {
        if let TranscriptMsg::ShowRawEvents(on) = m {
            if !seen.contains(&on) {
                seen.push(on);
            }
        }
    }
    seen
}

/// FR-005 (F1): the header toggle asks for the opposite of what is showing.
#[test]
fn pressing_the_header_toggle_asks_for_the_opposite() {
    // Only an error footer: nothing hidden, so no "Show" button; the toggle is the only sender.
    let (state, session) = state_with(&[result(0, true)]);
    assert_eq!(raw_events_sent(&state, session, false), vec![true]);
    assert_eq!(raw_events_sent(&state, session, true), vec![false]);
}

/// FR-008 (F2): the hidden notice's "Show" button turns the toggle on.
#[test]
fn pressing_the_hidden_notice_show_button_turns_raw_events_on() {
    let (state, session) = state_with(&[unreadable(0, "x"), init(1)]);
    // Off: both the toggle and the Show button send `true`, nothing sends `false`.
    assert_eq!(raw_events_sent(&state, session, false), vec![true]);
}

// ---- #1663: a user's turn is highlighted ----

fn seq_of(events: impl IntoIterator<Item = StructuredEvent>) -> Vec<SequencedEvent> {
    (0..)
        .zip(events)
        .map(|(seq, event)| SequencedEvent { seq, event })
        .collect()
}

/// How many pixels of `element`, drawn in `scheme`, are exactly `fill` (the research R6 probe: the
/// renderer's own pixels, not a predicate).
fn count_pixels(element: Element<'_, TranscriptMsg>, scheme: ColorScheme, fill: Rgb) -> usize {
    let element = element.map(|_| String::new());
    let (buf, _) = Mounted::new_in(element, Size::new(900.0, 6000.0))
        .in_scheme(scheme)
        .drawn(iced::Color::TRANSPARENT);
    buf.as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[..3] == [fill.r, fill.g, fill.b] && p[3] == 255)
        .count()
}

fn ctx_in<'a>(scheme: ColorScheme, cache: &'a BlockCache) -> RenderCtx<'a> {
    RenderCtx {
        roles: tokens::roles(scheme),
        scheme,
        cache,
        selected: None,
        row: None,
        show_raw: true,
    }
}

fn pixels_of(state: &State, session: SessionId, scheme: ColorScheme, fill: Rgb) -> usize {
    let cache = BlockCache::default();
    let element = micold_client::ui::transcript::view(
        state,
        session,
        &BlockRenderers::PLAIN,
        ctx_in(scheme, &cache),
    );
    count_pixels(element, scheme, fill)
}

const SCHEMES: [ColorScheme; 2] = [ColorScheme::Light, ColorScheme::Dark];

/// Fewer pixels than this of the fill are incidental (anti-aliasing); a user turn is thousands.
const SURFACE_MIN: usize = 1000;

/// The user-turn surface and the accent edge both appear (FR-001, FR-002).
fn highlighted(state: &State, session: SessionId, scheme: ColorScheme) -> bool {
    let r = tokens::roles(scheme);
    let n = pixels_of(state, session, scheme, r.surface_container_high);
    n >= SURFACE_MIN && pixels_of(state, session, scheme, r.primary) >= 40
}

fn bare(state: &State, session: SessionId, scheme: ColorScheme) -> bool {
    let r = tokens::roles(scheme);
    let n = pixels_of(state, session, scheme, r.surface_container_high);
    n < SURFACE_MIN
}

/// US1 scenario 1, US2 scenarios 1 and 2, SC-004: a typed message and a delivered queued message
/// are one entry kind, and both carry the highlight, in light and dark.
#[test]
fn a_typed_and_a_delivered_message_are_highlighted() {
    for text in ["typed", "delivered from the queue"] {
        let (state, session) = state_with(&seq_of([user_sent(text, &[])]));
        for scheme in SCHEMES {
            assert!(highlighted(&state, session, scheme), "{text:?} {scheme:?}");
        }
    }
}

/// FR-004, US2 scenario 4: the attachments line is inside the highlight; an empty message renders.
#[test]
fn a_message_with_attachments_or_no_text_is_highlighted() {
    for (text, attachments) in [("see", vec!["a.png"]), ("", vec![])] {
        let (state, session) = state_with(&seq_of([user_sent(text, &attachments)]));
        for scheme in SCHEMES {
            assert!(highlighted(&state, session, scheme), "{text:?} {scheme:?}");
        }
    }
}

/// FR-005, US1 scenario 1: the agent's entries carry neither the tint nor the edge.
#[test]
fn assistant_text_and_thinking_are_not_highlighted() {
    let (state, session) = state_with(&seq_of([
        assistant_block(ContentBlock::Text {
            text: "an answer".into(),
        }),
        thinking_block("a thought"),
    ]));
    for scheme in SCHEMES {
        assert!(bare(&state, session, scheme), "{scheme:?}");
    }
}

/// FR-009: a slash command card sits in the highlight, folded and unfolded, and keeps its fold
/// state when another entry arrives; two user turns in a row both show it.
#[test]
fn a_command_turn_is_highlighted_folded_and_unfolded() {
    let (mut state, session) = state_with(&seq_of([user_sent("/review the diff", &[])]));
    let keys: Vec<_> = state
        .transcript(session)
        .expect("held")
        .entries()
        .filter(|e| {
            matches!(
                e.kind(),
                micold_core::structured::transcript::EntryKind::UserMessage {
                    command: Some(_),
                    ..
                }
            )
        })
        .map(|e| e.key().clone())
        .collect();
    assert!(!keys.is_empty(), "the text reads as a command");
    for scheme in SCHEMES {
        assert!(highlighted(&state, session, scheme), "folded {scheme:?}");
    }
    state.toggle(session, keys[0].clone());
    for scheme in SCHEMES {
        assert!(highlighted(&state, session, scheme), "unfolded {scheme:?}");
    }
    let r = tokens::roles(ColorScheme::Light);
    let one = pixels_of(
        &state,
        session,
        ColorScheme::Light,
        r.surface_container_high,
    );
    let before = painted_of(&state, session);
    state.apply(
        session,
        0,
        &[SequencedEvent {
            seq: 1,
            event: user_sent("another", &[]),
        }],
        true,
    );
    let after = painted_of(&state, session);
    assert!(
        before.iter().all(|t| after.contains(t)),
        "the card stays open when an entry is appended"
    );
    assert!(
        after.iter().any(|t| t.contains("another")),
        "the second turn is shown: {after:?}"
    );
    let two = pixels_of(
        &state,
        session,
        ColorScheme::Light,
        r.surface_container_high,
    );
    assert!(
        two > one + one / 2,
        "two consecutive user turns both carry the surface: {one} then {two}"
    );
}

/// A sub-agent's thread is drawn by the same entry code: its user turn is highlighted once the
/// thread is open, and not while it is folded (FR-003).
#[test]
fn a_subagent_user_turn_is_highlighted_in_its_open_thread() {
    use micold_client::features::structured_insights::State as Insights;
    use micold_client::ui::transcript::{view_with_insights, InsightsCtx};
    let session = SessionId::new();
    let events =
        support::transcript::sequenced(&support::transcript::fixture_lines("subagent-turn"), 0);
    let mut state = State::default();
    state.apply(session, 0, &events, false);
    let mut insights = Insights::default();
    let main = state.transcript(session).expect("held");
    insights.apply(session, 0, &events, false, main);
    let parent = main
        .entries()
        .find_map(|e| match e.kind() {
            micold_core::structured::transcript::EntryKind::ToolCall(c)
                if c.subagent_messages > 0 =>
            {
                Some(c.id.clone())
            }
            _ => None,
        })
        .expect("a card with a thread");
    let scheme = ColorScheme::Light;
    let fill = tokens::roles(scheme).surface_container_high;
    let count = |insights: &Insights| {
        let cache = BlockCache::default();
        let element = view_with_insights(
            &state,
            InsightsCtx {
                state: insights,
                cli_running: true,
                mode: micold_core::session::PermissionMode::Default,
            },
            session,
            &BlockRenderers::PLAIN,
            ctx_in(scheme, &cache),
        );
        count_pixels(element, scheme, fill)
    };
    assert!(count(&insights) < SURFACE_MIN, "folded");
    let main = state.transcript(session).expect("held");
    insights.toggle_thread(session, &parent, main);
    assert!(count(&insights) >= SURFACE_MIN, "open");
}
