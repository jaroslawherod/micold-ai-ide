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
        ..Default::default()
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
    post_hook_declaring(addr, session, token, body.len(), body).await
}

/// As [`post_hook`], but with `Content-Length` stated independently of the body actually sent — so an
/// over-bound request can be exercised without pushing megabytes through the socket (BUG-010).
async fn post_hook_declaring(
    addr: &str,
    session: Uuid,
    token: Option<&str>,
    content_length: usize,
    body: &str,
) -> u16 {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    let auth = token
        .map(|t| format!("Authorization: Bearer {t}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /hook/{session} HTTP/1.1\r\nHost: localhost\r\n{auth}Content-Length: {content_length}\r\nConnection: close\r\n\r\n{body}",
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

/// A `PostToolUse`/`PreToolUse` body the size `claude` really sends for an `Edit`: the payload embeds
/// the tool's whole input *and* response, so it carries the edited file's full contents twice over
/// (`tool_response.originalFile` plus `tool_input.old_string`/`new_string`). `filler` bytes stand in
/// for that file. Measured maximum across 3,332 real payloads from this project's own transcripts:
/// 97,087 bytes (BUG-010).
fn agent_sized_payload(event: &str, filler: usize) -> String {
    let file = "x".repeat(filler);
    serde_json::json!({
        "session_id": Uuid::from_u128(SESSION_U128).to_string(),
        "transcript_path": "/home/user/.claude/projects/-home-user-project/session.jsonl",
        "cwd": "/home/user/project",
        "permission_mode": "default",
        "hook_event_name": event,
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "/home/user/project/specs/tasks.md",
            "old_string": file,
            "new_string": file,
        },
        "tool_response": { "filePath": "/home/user/project/specs/tasks.md", "originalFile": file },
    })
    .to_string()
}

/// BUG-010 / SC-022 — a payload sized by the agent's own tool I/O is accepted and applied, and the
/// bound that exists to stop unbounded buffering still refuses a payload past it. Both halves matter:
/// raising the bound out of reach would satisfy the first and quietly abandon the second.
#[tokio::test]
async fn an_agent_sized_payload_is_accepted_and_an_over_bound_one_is_refused() {
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

    // The reported symptom: `PostToolUse` after editing a large file. 40 KiB of file contents lands
    // ~120 KB on the wire, past the 64 KiB bound that shipped and past the measured 97,087 B maximum.
    let post_tool_use = agent_sized_payload("PostToolUse", 40 * 1024);
    assert!(
        post_tool_use.len() > 97_087,
        "the payload must exceed the measured real-world maximum to be a regression test"
    );
    let status = post_hook(&addr, id.0, Some(&token), &post_tool_use).await;
    assert_eq!(
        status, 200,
        "an agent-sized PostToolUse must not be refused"
    );

    // And an agent-sized payload that *does* carry a transition must still drive the FSM — a lost
    // `PreToolUse` is signal loss, not just noise (contracts/hooks.md §State machine).
    assert_eq!(activity(&state, id), ActivitySignal::Unknown);
    let status = post_hook(
        &addr,
        id.0,
        Some(&token),
        &agent_sized_payload("PreToolUse", 40 * 1024),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(activity(&state, id), ActivitySignal::Working);

    // The bound is still a bound: a declared length past it is refused before anything is buffered,
    // and changes nothing.
    let status = post_hook_declaring(
        &addr,
        id.0,
        Some(&token),
        micold_daemon::hooks::MAX_BODY + 1,
        r#"{"hook_event_name":"Stop"}"#,
    )
    .await;
    assert_eq!(
        status, 413,
        "a payload past the bound must still be refused"
    );
    assert_eq!(
        activity(&state, id),
        ActivitySignal::Working,
        "a refused hook applies no transition (H1 holds under refusal)"
    );

    cat.kill().expect("kill");
}

/// BUG-010 — nothing is answered before authentication beyond `403` (contracts/hooks.md §Listener
/// rule 7). An oversized body from a caller with the wrong token must be indistinguishable from any
/// other unauthenticated request.
#[tokio::test]
async fn an_over_bound_body_from_an_unauthenticated_caller_is_forbidden_not_too_large() {
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

    let status = post_hook_declaring(
        &addr,
        id.0,
        Some("not-the-token"),
        micold_daemon::hooks::MAX_BODY + 1,
        r#"{"hook_event_name":"Stop"}"#,
    )
    .await;
    assert_eq!(
        status, 403,
        "the size check must not answer ahead of the token check"
    );

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
