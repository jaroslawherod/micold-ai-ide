//! The quick switcher's client behaviour (feature 968).

mod support;

use micold_client::app::State;
use micold_client::features::quick_switch::State as SwitcherState;
use micold_core::session::SessionLocation;
use support::sidebar_fixture::{deliver, listed, session_at, state_with_active, worktree};

/// `/repo` open with worktrees `alpha` and `beta`, one session in `alpha` and one in Default.
#[allow(dead_code)]
pub fn switcher_fixture() -> State {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("alpha"), listed("beta")]);
    session_at(&mut state, SessionLocation::Default);
    session_at(&mut state, worktree("alpha"));
    state
}

#[test]
fn the_switcher_starts_closed() {
    let state = switcher_fixture();
    assert_eq!(state.quick_switch, SwitcherState::default());
    assert!(state.quick_switch.open.is_none());
}
