//! The worktree guard opt-out on the service (feature 1210, FR-008): `SettingsSet {
//! worktree_guard_enabled }` is stored in the settings file and reported in `Welcome`, as
//! `tool_server_enabled` is. On until the user turns it off.

use crate::conn;

use conn::{connect_with_settings, service, settings_set, SettingsPatch};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{DaemonMsg, OperationResult};

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
