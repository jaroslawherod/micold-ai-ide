//! US3 tests: the Escape key maps to dismissal only while the About overlay is open
//! (FR-011; edge case: Esc with no dialog open has no effect).

use micold_client::app::{on_escape, Message, State};
use micold_client::features::help::Msg as HelpMsg;

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

#[test]
fn escape_closes_when_about_open() {
    let mut state = State::default();
    state.update(Message::Help(HelpMsg::AboutOpened));
    assert_eq!(on_escape(&state), Some(Message::Help(HelpMsg::AboutClosed)));
}

/// Feature 1269 T023 (U10, FR-013): with the review shown and no dialog open, Escape goes nowhere,
/// as with the Changes view open: it never closes the review.
#[test]
fn escape_with_the_review_shown_keeps_it() {
    let mut state = State::default();
    let _ = micold_client::features::changes::opened(
        &mut state.changes,
        Default::default(),
        micold_core::session::SessionLocation::Default,
        None,
    );
    assert!(state.review_shown().is_some());
    assert_eq!(on_escape(&state), None);
}

#[test]
fn escape_is_noop_when_no_overlay() {
    let state = State::default();
    assert_eq!(open_dialog(&state), None);
    assert_eq!(on_escape(&state), None);
}
