//! The pull request switch on the service (feature 040, contracts/reading-and-wire.md §4):
//! `SettingsSet { pr_status_enabled }` is stored in the settings file, reported in `Welcome`, and
//! pushed to every connected client, as `tool_server_enabled` is. Off until the user turns it on.
//!
//! Drives `server::serve_connection` over an in-memory duplex against a settings file in a
//! temporary directory, so "persisted" is checked by starting a second service over that file.

use std::path::Path;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, DaemonSettings, OperationResult};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

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

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// Connect and complete the handshake. Returns the client and the settings `Welcome` reported.
async fn connect(state: &Arc<DaemonState>) -> (Client, DaemonSettings) {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { settings, .. }) => (client, settings),
        other => panic!("expected Welcome, got {other:?}"),
    }
}

/// Send a `SettingsSet` that names the pull request switch and the scrollback and nothing else,
/// and wait until the service acknowledges it.
async fn set(
    client: &mut Client,
    req: u64,
    pr_status_enabled: Option<bool>,
    scrollback: Option<usize>,
) {
    client
        .send(Frame::Control(ClientMsg::SettingsSet {
            req,
            scrollback_lines: scrollback,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled,
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
                Frame::Control(DaemonMsg::OperationError { req: r, message, .. }) if r == req => {
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

/// U67 (FR-030). The switch is off until the user turns it on; once on, the service holds it — a
/// client that connects next is told so — and it is in the settings file, so the service still
/// holds it after a restart.
#[tokio::test]
async fn turning_pull_request_status_on_is_persisted_and_reported_in_the_next_welcome() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, welcomed) = connect(&state).await;
    assert!(
        !welcomed.pr_status_enabled,
        "pull request status is off until the user turns it on"
    );

    set(&mut client, 1, Some(true), None).await;

    let (_next, welcomed) = connect(&state).await;
    assert!(
        welcomed.pr_status_enabled,
        "the next client to connect is told the switch is on"
    );
    let restarted = service(store.path());
    let (_after_restart, welcomed) = connect(&restarted).await;
    assert!(
        welcomed.pr_status_enabled,
        "the switch is in the settings file: a restarted service still has it on"
    );
}
