//! Theme resolution truth table (contracts/theme-behavior.md; FR-005, FR-007, FR-018).
//! Pure — runs under `cargo test --no-default-features`.

use micold_ai_ide::theme::{resolve, ColorScheme, SystemScheme, ThemePreference};

#[test]
fn fixed_light_ignores_the_os() {
    for system in [
        SystemScheme::Light,
        SystemScheme::Dark,
        SystemScheme::Unspecified,
    ] {
        assert_eq!(resolve(ThemePreference::Light, system), ColorScheme::Light);
    }
}

#[test]
fn fixed_dark_ignores_the_os() {
    for system in [
        SystemScheme::Light,
        SystemScheme::Dark,
        SystemScheme::Unspecified,
    ] {
        assert_eq!(resolve(ThemePreference::Dark, system), ColorScheme::Dark);
    }
}

#[test]
fn follow_system_tracks_the_os() {
    assert_eq!(
        resolve(ThemePreference::FollowSystem, SystemScheme::Light),
        ColorScheme::Light
    );
    assert_eq!(
        resolve(ThemePreference::FollowSystem, SystemScheme::Dark),
        ColorScheme::Dark
    );
}

#[test]
fn follow_system_falls_back_to_light_when_unspecified() {
    // FR-018: no OS preference available → light.
    assert_eq!(
        resolve(ThemePreference::FollowSystem, SystemScheme::Unspecified),
        ColorScheme::Light
    );
}

#[test]
fn default_preference_is_follow_system() {
    assert_eq!(ThemePreference::default(), ThemePreference::FollowSystem);
}
