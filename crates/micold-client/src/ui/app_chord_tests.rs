//! `app_chord` (feature 803, contracts/keys.md K3): the chords the app hears while the terminal is
//! not focused.

use super::*;
use iced::keyboard::{self, key::Code, key::Physical, Key, Modifiers};

fn primary_shift(key: &str, code: Code) -> keyboard::Event {
    #[cfg(target_os = "macos")]
    let modifiers = Modifiers::LOGO | Modifiers::SHIFT;
    #[cfg(not(target_os = "macos"))]
    let modifiers = Modifiers::CTRL | Modifiers::SHIFT;
    keyboard::Event::KeyPressed {
        key: Key::Character(key.into()),
        modified_key: Key::Character(key.to_uppercase().into()),
        physical_key: Physical::Code(code),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

#[test]
fn the_overview_chord_toggles_the_overview() {
    assert!(matches!(
        app_chord(primary_shift("o", Code::KeyO)),
        Some(Message::Overview(crate::features::overview::Msg::Toggled {
            settings_open: false
        }))
    ));
}

/// Feature 970 (FR-001): the full-screen chord reaches the Overview; its update ignores it while
/// the Overview is closed.
#[test]
fn the_full_screen_chord_toggles_full_screen() {
    assert!(matches!(
        app_chord(primary_shift("f", Code::KeyF)),
        Some(Message::Overview(
            crate::features::overview::Msg::FullScreenToggled {
                settings_open: false
            }
        ))
    ));
}

#[test]
fn the_switcher_chord_toggles_the_switcher_without_the_terminal() {
    let message = app_chord(primary_shift("p", Code::KeyP));
    assert!(
        matches!(
            message,
            Some(Message::QuickSwitch(
                crate::features::quick_switch::Msg::Toggled {
                    terminal_had_focus: false
                }
            ))
        ),
        "got {message:?}"
    );
}

#[test]
fn a_location_chord_navigates_by_its_physical_key() {
    use crate::keymap::{NavAction, NavLevel};
    use micold_core::navigation::Dir;
    let message = app_chord(primary_shift("б", Code::Comma));
    assert!(
        matches!(
            message,
            Some(Message::Navigation(
                crate::features::navigation::Msg::Pressed(NavAction {
                    level: NavLevel::Location,
                    dir: Dir::Previous
                })
            ))
        ),
        "got {message:?}"
    );
}

#[test]
fn other_keys_are_not_app_chords() {
    assert!(app_chord(primary_shift("e", Code::KeyE)).is_none());
}

// ---- `card_menu_key` (feature 972, FR-010): the open card menu's keys ----

fn named_key(key: keyboard::key::Named, modifiers: Modifiers) -> keyboard::Event {
    keyboard::Event::KeyPressed {
        key: Key::Named(key),
        modified_key: Key::Named(key),
        physical_key: Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

#[test]
fn the_card_menu_keys_step_and_activate_and_a_modified_key_is_not_theirs() {
    use crate::features::overview::Msg;
    use keyboard::key::Named;
    let key = |named| card_menu_key(named_key(named, Modifiers::empty()));
    assert!(
        matches!(
            key(Named::ArrowDown),
            Some(Message::Overview(Msg::CardMenuMoved {
                forward: true,
                items: 0
            }))
        ),
        "Down steps forward"
    );
    assert!(
        matches!(
            key(Named::ArrowUp),
            Some(Message::Overview(Msg::CardMenuMoved {
                forward: false,
                items: 0
            }))
        ),
        "Up steps back"
    );
    for named in [Named::Enter, Named::Space] {
        assert!(
            matches!(key(named), Some(Message::Overview(Msg::CardMenuActivated))),
            "{named:?} runs the highlighted entry"
        );
    }
    assert!(key(Named::Tab).is_none(), "another key is not the menu's");
    for modifiers in [Modifiers::SHIFT, Modifiers::CTRL, Modifiers::ALT] {
        assert!(
            card_menu_key(named_key(Named::ArrowDown, modifiers)).is_none(),
            "{modifiers:?}+Down is left to its owner"
        );
    }
}
