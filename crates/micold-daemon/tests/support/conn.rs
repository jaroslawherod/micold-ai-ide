//! Shared by the daemon tests that drive `serve_connection` over an in-memory duplex: open a
//! connection, do the `Hello`/`Welcome` handshake and, optionally, attach to a project. A test that
//! varies the `Hello` itself (fingerprint, version mismatch) builds its own.
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::attention::NotificationKinds;
use micold_core::mcp::policy::CrossSessionAccess;
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::OperationResult;
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg, DaemonSettings};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::AiCli;
use micold_core::settings::{
    DiffLayout, JiraBaseUrlChange, JiraConnectionChange, JsonFileSettingsStore, Settings,
    SettingsStore,
};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

/// The fields of a `ClientMsg::SettingsSet` other than `req`; `None` leaves a setting as it is.
/// Name the ones a test changes and take the rest from `Default`: `settings_set(1,
/// SettingsPatch { diff_layout: Some(l), ..Default::default() })`.
#[derive(Default)]
pub struct SettingsPatch {
    pub scrollback_lines: Option<usize>,
    pub env_include_enabled: Option<bool>,
    pub env_include_script_path: Option<String>,
    pub env_include_timeout_secs: Option<u64>,
    pub default_ai_cli: Option<AiCli>,
    pub pi_activity_component: Option<bool>,
    pub save_terminal_history: Option<bool>,
    pub tool_server_enabled: Option<bool>,
    pub cross_session_access: Option<CrossSessionAccess>,
    pub pr_status_enabled: Option<bool>,
    pub jira_enabled: Option<bool>,
    pub jira_connection: Option<micold_core::settings::JiraConnectionChange>,
    pub github_issues_enabled: Option<bool>,
    pub jira_base_url: Option<micold_core::settings::JiraBaseUrlChange>,
    pub desktop_notifications: Option<bool>,
    pub notification_kinds: Option<NotificationKinds>,
    pub long_task_threshold_secs: Option<u64>,
    pub diff_layout: Option<DiffLayout>,
    pub plan_usage_enabled: Option<bool>,
    pub plan_usage_warn_percent: Option<u8>,
}

/// The `SettingsSet` request `req` that carries `patch`.
pub fn settings_set(req: u64, patch: SettingsPatch) -> ClientMsg {
    let SettingsPatch {
        scrollback_lines,
        env_include_enabled,
        env_include_script_path,
        env_include_timeout_secs,
        default_ai_cli,
        pi_activity_component,
        save_terminal_history,
        tool_server_enabled,
        cross_session_access,
        pr_status_enabled,
        jira_enabled,
        jira_connection,
        github_issues_enabled,
        jira_base_url,
        desktop_notifications,
        notification_kinds,
        long_task_threshold_secs,
        diff_layout,
        plan_usage_enabled,
        plan_usage_warn_percent,
    } = patch;
    ClientMsg::SettingsSet {
        req,
        scrollback_lines,
        env_include_enabled,
        env_include_script_path,
        env_include_timeout_secs,
        default_ai_cli,
        pi_activity_component,
        save_terminal_history,
        tool_server_enabled,
        cross_session_access,
        pr_status_enabled,
        jira_enabled,
        jira_connection,
        github_issues_enabled,
        jira_base_url,
        desktop_notifications,
        notification_kinds,
        long_task_threshold_secs,
        diff_layout,
        plan_usage_enabled,
        plan_usage_warn_percent,
    }
}

/// The client end of a connection to the service.
pub type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// How long a connection waits for the `Welcome` the service owes it.
const WELCOME_WITHIN: Duration = Duration::from_secs(10);

async fn handshake(
    state: &Arc<DaemonState>,
    build: &str,
    instance: ClientInstance,
) -> (Client, DaemonMsg) {
    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: build.into(),
            client_instance: instance,
            client_package_version: PACKAGE_VERSION.into(),
            // The host-process placement presents no token, and a fingerprint mismatch is not a
            // refusal there. `BUILD_FINGERPRINT` because these tests compile against the same core
            // as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .expect("the hello is sent");
    let frame = tokio::time::timeout(WELCOME_WITHIN, client.next())
        .await
        .expect("the service answers the hello in time");
    match frame {
        Some(Ok(Frame::Control(welcome @ DaemonMsg::Welcome { .. }))) => (client, welcome),
        other => panic!("expected Welcome, got {other:?}"),
    }
}

/// Connect as build `"test"` and take the `Welcome`.
pub async fn connect(state: &Arc<DaemonState>) -> Client {
    connect_as(state, "test").await
}

/// As [`connect`], reporting `build` as the client build.
pub async fn connect_as(state: &Arc<DaemonState>, build: &str) -> Client {
    handshake(state, build, ClientInstance::current()).await.0
}

/// As [`connect`], also returning the settings the `Welcome` reported.
pub async fn connect_with_settings(state: &Arc<DaemonState>) -> (Client, DaemonSettings) {
    match handshake(state, "test", ClientInstance::current()).await {
        (client, DaemonMsg::Welcome { settings, .. }) => (client, settings),
        (_, other) => unreachable!("handshake returns a Welcome, not {other:?}"),
    }
}

/// Connect a window of build `build` that is its own client instance (pid 0, nonce `build`), so
/// several windows of one test are told apart. Returns it with the `Welcome` itself.
pub async fn connect_with_welcome(state: &Arc<DaemonState>, build: &str) -> (Client, DaemonMsg) {
    let instance = ClientInstance {
        pid: 0,
        nonce: build.into(),
    };
    handshake(state, build, instance).await
}

/// Handshake, then attach `project`, draining the `Attached` and `CatalogChanged` it produces.
pub async fn connect_and_attach(state: &Arc<DaemonState>, project: &Path) -> Client {
    let mut client = connect(state).await;
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }))
        .await
        .expect("the attach is sent");
    let mut attached = false;
    loop {
        match client.next().await.expect("stream open").expect("a frame") {
            Frame::Control(DaemonMsg::Attached { .. }) => attached = true,
            Frame::Control(DaemonMsg::CatalogChanged { .. }) if attached => return client,
            Frame::Control(_) | Frame::Grid(_) => {}
        }
    }
}

/// The `OperationOk`/`OperationError` answering `req`, within 20 s.
pub async fn answer(client: &mut Client, req: u64) -> DaemonMsg {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(
                    m @ (DaemonMsg::OperationError { req: r, .. }
                    | DaemonMsg::OperationOk { req: r, .. }),
                ) if r == req => return m,
                _ => continue,
            }
        }
    })
    .await
    .expect("the request is answered")
}

/// A service whose catalog and settings live in `store_dir`. Called twice over one directory, the
/// second service is the first one restarted.
pub fn service(store_dir: &Path) -> Arc<DaemonState> {
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
pub async fn set(
    client: &mut Client,
    req: u64,
    jira_enabled: Option<bool>,
    jira_connection: Option<JiraConnectionChange>,
    jira_base_url: Option<JiraBaseUrlChange>,
    scrollback_lines: Option<usize>,
) {
    let patch = SettingsPatch {
        jira_enabled,
        jira_connection,
        jira_base_url,
        scrollback_lines,
        ..SettingsPatch::default()
    };
    client
        .send(Frame::Control(settings_set(req, patch)))
        .await
        .unwrap();
    match answer(client, req).await {
        DaemonMsg::OperationOk {
            result: OperationResult::Ack,
            ..
        } => {}
        other => panic!("the service refused the settings: {other:?}"),
    }
}

/// The next `SettingsChanged` this client is pushed.
pub async fn next_settings_changed(client: &mut Client) -> DaemonSettings {
    let pushed = async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(DaemonMsg::SettingsChanged { settings }) => return settings,
                _ => continue,
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(5), pushed)
        .await
        .expect("the client was never told the settings changed")
}

/// Wait until the service counts two clients, so a push cannot be sent before the second one is
/// there to receive it.
pub async fn both_registered(state: &Arc<DaemonState>) {
    let registered = async {
        while state.client_count() != 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(5), registered)
        .await
        .expect("two clients are registered");
}

/// The settings document stored in `dir`.
pub fn stored(dir: &Path) -> Settings {
    JsonFileSettingsStore::at(dir.join("settings.json"))
        .load()
        .settings
}
