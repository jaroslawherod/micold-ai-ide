//! Spec 1857 (`contracts/mcp-parent.md`, Visibility; FR-015, FR-016, FR-017): a caller outside
//! `default` sees and addresses only its own worktree, its parent, its siblings and its direct
//! children; anything else is `not_found`, and the way to reach it is to message the parent.
//!
//! Fixture: `P` (no parent) with children `A` and `B`; `C`, a child of `A`; `X` (no parent) with
//! child `Y`. Each has one session; `default` has S1.

use crate::mcp_support;

use std::sync::Arc;

use mcp_support::*;
use micold_core::mcp::policy::CrossSessionAccess;
use micold_core::session::{AiCli, TerminalMode};
use micold_core::worktree_parent::ParentChoice;
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

const DEFAULT: u128 = 1;
const P: u128 = 10;
const A: u128 = 11;
const B: u128 = 12;
const C: u128 = 13;
const X: u128 = 14;
const Y: u128 = 15;

/// Worktree directory and session of each fixture member.
const MEMBERS: [(&str, u128); 6] = [("P", P), ("A", A), ("B", B), ("C", C), ("X", X), ("Y", Y)];
/// Child, parent.
const LINKS: [(&str, &str); 4] = [("A", "P"), ("B", "P"), ("C", "A"), ("Y", "X")];

struct Fixture {
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
    _project: tempfile::TempDir,
    _store: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let mut sessions = vec![session(
            sid(DEFAULT),
            None,
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        )];
        for (dir, id) in MEMBERS {
            add_worktree(project.path(), dir);
            sessions.push(session(
                sid(id),
                Some(dir),
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            ));
        }
        let state = state_over(
            vec![(project.path().to_path_buf(), true, sessions)],
            store.path(),
        );
        state
            .set_cross_session_access(CrossSessionAccess::Auto)
            .unwrap();
        state.refresh_worktrees(project.path());
        for (child, parent) in LINKS {
            state.record_worktree_parent(
                project.path(),
                child,
                Some(&ParentChoice::Worktree(parent.into())),
            );
        }
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            state,
            addr,
            _project: project,
            _store: store,
        }
    }

    async fn call(&self, caller: u128, tool: &str, args: Value) -> Value {
        call_ok(self.addr, &credential(&self.state, sid(caller)), tool, args).await
    }

    async fn err(&self, caller: u128, tool: &str, args: Value) -> Value {
        call_err(self.addr, &credential(&self.state, sid(caller)), tool, args).await
    }

    async fn error_category(&self, caller: u128, tool: &str, args: Value) -> String {
        let error = self.err(caller, tool, args).await;
        error["category"].as_str().unwrap().to_string()
    }

    /// The refs `list_worktrees` returns to `caller`, `default` included.
    async fn worktrees(&self, caller: u128) -> Vec<String> {
        let out = self.call(caller, "list_worktrees", json!({})).await;
        sorted(out["worktrees"].as_array().unwrap(), "ref")
    }

    /// The worktrees of the sessions `list_sessions` returns to `caller`.
    async fn session_worktrees(&self, caller: u128) -> Vec<String> {
        let out = self.call(caller, "list_sessions", json!({})).await;
        sorted(out["sessions"].as_array().unwrap(), "worktree")
    }
}

fn sorted(rows: &[Value], key: &str) -> Vec<String> {
    let mut refs: Vec<String> = rows
        .iter()
        .map(|r| r[key].as_str().unwrap().to_string())
        .collect();
    refs.sort();
    refs
}

fn session_arg(id: u128) -> Value {
    json!({"session": sid(id).0.to_string()})
}

#[tokio::test]
async fn a_worktree_with_a_parent_lists_itself_its_parent_siblings_and_children() {
    let f = Fixture::new().await;
    let expected = ["A", "B", "C", "P"];
    assert_eq!(f.worktrees(A).await, expected);
    assert_eq!(f.session_worktrees(A).await, expected);
}

#[tokio::test]
async fn an_unparented_worktree_sees_itself_and_its_children() {
    let f = Fixture::new().await;
    assert_eq!(f.worktrees(X).await, ["X", "Y"]);
    assert_eq!(f.session_worktrees(X).await, ["X", "Y"]);
    assert_eq!(f.worktrees(P).await, ["A", "B", "P"]);
}

#[tokio::test]
async fn a_default_caller_sees_everything() {
    let f = Fixture::new().await;
    assert_eq!(
        f.worktrees(DEFAULT).await,
        ["A", "B", "C", "P", "X", "Y", "default"]
    );
    assert_eq!(f.session_worktrees(DEFAULT).await.len(), 7);
}

#[tokio::test]
async fn every_tool_reports_an_out_of_scope_target_as_not_found() {
    let f = Fixture::new().await;
    let missing = |tool: &'static str, args: Value| (tool, args);
    let cases = [
        missing("get_session", session_arg(X)),
        missing("read_session_output", session_arg(X)),
        missing(
            "send_session_input",
            json!({"session": sid(X).0.to_string(), "text": "hi"}),
        ),
        missing("interrupt_session", session_arg(X)),
        missing("stop_session", session_arg(X)),
        missing("start_session", session_arg(X)),
        missing("create_session", json!({"worktree": "X"})),
        missing("create_session", json!({"worktree": "default"})),
        missing("get_session", session_arg(DEFAULT)),
        missing(
            "rename_worktree",
            json!({"worktree": "X", "display_name": "n"}),
        ),
        missing("delete_worktree", json!({"worktree": "X"})),
    ];
    for (tool, args) in cases {
        assert_eq!(
            f.error_category(A, tool, args.clone()).await,
            "not_found",
            "{tool} {args}"
        );
    }
    let hidden = f.err(A, "list_sessions", json!({"worktree": "X"})).await;
    assert_eq!(hidden["category"], "not_found", "{hidden}");
    assert!(
        f.call(A, "list_worktrees", json!({"parent": "X"})).await["worktrees"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn a_message_to_an_out_of_scope_session_is_refused_while_the_parent_is_reachable() {
    let f = Fixture::new().await;
    let to = |id: u128| json!({"session": sid(id).0.to_string(), "text": "please tell X"});
    assert_eq!(
        f.error_category(A, "send_session_input", to(X)).await,
        "not_found"
    );
    // The parent is in scope (FR-017): the call gets past the scope and fails only because the
    // fixture's sessions run no process; the real relay is covered by `mcp_cross_session`.
    assert_eq!(
        f.error_category(A, "send_session_input", to(P)).await,
        "conflict"
    );
}

#[tokio::test]
async fn an_explicit_parent_outside_the_callers_set_is_invalid_input() {
    let f = Fixture::new().await;
    for parent in ["default", "X", "nope"] {
        let args = json!({"branch": "fresh", "parent": parent});
        assert_eq!(
            f.error_category(A, "create_worktree", args).await,
            "invalid_input",
            "{parent}"
        );
    }
    let made = f
        .call(
            A,
            "create_worktree",
            json!({"branch": "kid", "parent": "B"}),
        )
        .await;
    assert_eq!(made["parent"], "B", "a sibling is in the set: {made}");
    let root = f
        .call(
            DEFAULT,
            "create_worktree",
            json!({"branch": "any", "parent": "X"}),
        )
        .await;
    assert_eq!(root["parent"], "X", "default names any: {root}");
}
