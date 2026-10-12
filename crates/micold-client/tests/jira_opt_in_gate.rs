//! The opt-in gate (feature 533, FR-001, FR-002, SC-001, US2 AS1-2): with default settings nothing
//! Jira is visible and no Jira request can be made.
//!
//! The gate is one function, `micold_core::jira::jira_visible`; every Jira control and request
//! checks it. This pins what it says for defaults, and that the Settings page for a default draft
//! holds only the switch.

use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_core::jira::{jira_visible, JiraConnection};
use micold_core::settings::Settings;

#[test]
fn default_settings_show_no_jira() {
    assert!(!jira_visible(&Settings::default()));
}

#[test]
fn a_stored_connection_does_not_open_the_gate() {
    let mut settings = Settings::default();
    settings.jira_connections.insert(
        "/work/app".to_owned(),
        JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap(),
    );
    assert!(!jira_visible(&settings), "the switch decides, not a stored connection");
    settings.jira_enabled = true;
    assert!(jira_visible(&settings));
}

#[test]
fn a_default_draft_holds_only_the_off_switch() {
    let draft = SettingsDraft::from_settings(&Settings::default());
    assert!(!draft.jira.enabled);
    assert_eq!(draft.jira.status(), None, "no status line while off");
    assert_eq!(draft.jira.connection, None);
    assert_eq!(draft.jira.project, None);
}

#[test]
fn the_jira_section_is_only_a_rail_entry_while_off() {
    // The rail lists the section so the switch can be found; its page is the switch alone.
    assert!(SettingsSection::ALL.contains(&SettingsSection::Jira));
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/settings/jira.rs"),
    )
    .unwrap();
    assert!(
        !source.contains("TextField"),
        "the page holds no credential or address field while the connection form is not built"
    );
}
