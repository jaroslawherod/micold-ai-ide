//! Feature 1210 (T008, T028) — the worktree guard over the real hook receiver: two worktrees of a
//! real repository in a real catalog, a session in worktree A and a Default session, each asking
//! the receiver to judge `Write` calls (contracts/pretooluse-guard.md). One session's guard never
//! affects the other's (FR-011, SC-001, Constitution Principles II and III).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree_guard::SessionBinding;
use micold_daemon::catalog::Catalog;
use micold_daemon::hooks::{GuardState, HookReceiver};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

const IN_A: u128 = 0xA;
const IN_ROOT: u128 = 0xB;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

struct Fixture {
    addr: String,
    state: Arc<DaemonState>,
    receiver: HookReceiver,
    root: PathBuf,
    _dirs: (tempfile::TempDir, tempfile::TempDir),
}

impl Fixture {
    fn wt(&self, name: &str) -> PathBuf {
        self.root.join(".claude/worktrees").join(name)
    }

    /// Post a `PreToolUse` for `tool` and return the status and the parsed body (`Null` if empty).
    async fn judge(&self, session: u128, tool: &str, path: &Path) -> (u16, Value) {
        let body = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": tool,
            "tool_input": {"file_path": path},
            "cwd": self.wt("a"),
        })
        .to_string();
        self.post(session, &body).await
    }

    /// Post a `PreToolUse` for a `Bash` `command` started in `cwd`.
    async fn bash(&self, session: u128, command: &str, cwd: &Path) -> (u16, Value) {
        let body = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": command},
            "cwd": cwd,
        })
        .to_string();
        self.post(session, &body).await
    }

    async fn post(&self, session: u128, body: &str) -> (u16, Value) {
        let id = SessionId::from_uuid(Uuid::from_u128(session));
        let token = self.receiver.token_for(id);
        let mut stream = TcpStream::connect(&self.addr).await.unwrap();
        let request = format!(
            "POST /hook/{} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            id.0,
            body.len()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let text = String::from_utf8_lossy(&response).into_owned();
        let status = text.split_whitespace().nth(1).unwrap().parse().unwrap();
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("");
        let value = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(body).expect("a JSON body")
        };
        (status, value)
    }
}

async fn fixture() -> Fixture {
    let project_dir = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(project_dir.path()).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["commit", "-q", "--allow-empty", "-m", "init"]);
    for name in ["a", "b"] {
        let dir = root.join(".claude/worktrees").join(name);
        git(
            &root,
            &["worktree", "add", "-q", "-b", name, dir.to_str().unwrap()],
        );
    }
    let session = |id: u128, location| {
        Session::restored(
            SessionId::from_uuid(Uuid::from_u128(id)),
            location,
            SessionLabel::Pending,
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        )
    };
    let mut sessions = BTreeMap::new();
    sessions.insert(
        root.clone(),
        vec![
            session(IN_A, SessionLocation::Worktree("a".into())),
            session(IN_ROOT, SessionLocation::Default),
        ],
    );
    let workspace = Workspace {
        projects: vec![Project::new(root.clone(), true, Availability::Available)],
        active: Some(root.clone()),
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
    state.refresh_worktrees(&root);
    let (receiver, listener) = HookReceiver::bind(store.path().join("hooks"))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(micold_daemon::hooks::serve(
        listener,
        receiver.tokens(),
        Arc::clone(&state),
    ));
    // What a launch does: freeze each session's guard from the catalog.
    for id in [IN_A, IN_ROOT] {
        let id = SessionId::from_uuid(Uuid::from_u128(id));
        state.set_session_guard(id, state.guard_state_for(id));
    }
    Fixture {
        addr,
        state,
        receiver,
        root,
        _dirs: (project_dir, store),
    }
}

fn reason(body: &Value) -> &str {
    assert_eq!(body["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(body["hookSpecificOutput"]["permissionDecision"], "deny");
    body["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .expect("a reason")
}

#[test]
fn the_guard_built_at_launch_carries_the_binding_and_the_project_root() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let f = rt.block_on(fixture());
    let guard = |id| {
        f.state
            .session_guard(SessionId::from_uuid(Uuid::from_u128(id)))
    };
    assert_eq!(
        guard(IN_A),
        Some(GuardState::On {
            binding: SessionBinding::Worktree(f.wt("a")),
            project: f.root.clone()
        })
    );
    assert_eq!(
        guard(IN_ROOT),
        Some(GuardState::On {
            binding: SessionBinding::Default(f.root.clone()),
            project: f.root.clone()
        })
    );
    assert!(f.state.guard_worktrees(&f.root).contains(&f.wt("b")));
}

#[tokio::test]
async fn a_worktree_session_is_denied_in_the_other_worktree_and_allowed_in_its_own() {
    let f = fixture().await;
    let (status, body) = f.judge(IN_A, "Write", &f.wt("b").join("src/x.rs")).await;
    assert_eq!(status, 200);
    assert!(reason(&body).contains("create_session"), "{body}");
    assert!(reason(&body).contains("x.rs"), "{body}");
    for allowed in [
        f.wt("a").join("src/x.rs"),
        f.root.join("target-shared/debug/x"),
        f.root.join(".claude/worktrees/autopilot-prefetch-1/x.rs"),
    ] {
        assert_eq!(f.judge(IN_A, "Edit", &allowed).await, (200, Value::Null));
    }
    // The project root is not the session's own worktree.
    let (_, body) = f.judge(IN_A, "MultiEdit", &f.root.join("src/x.rs")).await;
    assert!(reason(&body).contains("MultiEdit"));
}

#[tokio::test]
async fn a_default_session_is_allowed_in_the_root_and_denied_in_a_worktree_and_does_not_affect_the_other(
) {
    let f = fixture().await;
    assert_eq!(
        f.judge(IN_ROOT, "Write", &f.root.join("src/x.rs")).await,
        (200, Value::Null)
    );
    let (_, body) = f.judge(IN_ROOT, "Write", &f.wt("a").join("x.rs")).await;
    assert!(reason(&body).contains("create_session"));
    // The same path is the other session's own worktree, and still allowed there.
    assert_eq!(
        f.judge(IN_A, "Write", &f.wt("a").join("x.rs")).await,
        (200, Value::Null)
    );
}

#[tokio::test]
async fn a_bash_call_started_in_or_changing_into_another_worktree_is_denied() {
    let f = fixture().await;
    let (status, body) = f.bash(IN_A, "ls", &f.wt("b")).await;
    assert_eq!(status, 200);
    assert!(reason(&body).contains("create_session"), "{body}");
    assert!(reason(&body).contains("Bash"), "{body}");
    let (_, body) = f.bash(IN_A, "cd ../b && ls", &f.wt("a")).await;
    assert!(reason(&body).contains("create_session"), "{body}");
    for allowed in [
        "ls",
        "cd src && ls",
        "cargo build --target-dir ../../../target-shared",
    ] {
        assert_eq!(
            f.bash(IN_A, allowed, &f.wt("a")).await,
            (200, Value::Null),
            "{allowed}"
        );
    }
}

#[tokio::test]
async fn calls_the_guard_cannot_judge_or_does_not_judge_are_answered_as_before() {
    let f = fixture().await;
    let other = f.wt("b").join("x.rs");
    // A read is never judged.
    let read =
        json!({"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":other}});
    assert_eq!(f.post(IN_A, &read.to_string()).await, (200, Value::Null));
    // Fail open: no path in the payload, and a session whose guard was never set.
    let unreadable = json!({"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{}});
    assert_eq!(
        f.post(IN_A, &unreadable.to_string()).await,
        (200, Value::Null)
    );
    f.state
        .set_session_guard(SessionId::from_uuid(Uuid::from_u128(IN_A)), GuardState::Off);
    assert_eq!(f.judge(IN_A, "Write", &other).await, (200, Value::Null));
    // A later event is unaffected.
    let stop = json!({"hook_event_name":"Stop"});
    assert_eq!(f.post(IN_A, &stop.to_string()).await, (200, Value::Null));
}
