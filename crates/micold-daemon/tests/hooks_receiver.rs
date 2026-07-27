//! US2 (T045) — the loopback activity-hook receiver end to end (contracts/hooks.md §Listener).
//!
//! Drives the real HTTP listener over TCP (no `claude` needed): a correctly-authenticated
//! `UserPromptSubmit` moves the session to `Working`; a wrong/missing token is a bare `403` that
//! changes nothing; a `SessionStart` is accepted without a transition. The parsing rules
//! (head/path/token/event) are unit-tested inside `hooks.rs`; this proves the wiring — bind → POST →
//! the session's projected activity signal.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::sync::Arc;

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::ActivitySignal;
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::hooks::HookReceiver;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

const SESSION_U128: u128 = 0xDEC0DE;

/// A catalog with one live-able session at a project root, and its `DaemonState`.
fn state_with_session(id: SessionId) -> (Arc<DaemonState>, tempfile::TempDir, tempfile::TempDir) {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project.path().to_path_buf(), vec![session]);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.path().to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.path().to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
    };
    let projects_path = store.path().join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    );
    let state = Arc::new(DaemonState::new(catalog));
    (state, project, store)
}

/// Register a `cat` PTY under the catalog id so the session is live (a target for `note_activity`).
fn register_cat(state: &DaemonState, id: SessionId) -> Arc<PtySession> {
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn cat");
    state.register_session(session)
}

fn activity(state: &DaemonState, id: SessionId) -> ActivitySignal {
    state
        .catalog_snapshot()
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id == id)
        .expect("session in snapshot")
        .activity
}

/// POST a hook body to the receiver and return the numeric HTTP status code.
async fn post_hook(addr: &str, session: Uuid, token: Option<&str>, body: &str) -> u16 {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    let auth = token
        .map(|t| format!("Authorization: Bearer {t}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /hook/{session} HTTP/1.1\r\nHost: localhost\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.expect("write");
    stream.flush().await.expect("flush");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.expect("read");
    let text = String::from_utf8_lossy(&response);
    // "HTTP/1.1 200 OK\r\n..." → 200
    text.split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .expect("a status code in the response line")
}

#[tokio::test]
async fn an_authenticated_hook_drives_activity() {
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let (state, _project, _store) = state_with_session(id);
    let cat = register_cat(&state, id);

    let (receiver, listener) = HookReceiver::bind(std::env::temp_dir().join("micold-hooks-test"))
        .await
        .expect("bind receiver");
    let token = receiver.token_for(id);
    let addr = listener.local_addr().unwrap().to_string();
    let tokens = receiver.tokens();
    tokio::spawn(micold_daemon::hooks::serve(
        listener,
        tokens,
        Arc::clone(&state),
    ));

    // Before any hook: Unknown (H1).
    assert_eq!(activity(&state, id), ActivitySignal::Unknown);

    // Correct token + UserPromptSubmit → 200 and Working.
    let status = post_hook(
        &addr,
        id.0,
        Some(&token),
        r#"{"hook_event_name":"UserPromptSubmit"}"#,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(activity(&state, id), ActivitySignal::Working);

    // Stop → AwaitingInput.
    let status = post_hook(&addr, id.0, Some(&token), r#"{"hook_event_name":"Stop"}"#).await;
    assert_eq!(status, 200);
    assert_eq!(activity(&state, id), ActivitySignal::AwaitingInput);

    cat.kill().expect("kill");
}

#[tokio::test]
async fn a_wrong_token_is_forbidden_and_changes_nothing() {
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let (state, _project, _store) = state_with_session(id);
    let cat = register_cat(&state, id);

    let (receiver, listener) = HookReceiver::bind(std::env::temp_dir().join("micold-hooks-test"))
        .await
        .expect("bind receiver");
    let _real_token = receiver.token_for(id);
    let addr = listener.local_addr().unwrap().to_string();
    let tokens = receiver.tokens();
    tokio::spawn(micold_daemon::hooks::serve(
        listener,
        tokens,
        Arc::clone(&state),
    ));

    // Wrong token → 403, no state change.
    let status = post_hook(
        &addr,
        id.0,
        Some("not-the-token"),
        r#"{"hook_event_name":"Stop"}"#,
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(activity(&state, id), ActivitySignal::Unknown);

    // Missing token → 403 too.
    let status = post_hook(&addr, id.0, None, r#"{"hook_event_name":"Stop"}"#).await;
    assert_eq!(status, 403);
    assert_eq!(activity(&state, id), ActivitySignal::Unknown);

    cat.kill().expect("kill");
}

#[tokio::test]
async fn session_start_is_accepted_without_a_transition() {
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let (state, _project, _store) = state_with_session(id);
    let cat = register_cat(&state, id);

    let (receiver, listener) = HookReceiver::bind(std::env::temp_dir().join("micold-hooks-test"))
        .await
        .expect("bind receiver");
    let token = receiver.token_for(id);
    let addr = listener.local_addr().unwrap().to_string();
    let tokens = receiver.tokens();
    tokio::spawn(micold_daemon::hooks::serve(
        listener,
        tokens,
        Arc::clone(&state),
    ));

    let status = post_hook(
        &addr,
        id.0,
        Some(&token),
        r#"{"hook_event_name":"SessionStart"}"#,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(activity(&state, id), ActivitySignal::Unknown);

    cat.kill().expect("kill");
}
