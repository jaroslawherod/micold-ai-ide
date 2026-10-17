//! Applying a resolved navigation press (feature 803, contracts/ui.md U3).

use micold_client::app::{Message, State};
use micold_client::features::navigation::Step;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::sidebar::RowRef;
use micold_core::session::{SessionId, SessionLocation};

mod support;
use support::sidebar_fixture::{deliver, listed, session_at, state_with_active, worktree};

/// Default with the current session, `feat-a` with one, `feat-b` empty; the terminal focused.
fn arranged() -> (State, SessionId, SessionId) {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);
    let default = session_at(&mut state, SessionLocation::Default);
    let a = session_at(&mut state, worktree("feat-a"));
    state.session.active = Some(default);
    (state, default, a)
}

fn session_count(state: &State) -> usize {
    state.workspace.sessions.values().map(Vec::len).sum()
}

#[test]
fn a_session_step_selects_it_as_a_click_does_and_arms_the_reveal() {
    let (mut state, _, a) = arranged();
    let follow = state.apply_navigation(Step::Session(a), true);
    assert_eq!(follow, vec![Message::Session(SessionMsg::Selected(a))]);
    assert!(
        state.sidebar.pending_reveal_scroll,
        "the target row is scrolled into view (FR-009)"
    );
    assert_eq!(
        state.sidebar.keyboard_row, None,
        "the terminal keeps the keyboard"
    );
}

#[test]
fn a_stop_starts_nothing_and_gives_the_sidebar_the_keyboard() {
    let (mut state, default, _) = arranged();
    let before = session_count(&state);
    let follow = state.apply_navigation(Step::Stop(worktree("feat-b")), true);
    assert!(follow.is_empty(), "no message: nothing is started (FR-011)");
    assert_eq!(state.sidebar.nav_stop, Some(worktree("feat-b")));
    assert_eq!(
        state.sidebar.keyboard_row,
        Some(RowRef::Location(worktree("feat-b")))
    );
    assert!(
        state.session.terminal_released,
        "the terminal lets go of the keyboard"
    );
    assert!(
        state.sidebar.pending_reveal_scroll,
        "the stop's row is scrolled into view"
    );
    assert_eq!(
        state.session.active,
        Some(default),
        "the terminal keeps its session"
    );
    assert_eq!(session_count(&state), before, "SC-005");
}

#[test]
fn no_step_changes_nothing() {
    let (mut state, _, _) = arranged();
    let before = state.clone();
    assert!(state.apply_navigation(Step::None, true).is_empty());
    assert_eq!(state, before);
}

#[test]
fn the_selected_follow_up_closes_the_overview() {
    let (mut state, _, a) = arranged();
    state.overview.open = true;
    for message in state.apply_navigation(Step::Session(a), true) {
        state.update(message);
    }
    assert!(
        !state.overview.open,
        "a location chord closes the Overview (FR-012)"
    );
    assert_eq!(state.session.active, Some(a));
}
