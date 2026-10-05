//! Feature 010, BUG-592 (T164, SC-027): a refused shell open must reach the client that asked.
//!
//! BUG-012 made the daemon refuse to start anything in a working directory that no longer exists
//! (FR-006c), and `session_cwd_guard.rs` proves the refusal. What nothing asked was whether the
//! client hears about it. `ClientMsg::SessionOpenShell` is fire-and-forget: the `Err` arm logged at
//! `warn` and sent nothing, while the client had already switched to the new instance and asked to
//! attach it. The Terminal toggle on a session whose worktree was deleted did nothing on screen,
//! every time, with the only trace in the daemon log.
//!
//! So this drives the messages through a real connection: the defect is in what the daemon does
//! with the result, which a test reading the state directly would not see.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ShellOpenFailure};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, ShellInstanceId, TerminalMode,
};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use uuid::Uuid;

fn session_id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x0592))
}

/// A catalog holding one Regular-mode session at `location` within `project_dir`.
fn catalog_with_session_at(
    project_dir: &Path,
    store_dir: &Path,
    location: SessionLocation,
) -> Catalog {
    // Environment-include off: the shell needs nothing from the developer's `~/.bashrc`, and
    // sourcing it only makes the spawn slower and machine-dependent.
    JsonFileSettingsStore::at(store_dir.join("settings.json"))
        .save(&Settings {
            env_include_enabled: false,
            ..Settings::default()
        })
        .unwrap();
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_dir.to_path_buf(),
        vec![Session::restored(
            session_id(),
            location,
            SessionLabel::Named("Shell".into()),
            TerminalMode::Regular,
            AiCli::ClaudeCode,
        )],
    );
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(
                project_dir.to_path_buf(),
                false,
                Availability::Available,
            )],
            active: Some(project_dir.to_path_buf()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &std::sync::Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(state),
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
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Send `request`, then a `Ping`, and collect every control message up to its `Pong`.
///
/// The connection handles its messages in order, so whatever the daemon says about `request` it
/// has said before the `Pong`: an empty answer is a real "nothing was sent", not a race. Ten
/// seconds is a timeout, not a measurement.
async fn answers_to(client: &mut Client, request: ClientMsg) -> Vec<DaemonMsg> {
    const NONCE: u64 = 0x0592;
    client.send(Frame::Control(request)).await.unwrap();
    client
        .send(Frame::Control(ClientMsg::Ping { nonce: NONCE }))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut seen = Vec::new();
        loop {
            match client
                .next()
                .await
                .expect("the connection stays open")
                .unwrap()
            {
                Frame::Control(DaemonMsg::Pong { nonce }) if nonce == NONCE => return seen,
                Frame::Control(msg) => seen.push(msg),
                Frame::Grid(_) => {}
            }
        }
    })
    .await
    .expect("the daemon answers the ping")
}

fn refusals(answers: &[DaemonMsg]) -> Vec<(SessionId, ShellInstanceId, ShellOpenFailure)> {
    answers
        .iter()
        .filter_map(|m| match m {
            DaemonMsg::ShellOpenFailed {
                session,
                instance,
                reason,
            } => Some((*session, *instance, reason.clone())),
            _ => None,
        })
        .collect()
}

fn live_shells(state: &DaemonState) -> Vec<ShellInstanceId> {
    state
        .catalog_snapshot()
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id == session_id())
        .map(|s| s.live_shells.clone())
        .unwrap_or_default()
}

/// The reported path: a worktree deleted from outside the app, then the Terminal toggle (or "+").
#[tokio::test]
async fn a_shell_open_into_a_missing_worktree_is_reported_to_the_client() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_session_at(
        project.path(),
        store.path(),
        // `SessionLocation::cwd` joins this under `.claude/worktrees`; it is never created.
        SessionLocation::Worktree("deleted-outside-the-app".into()),
    )));
    let mut client = connect(&state).await;
    let instance = ShellInstanceId(1);

    let answers = answers_to(
        &mut client,
        ClientMsg::SessionOpenShell {
            session: session_id(),
            instance,
        },
    )
    .await;

    assert_eq!(
        refusals(&answers),
        vec![(session_id(), instance, ShellOpenFailure::WorkingDirMissing)],
        "the client that asked must be told the open was refused, and why — before this the daemon \
         logged it at `warn` and told nobody, so the toggle did nothing on screen. Got: {answers:?}"
    );
    assert!(
        live_shells(&state).is_empty() && state.live_session(session_id()).is_none(),
        "and the refusal is not covering for a process that was registered anyway (FR-006c)"
    );
}

/// The same refusal on the restart path (review F1): `SessionRestartShell` respawns through the
/// same `open_shell`, and its `Err` arm was as silent.
#[tokio::test]
async fn a_shell_restart_into_a_missing_worktree_is_reported_to_the_client() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_session_at(
        project.path(),
        store.path(),
        SessionLocation::Worktree("deleted-outside-the-app".into()),
    )));
    let mut client = connect(&state).await;
    let instance = ShellInstanceId(2);

    let answers = answers_to(
        &mut client,
        ClientMsg::SessionRestartShell {
            session: session_id(),
            instance,
        },
    )
    .await;

    assert_eq!(
        refusals(&answers),
        vec![(session_id(), instance, ShellOpenFailure::WorkingDirMissing)],
        "a refused restart must reach the client too. Got: {answers:?}"
    );
    assert!(live_shells(&state).is_empty());
}

/// The other half: only a refusal is reported. A session whose directory exists opens its shell
/// and hears nothing of the kind, so the fix cannot be satisfied by reporting every open.
#[tokio::test]
async fn a_shell_open_in_an_existing_directory_opens_and_reports_nothing() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_session_at(
        project.path(),
        store.path(),
        SessionLocation::Default,
    )));
    let mut client = connect(&state).await;
    let instance = ShellInstanceId(1);

    let answers = answers_to(
        &mut client,
        ClientMsg::SessionOpenShell {
            session: session_id(),
            instance,
        },
    )
    .await;

    assert!(
        refusals(&answers).is_empty(),
        "an open that succeeded must not be reported as refused. Got: {answers:?}"
    );
    assert_eq!(live_shells(&state), vec![instance], "the instance opened");
    let _ = state.close_shell(session_id(), instance);
}
