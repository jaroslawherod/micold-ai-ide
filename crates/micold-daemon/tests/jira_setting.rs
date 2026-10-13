//! The Jira switch and connections on the service (feature 533, contracts/jira-connection.md §1):
//! `SettingsSet { jira_enabled, jira_connection }` is stored in the settings file and pushed to
//! every connected client as `SettingsChanged`. The service stores them and never reads them
//! (FR-021); two projects' changes arriving back to back both survive (FR-022).
//!
//! Drives `server::serve_connection` over an in-memory duplex against a settings file in a
//! temporary directory, so "persisted" is checked by starting a second service over that file.

#[path = "support/conn.rs"]
mod conn;

use conn::{connect_with_settings, Client};
use std::path::Path;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::jira::JiraConnection;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, DaemonSettings, OperationResult};
use micold_core::settings::{
    JiraChange, JiraConnectionChange, JsonFileSettingsStore, SettingsStore,
};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

/// A service whose catalog and settings live in `store_dir`. Called twice over one directory, the
/// second service is the first one restarted.
fn service(store_dir: &Path) -> Arc<DaemonState> {
    let projects_path = store_dir.join("projects.json");
    if !projects_path.exists() {
        JsonFileStore::at(projects_path.clone())
            .save(&Workspace::default())
            .unwrap();
    }
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )))
}

/// Send a `SettingsSet` that names the Jira fields and the scrollback and nothing else, and wait
/// until the service acknowledges it.
async fn set(
    client: &mut Client,
    req: u64,
    jira_enabled: Option<bool>,
    jira_connection: Option<JiraConnectionChange>,
    scrollback: Option<usize>,
) {
    client
        .send(Frame::Control(ClientMsg::SettingsSet {
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req,
            scrollback_lines: scrollback,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: None,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: None,
            jira_enabled,
            jira_connection,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: None,
        }))
        .await
        .unwrap();
    let acknowledged = async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(DaemonMsg::OperationOk {
                    req: r,
                    result: OperationResult::Ack,
                }) if r == req => return,
                Frame::Control(DaemonMsg::OperationError {
                    req: r, message, ..
                }) if r == req => {
                    panic!("the service refused the settings: {message}")
                }
                _ => continue,
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), acknowledged)
        .await
        .expect("the service acknowledges the settings");
}

/// The next `SettingsChanged` this client is pushed.
async fn next_settings_changed(client: &mut Client) -> DaemonSettings {
    let pushed = async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(DaemonMsg::SettingsChanged { settings }) => return settings,
                _ => continue,
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), pushed)
        .await
        .expect("the client was never told the settings changed")
}

/// Wait until the service counts two clients, so a push cannot be sent before the second one is
/// there to receive it.
async fn both_registered(state: &Arc<DaemonState>) {
    let registered = async {
        while state.client_count() != 2 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), registered)
        .await
        .expect("two clients are registered");
}

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

fn stored(dir: &Path) -> micold_core::settings::Settings {
    JsonFileSettingsStore::at(dir.join("settings.json"))
        .load()
        .settings
}

/// FR-002: off until turned on; once on, the service holds it, across a restart.
#[tokio::test]
async fn the_switch_is_stored_and_reported_to_the_next_client() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, welcomed) = connect_with_settings(&state).await;
    assert!(!welcomed.jira_enabled, "off until the user turns it on");
    assert!(welcomed.jira_connections.is_empty());

    set(&mut client, 1, Some(true), None, None).await;

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

    set(&mut client, 1, None, Some(upsert("/work/a", "ABC")), None).await;
    assert_eq!(
        stored(store.path()).jira_connections["/work/a"],
        connection("ABC")
    );

    set(&mut client, 2, None, Some(removal("/work/a")), None).await;
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

    set(&mut first, 1, None, Some(upsert("/work/a", "AAA")), None).await;
    set(&mut second, 1, None, Some(upsert("/work/b", "BBB")), None).await;

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
    )
    .await;
    set(&mut client, 2, None, None, Some(4321)).await;

    let settings = stored(store.path());
    assert_eq!(settings.scrollback_lines, 4321);
    assert!(settings.jira_enabled);
    assert_eq!(settings.jira_connections["/work/a"], connection("ABC"));
}
