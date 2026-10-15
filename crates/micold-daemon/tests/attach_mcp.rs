//! Feature 582 (contracts/list-resumable-sessions-tool.md): an agent lists resumable sessions
//! through `POST /mcp`. Worktrees are not attached (feature 552).
//!
//! The fixture is a real git repository with worktrees `a`, `b`, `c` under `.claude/worktrees`,
//! the caller S3 in `b` and S1 in the project root (a Default session).

#[path = "support/mcp.rs"]
mod mcp_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mcp_support::*;
use micold_core::session::{AiCli, TerminalMode};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};
use uuid::Uuid;

const ROOT: u128 = 1;
const CALLER: u128 = 3;

struct Fixture {
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
    project: tempfile::TempDir,
    _store: tempfile::TempDir,
}

/// One scratch home for every provider store in this test binary, set once.
fn provider_home() -> &'static Path {
    static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    HOME.get_or_init(|| {
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("CLAUDE_CONFIG_DIR", home.path().join(".claude"));
        std::env::set_var("COPILOT_HOME", home.path().join(".copilot"));
        std::env::set_var("PI_CODING_AGENT_DIR", home.path().join(".pi"));
        home
    })
    .path()
}

/// A Claude transcript with id `id` for a session that ran in `cwd`.
fn seed_claude_as(cwd: &Path, id: Uuid) {
    let encoded: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = provider_home().join(".claude/projects").join(encoded);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"cwd\":{},\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n",
            serde_json::to_string(&cwd.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
}

fn seed_claude(cwd: &Path) -> Uuid {
    let id = Uuid::new_v4();
    seed_claude_as(cwd, id);
    id
}

impl Fixture {
    async fn new() -> Self {
        provider_home();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        for name in ["a", "b", "c"] {
            add_worktree(project.path(), name);
        }
        let sessions = vec![
            session(sid(ROOT), None, TerminalMode::AiCli, AiCli::ClaudeCode),
            session(
                sid(CALLER),
                Some("b"),
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            ),
        ];
        let state = state_over(
            vec![(project.path().to_path_buf(), true, sessions)],
            store.path(),
        );
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            state,
            addr,
            project,
            _store: store,
        }
    }

    fn dir(&self, name: &str) -> PathBuf {
        self.project.path().join(".claude/worktrees").join(name)
    }

    async fn ok(&self, caller: u128, tool: &str, args: Value) -> Value {
        call_ok(self.addr, &credential(&self.state, sid(caller)), tool, args).await
    }

    async fn err(&self, caller: u128, tool: &str, args: Value) -> Value {
        call_err(self.addr, &credential(&self.state, sid(caller)), tool, args).await
    }
}

#[tokio::test]
async fn resumable_sessions_page_newest_first_with_limit_and_offset_and_skip_catalog_sessions() {
    let fx = Fixture::new().await;
    let first = seed_claude(&fx.dir("a"));
    std::thread::sleep(std::time::Duration::from_millis(30));
    let second = seed_claude(fx.project.path());
    std::thread::sleep(std::time::Duration::from_millis(30));
    let third = seed_claude(&fx.dir("a"));
    // A transcript of a session the catalog already holds is never listed (FR-007).
    seed_claude_as(fx.project.path(), sid(ROOT).0);

    let all = fx.ok(ROOT, "list_resumable_sessions", json!({})).await;
    let ids: Vec<String> = all["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["session"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ids,
        [third, second, first].map(|i| i.to_string()),
        "newest first, and no catalog session"
    );
    assert_eq!(all["total"], 3);
    let top = &all["sessions"][0];
    assert_eq!(top["provider"], "claude_code");
    assert_eq!(top["worktree"], "a");
    assert_eq!(top["status"], "resumable");
    assert!(top["last_activity"].as_str().unwrap().contains('T'));
    assert_eq!(all["sessions"][1]["worktree"], "default");
    assert_eq!(all["sessions"][1]["status"], "resumable");

    let page = fx
        .ok(
            CALLER,
            "list_resumable_sessions",
            json!({"limit": 1, "offset": 1}),
        )
        .await;
    assert_eq!(page["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(page["sessions"][0]["session"], second.to_string());
    assert_eq!(page["total"], 3);

    let filtered = fx
        .ok(ROOT, "list_resumable_sessions", json!({"worktree": "a"}))
        .await;
    assert_eq!(filtered["sessions"].as_array().unwrap().len(), 2);
    assert_eq!(filtered["total"], 2);

    let error = fx
        .err(ROOT, "list_resumable_sessions", json!({"limit": 201}))
        .await;
    assert_eq!(error["category"], "invalid_input");
}
