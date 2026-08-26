//! US2 tests: opening the About overlay via Help → About (FR-004, FR-005, FR-015).

use micold_client::app::{Message, State};
use micold_client::features::help::Msg as HelpMsg;

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

#[test]
fn about_opened_shows_overlay() {
    let mut state = State::default();
    assert_eq!(open_dialog(&state), None);
    state.update(Message::Help(HelpMsg::AboutOpened));
    assert_eq!(open_dialog(&state), Some("about"));
}

#[test]
fn about_opened_is_idempotent_single_instance() {
    let mut state = State::default();
    state.update(Message::Help(HelpMsg::AboutOpened));
    state.update(Message::Help(HelpMsg::AboutOpened));
    // A second activation must not create a second dialog (FR-015).
    assert_eq!(open_dialog(&state), Some("about"));
}

#[test]
fn help_menu_toggles_open_and_closed() {
    let mut state = State::default();
    assert!(!state.help_menu_open);
    state.update(Message::Help(HelpMsg::MenuToggled));
    assert!(state.help_menu_open);
    state.update(Message::Help(HelpMsg::MenuToggled));
    assert!(!state.help_menu_open);
}

#[test]
fn opening_about_collapses_the_help_menu() {
    let mut state = State::default();
    state.update(Message::Help(HelpMsg::MenuToggled));
    state.update(Message::Help(HelpMsg::AboutOpened));
    assert_eq!(open_dialog(&state), Some("about"));
    assert!(!state.help_menu_open);
}
