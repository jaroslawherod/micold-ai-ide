//! US2 tests: opening the About overlay via Help → About (FR-004, FR-005, FR-015).

use micold_client::app::{Message, Overlay, State};

#[test]
fn about_opened_shows_overlay() {
    let mut state = State::default();
    assert_eq!(state.overlay, Overlay::None);
    state.update(Message::AboutOpened);
    assert_eq!(state.overlay, Overlay::About);
}

#[test]
fn about_opened_is_idempotent_single_instance() {
    let mut state = State::default();
    state.update(Message::AboutOpened);
    state.update(Message::AboutOpened);
    // A second activation must not create a second dialog (FR-015).
    assert_eq!(state.overlay, Overlay::About);
}

#[test]
fn help_menu_toggles_open_and_closed() {
    let mut state = State::default();
    assert!(!state.help_menu_open);
    state.update(Message::HelpMenuToggled);
    assert!(state.help_menu_open);
    state.update(Message::HelpMenuToggled);
    assert!(!state.help_menu_open);
}

#[test]
fn opening_about_collapses_the_help_menu() {
    let mut state = State::default();
    state.update(Message::HelpMenuToggled);
    state.update(Message::AboutOpened);
    assert_eq!(state.overlay, Overlay::About);
    assert!(!state.help_menu_open);
}
