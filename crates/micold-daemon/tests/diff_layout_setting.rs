//! The Changes view's diff layout on the service (feature 482, R12, contracts/review-wire.md):
//! `SettingsSet { diff_layout }` is stored in the settings file, reported in `Welcome` after a
//! restart and pushed to every attached client, as `pr_status_enabled` is. The review messages
//! are refused until the review store serves them (T057 retires the `ReviewEdit` assertion, T072
//! the `ReviewSend` one).

use std::path::Path;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, DaemonSettings, ErrorKind, OperationResult, ReviewEditOp,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::{DiffLayout, JsonFileSettingsStore};
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

/// A `SettingsSet` that names only the diff layout.
fn set_layout(req: u64, layout: DiffLayout) -> ClientMsg {
    ClientMsg::SettingsSet {
        req,
        scrollback_lines: None,
        env_include_enabled: None,
        env_include_script_path: None,
        env_include_timeout_secs: None,
        default_ai_cli: None,
        pi_activity_component: None,
        tool_server_enabled: None,
        cross_session_access: None,
        pr_status_enabled: None,
        desktop_notifications: None,
        diff_layout: Some(layout),
    }
}

/// The answer to request `req`: `Ok(())` for an `Ack`, else the error's kind and message.
async fn answer(client: &mut Client, req: u64) -> Result<(), (ErrorKind, String)> {
    let answered = async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(DaemonMsg::OperationOk {
                    req: r,
                    result: OperationResult::Ack,
                }) if r == req => return Ok(()),
                Frame::Control(DaemonMsg::OperationError {
                    req: r,
                    kind,
                    message,
                    ..
                }) if r == req => return Err((kind, message)),
                _ => continue,
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), answered)
        .await
        .expect("the service answers the request")
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

/// R12. The layout is Unified until changed; once set it is in the settings file, so a restarted
/// service reports it in `Welcome`.
#[tokio::test]
async fn setting_the_diff_layout_is_persisted_and_reported_in_welcome_after_a_restart() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, welcomed) = connect(&state).await;
    assert_eq!(welcomed.diff_layout, DiffLayout::Unified, "unified until changed");

    client
        .send(Frame::Control(set_layout(1, DiffLayout::SideBySide)))
        .await
        .unwrap();
    answer(&mut client, 1).await.expect("the layout is stored");

    let settings = std::fs::read_to_string(store.path().join("settings.json")).unwrap();
    assert!(
        settings.contains("side_by_side"),
        "the layout is in the settings file: {settings}"
    );
    let restarted = service(store.path());
    let (_after_restart, welcomed) = connect(&restarted).await;
    assert_eq!(
        welcomed.diff_layout,
        DiffLayout::SideBySide,
        "a restarted service reports the stored layout"
    );
}

/// R12. Every open window follows the layout: a second attached client is pushed it.
#[tokio::test]
async fn setting_the_diff_layout_is_pushed_to_a_second_client() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut a, _) = connect(&state).await;
    let (mut b, _) = connect(&state).await;
    both_registered(&state).await;

    a.send(Frame::Control(set_layout(1, DiffLayout::SideBySide)))
        .await
        .unwrap();

    assert_eq!(
        next_settings_changed(&mut b).await.diff_layout,
        DiffLayout::SideBySide,
        "the window that did nothing is pushed the layout"
    );
}

/// Placeholder until the review store: both review messages are refused, not dropped.
#[tokio::test]
async fn review_edit_and_review_send_are_refused_until_served() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, _) = connect(&state).await;

    // Retired by T057.
    client
        .send(Frame::Control(ClientMsg::ReviewEdit {
            req: 1,
            project: store.path().to_path_buf(),
            worktree_dir: String::new(),
            edit: ReviewEditOp::ClearSent,
        }))
        .await
        .unwrap();
    let (kind, message) = answer(&mut client, 1).await.expect_err("ReviewEdit is refused");
    assert_eq!(kind, ErrorKind::Refused);
    assert!(message.contains("not available"), "{message}");

    // Retired by T072.
    client
        .send(Frame::Control(ClientMsg::ReviewSend {
            req: 2,
            project: store.path().to_path_buf(),
            worktree_dir: String::new(),
            outdated: Vec::new(),
        }))
        .await
        .unwrap();
    let (kind, _) = answer(&mut client, 2).await.expect_err("ReviewSend is refused");
    assert_eq!(kind, ErrorKind::Refused);
}
