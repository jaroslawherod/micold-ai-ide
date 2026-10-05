//! Feature 582, M3 (US3; FR-005, FR-011, FR-015; SC-004; contracts/attach-worktree-tool.md,
//! contracts/list-resumable-sessions-tool.md): an agent attaches a provider worktree and lists
//! resumable sessions through `POST /mcp`.
//!
//! The fixture is a real git repository with worktrees `a`, `b`, `c` under `.claude/worktrees`
//! (none has a provenance record, so each is hidden), the caller S3 in `b` and S1 in the project
//! root (a Default session).

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

    /// The `list_worktrees` row of `name`, hidden ones included.
    async fn row(&self, name: &str) -> Option<Value> {
        let rows = self
            .ok(CALLER, "list_worktrees", json!({"include_hidden": true}))
            .await;
        rows["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["ref"] == name)
            .cloned()
    }

    async fn visible_refs(&self) -> Vec<String> {
        let rows = self.ok(CALLER, "list_worktrees", json!({})).await;
        rows["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["ref"].as_str().unwrap().to_string())
            .collect()
    }
}

async fn assert_attached(fx: &Fixture, name: &str) {
    assert!(
        fx.visible_refs().await.contains(&name.to_string()),
        "{name} is listed without include_hidden"
    );
}

#[tokio::test]
async fn attaching_by_dir_name_lists_the_worktree_as_not_hidden() {
    let fx = Fixture::new().await;
    assert!(!fx.visible_refs().await.contains(&"a".to_string()));

    let out = fx
        .ok(CALLER, "attach_worktree", json!({"worktree": "a"}))
        .await;

    assert_eq!(out["outcome"], "attached");
    assert_eq!(out["worktree"], "a");
    assert_eq!(out["branch"], "a");
    assert!(out["path"].as_str().unwrap().ends_with('a'));
    assert_attached(&fx, "a").await;
    let row = fx.row("a").await.unwrap();
    assert_eq!(row["assistant_owned"], json!(false), "SC-004");
}

#[tokio::test]
async fn attaching_by_absolute_path_and_by_branch_resolves_the_same_worktree() {
    let fx = Fixture::new().await;
    let by_path = fx
        .ok(
            CALLER,
            "attach_worktree",
            json!({"worktree": fx.dir("c").to_string_lossy()}),
        )
        .await;
    assert_eq!(by_path["worktree"], "c");
    assert_eq!(by_path["outcome"], "attached");

    let named = fx.dir("named").to_string_lossy().into_owned();
    git(
        fx.project.path(),
        &["worktree", "add", "-q", "-b", "feature/x", &named],
    );
    let by_branch = fx
        .ok(CALLER, "attach_worktree", json!({"worktree": "feature/x"}))
        .await;
    assert_eq!(by_branch["worktree"], "named");
    assert_eq!(by_branch["branch"], "feature/x");
    assert_attached(&fx, "named").await;
}

#[tokio::test]
async fn a_path_outside_the_project_and_another_projects_worktree_are_not_found_and_change_nothing()
{
    let fx = Fixture::new().await;
    let other = tempfile::tempdir().unwrap();
    init_repo(other.path());
    let foreign = add_worktree(other.path(), "foreign");
    let outside = tempfile::tempdir().unwrap();
    let before = fx.visible_refs().await;

    for reference in [
        foreign.to_string_lossy().into_owned(),
        outside.path().to_string_lossy().into_owned(),
        "no-such-thing".to_string(),
    ] {
        let error = fx
            .err(CALLER, "attach_worktree", json!({"worktree": reference}))
            .await;
        assert_eq!(error["category"], "not_found", "{reference}: {error}");
    }
    assert_eq!(
        fx.visible_refs().await,
        before,
        "scenario 2: nothing changed"
    );
}

#[tokio::test]
async fn default_an_ambiguous_branch_and_an_unavailable_worktree_are_invalid_input() {
    let fx = Fixture::new().await;
    // Two worktrees on one branch (git needs --force for the second).
    git(
        fx.project.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "shared",
            &fx.dir("p").to_string_lossy(),
        ],
    );
    git(
        fx.project.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-f",
            &fx.dir("q").to_string_lossy(),
            "shared",
        ],
    );
    // One whose directory is gone.
    add_worktree(fx.project.path(), "gone");
    std::fs::remove_dir_all(fx.dir("gone")).unwrap();
    // One outside the managed directory.
    let elsewhere = tempfile::tempdir().unwrap();
    let outside = elsewhere.path().join("outside");
    git(
        fx.project.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "elsewhere",
            &outside.to_string_lossy(),
        ],
    );

    for reference in ["default", "shared", "gone"] {
        let error = fx
            .err(CALLER, "attach_worktree", json!({"worktree": reference}))
            .await;
        assert_eq!(error["category"], "invalid_input", "{reference}: {error}");
    }
    let hidden = fx.visible_refs().await;
    // A worktree outside the managed directory is not one of the project's unless the user included
    // it, so it is not found.
    for reference in [
        "elsewhere".to_string(),
        outside.to_string_lossy().into_owned(),
    ] {
        let error = fx
            .err(CALLER, "attach_worktree", json!({"worktree": reference}))
            .await;
        assert_eq!(error["category"], "not_found", "{reference}: {error}");
    }
    for name in ["p", "q", "gone"] {
        assert!(!hidden.contains(&name.to_string()), "{name} stays hidden");
    }
}

#[tokio::test]
async fn attaching_again_reports_already_attached_and_creates_no_duplicate() {
    let fx = Fixture::new().await;
    fx.ok(CALLER, "attach_worktree", json!({"worktree": "c"}))
        .await;

    let again = fx
        .ok(CALLER, "attach_worktree", json!({"worktree": "c"}))
        .await;

    assert_eq!(again["outcome"], "already_attached");
    let rows = fx
        .ok(CALLER, "list_worktrees", json!({"include_hidden": true}))
        .await;
    let count = rows["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["ref"] == "c")
        .count();
    assert_eq!(count, 1, "scenario 3");
}

#[tokio::test]
async fn a_default_caller_is_refused_and_nothing_is_attached() {
    let fx = Fixture::new().await;

    let error = fx
        .err(ROOT, "attach_worktree", json!({"worktree": "a"}))
        .await;

    assert_eq!(error["category"], "refused_by_policy");
    // Refused before the reference is looked at: not `not_found`, not `invalid_input`.
    for reference in ["no-such-thing", "default"] {
        let error = fx
            .err(ROOT, "attach_worktree", json!({"worktree": reference}))
            .await;
        assert_eq!(error["category"], "refused_by_policy", "{reference}");
    }
    assert!(error["message"].as_str().unwrap().contains("Principle III"));
    assert!(
        !fx.visible_refs().await.contains(&"a".to_string()),
        "scenario 4"
    );
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
    assert_eq!(top["status"], "needs_worktree_attach");
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
