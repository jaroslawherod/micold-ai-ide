//! US3 tests: dismissing the About overlay and returning unchanged (FR-010, FR-011, FR-012).

use micold_ai_ide::app::{Message, Overlay, State};

#[test]
fn about_closed_hides_overlay() {
    let mut state = State::default();
    state.update(Message::AboutOpened);
    assert_eq!(state.overlay, Overlay::About);
    state.update(Message::AboutClosed);
    assert_eq!(state.overlay, Overlay::None);
}

#[test]
fn about_closed_is_noop_when_already_none() {
    let mut state = State::default();
    let before = state.clone();
    state.update(Message::AboutClosed);
    // Esc / close with no dialog open is a no-op (edge case).
    assert_eq!(state, before);
    assert_eq!(state.overlay, Overlay::None);
}

#[test]
fn open_then_close_returns_to_prior_state() {
    // Dismissing returns the window to its exact pre-dialog state (FR-012).
    let mut state = State::default();
    let before = state.clone();
    state.update(Message::AboutOpened);
    state.update(Message::AboutClosed);
    assert_eq!(state, before);
}
