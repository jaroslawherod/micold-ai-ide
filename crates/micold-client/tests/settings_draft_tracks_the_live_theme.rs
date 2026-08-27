//! The open Settings form's copy of the theme follows the live one (feature 027, BUG-001).
//!
//! # Why one setting needs this and the others do not
//!
//! Every field in Settings is drafted: typed into a copy, applied on Save, discarded on Cancel.
//! That works because the form is the only thing that writes those values. The theme is the one
//! exception — the app bar's overflow menu cycles it too, and applies it immediately.
//!
//! Two writers were harmless while Settings was a 420dp modal, because the modal covered the app
//! bar and the menu could not be reached. Feature 027 made Settings a full-surface view with the
//! app bar still on screen (FR-026), so both writers are now reachable at once — and the draft,
//! seeded when the view opened, still said what the theme was *then*. Cycling the menu and
//! pressing Save reverted the theme to that stale value: the user's most recent choice, made two
//! seconds earlier and visibly applied, was undone by a button labelled Save.
//!
//! The fix is not to stop drafting the theme — Cancel must still discard an Appearance edit. It
//! is that a live change from outside the form is *newer than the draft*, so the draft takes it.

use micold_client::app::{Message, State};
use micold_client::features::settings::SettingsDraft;
use micold_core::theme::ThemePreference;

/// State with Settings open on a draft seeded from `theme`, the way the shell seeds it.
fn open_on(theme: ThemePreference) -> State {
    let mut draft = SettingsDraft::default();
    draft.appearance.theme = theme;
    draft.terminal.scrollback_lines = "5000".into();
    draft.environment.timeout_secs = "5".into();
    State {
        theme_pref: theme,
        settings_draft: Some(draft),
        ..State::default()
    }
}

/// What Save would write, which is the only thing that decides whether the theme reverts.
fn theme_on_save(state: &State) -> ThemePreference {
    state
        .settings_draft
        .as_ref()
        .expect("the form is open")
        .validate()
        .expect("the seeded draft is valid")
        .theme
}

#[test]
fn cycling_the_theme_from_the_app_bar_is_what_save_then_writes() {
    let mut state = open_on(ThemePreference::Light);

    state.update(Message::ThemeModeCycled);

    assert_eq!(
        state.theme_pref,
        ThemePreference::Dark,
        "precondition: the menu applies its choice immediately"
    );
    assert_eq!(
        theme_on_save(&state),
        ThemePreference::Dark,
        "Save would write `{:?}` — the theme the form was opened on — over the one the user chose \
         from the app bar afterwards and can see applied. A Save that undoes the user's most \
         recent visible choice is the worst shape this can take: it is silent, and the control \
         that caused it is not on the section they were looking at.",
        theme_on_save(&state)
    );
}

#[test]
fn choosing_a_theme_outright_is_what_save_then_writes() {
    let mut state = open_on(ThemePreference::FollowSystem);

    state.update(Message::ThemePreferenceChanged(ThemePreference::Dark));

    assert_eq!(state.theme_pref, ThemePreference::Dark);
    assert_eq!(
        theme_on_save(&state),
        ThemePreference::Dark,
        "the menu's outright picks need the same tracking as its cycle — they are two ways to \
         reach one setting, and a fix that covered only the cycle would leave the bug reachable"
    );
}

/// The tracking must not run the other way: the form still drafts.
#[test]
fn an_appearance_edit_is_still_only_a_draft_until_save() {
    let mut state = open_on(ThemePreference::Light);

    state.update(Message::SettingsThemeChanged(ThemePreference::Dark));

    assert_eq!(
        state.theme_pref,
        ThemePreference::Light,
        "editing the Appearance section must not apply the theme before Save, or Cancel would \
         have nothing to discard"
    );
    assert_eq!(theme_on_save(&state), ThemePreference::Dark);
}

/// And with no form open there is nothing to track, which must not panic or resurrect a draft.
#[test]
fn the_menu_still_works_with_settings_closed() {
    let mut state = State {
        theme_pref: ThemePreference::Light,
        ..State::default()
    };

    state.update(Message::ThemeModeCycled);

    assert_eq!(state.theme_pref, ThemePreference::Dark);
    assert!(
        state.settings_draft.is_none(),
        "tracking a value into a form that is not open would open one"
    );
}
