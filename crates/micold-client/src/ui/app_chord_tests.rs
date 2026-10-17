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
