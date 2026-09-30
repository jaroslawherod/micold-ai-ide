//! Feature 034 (contracts/mcp-tools.md `create_worktree`; US2 scenarios 1–2, US3 scenario 6; A7,
//! A8, U145–U151, U219): an agent creates a worktree through `POST /mcp` exactly as the
//! create-worktree dialog would.
//!
//! The fixture is a real git repository with worktree `b` and a free local branch `taken`, and
//! sessions S1 (project root) and S3 (`b`, the usual caller), plus S10–S19 in `b` for the race.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::naming::{dir_name_from_branch, NamingError};
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// SC-003: a change reaches every window within this bound.
const WINDOW_BOUND: Duration = Duration::from_secs(2);

struct Fixture {
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
    project: tempfile::TempDir,
    _store: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        add_worktree(project.path(), "b");
        git(project.path(), &["branch", "taken"]);
        let mut sessions = vec![
            session(sid(1), None, TerminalMode::AiCli, AiCli::ClaudeCode),
            session(sid(3), Some("b"), TerminalMode::AiCli, AiCli::ClaudeCode),
        ];
        sessions.extend(
            (10..20).map(|n| session(sid(n), Some("b"), TerminalMode::AiCli, AiCli::ClaudeCode)),
        );
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

    fn repo(&self) -> &Path {
        self.project.path()
    }

    async fn call(&self, caller: SessionId, args: Value) -> Value {
        call_tool(
            self.addr,
            &credential(&self.state, caller),
            "create_worktree",
            args,
        )
        .await
    }

    async fn ok(&self, caller: SessionId, args: Value) -> Value {
        call_ok(
            self.addr,
            &credential(&self.state, caller),
            "create_worktree",
            args,
        )
        .await
    }

    async fn err(&self, caller: SessionId, args: Value) -> Value {
        call_err(
            self.addr,
            &credential(&self.state, caller),
            "create_worktree",
            args,
        )
        .await
    }

    /// The directories under the managed root.
    fn on_disk(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.repo().join(".claude/worktrees"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The worktree refs `list_worktrees` reports.
    async fn listed(&self) -> Vec<String> {
        let out = call_ok(
            self.addr,
            &credential(&self.state, sid(3)),
            "list_worktrees",
            json!({"include_hidden": true}),
        )
        .await;
        out["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["ref"].as_str().unwrap().to_string())
            .collect()
    }

    /// Nothing on disk or in the listing changed from `before`.
    async fn assert_unchanged(&self, before: &(Vec<String>, Vec<String>)) {
        assert_eq!(
            &(self.on_disk(), self.listed().await),
            before,
            "a failed create changes nothing (FR-013)"
        );
    }

    async fn snapshot(&self) -> (Vec<String>, Vec<String>) {
        (self.on_disk(), self.listed().await)
    }
}

fn native(path: PathBuf) -> String {
    path.display().to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_worktree_makes_the_dialogs_worktree_and_every_window_sees_it() {
    let f = Fixture::new().await;
    let mut window = fake_window(&f.state);
    let asked = Instant::now();
    let row = f.ok(sid(3), json!({"branch": "feat-x"})).await;
    let repo = f.repo().to_path_buf();
    let seen = window_sees(&mut window, WINDOW_BOUND, |catalog| {
        catalog
            .projects
            .iter()
            .any(|p| p.path == repo && p.worktrees.iter().any(|w| w.dir_name == "feat-x"))
    })
    .await;
    assert!(
        seen && asked.elapsed() < WINDOW_BOUND + Duration::from_secs(1),
        "a window sees the new worktree without a refresh (SC-003)"
    );
    assert_eq!(
        row,
        json!({
            "ref": "feat-x",
            "display_name": row["display_name"],
            "branch": "feat-x",
            "path": native(f.repo().join(".claude").join("worktrees").join("feat-x")),
            "status": "clean",
            "app_created": true,
            "assistant_owned": false,
            "session_count": 0,
        })
    );
    assert!(f.repo().join(".claude/worktrees/feat-x/.git").exists());
    let (records, _) = f.state.provenance(f.repo());
    assert!(
        records.contains("feat-x"),
        "recorded as created by the app, as the dialog's create is (FR-009)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_name_the_directory_is_derived_from_the_branch() {
    let f = Fixture::new().await;
    let row = f.ok(sid(3), json!({"branch": "feat/login-form"})).await;
    assert_eq!(row["ref"], dir_name_from_branch("feat/login-form"));
    assert_eq!(row["branch"], "feat/login-form");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_existing_or_checked_out_branch_is_a_conflict_and_nothing_changes() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;

    let existing = f.err(sid(3), json!({"branch": "taken"})).await;
    assert_eq!(existing["category"], "conflict", "{existing}");
    let message = existing["message"].as_str().unwrap();
    assert!(
        message.contains("already exists") && message.contains("existing_local"),
        "names the pre-flight's situation and the mode that fits it: {message}"
    );
    f.assert_unchanged(&before).await;

    let held = f
        .err(sid(3), json!({"branch": "b", "name": "b-again"}))
        .await;
    assert_eq!(held["category"], "conflict", "{held}");
    assert!(
        held["message"]
            .as_str()
            .unwrap()
            .contains("already checked out in the worktree 'b'"),
        "the dialog's own sentence for a held branch: {held}"
    );
    f.assert_unchanged(&before).await;

    let root = f
        .err(sid(3), json!({"branch": "main", "name": "main-again"}))
        .await;
    assert_eq!(root["category"], "conflict", "{root}");
    f.assert_unchanged(&before).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_the_naming_rules_reject_is_invalid_input_and_creates_nothing() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;

    let empty = f
        .err(sid(3), json!({"branch": "feat-y", "name": "!!!"}))
        .await;
    assert_eq!(empty["category"], "invalid_input");
    assert!(
        empty["message"]
            .as_str()
            .unwrap()
            .contains(&NamingError::EmptyNameAfterSlug.to_string()),
        "the dialog's message: {empty}"
    );

    let spaced = f
        .err(sid(3), json!({"branch": "feat-y", "name": "Feat Y"}))
        .await;
    assert_eq!(spaced["category"], "invalid_input");
    assert!(
        spaced["message"].as_str().unwrap().contains("feat-y"),
        "says which name the rules would accept: {spaced}"
    );
    f.assert_unchanged(&before).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_branch_name_git_rejects_is_invalid_input_with_gits_message() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;
    let error = f
        .err(sid(3), json!({"branch": "bad..branch", "name": "bad"}))
        .await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("not a valid branch name"),
        "git's own words: {error}"
    );
    f.assert_unchanged(&before).await;
}

/// `git check-ref-format --branch` expands `@{-N}` to the branch checked out before, so it would
/// pass; the agent names a branch, never a reflog shorthand (EC-3, U229).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_previous_branch_shorthand_is_invalid_input() {
    let f = Fixture::new().await;
    // A checkout history, so `@{-1}` names a real branch (`prev`) for git to expand to.
    git(f.project.path(), &["checkout", "-q", "-b", "prev"]);
    git(f.project.path(), &["checkout", "-q", "-"]);
    let before = f.snapshot().await;
    let error = f.err(sid(3), json!({"branch": "@{-1}"})).await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    f.assert_unchanged(&before).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn existing_local_checks_out_a_free_branch() {
    let f = Fixture::new().await;
    let row = f
        .ok(sid(3), json!({"branch": "taken", "mode": "existing_local"}))
        .await;
    assert_eq!(row["ref"], "taken");
    assert_eq!(row["branch"], "taken");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_concurrent_creates_of_one_branch_leave_one_worktree() {
    let f = Arc::new(Fixture::new().await);
    let calls: Vec<_> = (10..20)
        .map(|n| {
            let f = Arc::clone(&f);
            tokio::spawn(async move { f.call(sid(n), json!({"branch": "race"})).await })
        })
        .collect();
    let mut ok = 0;
    let mut conflicts = 0;
    for call in calls {
        let result = call.await.unwrap();
        if result["isError"] == json!(false) {
            ok += 1;
        } else if result["structuredContent"]["error"]["category"] == "conflict" {
            conflicts += 1;
        } else {
            panic!("neither a success nor a conflict: {result}");
        }
    }
    assert_eq!((ok, conflicts), (1, 9), "SC-005");
    assert_eq!(f.on_disk(), ["b", "race"], "exactly one worktree on disk");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn back_to_back_creates_from_one_caller_all_succeed() {
    let f = Fixture::new().await;
    for n in 0..5 {
        f.ok(sid(3), json!({"branch": format!("burst-{n}")})).await;
    }
    assert_eq!(
        f.on_disk(),
        ["b", "burst-0", "burst-1", "burst-2", "burst-3", "burst-4"],
        "no cap beyond the user's own (EC-15, D3)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_default_session_is_refused_by_principle_iii_and_nothing_changes() {
    let f = Fixture::new().await;
    let before = f.snapshot().await;
    let (records_before, _) = f.state.provenance(f.repo());
    let error = f.err(sid(1), json!({"branch": "feat-x"})).await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("Principle III"),
        "{error}"
    );
    f.assert_unchanged(&before).await;
    let (records, _) = f.state.provenance(f.repo());
    assert_eq!(records, records_before, "no provenance record either");
}
