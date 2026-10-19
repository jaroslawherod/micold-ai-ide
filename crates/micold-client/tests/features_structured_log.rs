//! T039 (feature 938, FR-022a, US6 scenario 3): how event messages become a structured session's
//! log lines — history replaces, later history and live events append, the omitted marker comes
//! first, and two sessions' logs stay apart.

use micold_client::features::structured_log::State;
use micold_core::session::SessionId;
use micold_core::structured::{omitted_line, SequencedEvent, StructuredEvent};

fn event(seq: u64, text: &str) -> SequencedEvent {
    SequencedEvent {
        seq,
        event: StructuredEvent::Unreadable { text: text.into() },
    }
}

fn rows(state: &State, session: SessionId) -> Vec<String> {
    state.rows(session).map(str::to_owned).collect()
}

#[test]
fn the_first_history_message_fills_the_log() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &[event(0, "a"), event(1, "b")], false);
    let rows = rows(&state, s);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].contains('a') && rows[1].contains('b'));
}

#[test]
fn later_history_messages_append() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &[event(0, "a")], false);
    state.apply(s, 0, &[event(1, "b")], false);
    assert_eq!(rows(&state, s).len(), 2);
}

#[test]
fn live_events_append() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &[event(0, "a")], false);
    state.apply(s, 0, &[event(1, "b")], true);
    state.apply(s, 0, &[event(2, "c")], true);
    assert_eq!(rows(&state, s).len(), 3);
}

#[test]
fn a_history_after_live_events_replaces_the_lines() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &[event(0, "a")], false);
    state.apply(s, 0, &[event(1, "b")], true);
    state.apply(s, 0, &[event(0, "x"), event(1, "y")], false);
    let rows = rows(&state, s);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].contains('x'), "{rows:?}");
}

#[test]
fn omitted_events_put_the_marker_first() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 5, &[event(5, "a")], false);
    let rows = rows(&state, s);
    assert_eq!(rows[0], omitted_line(5));
    assert_eq!(rows.len(), 2);
}

#[test]
fn a_replacing_history_drops_the_old_marker() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 5, &[event(5, "a")], false);
    state.apply(s, 0, &[event(6, "b")], true);
    state.apply(s, 0, &[event(0, "x")], false);
    assert_eq!(rows(&state, s).len(), 1);
}

#[test]
fn two_sessions_logs_stay_separate() {
    let (mut state, a, b) = (State::default(), SessionId::new(), SessionId::new());
    state.apply(a, 0, &[event(0, "a")], false);
    state.apply(b, 0, &[event(0, "b1"), event(1, "b2")], false);
    assert_eq!(rows(&state, a).len(), 1);
    assert_eq!(rows(&state, b).len(), 2);
    assert_eq!(rows(&state, SessionId::new()).len(), 0);
}

#[test]
fn a_snapshot_sent_again_without_live_events_replaces_the_lines() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &[event(0, "a"), event(1, "b")], false);
    // The window views the session again: the daemon sends the whole history afresh.
    state.apply(s, 0, &[event(0, "a"), event(1, "b")], false);
    assert_eq!(rows(&state, s).len(), 2);
}

#[test]
fn a_snapshot_that_continues_the_log_appends_to_it() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &[event(0, "a"), event(1, "b")], false);
    state.apply(s, 0, &[event(2, "c")], false);
    assert_eq!(rows(&state, s).len(), 3);
}

#[test]
fn forgetting_a_session_drops_its_log() {
    let (mut state, s) = (State::default(), SessionId::new());
    state.apply(s, 0, &[event(0, "a")], false);
    state.forget(s);
    assert_eq!(rows(&state, s).len(), 0);
}
