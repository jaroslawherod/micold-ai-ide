//! T020, T021 (feature 939, data-model.md *View state*, research R9): each window keeps one
//! transcript per structured session, with the cards the user unfolded and whether the view follows
//! the end; the app routes the wire's events and the view's messages to it.

use micold_client::app::{Message, State as App};
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::structured_transcript::{
    Follow, FollowChange, Scroll, State, TranscriptMsg, END_SLACK,
};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{EntryKey, EntryKind};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent, MAX_EVENTS};
use serde_json::json;

fn assistant(blocks: Vec<ContentBlock>) -> StructuredEvent {
    StructuredEvent::Assistant { error: None, model: None, 
        message_id: None,
        raw: json!({"type": "assistant", "message": {"content": []}}).to_string(),
        blocks,
        parent_tool_use_id: None,
    }
}

fn tool_use(id: &str) -> StructuredEvent {
    assistant(vec![ContentBlock::ToolUse {
        id: id.into(),
        name: "Bash".into(),
        input: json!({"command": "ls"}).to_string(),
    }])
}

fn tool_result(id: &str) -> StructuredEvent {
    StructuredEvent::User { is_synthetic: false, 
        raw: json!({"type": "user", "message": {"content": [
            {"type": "tool_result", "tool_use_id": id, "content": "ok"}
        ]}})
        .to_string(),
        blocks: vec![ContentBlock::ToolResult {
            tool_use_id: id.into(),
            content: "ok".into(),
            is_error: false,
        }],
        parent_tool_use_id: None,
    }
}

fn text(t: &str) -> StructuredEvent {
    assistant(vec![ContentBlock::Text { text: t.into() }])
}

fn thinking(t: &str) -> StructuredEvent {
    assistant(vec![ContentBlock::Thinking { thinking: t.into() }])
}

/// `events`, numbered from `first`.
fn seq(first: u64, events: impl IntoIterator<Item = StructuredEvent>) -> Vec<SequencedEvent> {
    events
        .into_iter()
        .zip(first..)
        .map(|(event, seq)| SequencedEvent { seq, event })
        .collect()
}

fn tool(id: &str) -> EntryKey {
    EntryKey::Tool(id.into())
}

fn entries(state: &State, s: SessionId) -> usize {
    state.transcript(s).map_or(0, |t| t.entries().len())
}

/// A session holding the tool card `a` and the thinking card at seq 1.
fn with_cards() -> (State, SessionId) {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &seq(0, [tool_use("a"), thinking("hm")]), false);
    (state, s)
}

// ---- T020: the reducer ----

#[test]
fn applied_events_are_the_sessions_transcript() {
    let (state, s) = with_cards();
    assert_eq!(
        entries(&state, s),
        2,
        "the applied events open one tool card and one thinking card"
    );
    let card = state.transcript(s).unwrap().entry(&tool("a"));
    assert_eq!(
        card.map(|e| e.key()),
        Some(&tool("a")),
        "the tool card is found under its tool use id"
    );
    assert!(
        matches!(card.map(|e| e.kind()), Some(EntryKind::ToolCall(c)) if c.id == "a"),
        "the tool card's entry is a tool call for that id"
    );
    assert!(
        state.transcript(SessionId::new()).is_none(),
        "a session that applied nothing has no transcript"
    );
}

#[test]
fn every_card_starts_folded_and_toggle_unfolds_and_folds_it() {
    let (mut state, s) = with_cards();
    let think = EntryKey::At { seq: 1, block: 0 };
    assert!(!state.is_open(s, &tool("a")), "a tool card starts folded");
    assert!(!state.is_open(s, &think), "a thinking card starts folded");
    state.toggle(s, tool("a"));
    assert!(state.is_open(s, &tool("a")), "a toggle unfolds the card");
    assert!(!state.is_open(s, &think), "only the toggled card opens");
    state.toggle(s, tool("a"));
    assert!(!state.is_open(s, &tool("a")), "a second toggle folds it");
}

#[test]
fn an_open_card_stays_open_across_new_events() {
    let (mut state, s) = with_cards();
    state.toggle(s, tool("a"));
    state.apply(s, 0, &seq(2, [tool_result("a"), text("done")]), true);
    assert!(state.is_open(s, &tool("a")), "the result filling the card");
    state.apply(s, 0, &seq(4, [tool_use("b")]), true);
    assert!(state.is_open(s, &tool("a")), "a later card");
    assert!(!state.is_open(s, &tool("b")), "the new card starts folded");
}

#[test]
fn an_open_card_stays_open_while_another_session_is_shown() {
    let (mut state, s) = with_cards();
    let other = SessionId::new();
    state.toggle(s, tool("a"));
    state.apply(other, 0, &seq(0, [tool_use("a")]), false);
    state.toggle(other, tool("a"));
    state.toggle(other, tool("a"));
    assert!(state.is_open(s, &tool("a")));
}

#[test]
fn a_rebuild_keeps_open_only_the_cards_still_present() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &seq(0, [tool_use("a"), tool_use("b")]), false);
    state.toggle(s, tool("a"));
    state.toggle(s, tool("b"));
    // A re-sync of the same history: both cards are still there.
    state.apply(s, 0, &seq(0, [tool_use("a"), tool_use("b")]), false);
    assert!(state.is_open(s, &tool("a")) && state.is_open(s, &tool("b")));
    // A history that holds only `a`.
    state.apply(s, 0, &seq(0, [tool_use("a"), text("x")]), false);
    assert_eq!(entries(&state, s), 2, "no duplicated entries");
    assert!(
        state.is_open(s, &tool("a")),
        "a card still present keeps its fold"
    );
    assert!(
        !state.is_open(s, &tool("b")),
        "a card that left is forgotten"
    );
    // `b` comes back in a later history: it starts folded again.
    state.apply(s, 0, &seq(0, [tool_use("b")]), false);
    assert!(!state.is_open(s, &tool("b")));
}

#[test]
fn a_toggle_of_a_card_not_in_the_transcript_opens_nothing() {
    let (mut state, s) = with_cards();
    state.toggle(s, tool("gone"));
    assert!(!state.is_open(s, &tool("gone")));
}

#[test]
fn a_card_the_bound_dropped_is_forgotten() {
    let (mut state, s) = with_cards();
    state.toggle(s, tool("a"));
    let filler = (0..MAX_EVENTS).map(|i| text(&i.to_string()));
    state.apply(s, 0, &seq(2, filler), true);
    assert!(state.transcript(s).unwrap().entry(&tool("a")).is_none());
    assert!(
        !state.is_open(s, &tool("a")),
        "the card left with its events"
    );
}

#[test]
fn a_transcript_starts_following() {
    let (state, s) = with_cards();
    assert_eq!(state.follow(s), Follow::Following);
    assert_eq!(state.follow(SessionId::new()), Follow::Following);
}

#[test]
fn scrolling_away_from_the_end_detaches_and_back_to_it_resumes() {
    let (mut state, s) = with_cards();
    // content 1000, viewport 200: the end is offset 800.
    assert_eq!(state.scrolled(s, 800, 200, 1000), FollowChange::None);
    assert_eq!(state.follow(s), Follow::Following);
    assert_eq!(state.scrolled(s, 300, 200, 1000), FollowChange::Detached);
    assert_eq!(state.follow(s), Follow::Detached { offset: 300 });
    assert_eq!(state.scrolled(s, 250, 200, 1000), FollowChange::None);
    assert_eq!(state.follow(s), Follow::Detached { offset: 250 });
    // Within END_SLACK of the end counts as the end.
    let near = 800 - END_SLACK;
    assert_eq!(END_SLACK, 4);
    assert_eq!(state.scrolled(s, near - 1, 200, 1000), FollowChange::None);
    assert_eq!(state.follow(s), Follow::Detached { offset: near - 1 });
    assert_eq!(state.scrolled(s, near, 200, 1000), FollowChange::Resumed);
    assert_eq!(state.follow(s), Follow::Following);
}

#[test]
fn content_shorter_than_the_view_is_at_its_end() {
    let (mut state, s) = with_cards();
    assert_eq!(state.scrolled(s, 0, 500, 100), FollowChange::None);
    assert_eq!(state.follow(s), Follow::Following);
}

#[test]
fn new_events_leave_the_follow_state_as_it_is() {
    // SC-003 at the state: following stays following as content is added, and a detached view
    // keeps its offset, so the visible content does not move.
    let (mut state, s) = with_cards();
    state.apply(s, 0, &seq(2, [text("more")]), true);
    assert_eq!(state.follow(s), Follow::Following);
    state.scrolled(s, 10, 200, 1000);
    state.apply(s, 0, &seq(3, [text("and more")]), true);
    assert_eq!(state.follow(s), Follow::Detached { offset: 10 });
}

#[test]
fn jump_to_latest_follows_again() {
    let (mut state, s) = with_cards();
    state.scrolled(s, 0, 200, 1000);
    state.jump_to_latest(s);
    assert_eq!(state.follow(s), Follow::Following);
}

#[test]
fn two_sessions_share_no_entries_fold_or_follow() {
    let (mut state, a) = with_cards();
    let b = SessionId::new();
    state.apply(b, 0, &seq(0, [tool_use("a")]), false);
    state.toggle(a, tool("a"));
    state.scrolled(a, 0, 200, 1000);
    assert_eq!(entries(&state, a), 2);
    assert_eq!(entries(&state, b), 1);
    assert!(
        !state.is_open(b, &tool("a")),
        "the same key in another session stays folded"
    );
    assert_eq!(state.follow(b), Follow::Following);
}

#[test]
fn forgetting_a_session_drops_its_transcript_and_view_state() {
    let (mut state, s) = with_cards();
    state.toggle(s, tool("a"));
    state.scrolled(s, 0, 200, 1000);
    state.forget(s);
    assert!(state.transcript(s).is_none());
    assert!(!state.is_open(s, &tool("a")));
    assert_eq!(state.follow(s), Follow::Following);
}

// ---- T021: the app's routing and the scroll requests ----

fn app_with_cards() -> (App, SessionId) {
    let (mut app, s) = (App::default(), SessionId::new());
    app.structured_events(
        s,
        0,
        &seq(0, [tool_use("a"), thinking("hm")]),
        false,
        &micold_client::ui::syntax::GRAMMAR,
    );
    (app, s)
}

#[test]
fn structured_events_reach_the_transcript() {
    let (app, s) = app_with_cards();
    assert_eq!(entries(&app.structured_transcript, s), 2);
}

#[test]
fn the_session_family_carries_the_transcript_messages() {
    let (mut app, s) = app_with_cards();
    app.update(Message::Session(SessionMsg::Transcript(
        TranscriptMsg::Toggle {
            session: s,
            key: tool("a"),
        },
    )));
    assert!(app.structured_transcript.is_open(s, &tool("a")));
    app.update(Message::Session(SessionMsg::Transcript(
        TranscriptMsg::Scrolled {
            session: s,
            offset: 0,
            viewport: 200,
            content: 1000,
        },
    )));
    assert_eq!(
        app.structured_transcript.follow(s),
        Follow::Detached { offset: 0 }
    );
}

#[test]
fn following_again_and_showing_ask_for_the_scroll_the_follow_state_needs() {
    let (mut app, s) = app_with_cards();
    let scrolled = |offset| TranscriptMsg::Scrolled {
        session: s,
        offset,
        viewport: 200,
        content: 1000,
    };
    assert_eq!(
        app.update_transcript(TranscriptMsg::Shown { session: s })
            .scroll,
        Some(Scroll::End),
        "shown while following: open at the end (FR-018)"
    );
    assert_eq!(
        app.update_transcript(scrolled(120)).scroll,
        None,
        "detaching moves nothing"
    );
    assert_eq!(
        app.update_transcript(TranscriptMsg::Shown { session: s })
            .scroll,
        Some(Scroll::To(120)),
        "shown while detached: back where the user left it"
    );
    assert_eq!(
        app.update_transcript(scrolled(800)).scroll,
        Some(Scroll::End),
        "scrolling back to the end resumes following"
    );
    app.update_transcript(scrolled(5));
    assert_eq!(
        app.update_transcript(TranscriptMsg::JumpToLatest { session: s })
            .scroll,
        Some(Scroll::End)
    );
    assert_eq!(app.structured_transcript.follow(s), Follow::Following);
    assert_eq!(
        app.update_transcript(TranscriptMsg::Toggle {
            session: s,
            key: tool("a"),
        })
        .scroll,
        None
    );
}

#[test]
fn the_scroll_id_names_the_session() {
    let s = SessionId::new();
    assert_eq!(
        micold_client::ui::transcript::transcript_scroll_id(s),
        iced::advanced::widget::Id::from(format!("transcript-{s}"))
    );
    assert_ne!(
        micold_client::ui::transcript::transcript_scroll_id(s),
        micold_client::ui::transcript::transcript_scroll_id(SessionId::new())
    );
}

#[test]
fn forgetting_a_structured_session_drops_its_transcript() {
    let (mut app, s) = app_with_cards();
    app.forget_structured(s);
    assert!(app.structured_transcript.transcript(s).is_none());
}
