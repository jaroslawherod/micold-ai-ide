//! Feature 1289, contract C4 (A7): the `create_worktree` result says whether setup started.
// unix-only: the setup command `true` is a POSIX shell builtin; on Windows setup runs in PowerShell
#![cfg(unix)]

use crate::mcp_support;
use std::sync::Arc;

use mcp_support::*;
use micold_core::session::{AiCli, TerminalMode};
use micold_core::setup::SetupCommand;
use micold_daemon::state::DaemonState;
use serde_json::json;

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
        let state = state_over(
            vec![(
                project.path().to_path_buf(),
                true,
                vec![session(
                    sid(3),
                    Some("b"),
                    TerminalMode::AiCli,
                    AiCli::ClaudeCode,
                )],
            )],
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

    async fn create(&self, branch: &str) -> serde_json::Value {
        call_ok(
            self.addr,
            &credential(&self.state, sid(3)),
            "create_worktree",
            json!({ "branch": branch }),
        )
        .await
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_result_says_setup_started_when_the_project_has_a_command() {
    let f = Fixture::new().await;
    f.state
        .set_setup_command(f.project.path(), SetupCommand::parse("true"));
    let row = f.create("feat-x").await;
    assert_eq!(row["setup"], "started", "{row}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_result_says_none_without_a_command_and_keeps_next_step() {
    let f = Fixture::new().await;
    let without = f.create("feat-a").await;
    assert_eq!(without["setup"], "none", "{without}");
    f.state
        .set_setup_command(f.project.path(), SetupCommand::parse("true"));
    let with = f.create("feat-b").await;
    assert_eq!(
        with["next_step"], without["next_step"],
        "next_step is unchanged by setup"
    );
    assert!(
        with["next_step"]
            .as_str()
            .is_some_and(|s| s.contains("create_session")),
        "{with}"
    );
}
