//! Applying a resolved navigation press (feature 803, contracts/ui.md U3).

use micold_client::app::Message;
use micold_client::features::navigation::Step;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::sidebar::RowRef;

use crate::support;
use support::sidebar_fixture::{arranged, press, session_count, worktree};

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

#[test]
fn a_session_step_without_terminal_focus_still_selects_it() {
    let (mut state, _, a) = arranged();
    let follow = state.apply_navigation(Step::Session(a), false);
    assert_eq!(follow, vec![Message::Session(SessionMsg::Selected(a))]);
}

#[test]
fn a_session_step_without_terminal_focus_leaves_the_keyboard_on_its_row() {
    let (mut state, _, a) = arranged();
    press(&mut state, Step::Session(a), false);
    assert_eq!(state.session.active, Some(a));
    assert!(
        state.session.terminal_released,
        "the terminal does not take the keyboard (FR-010)"
    );
    assert_eq!(state.sidebar.keyboard_row, Some(RowRef::Session(a)));
}

#[test]
fn a_session_step_with_terminal_focus_gives_the_terminal_the_keyboard() {
    let (mut state, _, a) = arranged();
    press(&mut state, Step::Session(a), true);
    assert_eq!(state.session.active, Some(a));
    assert_eq!(state.sidebar.keyboard_row, None);
    assert!(!state.session.terminal_released);
}

#[test]
fn finishing_with_terminal_focus_changes_nothing() {
    let (mut state, _, a) = arranged();
    let before = state.clone();
    state.finish_navigation(&Step::Session(a), true);
    assert_eq!(state, before);
}

#[test]
fn after_a_stop_the_next_press_keeps_the_keyboard_in_the_sidebar() {
    let (mut state, _, a) = arranged();
    press(&mut state, Step::Stop(worktree("feat-b")), true);
    assert!(!state.terminal_focused(), "the stop released the terminal");
    let had_focus = state.terminal_focused();
    press(&mut state, Step::Session(a), had_focus);
    assert_eq!(state.sidebar.keyboard_row, Some(RowRef::Session(a)));
    assert!(state.session.terminal_released);
}
// --- Feature 1718 (T003, FR-006, US3.2): chords and reveals walk the flat list ------------------

mod over_the_flat_list {
    use super::support::sidebar_fixture::{five_states, session_in, worktree};
    use micold_client::features::navigation::{self, Step};
    use micold_client::features::sidebar::{location_row, SidebarEntry};
    use micold_client::keymap::{NavAction, NavLevel};
    use micold_core::navigation::Dir;
    use micold_core::session::SessionLocation;

    /// Chords step through `sidebar_entries()` in order, a Done row included, with no header stop.
    #[test]
    fn the_location_order_is_the_stable_order() {
        let state = five_states();
        let order = navigation::location_order(&state);
        let stable: Vec<SessionLocation> = state
            .sidebar_entries()
            .into_iter()
            .map(|e| match e {
                SidebarEntry::Default(_) => SessionLocation::Default,
                SidebarEntry::Worktree(n) => n.location(),
                SidebarEntry::Group(_) => unreachable!(),
            })
            .collect();
        assert_eq!(order, stable);
        assert!(order.contains(&worktree("done")), "Done is a stop");
    }

    /// Stepping Next from Default visits every location once and then stops, with nothing to expand.
    #[test]
    fn next_chords_visit_every_row_once() {
        let mut state = five_states();
        state.sidebar.nav_stop = Some(SessionLocation::Default);
        let order = navigation::location_order(&state);
        let mut visited = vec![SessionLocation::Default];
        for _ in 1..order.len() {
            let step = navigation::update(
                &state,
                NavAction {
                    level: NavLevel::Location,
                    dir: Dir::Next,
                },
            );
            state.apply_navigation(step.clone(), false);
            let location = match step {
                Step::Stop(location) => location,
                Step::Session(id) => state
                    .active_sessions()
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.location.clone())
                    .expect("the chord's session is open"),
                other => panic!("a chord stops on a location row, never a header: {other:?}"),
            };
            // A session step only returns the `Selected` message; stand in for it by moving the stop.
            state.sidebar.nav_stop = Some(location.clone());
            visited.push(location);
        }
        assert_eq!(visited, order);
    }

    /// A chord onto a Done row selects it and the reveal brings it into view; nothing opens.
    #[test]
    fn a_chord_onto_a_done_row_reveals_it() {
        let mut state = five_states();
        state.sidebar.viewport_height = 60;
        state.apply_navigation(Step::Stop(worktree("done")), false);
        assert!(state.reveal_ready(), "the stop armed the reveal");
        assert!(location_row(&state.sidebar_list(), &worktree("done")).is_some());
        let offset = state
            .reveal_scroll_offset()
            .expect("below a 60 px viewport");
        state.sidebar.scroll_offset = offset;
        assert_eq!(state.reveal_scroll_offset(), None, "now fully in view");
    }

    /// The current session in a Done location is listed, so its reveal has nothing to expand.
    #[test]
    fn the_current_session_in_a_done_row_is_listed() {
        let mut state = five_states();
        let id = session_in(
            &mut state,
            worktree("done"),
            micold_core::session::SessionLifecycle::Idle,
            micold_core::protocol::messages::ActivitySignal::AwaitingInput,
            false,
        );
        state.session.active = Some(id);
        state.reveal_scroll_armed();
        state.sidebar.viewport_height = 60;
        assert!(state.current_session_is_listed());
        assert!(state.reveal_ready());
    }
}
