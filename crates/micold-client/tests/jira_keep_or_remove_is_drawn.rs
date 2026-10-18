//! The Keep-or-Remove question is on the page while it is pending (feature 533, T078, US2 AS5).
//!
//! Found by the close unit's visual pass (B7): with the switch turned off over a stored connection,
//! Save raised the question but the Jira page drew the credential form, and so the question, only
//! while the switch was on. Nothing appeared and Save seemed to do nothing. The reducer tests read
//! `draft.jira.prompt`; this one reads what the renderer painted.

#[path = "support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::settings::{Msg, SettingsDraft, SettingsSection};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::jira::JiraConnection;
use micold_core::settings::Settings;
use support::layout as lay;

/// Settings open on the Jira page of `/work/app`, a connection stored, the switch on.
fn jira_page() -> State {
    let settings = Settings {
        jira_enabled: true,
        ..Settings::default()
    };
    let mut draft = SettingsDraft::from_settings(&settings);
    let connection =
        JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap();
    let connections = BTreeMap::from([("/work/app".to_owned(), connection)]);
    draft
        .jira
        .follow(Some(PathBuf::from("/work/app")), &connections, true);
    draft.section = SettingsSection::Jira;
    let mut state = State::default();
    state.settings.settings_draft = Some(draft);
    state.window.window_size = (1280, 2400);
    state
}

fn painted(state: &State) -> Vec<String> {
    let mut renderer = lay::renderer();
    let element = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    lay::painted_text_settled_at(element, &mut renderer, iced::Size::new(1280.0, 2400.0))
        .0
        .into_iter()
        .map(|drawn| drawn.content)
        .collect()
}

fn shows(painted: &[String], needle: &str) -> bool {
    painted.iter().any(|content| content.contains(needle))
}

/// What Save does with the switch off over a stored connection (`on_settings_saved`).
fn switch_off_and_save(state: &mut State) {
    state.update(micold_client::app::Message::Settings(
        Msg::JiraEnabledToggled(false),
    ));
    let draft = state.settings.settings_draft.as_mut().unwrap();
    assert!(draft.needs_keep_or_remove());
    draft.raise_keep_or_remove();
}

#[test]
fn the_question_is_drawn_after_the_switch_goes_off_and_save_is_pressed() {
    let mut state = jira_page();
    switch_off_and_save(&mut state);
    let painted = painted(&state);
    assert!(
        shows(&painted, "Keep the Jira connection for app?"),
        "painted: {painted:?}"
    );
    assert!(shows(&painted, "Keep") && shows(&painted, "Remove"));
}

#[test]
fn nothing_is_asked_until_save_raises_the_question() {
    let mut state = jira_page();
    state.update(micold_client::app::Message::Settings(
        Msg::JiraEnabledToggled(false),
    ));
    assert!(!shows(&painted(&state), "Keep the Jira connection"));
}

#[test]
fn with_the_switch_off_the_credential_fields_stay_hidden_beside_the_question() {
    let mut state = jira_page();
    switch_off_and_save(&mut state);
    let painted = painted(&state);
    assert!(!shows(&painted, "Site URL"), "painted: {painted:?}");
    assert!(!shows(&painted, "Account email"));
}

#[test]
fn the_credential_fields_are_painted_only_with_the_switch_on() {
    let on = painted(&jira_page());
    for label in ["Site URL", "Project key", "Account email"] {
        assert!(shows(&on, label), "{label} missing with the switch on: {on:?}");
    }
    let mut state = jira_page();
    state.update(micold_client::app::Message::Settings(
        Msg::JiraEnabledToggled(false),
    ));
    let off = painted(&state);
    for label in ["Site URL", "Project key", "Account email"] {
        assert!(!shows(&off, label), "{label} painted with the switch off");
    }
    assert!(shows(&off, "Enable Jira integration"));
}
