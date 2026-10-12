//! The Jira switch and connections on disk (feature 533, FR-002, US2 AS6, contracts/
//! jira-connection.md §1): off by default, a pre-feature file reads as off with nothing else
//! changed, and an entry that no longer parses is dropped on read.

use micold_core::jira::JiraConnection;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore, SETTINGS_VERSION};
use micold_core::store::LoadStatus;

fn store(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
    JsonFileSettingsStore::at(dir.path().join("settings.json"))
}

fn connection() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

#[test]
fn defaults_are_off_and_empty() {
    let settings = Settings::default();
    assert!(!settings.jira_enabled);
    assert!(settings.jira_connections.is_empty());
}

#[test]
fn a_file_written_before_the_feature_reads_as_off() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{ "settings_version": 4, "scrollback_lines": 20000 }"#,
    )
    .unwrap();
    let loaded = store(&dir).load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    assert!(!loaded.settings.jira_enabled);
    assert!(loaded.settings.jira_connections.is_empty());
    assert_eq!(loaded.settings.scrollback_lines, 20000);
    assert_eq!(SETTINGS_VERSION, 4);
}

#[test]
fn the_switch_and_a_connection_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings {
        jira_enabled: true,
        ..Settings::default()
    };
    settings
        .jira_connections
        .insert("/work/app".to_owned(), connection());
    store(&dir).save(&settings).unwrap();

    let loaded = store(&dir).load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    assert!(loaded.settings.jira_enabled);
    assert_eq!(loaded.settings.jira_connections["/work/app"], connection());
}

#[test]
fn a_connection_that_no_longer_parses_is_dropped_and_the_rest_kept() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{
            "settings_version": 4,
            "jira_enabled": true,
            "jira_connections": {
                "/a": { "site": "http://insecure.example.com", "project_key": "ABC", "email": "me@acme.com" },
                "/b": { "site": "https://acme.atlassian.net", "project_key": "ABC", "email": "me@acme.com" },
                "/c": "garbage"
            }
        }"#,
    )
    .unwrap();
    let loaded = store(&dir).load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    assert!(loaded.settings.jira_enabled);
    let keys: Vec<_> = loaded.settings.jira_connections.keys().cloned().collect();
    assert_eq!(keys, vec!["/b".to_owned()]);
}
