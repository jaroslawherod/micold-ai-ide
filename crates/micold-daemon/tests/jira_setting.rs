//! The Jira switch and connections on the service (feature 533, contracts/jira-connection.md §1):
//! `SettingsSet { jira_enabled, jira_connection }` is stored in the settings file and pushed to
//! every connected client as `SettingsChanged`. The service stores them and never reads them
//! (FR-021); two projects' changes arriving back to back both survive (FR-022).
//!
//! Drives `server::serve_connection` over an in-memory duplex against a settings file in a
//! temporary directory, so "persisted" is checked by starting a second service over that file.

#[path = "support/conn.rs"]
mod conn;

use conn::{both_registered, connect_with_settings, next_settings_changed, service, set, stored};

use micold_core::jira::JiraConnection;
use micold_core::settings::{JiraChange, JiraConnectionChange};

fn connection(key: &str) -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", key, "me@acme.com").unwrap()
}

fn upsert(project: &str, key: &str) -> JiraConnectionChange {
    JiraConnectionChange {
        project: project.to_owned(),
        change: JiraChange::Upsert(connection(key)),
    }
}

fn removal(project: &str) -> JiraConnectionChange {
    JiraConnectionChange {
        project: project.to_owned(),
        change: JiraChange::Remove,
    }
}

/// FR-002: off until turned on; once on, the service holds it, across a restart.
#[tokio::test]
async fn the_switch_is_stored_and_reported_to_the_next_client() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, welcomed) = connect_with_settings(&state).await;
    assert!(!welcomed.jira_enabled, "off until the user turns it on");
    assert!(welcomed.jira_connections.is_empty());

    set(&mut client, 1, Some(true), None, None, None).await;

    let (_next, welcomed) = connect_with_settings(&state).await;
    assert!(welcomed.jira_enabled);
    let restarted = service(store.path());
    let (_again, welcomed) = connect_with_settings(&restarted).await;
    assert!(welcomed.jira_enabled, "the switch is in the settings file");
}

/// FR-022: a second connected client is pushed the switch and the connection.
#[tokio::test]
async fn a_change_is_echoed_to_a_second_connected_client() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut first, _) = connect_with_settings(&state).await;
    let (mut second, _) = connect_with_settings(&state).await;
    both_registered(&state).await;

    set(
        &mut first,
        1,
        Some(true),
        Some(upsert("/work/a", "ABC")),
        None,
        None,
    )
    .await;

    let pushed = loop {
        let pushed = next_settings_changed(&mut second).await;
        if pushed.jira_connections.contains_key("/work/a") {
            break pushed;
        }
    };
    assert!(pushed.jira_enabled);
    assert_eq!(pushed.jira_connections["/work/a"], connection("ABC"));
}

/// Upsert, then removal, for one project.
#[tokio::test]
async fn an_upsert_and_a_removal_are_applied_and_stored() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, _) = connect_with_settings(&state).await;

    set(
        &mut client,
        1,
        None,
        Some(upsert("/work/a", "ABC")),
        None,
        None,
    )
    .await;
    assert_eq!(
        stored(store.path()).jira_connections["/work/a"],
        connection("ABC")
    );

    set(&mut client, 2, None, Some(removal("/work/a")), None, None).await;
    assert!(stored(store.path()).jira_connections.is_empty());
    let (_next, welcomed) = connect_with_settings(&state).await;
    assert!(welcomed.jira_connections.is_empty());
}

/// Two projects' changes back to back both survive: no whole-map overwrite.
#[tokio::test]
async fn two_projects_changes_back_to_back_both_survive() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut first, _) = connect_with_settings(&state).await;
    let (mut second, _) = connect_with_settings(&state).await;

    set(
        &mut first,
        1,
        None,
        Some(upsert("/work/a", "AAA")),
        None,
        None,
    )
    .await;
    set(
        &mut second,
        1,
        None,
        Some(upsert("/work/b", "BBB")),
        None,
        None,
    )
    .await;

    let on_disk = stored(store.path()).jira_connections;
    assert_eq!(on_disk.len(), 2, "{on_disk:?}");
    assert_eq!(on_disk["/work/a"], connection("AAA"));
    assert_eq!(on_disk["/work/b"], connection("BBB"));
}

/// A save of an unrelated field keeps both Jira fields.
#[tokio::test]
async fn an_unrelated_save_keeps_the_jira_fields() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, _) = connect_with_settings(&state).await;

    set(
        &mut client,
        1,
        Some(true),
        Some(upsert("/work/a", "ABC")),
        None,
        None,
    )
    .await;
    set(&mut client, 2, None, None, None, Some(4321)).await;

    let settings = stored(store.path());
    assert_eq!(settings.scrollback_lines, 4321);
    assert!(settings.jira_enabled);
    assert_eq!(settings.jira_connections["/work/a"], connection("ABC"));
}
