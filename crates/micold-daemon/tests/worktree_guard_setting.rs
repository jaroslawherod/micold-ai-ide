//! The worktree guard opt-out on the service (feature 1210, FR-008): `SettingsSet {
//! worktree_guard_enabled }` is stored in the settings file and reported in `Welcome`, as
//! `tool_server_enabled` is. On until the user turns it off.

#[path = "support/conn.rs"]
mod conn;

use conn::{connect_with_settings, settings_set, SettingsPatch};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{DaemonMsg, OperationResult};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

fn service(store_dir: &std::path::Path) -> Arc<DaemonState> {
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

#[tokio::test]
async fn turning_the_guard_off_is_persisted_and_reported_in_the_next_welcome() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, welcomed) = connect_with_settings(&state).await;
    assert!(
        welcomed.worktree_guard_enabled,
        "the guard is on by default"
    );

    client
        .send(Frame::Control(settings_set(
            1,
            SettingsPatch {
                worktree_guard_enabled: Some(false),
                ..Default::default()
            },
        )))
        .await
        .unwrap();
    let acknowledged = async {
        loop {
            if let Frame::Control(DaemonMsg::OperationOk {
                req: 1,
                result: OperationResult::Ack,
            }) = client.next().await.expect("stream open").unwrap()
            {
                return;
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), acknowledged)
        .await
        .expect("the service acknowledges the settings");

    let restarted = service(store.path());
    let (_c, welcomed) = connect_with_settings(&restarted).await;
    assert!(
        !welcomed.worktree_guard_enabled,
        "the setting is in the settings file: a restarted service still has it off"
    );
}
