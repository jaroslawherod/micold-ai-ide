//! Feature 034 (contracts/mcp-tools.md; US1 scenarios 2 and 3; A2, A3, U124–U140): the read-only
//! tools, called through `POST /mcp` as a bound session would call them.
//!
//! The fixture is a real git repository with worktrees `a` and `b`, a hidden assistant worktree,
//! and sessions S1 (project root), S2 (`a`) and S3 (`b`, the caller). A second project holds S9, so
//! every scoping rule has something to leak.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::path::{Path, PathBuf};

use mcp_support::*;
use micold_core::session::{AiCli, SessionId, SessionLifecycle, TerminalMode};
use serde_json::{json, Value};

/// An assistant's worktree: directly under the managed root, never recorded as the user's.
const AGENT_WORKTREE: &str = "agent-0123456789abcdef0";

struct Fixture {
    state: std::sync::Arc<micold_daemon::state::DaemonState>,
    addr: std::net::SocketAddr,
    project: tempfile::TempDir,
    _other: tempfile::TempDir,
    _store: tempfile::TempDir,
}

impl Fixture {
    fn repo(&self) -> &Path {
        self.project.path()
    }

    fn cred(&self, id: SessionId) -> String {
        credential(&self.state, id)
    }

    async fn ok(&self, caller: SessionId, tool: &str, args: Value) -> Value {
        call_ok(self.addr, &self.cred(caller), tool, args).await
    }

    async fn err(&self, caller: SessionId, tool: &str, args: Value) -> Value {
        call_err(self.addr, &self.cred(caller), tool, args).await
    }
}

/// S1 root (Claude), S2 in `a` (Copilot), S3 in `b` (Claude), plus the hidden agent worktree; a
/// second project holds S9. `extra` adds sessions to the first project.
async fn fixture_with(extra: Vec<micold_core::session::Session>) -> Fixture {
    let project = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "a");
    add_worktree(project.path(), "b");
    add_worktree(project.path(), AGENT_WORKTREE);
    init_repo(other.path());
    let mut sessions = vec![
        session(sid(1), None, TerminalMode::AiCli, AiCli::ClaudeCode),
        session(sid(2), Some("a"), TerminalMode::AiCli, AiCli::Copilot),
        session(sid(3), Some("b"), TerminalMode::AiCli, AiCli::ClaudeCode),
    ];
    sessions.extend(extra);
    let state = state_over(
        vec![
            (project.path().to_path_buf(), true, sessions),
            (
                other.path().to_path_buf(),
                true,
                vec![session(
                    sid(9),
                    None,
                    TerminalMode::AiCli,
                    AiCli::ClaudeCode,
                )],
            ),
        ],
        store.path(),
    );
    let addr = serve_tool_server(&state, store.path().join("mcp")).await;
    Fixture {
        state,
        addr,
        project,
        _other: other,
        _store: store,
    }
}

async fn fixture() -> Fixture {
    fixture_with(Vec::new()).await
}

fn refs(rows: &Value, key: &str) -> Vec<String> {
    rows[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["ref"].as_str().unwrap().to_string())
        .collect()
}

fn row<'a>(rows: &'a Value, key: &str, r: &str) -> &'a Value {
    rows[key]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["ref"] == r)
        .unwrap_or_else(|| panic!("no row {r} in {rows}"))
}

fn native(path: PathBuf) -> String {
    path.display().to_string()
}

#[tokio::test]
async fn whoami_names_the_caller_its_project_worktree_and_cli() {
    let f = fixture().await;
    let who = f.ok(sid(3), "whoami", json!({})).await;
    let name = f.repo().file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(
        who,
        json!({
            "session": sid(3).0.to_string(),
            "project": {"name": name, "path": native(f.repo().to_path_buf())},
            "worktree": "b",
            "ai_cli": "claude_code",
        })
    );
}

#[tokio::test]
async fn list_worktrees_is_default_then_the_sidebars_set() {
    let f = fixture().await;
    let out = f.ok(sid(3), "list_worktrees", json!({})).await;
    assert_eq!(refs(&out, "worktrees"), ["default", "a", "b"]);

    let default = row(&out, "worktrees", "default");
    assert_eq!(default["branch"], Value::Null);
    assert_eq!(default["path"], native(f.repo().to_path_buf()));
    assert_eq!(default["session_count"], 1);

    let b = row(&out, "worktrees", "b");
    assert_eq!(b["branch"], "b");
    assert_eq!(b["status"], "clean");
    assert_eq!(b["display_name"], "b");
    assert_eq!(
        b["app_created"],
        json!(true),
        "a worktree with sessions is the user's"
    );
    assert_eq!(b["assistant_owned"], json!(false));
    assert_eq!(
        b["path"],
        native(f.repo().join(".claude").join("worktrees").join("b"))
    );

    // The same set the sidebar's snapshot holds, less the hidden row.
    let snapshot = f.state.catalog_snapshot();
    let project = snapshot
        .projects
        .iter()
        .find(|p| p.path == f.repo())
        .unwrap();
    let mut sidebar: Vec<String> = project
        .worktrees
        .iter()
        .map(|w| w.dir_name.clone())
        .filter(|d| d != AGENT_WORKTREE)
        .collect();
    sidebar.insert(0, "default".into());
    assert_eq!(refs(&out, "worktrees"), sidebar);
}

#[tokio::test]
async fn each_worktree_counts_the_sessions_it_hosts() {
    let f = fixture_with(vec![session(
        sid(4),
        Some("a"),
        TerminalMode::AiCli,
        AiCli::Pi,
    )])
    .await;
    let out = f.ok(sid(3), "list_worktrees", json!({})).await;
    assert_eq!(row(&out, "worktrees", "default")["session_count"], 1);
    assert_eq!(row(&out, "worktrees", "a")["session_count"], 2);
    assert_eq!(row(&out, "worktrees", "b")["session_count"], 1);
}

#[tokio::test]
async fn an_assistant_owned_worktree_is_listed_only_with_include_hidden() {
    let f = fixture().await;
    let hidden = f.ok(sid(3), "list_worktrees", json!({})).await;
    assert!(!refs(&hidden, "worktrees").contains(&AGENT_WORKTREE.to_string()));

    let all = f
        .ok(sid(3), "list_worktrees", json!({"include_hidden": true}))
        .await;
    let agent = row(&all, "worktrees", AGENT_WORKTREE);
    assert_eq!(agent["assistant_owned"], json!(true));
    assert_eq!(agent["app_created"], json!(false));
}

#[tokio::test]
async fn a_worktree_whose_directory_is_gone_is_missing() {
    let f = fixture().await;
    std::fs::remove_dir_all(f.repo().join(".claude/worktrees/a")).unwrap();
    let out = f.ok(sid(3), "list_worktrees", json!({})).await;
    assert_eq!(row(&out, "worktrees", "a")["status"], "missing");
}

#[tokio::test]
async fn a_directory_git_does_not_know_is_prunable() {
    let f = fixture().await;
    let orphan = f.repo().join(".claude/worktrees/orphan");
    std::fs::create_dir_all(&orphan).unwrap();
    f.state
        .record_worktree_provenance(f.repo(), "orphan")
        .unwrap();
    let out = f.ok(sid(3), "list_worktrees", json!({})).await;
    assert_eq!(row(&out, "worktrees", "orphan")["status"], "prunable");
}

#[tokio::test]
async fn list_sessions_marks_only_the_caller() {
    let f = fixture().await;
    let out = f.ok(sid(3), "list_sessions", json!({})).await;
    let mut listed = refs(&out, "sessions");
    listed.sort();
    assert_eq!(
        listed,
        [sid(1), sid(2), sid(3)].map(|s| s.0.to_string()),
        "the other project's session is not listed"
    );
    for s in out["sessions"].as_array().unwrap() {
        assert_eq!(s["is_caller"], json!(s["ref"] == sid(3).0.to_string()));
    }
    let s2 = row(&out, "sessions", &sid(2).0.to_string());
    assert_eq!(
        *s2,
        json!({
            "ref": sid(2).0.to_string(),
            "label": "New session",
            "ai_cli": "copilot",
            "lifecycle": "idle",
            "activity": "unknown",
            "worktree": "a",
            "is_caller": false,
        })
    );
}

#[tokio::test]
async fn list_sessions_filters_by_worktree() {
    let f = fixture().await;
    let a = f
        .ok(sid(3), "list_sessions", json!({"worktree": "a"}))
        .await;
    assert_eq!(refs(&a, "sessions"), [sid(2).0.to_string()]);
    let root = f
        .ok(sid(3), "list_sessions", json!({"worktree": "default"}))
        .await;
    assert_eq!(refs(&root, "sessions"), [sid(1).0.to_string()]);
}

#[tokio::test]
async fn list_sessions_on_an_unknown_worktree_is_not_found() {
    let f = fixture().await;
    let error = f
        .err(sid(3), "list_sessions", json!({"worktree": "nope"}))
        .await;
    assert_eq!(error["category"], "not_found");
}

#[tokio::test]
async fn a_regular_terminal_session_is_listed_as_such() {
    let f = fixture_with(vec![session(
        sid(5),
        None,
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    )])
    .await;
    let out = f
        .ok(
            sid(3),
            "get_session",
            json!({"session": sid(5).0.to_string()}),
        )
        .await;
    assert_eq!(out["ai_cli"], "regular_terminal");
}

#[tokio::test]
async fn get_session_reports_a_failure_reason_only_for_a_failed_session() {
    let mut failed = session(sid(6), Some("a"), TerminalMode::AiCli, AiCli::ClaudeCode);
    failed.lifecycle = SessionLifecycle::Failed {
        reason: "claude exited three times".into(),
        attempts: 3,
    };
    let f = fixture_with(vec![failed]).await;
    let out = f
        .ok(
            sid(3),
            "get_session",
            json!({"session": sid(6).0.to_string()}),
        )
        .await;
    assert_eq!(out["lifecycle"], "failed");
    assert_eq!(out["failure_reason"], "claude exited three times");

    let idle = f
        .ok(
            sid(3),
            "get_session",
            json!({"session": sid(2).0.to_string()}),
        )
        .await;
    assert_eq!(idle["lifecycle"], "idle");
    assert!(idle.get("failure_reason").is_none(), "{idle}");
}

#[tokio::test]
async fn an_unknown_or_foreign_session_is_not_found_with_the_same_message() {
    let f = fixture().await;
    let unknown = sid(77);
    let foreign = sid(9);
    let e1 = f
        .err(
            sid(3),
            "get_session",
            json!({"session": unknown.0.to_string()}),
        )
        .await;
    let e2 = f
        .err(
            sid(3),
            "get_session",
            json!({"session": foreign.0.to_string()}),
        )
        .await;
    assert_eq!(e1["category"], "not_found");
    assert_eq!(e2["category"], "not_found");
    assert_eq!(
        e1["message"],
        format!("no session {} in this project", unknown.0)
    );
    assert_eq!(
        e2["message"],
        format!("no session {} in this project", foreign.0)
    );
}

#[tokio::test]
async fn list_branches_reports_where_each_branch_is_checked_out() {
    let f = fixture().await;
    git(f.repo(), &["branch", "free"]);
    let out = f.ok(sid(3), "list_branches", json!({})).await;
    let branch = |name: &str| -> Value {
        out["branches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == name)
            .unwrap_or_else(|| panic!("no branch {name} in {out}"))
            .clone()
    };
    assert_eq!(branch("a")["checked_out_in"], "a");
    assert_eq!(branch("a")["kind"], "local");
    assert_eq!(
        branch("a")["unavailable_reason"],
        "'a' is already checked out in the worktree 'a'."
    );
    assert_eq!(branch("main")["checked_out_in"], "default");
    assert_eq!(branch("free")["checked_out_in"], Value::Null);
    assert_eq!(branch("free")["unavailable_reason"], Value::Null);
}

#[tokio::test]
async fn list_branches_reports_a_remote_tracking_branch_as_remote() {
    let f = fixture().await;
    git(
        f.repo(),
        &["update-ref", "refs/remotes/origin/elsewhere", "HEAD"],
    );
    let out = f.ok(sid(3), "list_branches", json!({})).await;
    let remote = out["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "elsewhere")
        .unwrap_or_else(|| panic!("no remote branch in {out}"));
    assert_eq!(remote["kind"], "remote");
    assert_eq!(remote["checked_out_in"], Value::Null);
}

#[tokio::test]
async fn an_empty_project_answers_default_and_the_caller_alone() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    let state = state_over(
        vec![(
            project.path().to_path_buf(),
            true,
            vec![session(
                sid(1),
                None,
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            )],
        )],
        store.path(),
    );
    let addr = serve_tool_server(&state, store.path().join("mcp")).await;
    let cred = credential(&state, sid(1));
    let worktrees = call_ok(addr, &cred, "list_worktrees", json!({})).await;
    assert_eq!(refs(&worktrees, "worktrees"), ["default"]);
    let sessions = call_ok(addr, &cred, "list_sessions", json!({})).await;
    assert_eq!(refs(&sessions, "sessions"), [sid(1).0.to_string()]);
    let branches = call_ok(addr, &cred, "list_branches", json!({})).await;
    assert_eq!(branches["branches"][0]["name"], "main");
}

#[tokio::test]
async fn a_returned_worktree_path_is_in_the_platforms_native_form() {
    let f = fixture().await;
    let out = f.ok(sid(3), "list_worktrees", json!({})).await;
    let path = row(&out, "worktrees", "a")["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        path,
        native(f.repo().join(".claude").join("worktrees").join("a"))
    );
    if cfg!(windows) {
        assert!(!path.contains('/'), "{path}");
    }
}
