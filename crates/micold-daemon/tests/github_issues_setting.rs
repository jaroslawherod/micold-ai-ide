//! The GitHub issues switch on the service (feature 1102, FR-001, FR-007, FR-008, contracts/
//! wire.md W2): on by default, `SettingsSet { github_issues_enabled }` is stored in the settings
//! file and pushed to every connected client, the last write wins, and a restart keeps it.

#[path = "support/conn.rs"]
mod conn;

use conn::{connect_with_settings, settings_set, Client, SettingsPatch};
use std::path::Path;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{DaemonMsg, DaemonSettings, OperationResult};
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

/// A service whose catalog and settings live in `store_dir`; called twice over one directory, the
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

/// Send `SettingsSet { github_issues_enabled }` and wait for the acknowledgement.
async fn set(client: &mut Client, req: u64, on: Option<bool>) {
    client
        .send(Frame::Control(settings_set(
            req,
            SettingsPatch {
                github_issues_enabled: on,
                ..SettingsPatch::default()
            },
        )))
        .await
        .unwrap();
    let acknowledged = async {
        loop {
            if let Frame::Control(DaemonMsg::OperationOk {
                req: r,
                result: OperationResult::Ack,
            }) = client.next().await.expect("stream open").unwrap()
            {
                if r == req {
                    return;
                }
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), acknowledged)
        .await
        .expect("the service acknowledges the settings");
}

async fn next_settings_changed(client: &mut Client) -> DaemonSettings {
    let pushed = async {
        loop {
            if let Frame::Control(DaemonMsg::SettingsChanged { settings }) =
                client.next().await.expect("stream open").unwrap()
            {
                return settings;
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), pushed)
        .await
        .expect("the client was never told the settings changed")
}

fn stored(dir: &Path) -> micold_core::settings::Settings {
    JsonFileSettingsStore::at(dir.join("settings.json"))
        .load()
        .settings
}

/// FR-007: on by default; FR-001/FR-008: once off, the service holds it across a restart.
#[tokio::test]
async fn the_switch_is_on_by_default_and_off_is_stored_and_survives_a_restart() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, welcomed) = connect_with_settings(&state).await;
    assert!(
        welcomed.github_issues_enabled,
        "on until the user turns it off"
    );
    assert!(state.github_issues_enabled());

    set(&mut client, 1, Some(false)).await;

    assert!(!state.github_issues_enabled());
    assert!(!stored(store.path()).github_issues_enabled);
    let (_next, welcomed) = connect_with_settings(&state).await;
    assert!(!welcomed.github_issues_enabled);
    let restarted = service(store.path());
    let (_again, welcomed) = connect_with_settings(&restarted).await;
    assert!(
        !welcomed.github_issues_enabled,
        "the switch is in the settings file"
    );
}

/// FR-008: a second connected client is pushed the change, and the last write wins.
#[tokio::test]
async fn a_change_is_echoed_to_a_second_client_and_the_last_write_wins() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut first, _) = connect_with_settings(&state).await;
    let (mut second, _) = connect_with_settings(&state).await;
    let registered = async {
        while state.client_count() != 2 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), registered)
        .await
        .expect("two clients are registered");

    set(&mut first, 1, Some(false)).await;
    assert!(
        !next_settings_changed(&mut second)
            .await
            .github_issues_enabled
    );

    set(&mut second, 1, Some(true)).await;
    set(&mut first, 2, Some(false)).await;
    assert!(
        !stored(store.path()).github_issues_enabled,
        "the later write is the value"
    );
    set(&mut second, 2, None).await;
    assert!(!state.github_issues_enabled(), "None leaves it unchanged");
}

/// An unrelated save keeps the switch (the service-write copy carries it).
#[tokio::test]
async fn an_unrelated_save_keeps_the_switch() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, _) = connect_with_settings(&state).await;
    set(&mut client, 1, Some(false)).await;
    client
        .send(Frame::Control(settings_set(
            2,
            SettingsPatch {
                scrollback_lines: Some(4321),
                ..SettingsPatch::default()
            },
        )))
        .await
        .unwrap();
    // the ack of req 2 ends the wait
    let acknowledged = async {
        loop {
            if let Frame::Control(DaemonMsg::OperationOk { req: 2, .. }) =
                client.next().await.expect("stream open").unwrap()
            {
                return;
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), acknowledged)
        .await
        .unwrap();
    let settings = stored(store.path());
    assert_eq!(settings.scrollback_lines, 4321);
    assert!(!settings.github_issues_enabled);
}
