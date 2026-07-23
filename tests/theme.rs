//! Theme resolution truth table (contracts/theme-behavior.md; FR-005, FR-007, FR-018).
//! Pure — runs under `cargo test --no-default-features`.

use micold_ai_ide::theme::{
    observe_system_scheme, resolve, ColorScheme, SystemScheme, ThemePreference,
};

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

// FR-021 / BUG-001: a single detection failure (e.g. a `dark_light::detect()` timeout under
// CPU load) must not be treated as a genuine "OS reports no preference" — it must retain the
// last-known scheme instead of flashing to the FR-018 fallback.
#[test]
fn transient_detection_failure_keeps_the_last_known_scheme() {
    for last_known in [
        SystemScheme::Light,
        SystemScheme::Dark,
        SystemScheme::Unspecified,
    ] {
        assert_eq!(observe_system_scheme(Err(()), last_known), last_known);
    }
}

#[test]
fn successful_detection_always_updates_the_scheme() {
    for last_known in [
        SystemScheme::Light,
        SystemScheme::Dark,
        SystemScheme::Unspecified,
    ] {
        for detected in [
            SystemScheme::Light,
            SystemScheme::Dark,
            SystemScheme::Unspecified,
        ] {
            assert_eq!(observe_system_scheme(Ok(detected), last_known), detected);
        }
    }
}

#[test]
fn a_transient_failure_does_not_flash_to_light_when_the_os_is_dark() {
    // The exact reported symptom: OS is Dark, one poll times out, the app must stay Dark
    // rather than resolving to Light for that cycle.
    let after_timeout = observe_system_scheme(Err(()), SystemScheme::Dark);
    assert_eq!(
        resolve(ThemePreference::FollowSystem, after_timeout),
        ColorScheme::Dark
    );
}
