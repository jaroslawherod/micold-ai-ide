//! The open Settings form's copy of the theme follows the live one (feature 027, BUG-001).
//!
//! # Why one setting needs this and the others do not
//!
//! Every field in Settings is drafted: typed into a copy, applied on Save, discarded on Cancel.
//! That works because the form is the only thing that writes those values. The theme was the one
//! exception — the app bar's overflow menu cycled it too, and applied it immediately.
//!
//! Two writers were harmless while Settings was a 420dp modal, because the modal covered the app
//! bar and the menu could not be reached. Feature 027 made Settings a full-surface view with the
//! app bar still on screen (FR-026), so both writers became reachable at once — and the draft,
//! seeded when the view opened, still said what the theme was *then*. Cycling the menu and
//! pressing Save reverted the theme to that stale value: the user's most recent choice, made two
//! seconds earlier and visibly applied, was undone by a button labelled Save.
//!
//! The fix is not to stop drafting the theme — Cancel must still discard an Appearance edit. It
//! is that a live change from outside the form is *newer than the draft*, so the draft takes it.
//!
//! # This outlived the control that caused it, deliberately
//!
//! FR-026e has since removed the app bar's cycle, so nothing writes the theme from outside the
//! form today and the two-writer condition is gone. What is asserted here is the *rule*, driven
//! through `Message::ThemePreferenceChanged` — the reducer's contract for a live theme change. A
//! second writer is a plausible thing to add back (a keyboard shortcut, a tray item, a system
//! integration), and the day someone does, this is what tells them the draft has to follow.

use micold_client::app::{Message, State};
use micold_client::features::settings::Msg as SettingsMsg;
use micold_client::features::settings::SettingsDraft;
use micold_core::theme::ThemePreference;

/// State with Settings open on a draft seeded from `theme`, the way the shell seeds it.
fn open_on(theme: ThemePreference) -> State {
    let mut draft = SettingsDraft::default();
    draft.appearance.theme = theme;
    draft.terminal.scrollback_lines = "5000".into();
    draft.environment.timeout_secs = "5".into();
    let mut state = State::default();
    state.settings.theme_pref = theme;
    state.settings.settings_draft = Some(draft);
    state
}

/// What Save would write, which is the only thing that decides whether the theme reverts.
fn theme_on_save(state: &State) -> ThemePreference {
    state
        .settings
        .settings_draft
        .as_ref()
        .expect("the form is open")
        .validate()
        .expect("the seeded draft is valid")
        .theme
}

#[test]
fn a_live_theme_change_is_what_save_then_writes() {
    let mut state = open_on(ThemePreference::Light);

    state.update(Message::Settings(SettingsMsg::ThemePreferenceChanged(
        ThemePreference::Dark,
    )));

    assert_eq!(
        state.settings.theme_pref,
        ThemePreference::Dark,
        "precondition: a change from outside the form applies immediately"
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

/// The tracking must not run the other way: the form still drafts.
#[test]
fn an_appearance_edit_is_still_only_a_draft_until_save() {
    let mut state = open_on(ThemePreference::Light);

    state.update(Message::Settings(SettingsMsg::ThemeChanged(
        ThemePreference::Dark,
    )));

    assert_eq!(
        state.settings.theme_pref,
        ThemePreference::Light,
        "editing the Appearance section must not apply the theme before Save, or Cancel would \
         have nothing to discard"
    );
    assert_eq!(theme_on_save(&state), ThemePreference::Dark);
}

/// And with no form open there is nothing to track, which must not panic or resurrect a draft.
#[test]
fn a_live_change_with_settings_closed_opens_no_form() {
    let mut state = State::default();
    state.settings.theme_pref = ThemePreference::Light;

    state.update(Message::Settings(SettingsMsg::ThemePreferenceChanged(
        ThemePreference::Dark,
    )));

    assert_eq!(state.settings.theme_pref, ThemePreference::Dark);
    assert!(
        state.settings.settings_draft.is_none(),
        "tracking a value into a form that is not open would open one"
    );
}
