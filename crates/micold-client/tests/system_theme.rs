//! Regression test for the theme-subscription boot panic, found by the `run` skill's sanity
//! check (2026-07-23): `os_theme_poll`'s `Subscription::map` closure used to capture the
//! previous `SystemScheme` so it could apply the last-known fallback itself, which crashed the
//! app on launch (iced panics if a subscription's mapping closure captures anything — it breaks
//! the stable identity iced needs to avoid restarting the underlying timer every frame).
//!
//! The fix moved the fallback into the reducer, which already holds the previous scheme in
//! `State::system_scheme`; `Message::Settings(SettingsMsg::SystemThemeChanged)` now carries the raw detection outcome
//! instead of an already-resolved scheme. This test exercises that reducer logic directly.

use micold_client::app::{Message, State};
use micold_client::features::settings;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_core::theme::SystemScheme;

#[test]
fn a_successful_detection_updates_the_scheme() {
    let mut state = State {
        settings: settings::State {
            system_scheme: SystemScheme::Light,
            ..Default::default()
        },
        ..State::default()
    };

    state.update(Message::Settings(SettingsMsg::SystemThemeChanged(Ok(
        SystemScheme::Dark,
    ))));

    assert_eq!(state.settings.system_scheme, SystemScheme::Dark);
}

#[test]
fn a_transient_detection_failure_keeps_the_last_known_scheme() {
    let mut state = State {
        settings: settings::State {
            system_scheme: SystemScheme::Dark,
            ..Default::default()
        },
        ..State::default()
    };

    state.update(Message::Settings(SettingsMsg::SystemThemeChanged(Err(()))));

    assert_eq!(
        state.settings.system_scheme,
        SystemScheme::Dark,
        "a transient dark_light::detect() failure must not overwrite the last-known scheme"
    );
}

#[test]
fn a_genuine_unspecified_reading_is_applied_like_any_other_successful_read() {
    let mut state = State {
        settings: settings::State {
            system_scheme: SystemScheme::Dark,
            ..Default::default()
        },
        ..State::default()
    };

    state.update(Message::Settings(SettingsMsg::SystemThemeChanged(Ok(
        SystemScheme::Unspecified,
    ))));

    assert_eq!(
        state.settings.system_scheme,
        SystemScheme::Unspecified,
        "Ok(Unspecified) is a genuine OS reading, not a failure — it must still update the scheme"
    );
}
