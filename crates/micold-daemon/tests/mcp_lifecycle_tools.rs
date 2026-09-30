//! Feature 034 (contracts/mcp-tools.md `start_session`, `rename_worktree`; US3 scenarios 1 and 6;
//! FR-012a, FR-015a; A12, U163–U167): an agent starts a session and renames a worktree through
//! `POST /mcp`, exactly as the sidebar would.
//!
//! The fixture is a real git repository with worktree `b`, the caller S3 in `b`, S1 in the project
//! root, and shell sessions in `b` in each lifecycle the tools must handle. Shell sessions start the
//! platform shell, so no stand-in CLI is needed.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::sync::Arc;
use std::time::Duration;

use mcp_support::*;
use micold_core::protocol::messages::{CatalogSnapshot, WireLifecycle};
use micold_core::session::{AiCli, SessionId, SessionLifecycle, TerminalMode};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// SC-003: a change reaches every window within this bound.
const WINDOW_BOUND: Duration = Duration::from_secs(2);
/// A shell starts well within this.
const START_BOUND: Duration = Duration::from_secs(20);

const CALLER: u128 = 3;
const ROOT: u128 = 1;
const IDLE: u128 = 20;
const FAILED: u128 = 21;
const RESUMABLE: u128 = 22;
const STARTING: u128 = 23;
const RESTARTING: u128 = 24;
/// A Claude Code session marked resumable whose conversation no CLI has: its start fails before
/// anything is spawned, whether or not `claude` is installed.
const UNRESUMABLE: u128 = 25;

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
        add_worktree(project.path(), "b");
        let shell = |n: u128, lifecycle: SessionLifecycle| {
            let mut s = session(sid(n), Some("b"), TerminalMode::Regular, AiCli::ClaudeCode);
            s.lifecycle = lifecycle;
            s
        };
        let sessions = vec![
            session(sid(ROOT), None, TerminalMode::AiCli, AiCli::ClaudeCode),
            session(
                sid(CALLER),
                Some("b"),
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            ),
            shell(IDLE, SessionLifecycle::Idle),
            shell(
                FAILED,
                SessionLifecycle::Failed {
                    reason: "it crashed three times".into(),
                    attempts: 3,
                },
            ),
            shell(RESUMABLE, SessionLifecycle::InterruptedResumable),
            shell(STARTING, SessionLifecycle::Starting),
            shell(RESTARTING, SessionLifecycle::Restarting { attempts: 1 }),
            {
                let mut s = session(
                    sid(UNRESUMABLE),
                    Some("b"),
                    TerminalMode::AiCli,
                    AiCli::ClaudeCode,
                );
                s.lifecycle = SessionLifecycle::InterruptedResumable;
                s
            },
        ];
        let state = state_over(
            vec![(project.path().to_path_buf(), true, sessions)],
            store.path(),
        );
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            state,
            addr,
            _project: project,
            _store: store,
        }
    }

    async fn ok(&self, caller: u128, tool: &str, args: Value) -> Value {
        call_ok(self.addr, &credential(&self.state, sid(caller)), tool, args).await
    }

    async fn err(&self, caller: u128, tool: &str, args: Value) -> Value {
        call_err(self.addr, &credential(&self.state, sid(caller)), tool, args).await
    }

    async fn start(&self, target: u128) -> Value {
        self.ok(
            CALLER,
            "start_session",
            json!({"session": sid(target).0.to_string()}),
        )
        .await
    }

    async fn lifecycle_of(&self, target: u128) -> String {
        self.ok(
            CALLER,
            "get_session",
            json!({"session": sid(target).0.to_string()}),
        )
        .await["lifecycle"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn display_name_of(&self, worktree: &str) -> String {
        let out = self
            .ok(CALLER, "list_worktrees", json!({"include_hidden": true}))
            .await;
        out["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["ref"] == worktree)
            .unwrap()["display_name"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for n in [IDLE, FAILED, RESUMABLE, STARTING, RESTARTING] {
            if let Some(pty) = self.state.live_session(sid(n)) {
                let _ = pty.kill();
            }
        }
    }
}

fn lifecycle_in(catalog: &CatalogSnapshot, id: SessionId) -> Option<WireLifecycle> {
    catalog
        .projects
        .iter()
        .flat_map(|p| p.sessions.iter())
        .find(|s| s.id == id)
        .map(|s| s.lifecycle.clone())
}

/// A12 (US3 s1): each startable session goes through `Starting` to `Running` in every window, and
/// the tool reports the running session.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_session_moves_an_idle_failed_or_resumable_session_through_starting_to_running() {
    let f = Fixture::new().await;
    for target in [IDLE, FAILED, RESUMABLE] {
        let mut window = fake_window(&f.state);
        let out = f.start(target).await;
        assert_eq!(out["lifecycle"], "running", "session {target}: {out}");
        assert!(
            f.state.live_session(sid(target)).is_some(),
            "session {target} has a process"
        );
        assert!(
            window_sees(&mut window, WINDOW_BOUND, |c| {
                lifecycle_in(c, sid(target)) == Some(WireLifecycle::Starting)
            })
            .await,
            "session {target}: every window saw it starting"
        );
        assert!(
            window_sees(&mut window, START_BOUND, |c| {
                lifecycle_in(c, sid(target)) == Some(WireLifecycle::Running)
            })
            .await,
            "session {target}: every window saw it running"
        );
    }
}

/// U163: a running session is left alone: the same process, reported running.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_session_on_a_running_session_reports_running_and_spawns_nothing() {
    let f = Fixture::new().await;
    f.start(IDLE).await;
    let before = f.state.live_session(sid(IDLE)).expect("started");
    let out = f.start(IDLE).await;
    assert_eq!(out["lifecycle"], "running");
    let after = f.state.live_session(sid(IDLE)).expect("still live");
    assert!(Arc::ptr_eq(&before, &after), "no second process");
}

/// U164, U165 (FR-012a): a session already starting or restarting succeeds unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_session_on_a_starting_or_restarting_session_succeeds_unchanged() {
    let f = Fixture::new().await;
    for (target, expected) in [(STARTING, "starting"), (RESTARTING, "restarting")] {
        let out = f.start(target).await;
        assert_eq!(out["lifecycle"], expected, "session {target}: {out}");
        assert_eq!(f.lifecycle_of(target).await, expected, "unchanged");
        assert!(
            f.state.live_session(sid(target)).is_none(),
            "session {target}: nothing was spawned"
        );
    }
}

/// A start that fails ends `Starting`: the call fails with the reason (FR-013), and every window
/// shows the failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_start_that_fails_is_reported_failed_not_left_starting() {
    let f = Fixture::new().await;
    let mut window = fake_window(&f.state);
    let error = f
        .err(
            CALLER,
            "start_session",
            json!({"session": sid(UNRESUMABLE).0.to_string()}),
        )
        .await;
    assert_eq!(error["category"], "service_error", "{error}");
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("did not start"), "{message}");
    assert!(
        !message.ends_with("its process could not be started"),
        "the recorded reason is given: {message}"
    );
    assert!(
        window_sees(&mut window, WINDOW_BOUND, |c| {
            matches!(
                lifecycle_in(c, sid(UNRESUMABLE)),
                Some(WireLifecycle::Failed { .. })
            )
        })
        .await,
        "every window sees the failure"
    );
    assert_eq!(f.lifecycle_of(UNRESUMABLE).await, "failed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_session_on_an_unknown_session_is_not_found() {
    let f = Fixture::new().await;
    let error = f
        .err(
            CALLER,
            "start_session",
            json!({"session": sid(99).0.to_string()}),
        )
        .await;
    assert_eq!(error["category"], "not_found", "{error}");
}

/// U166 (SC-003): the new name is in the tool's row, in `list_worktrees`, and in every window.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rename_worktree_changes_the_display_name_and_every_window_sees_it() {
    let f = Fixture::new().await;
    let mut window = fake_window(&f.state);
    let row = f
        .ok(
            CALLER,
            "rename_worktree",
            json!({"worktree": "b", "display_name": "  Login fix "}),
        )
        .await;
    assert_eq!(row["ref"], "b");
    assert_eq!(row["display_name"], "Login fix");
    assert_eq!(f.display_name_of("b").await, "Login fix");
    assert!(
        window_sees(&mut window, WINDOW_BOUND, |c| {
            c.projects
                .iter()
                .flat_map(|p| p.worktrees.iter())
                .any(|w| w.dir_name == "b" && w.display_name == "Login fix")
        })
        .await,
        "every window receives the rename within 2 s"
    );
}

/// U167
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rename_worktree_on_an_unknown_worktree_is_not_found() {
    let f = Fixture::new().await;
    let error = f
        .err(
            CALLER,
            "rename_worktree",
            json!({"worktree": "nope", "display_name": "X"}),
        )
        .await;
    assert_eq!(error["category"], "not_found", "{error}");
}

/// US3 s6 (FR-015a): a Default session's agent may not rename a worktree, and nothing changes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_default_session_is_refused_rename_worktree() {
    let f = Fixture::new().await;
    let before = f.display_name_of("b").await;
    let error = f
        .err(
            ROOT,
            "rename_worktree",
            json!({"worktree": "b", "display_name": "X"}),
        )
        .await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("Principle III"),
        "{error}"
    );
    assert_eq!(f.display_name_of("b").await, before, "nothing changed");
}
