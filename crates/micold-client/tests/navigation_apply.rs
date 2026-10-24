//! Applying a resolved navigation press (feature 803, contracts/ui.md U3).

use micold_client::app::Message;
use micold_client::features::navigation::Step;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::sidebar::RowRef;

mod support;
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

// --- Feature 1125 (T008, FR-006, US5.4, R5): chords and reveals into a closed group -----------

mod into_a_closed_group {
    use super::support::sidebar_fixture::{five_states, worktree};
    use micold_client::app::State;
    use micold_client::features::navigation::{self, Step};
    use micold_client::features::sidebar::{location_row, ListRow};
    use micold_client::keymap::{NavAction, NavLevel};
    use micold_core::navigation::Dir;
    use micold_core::session::SessionLocation;
    use micold_core::sidebar_groups::StateGroup;

    fn done_open(state: &State) -> bool {
        state
            .sidebar_list()
            .iter()
            .any(|row| matches!(row, ListRow::Header(h) if h.group == StateGroup::Done && h.open))
    }

    fn chosen(state: &State, group: StateGroup) -> Option<bool> {
        state
            .workspace
            .active
            .as_ref()
            .and_then(|p| state.sidebar.group_choices.get(p))
            .and_then(|c| c.get(&group).copied())
    }

    /// Chords step through the grouped order, closed groups included.
    #[test]
    fn the_location_order_is_the_grouped_order_with_closed_groups() {
        let state = five_states();
        let order = navigation::location_order(&state);
        let grouped: Vec<SessionLocation> = state
            .sidebar_groups()
            .into_iter()
            .flat_map(|g| g.entries)
            .map(|e| match e {
                micold_client::features::sidebar::SidebarEntry::Default(_) => {
                    SessionLocation::Default
                }
                micold_client::features::sidebar::SidebarEntry::Worktree(n) => n.location(),
                micold_client::features::sidebar::SidebarEntry::Group(_) => unreachable!(),
            })
            .collect();
        assert_eq!(order, grouped);
        assert_eq!(
            order.last(),
            Some(&worktree("done")),
            "closed Done included"
        );
    }

    /// A next-location chord landing on a location in closed Done opens Done, keeps the row, and
    /// the reveal brings it fully into view.
    #[test]
    fn a_chord_into_closed_done_opens_it_and_reveals_the_row() {
        let mut state = five_states();
        state.sidebar.nav_stop = Some(SessionLocation::Default);
        state.sidebar.viewport_height = 60;
        let order = navigation::location_order(&state);
        let at = order
            .iter()
            .position(|l| *l == SessionLocation::Default)
            .unwrap();
        let mut step = Step::None;
        // Step on from Default until the chord lands on `done`.
        for _ in at..order.len() - 1 {
            step = navigation::update(
                &state,
                NavAction {
                    level: NavLevel::Location,
                    dir: Dir::Next,
                },
            );
            state.apply_navigation(step.clone(), false);
        }
        assert_eq!(step, Step::Stop(worktree("done")));
        assert!(!done_open(&state), "not before the reveal drains");
        assert!(state.reveal_ready(), "the stop armed the reveal");
        assert!(done_open(&state));
        assert_eq!(chosen(&state, StateGroup::Done), Some(true));
        let list = state.sidebar_list();
        assert!(location_row(&list, &worktree("done")).is_some());
        assert!(state.current_session_is_listed());
        let offset = state
            .reveal_scroll_offset()
            .expect("below a 60 px viewport");
        state.sidebar.scroll_offset = offset;
        assert_eq!(state.reveal_scroll_offset(), None, "now fully in view");
    }

    /// A chord into closed Done opens it even while the current session's row is drawn in an
    /// open group: the reveal's target is the chord's row, not the current session's.
    #[test]
    fn a_chord_into_closed_done_opens_it_while_the_current_session_is_drawn() {
        let mut state = five_states();
        let working = state
            .active_sessions()
            .iter()
            .find(|s| s.location == worktree("work"))
            .map(|s| s.id)
            .expect("five_states has a session in `work`");
        state.session.active = Some(working);
        state.sidebar.viewport_height = 60;
        assert!(
            state.current_session_is_listed(),
            "Working is open, its row drawn"
        );
        state.apply_navigation(Step::Stop(worktree("done")), false);
        assert!(!done_open(&state));
        assert!(state.reveal_ready());
        assert!(done_open(&state));
        assert_eq!(chosen(&state, StateGroup::Done), Some(true));
        assert!(location_row(&state.sidebar_list(), &worktree("done")).is_some());
        let offset = state
            .reveal_scroll_offset()
            .expect("below a 60 px viewport");
        state.sidebar.scroll_offset = offset;
        assert_eq!(
            state.reveal_scroll_offset(),
            None,
            "the chord's row is in view"
        );
    }

    /// The Overview's reveal opens the closed group of its row; an open group's choice stays
    /// unrecorded.
    #[test]
    fn the_overviews_reveal_opens_its_rows_group() {
        let mut state = five_states();
        state.sidebar.revealed = Some(worktree("work"));
        state.open_group_of_revealed();
        assert_eq!(
            chosen(&state, StateGroup::Working),
            None,
            "Working was open"
        );
        state.sidebar.revealed = Some(worktree("done"));
        state.open_group_of_revealed();
        assert_eq!(chosen(&state, StateGroup::Done), Some(true));
    }

    /// The current session's closed group opens when its reveal drains.
    #[test]
    fn the_current_sessions_closed_group_opens_on_its_reveal() {
        let mut state = five_states();
        let id = super::support::sidebar_fixture::session_in(
            &mut state,
            worktree("done"),
            micold_core::session::SessionLifecycle::Idle,
            micold_core::protocol::messages::ActivitySignal::AwaitingInput,
            false,
        );
        state.session.active = Some(id);
        state.update(micold_client::app::Message::Sidebar(
            micold_client::features::sidebar::Msg::GroupToggled(StateGroup::Done),
        ));
        assert!(!done_open(&state), "the user closed it");
        state.reveal_scroll_armed();
        state.sidebar.viewport_height = 60;
        assert!(state.reveal_ready());
        assert!(done_open(&state));
    }
}
