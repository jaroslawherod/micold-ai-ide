//! US3 tests: dismissing the About overlay and returning unchanged (FR-010, FR-011, FR-012).

use micold_client::app::{Message, State};

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

#[test]
fn about_closed_hides_overlay() {
    let mut state = State::default();
    state.update(Message::AboutOpened);
    assert_eq!(open_dialog(&state), Some("about"));
    state.update(Message::AboutClosed);
    assert_eq!(open_dialog(&state), None);
}

#[test]
fn about_closed_is_noop_when_already_none() {
    let mut state = State::default();
    let before = state.clone();
    state.update(Message::AboutClosed);
    // Esc / close with no dialog open is a no-op (edge case).
    assert_eq!(state, before);
    assert_eq!(open_dialog(&state), None);
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
