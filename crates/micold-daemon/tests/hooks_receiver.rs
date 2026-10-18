//! US2 (T045) — the loopback activity-hook receiver end to end (contracts/hooks.md §Listener).
//!
//! Drives the real HTTP listener over TCP (no `claude` needed): a correctly-authenticated
//! `UserPromptSubmit` moves the session to `Working`; a wrong/missing token is a bare `403` that
//! changes nothing; a `SessionStart` is accepted without a transition. The parsing rules
//! (head/path/token/event) are unit-tested inside `hooks.rs`; this proves the wiring — bind → POST →
//! the session's projected activity signal.

use std::collections::BTreeMap;
use std::sync::Arc;

#[path = "support/mcp.rs"]
mod mcp_support;

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::ActivitySignal;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
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

/// Every id in `ids` in the one project, any of them live-able.
fn state_with_sessions(
    ids: &[SessionId],
) -> (Arc<DaemonState>, tempfile::TempDir, tempfile::TempDir) {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let sessions_of: Vec<Session> = ids
        .iter()
        .map(|&id| {
            Session::restored(
                id,
                SessionLocation::Default,
                SessionLabel::Pending,
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            )
        })
        .collect();
    let mut sessions = BTreeMap::new();
    sessions.insert(project.path().to_path_buf(), sessions_of);
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

/// Register a `cat` PTY (`cmd /q` on Windows) under the catalog id so the session is live (a target
/// for `note_activity`).
fn register_cat(state: &DaemonState, id: SessionId) -> Arc<PtySession> {
    #[cfg(unix)]
    let mut cmd = CommandBuilder::new("cat");
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("cmd");
        cmd.arg("/q");
        cmd
    };
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn cat");
    state.register_session(session)
}

/// Everything a receiver test needs: the catalog session `id` (live as a `cat` PTY), the listener's
/// `addr`, the session's hook `token`, its `state` and the `cat` process to kill at the end. The
/// directory guards keep the catalog's files for the test's lifetime.
struct Receiver {
    id: SessionId,
    addr: String,
    token: String,
    state: Arc<DaemonState>,
    cat: Arc<PtySession>,
    _dirs: (tempfile::TempDir, tempfile::TempDir),
}

/// Bind a receiver for a fresh live session and serve it.
async fn spawn_receiver() -> Receiver {
    spawn_receiver_with(&[]).await.0
}

/// As [`spawn_receiver`], plus one more live session per id in `others` (returned as `cat`s to kill).
async fn spawn_receiver_with(others: &[SessionId]) -> (Receiver, Vec<Arc<PtySession>>) {
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let all: Vec<SessionId> = std::iter::once(id).chain(others.iter().copied()).collect();
    let (state, project, store) = state_with_sessions(&all);
    let cat = register_cat(&state, id);
    let other_cats = others.iter().map(|&o| register_cat(&state, o)).collect();
    let (receiver, listener) = HookReceiver::bind(std::env::temp_dir().join("micold-hooks-test"))
        .await
        .expect("bind receiver");
    let token = receiver.token_for(id);
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(micold_daemon::hooks::serve(
        listener,
        receiver.tokens(),
        Arc::clone(&state),
    ));
    let receiver = Receiver {
        id,
        addr,
        token,
        state,
        cat,
        _dirs: (project, store),
    };
    (receiver, other_cats)
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
    let Receiver {
        id,
        addr,
        token,
        state,
        cat,
        _dirs,
    } = spawn_receiver().await;

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
    let Receiver {
        id,
        addr,
        token: _token,
        state,
        cat,
        _dirs,
    } = spawn_receiver().await;

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
    let Receiver {
        id,
        addr,
        token,
        state,
        cat,
        _dirs,
    } = spawn_receiver().await;

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
    let Receiver {
        id,
        addr,
        token: _token,
        state: _,
        cat,
        _dirs,
    } = spawn_receiver().await;

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
    let Receiver {
        id,
        addr,
        token,
        state,
        cat,
        _dirs,
    } = spawn_receiver().await;

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

// ---- Feature 490: the status-line relay in the settings file (contract S1; FR-002, FR-020) ----

mod status_line_block {
    use std::path::{Path, PathBuf};

    use micold_core::mcp::binding::ConfigLocations;
    use micold_core::plan_usage::user_status_line::RelayConfig;
    use micold_core::session::SessionId;
    use micold_daemon::hooks::{HookReceiver, StatusContext};
    use serde_json::Value;
    use uuid::Uuid;

    struct Fixture {
        receiver: HookReceiver,
        dir: tempfile::TempDir,
        session: SessionId,
    }

    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let (receiver, _listener) = HookReceiver::bind(dir.path().join("hooks")).await.unwrap();
        Fixture {
            receiver,
            dir,
            session: SessionId(Uuid::new_v4()),
        }
    }

    impl Fixture {
        fn cwd(&self) -> PathBuf {
            self.dir.path().join("project")
        }

        fn context(&self) -> StatusContext {
            StatusContext {
                cwd: self.cwd(),
                locations: ConfigLocations {
                    home: Some(self.dir.path().join("home")),
                    ..ConfigLocations::default()
                },
            }
        }

        fn user_line(&self, block: &str) {
            let path = self.cwd().join(".claude/settings.json");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, format!(r#"{{"statusLine": {block}}}"#)).unwrap();
        }

        fn prepare(&self, on: bool) -> Value {
            self.receiver.set_plan_usage(on);
            let path = self
                .receiver
                .prepare_settings(self.session, self.context())
                .unwrap();
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
        }

        fn relay_file(&self) -> PathBuf {
            self.receiver.relay_file(self.session)
        }
    }

    fn quoted(path: &Path) -> String {
        format!("\"{}\"", path.to_string_lossy().replace('\\', "/"))
    }

    #[tokio::test]
    async fn on_the_block_runs_the_daemon_as_the_relay_with_quoted_forward_slash_paths() {
        let f = fixture().await;
        let doc = f.prepare(true);
        let block = &doc["statusLine"];
        assert_eq!(block["type"], "command");
        let exe = std::env::current_exe().unwrap();
        assert_eq!(
            block["command"].as_str().unwrap(),
            format!("{} status-line {}", quoted(&exe), quoted(&f.relay_file()))
        );
        assert!(block.get("padding").is_none() && block.get("refreshInterval").is_none());
    }

    #[tokio::test]
    async fn padding_and_refresh_interval_come_from_the_users_status_line() {
        let f = fixture().await;
        f.user_line(
            r#"{"type": "command", "command": "mine.sh", "padding": 1, "refreshInterval": 7}"#,
        );
        let doc = f.prepare(true);
        assert_eq!(doc["statusLine"]["padding"], 1);
        assert_eq!(doc["statusLine"]["refreshInterval"], 7);
    }

    #[tokio::test]
    async fn the_relay_file_names_the_route_the_token_and_the_users_command() {
        let f = fixture().await;
        f.user_line(r#"{"type": "command", "command": "mine.sh"}"#);
        f.prepare(true);
        let config: RelayConfig =
            serde_json::from_slice(&std::fs::read(f.relay_file()).unwrap()).unwrap();
        assert_eq!(config.token, f.receiver.token_for(f.session));
        assert!(
            config.url.starts_with("http://127.0.0.1:")
                && config.url.ends_with(&format!("/status/{}", f.session.0)),
            "{}",
            config.url
        );
        assert_eq!(config.user_command.as_deref(), Some("mine.sh"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn the_relay_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let f = fixture().await;
        f.prepare(true);
        let mode = std::fs::metadata(f.relay_file())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "mode {mode:o}");
    }

    #[tokio::test]
    async fn off_there_is_no_block_no_relay_file_and_the_hooks_are_identical() {
        let f = fixture().await;
        let on = f.prepare(true);
        let off = f.prepare(false);
        assert!(off.get("statusLine").is_none());
        assert!(
            !f.relay_file().exists(),
            "turning off deletes the relay file"
        );
        assert_eq!(on["hooks"], off["hooks"]);
        assert_eq!(off.as_object().unwrap().len(), 1, "only `hooks`: {off}");
    }

    #[tokio::test]
    async fn a_forgotten_session_is_not_rewritten_by_the_switch() {
        let f = fixture().await;
        f.prepare(true);
        f.receiver.forget(f.session);
        f.receiver.set_plan_usage(true);
        assert!(
            !f.relay_file().exists(),
            "an ended session's relay file stays gone"
        );
    }

    #[tokio::test]
    async fn forget_deletes_the_relay_file() {
        let f = fixture().await;
        f.prepare(true);
        assert!(f.relay_file().exists());
        f.receiver.forget(f.session);
        assert!(!f.relay_file().exists());
    }

    #[tokio::test]
    async fn set_plan_usage_rewrites_every_prepared_session() {
        let f = fixture().await;
        f.prepare(true);
        let settings = f.receiver.settings_file(f.session);
        f.receiver.set_plan_usage(false);
        let doc: Value = serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
        assert!(doc.get("statusLine").is_none());
        assert!(!f.relay_file().exists());
        f.receiver.set_plan_usage(true);
        let doc: Value = serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
        assert_eq!(doc["statusLine"]["type"], "command");
        assert!(f.relay_file().exists());
    }
}

/// Feature 552, contract C2: a sub-agent start and stop posted for a known session are recorded
/// on it; a report with no `agent_id` is answered `200` and records nothing; invalid JSON is `400`.
/// The activity state is not touched, and the body is not echoed.
#[tokio::test]
async fn sub_agent_reports_are_recorded_on_the_origin_session() {
    let Receiver {
        id,
        addr,
        token,
        state,
        cat,
        _dirs,
    } = spawn_receiver().await;
    let agents = || {
        state
            .catalog_snapshot()
            .projects
            .into_iter()
            .flat_map(|p| p.sessions)
            .find(|s| s.id == id)
            .expect("session in snapshot")
            .agents
    };

    let start = r#"{"hook_event_name":"SubagentStart","agent_id":"a15f822c8aa66a621","agent_type":"Explore"}"#;
    assert_eq!(post_hook(&addr, id.0, Some(&token), start).await, 200);
    assert_eq!(agents().len(), 1);
    assert_eq!(
        agents()[0].state,
        micold_core::agent_run::AgentRunState::Running
    );

    let stop = r#"{"hook_event_name":"SubagentStop","agent_id":"a15f822c8aa66a621","agent_transcript_path":"/t/a.jsonl"}"#;
    assert_eq!(post_hook(&addr, id.0, Some(&token), stop).await, 200);
    let runs = agents();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        runs[0].state,
        micold_core::agent_run::AgentRunState::Stopped
    );
    assert_eq!(runs[0].transcript.as_deref(), Some("/t/a.jsonl"));
    assert_eq!(activity(&state, id), ActivitySignal::Unknown);

    for ignored in [
        r#"{"hook_event_name":"SubagentStart"}"#,
        r#"{"hook_event_name":"SubagentStop","agent_id":""}"#,
        r#"{"hook_event_name":"SubagentStop","agent_id":42}"#,
    ] {
        assert_eq!(
            post_hook(&addr, id.0, Some(&token), ignored).await,
            200,
            "{ignored}"
        );
    }
    assert_eq!(agents().len(), 1, "nothing more was recorded");
    assert_eq!(post_hook(&addr, id.0, Some(&token), "{not json").await, 400);

    cat.kill().expect("kill");
}

/// The `hook_activity` the catalog snapshot carries for `id`.
fn hook_activity(
    state: &DaemonState,
    id: SessionId,
) -> Option<micold_core::hook_activity::HookActivity> {
    state
        .catalog_snapshot()
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id == id)
        .expect("session in snapshot")
        .hook_activity
}

/// Feature 813 (W2, W3, FR-020): a `TodoWrite` and a prompt reach only their own live session's
/// `hook_activity`, each change is broadcast, and neither text is ever logged.
#[tokio::test]
async fn hook_activity_is_captured_per_session_broadcast_and_never_logged() {
    use micold_core::hook_activity::TodoStatus;
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::{ClientIdentity, ClientInstance, DaemonMsg};

    let log = mcp_support::log();
    let other = SessionId::from_uuid(Uuid::from_u128(SESSION_U128 + 1));
    let (r, others) = spawn_receiver_with(&[other]).await;
    let Receiver {
        id,
        addr,
        token,
        state,
        cat,
        _dirs,
    } = r;
    let (_client, mut rx) = state.register(ClientIdentity::new(
        "test",
        ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));
    assert_eq!(hook_activity(&state, id), None);

    let todo_body = r#"{"hook_event_name":"PostToolUse","tool_name":"TodoWrite","tool_input":{"todos":[
        {"content":"SECRET-TODO-DONE","activeForm":"SECRET-DOING-DONE","status":"completed"},
        {"content":"SECRET-TODO-NOW","activeForm":"SECRET-DOING-NOW","status":"in_progress"}]}}"#;
    assert_eq!(post_hook(&addr, id.0, Some(&token), todo_body).await, 200);
    let got = hook_activity(&state, id).expect("todos captured");
    assert_eq!(got.todos.len(), 2);
    assert_eq!(got.todos[0].status, TodoStatus::Completed);
    assert_eq!(got.todos[1].status, TodoStatus::InProgress);
    assert_eq!(got.todos[1].text, "SECRET-TODO-NOW");

    let prompt_body = r#"{"hook_event_name":"UserPromptSubmit","prompt":"SECRET-PROMPT-TEXT"}"#;
    assert_eq!(post_hook(&addr, id.0, Some(&token), prompt_body).await, 200);
    let got = hook_activity(&state, id).expect("prompt captured");
    assert_eq!(got.prompt.as_deref(), Some("SECRET-PROMPT-TEXT"));
    assert_eq!(got.todos.len(), 2, "the prompt leaves the todos alone");

    assert_eq!(
        hook_activity(&state, other),
        None,
        "another session is untouched"
    );

    let mut broadcasts = 0;
    while let Ok(frame) = rx.try_recv() {
        if let Frame::Control(DaemonMsg::CatalogChanged { catalog }) = frame {
            let mine = catalog
                .projects
                .iter()
                .flat_map(|p| &p.sessions)
                .find(|s| s.id == id)
                .and_then(|s| s.hook_activity.clone());
            if mine.is_some() {
                broadcasts += 1;
            }
        }
    }
    assert!(
        broadcasts >= 2,
        "each captured change is broadcast, saw {broadcasts}"
    );

    let logged = String::from_utf8_lossy(&log.lock().unwrap()).into_owned();
    assert!(
        !logged.contains("SECRET-"),
        "a prompt or todo text reached the log"
    );

    cat.kill().expect("kill");
    for c in others {
        c.kill().expect("kill");
    }
}
