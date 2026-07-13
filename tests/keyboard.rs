//! US3 tests: the Escape key maps to dismissal only while the About overlay is open
//! (FR-011; edge case: Esc with no dialog open has no effect).

use micold_ai_ide::app::{on_escape, Message, Overlay, State};

#[test]
fn escape_closes_when_about_open() {
    let mut state = State::default();
    state.update(Message::AboutOpened);
    assert_eq!(on_escape(&state), Some(Message::AboutClosed));
}

#[test]
fn escape_is_noop_when_no_overlay() {
    let state = State::default();
    assert_eq!(state.overlay, Overlay::None);
    assert_eq!(on_escape(&state), None);
}
