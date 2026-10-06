//! Feature 011, BUG-442 (FR-007(b)): a session's manual restart re-sources the environment-include
//! script for that session's directory, in the service.
//!
//! Since feature 010 the service spawns every session process from its own per-directory cache.
//! A Settings save drops that cache (trigger (a)); the restart controls did not, so a restarted
//! session got the environment from before whatever the user changed in their rc files, and the
//! documented recovery path ("fix the script, then restart the session") did nothing.
//!
//! Driven through a real connection, because the defect is in what the service does with each
//! message: a restart and a plain start or open must be told apart there. The include script
//! appends one line to a log per run, so the number of runs is what the tests read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, WireLifecycle};
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

/// How long a wait for the service to reach a state may take. It bounds a failure, not a pass.
const PATIENCE: Duration = Duration::from_secs(30);

/// The AI CLI the AI-tab tests start. Its stand-in is put on the session's `PATH` by the include
/// script itself, so nothing here touches the test process's own `PATH`.
const CLI: AiCli = AiCli::ClaudeCode;

fn session_id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x0442))
}

/// The include script, its run log, and the directory it puts on `PATH`.
struct Include {
    script: PathBuf,
    /// One line per run of the script.
    runs: PathBuf,
    _bin: tempfile::TempDir,
}

/// An include script that logs each run and puts a directory holding a stand-in for [`CLI`] in
/// front of `PATH`. The stand-in stays running until it is stopped, as a real CLI does.
fn include_script(dir: &Path) -> Include {
    let runs = dir.join("runs");
    let bin = tempfile::tempdir().unwrap();
    write_cli_stand_in(bin.path(), CLI.provider().command());
    let (name, body) = if cfg!(windows) {
        (
            "env-include.ps1",
            format!(
                "Add-Content -Path '{runs}' -Value run\r\n\
                 $env:PATH = '{bin};' + $env:PATH\r\n",
                runs = runs.display(),
                bin = bin.path().display(),
            ),
        )
    } else {
        (
            "env-include.sh",
            format!(
                "echo run >> '{runs}'\n\
                 export PATH=\"{bin}:$PATH\"\n",
                runs = runs.display(),
                bin = bin.path().display(),
            ),
        )
    };
    let script = dir.join(name);
    std::fs::write(&script, body).unwrap();
    Include {
        script,
        runs,
        _bin: bin,
    }
}

#[cfg(unix)]
fn write_cli_stand_in(dir: &Path, command: &str) {
    use std::os::unix::fs::PermissionsExt;
    let stub = dir.join(command);
    std::fs::write(&stub, "#!/bin/sh\nexec sleep 600\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(windows)]
fn write_cli_stand_in(dir: &Path, command: &str) {
    std::fs::write(
        dir.join(format!("{command}.cmd")),
        "@echo off\r\nping -n 600 127.0.0.1 >nul\r\n",
    )
    .unwrap();
}

/// A service with environment-include on and `script` configured, holding one session in `mode`
/// at the root of `project_dir`.
fn service(project_dir: &Path, store: &Path, script: &Path, mode: TerminalMode) -> Arc<DaemonState> {
    JsonFileSettingsStore::at(store.join("settings.json"))
        .save(&Settings {
            env_include_enabled: true,
            env_include_script_path: script.to_string_lossy().into_owned(),
            env_include_timeout_secs: PATIENCE.as_secs(),
            ..Settings::default()
        })
        .unwrap();
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_dir.to_path_buf(),
        vec![Session::restored(
            session_id(),
            SessionLocation::Default,
            SessionLabel::Named("Restart me".into()),
            mode,
            CLI,
        )],
    );
    let projects_path = store.join("projects.json");
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
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

fn runs(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &Arc<DaemonState>) -> Client {
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
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Send `request`, then a `Ping`, and read everything up to its `Pong`: the connection handles its
/// messages in order, so whatever `request` does on the connection loop is done by then.
async fn send_and_settle(client: &mut Client, request: ClientMsg) {
    const NONCE: u64 = 0x0442;
    client.send(Frame::Control(request)).await.unwrap();
    client
        .send(Frame::Control(ClientMsg::Ping { nonce: NONCE }))
        .await
        .unwrap();
    tokio::time::timeout(PATIENCE, async {
        loop {
            match client
                .next()
                .await
                .expect("the connection stays open")
                .unwrap()
            {
                Frame::Control(DaemonMsg::Pong { nonce }) if nonce == NONCE => return,
                _ => {}
            }
        }
    })
    .await
    .expect("the service answers the ping");
}

/// Waits, without blocking the runtime, until `cond` holds.
async fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !cond() {
        assert!(Instant::now() < deadline, "fixture check: {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn lifecycle(state: &DaemonState, project: &Path) -> Option<WireLifecycle> {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == session_id())
        .map(|s| s.lifecycle)
}

/// T041 Case 1: a Regular Terminal instance's restart control re-runs the script.
#[tokio::test]
async fn restarting_a_shell_instance_re_sources_its_directory() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let include = include_script(store.path());
    let state = service(
        project.path(),
        store.path(),
        &include.script,
        TerminalMode::Regular,
    );
    let mut client = connect(&state).await;
    let instance = ShellInstanceId(1);

    send_and_settle(
        &mut client,
        ClientMsg::SessionOpenShell {
            session: session_id(),
            instance,
        },
    )
    .await;
    assert_eq!(runs(&include.runs), 1, "fixture check: the open ran the script");

    send_and_settle(
        &mut client,
        ClientMsg::SessionRestartShell {
            session: session_id(),
            instance,
        },
    )
    .await;
    assert_eq!(
        runs(&include.runs),
        2,
        "the user's manual restart of a terminal must re-source the script for its directory, so \
         a change to their rc files reaches the restarted shell; it was served the cached \
         environment from before (FR-007(b))"
    );
    let _ = state.close_shell(session_id(), instance);
}

/// T041 Case 2: opening a second instance is not a restart, and stays cached (FR-020).
#[tokio::test]
async fn opening_another_shell_instance_reuses_the_cached_environment() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let include = include_script(store.path());
    let state = service(
        project.path(),
        store.path(),
        &include.script,
        TerminalMode::Regular,
    );
    let mut client = connect(&state).await;

    for instance in [ShellInstanceId(1), ShellInstanceId(2)] {
        send_and_settle(
            &mut client,
            ClientMsg::SessionOpenShell {
                session: session_id(),
                instance,
            },
        )
        .await;
    }
    assert_eq!(
        runs(&include.runs),
        1,
        "only a restart re-sources: a second terminal in the same directory shares the first's \
         resolved environment (FR-020)"
    );
    for instance in [ShellInstanceId(1), ShellInstanceId(2)] {
        let _ = state.close_shell(session_id(), instance);
    }
}

/// Starts the AI-CLI session, waits until it is live, stops it, and waits until it is `Idle`: the
/// state a user's restart control acts on. Then sends `again` and waits until the session is live
/// once more.
async fn start_stop_then(again: ClientMsg) -> usize {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let include = include_script(store.path());
    let state = service(
        project.path(),
        store.path(),
        &include.script,
        TerminalMode::AiCli,
    );
    let mut client = connect(&state).await;

    send_and_settle(
        &mut client,
        ClientMsg::SessionStart {
            session: session_id(),
        },
    )
    .await;
    wait_until("the first start never went live", || {
        state.live_session(session_id()).is_some()
    })
    .await;
    assert_eq!(runs(&include.runs), 1, "fixture check: the start ran the script");

    send_and_settle(
        &mut client,
        ClientMsg::SessionStop {
            session: session_id(),
        },
    )
    .await;
    wait_until("the stop never settled Idle", || {
        state.live_session(session_id()).is_none()
            && matches!(lifecycle(&state, project.path()), Some(WireLifecycle::Idle))
    })
    .await;

    send_and_settle(&mut client, again).await;
    wait_until("the second start never went live", || {
        state.live_session(session_id()).is_some()
    })
    .await;
    let n = runs(&include.runs);
    for pty in state.remove_session(session_id()) {
        let _ = pty.kill();
    }
    n
}

/// T042: the AI CLI tab's restart control (`SessionRestart`) re-runs the script.
#[tokio::test]
async fn restarting_the_ai_cli_re_sources_its_directory() {
    let runs = start_stop_then(ClientMsg::SessionRestart {
        session: session_id(),
    })
    .await;
    assert_eq!(
        runs, 2,
        "the user's manual restart of the AI CLI must re-source the script for the session's \
         directory before it relaunches; it was served the cached environment (FR-007(b))"
    );
}

/// T042: a plain `SessionStart` — selecting a session, reconnecting — is not a restart, and stays
/// cached (FR-007's BUG-442 clarification).
#[tokio::test]
async fn a_plain_start_reuses_the_cached_environment() {
    let runs = start_stop_then(ClientMsg::SessionStart {
        session: session_id(),
    })
    .await;
    assert_eq!(
        runs, 1,
        "only the manual restart control re-sources: a passive start is served the cached \
         environment (FR-007, FR-020)"
    );
}
