//! The wiring from app state to the navigation drawer (feature 970, FR-002, FR-008, FR-009).

use super::*;

fn full_screen_state(on: bool) -> State {
    let mut state = State::default();
    state.overview.open = on;
    state.overview.full_screen = on;
    state
}

#[test]
fn full_screen_retracts_the_drawer() {
    assert!(drawer_flags(&full_screen_state(true)).retracted);
}

#[test]
fn outside_full_screen_the_drawer_is_not_retracted() {
    assert!(!drawer_flags(&full_screen_state(false)).retracted);
}

#[test]
fn reduce_motion_makes_the_drawer_instant() {
    let mut state = State::default();
    state.settings.reduce_motion = true;
    assert!(drawer_flags(&state).instant);
}

#[test]
fn without_reduce_motion_the_drawer_animates() {
    let mut state = State::default();
    state.settings.reduce_motion = false;
    assert!(!drawer_flags(&state).instant);
}

/// Finding 10: full screen frees the whole drawer, handle included when expanded.
#[test]
fn freed_width_counts_the_handle_only_when_the_sidebar_is_expanded() {
    let mut state = State::default();
    state.sidebar.hidden = false;
    assert_eq!(
        freed_width(&state),
        f32::from(state.sidebar_width_px()) + crate::ui::material::RESIZE_HANDLE_WIDTH
    );
    state.sidebar.hidden = true;
    assert_eq!(freed_width(&state), crate::ui::sidebar::STRIP_WIDTH);
}
