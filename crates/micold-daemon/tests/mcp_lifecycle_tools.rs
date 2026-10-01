//! Feature 034 (contracts/mcp-tools.md; US3; FR-009, FR-011, FR-012a, FR-014, FR-015, FR-015a):
//! an agent starts, stops, interrupts and deletes sessions and renames and deletes worktrees
//! through `POST /mcp`, exactly as the sidebar would.
//!
//! Part 1 (M4; A12, U163–U167): `start_session`, `rename_worktree`. Part 2 (M5; A13–A18,
//! U168–U171, U186–U193): the destructive tools, each of which waits for a window's answer. A
//! test window answers every prompt it is shown ([`answering_window`]) and counts them, so "no
//! prompt was raised" is observable.
//!
//! The fixture is a real git repository with worktrees `b`, `c` and `d`, the caller S3 in `b`, S1
//! in the project root, shell sessions in `b` in each lifecycle the tools must handle, a shell
//! session in `c`, and nothing in `d`. Shell sessions start the platform shell, so no stand-in CLI
//! is needed.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::SinkExt;
use mcp_support::*;
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientInstance, ClientMsg, DaemonMsg, WireLifecycle,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
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
/// An idle shell session in worktree `c`.
const IN_C: u128 = 30;

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
        add_worktree(project.path(), "c");
        add_worktree(project.path(), "d");
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
            session(
                sid(IN_C),
                Some("c"),
                TerminalMode::Regular,
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

    fn project(&self) -> &std::path::Path {
        self._project.path()
    }

    fn worktree_dir(&self, name: &str) -> std::path::PathBuf {
        self.project().join(".claude/worktrees").join(name)
    }

    fn has_branch(&self, name: &str) -> bool {
        let out = std::process::Command::new("git")
            .args(["branch", "--list", name])
            .current_dir(self.project())
            .output()
            .unwrap();
        !String::from_utf8_lossy(&out.stdout).trim().is_empty()
    }

    async fn worktree_refs(&self) -> Vec<String> {
        self.ok(CALLER, "list_worktrees", json!({"include_hidden": true}))
            .await["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["ref"].as_str().unwrap().to_string())
            .collect()
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
        for n in [IDLE, FAILED, RESUMABLE, STARTING, RESTARTING, IN_C] {
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

// ---------------------------------------------------------------------------------------------
// Part 2 (M5): the destructive tools.
// ---------------------------------------------------------------------------------------------

fn target(n: u128) -> Value {
    json!({"session": sid(n).0.to_string()})
}

/// A window that answers every prompt it is shown with `allow`; returns how many it was shown.
fn answering_window(state: &Arc<DaemonState>, allow: bool) -> Arc<AtomicUsize> {
    let mut rx = fake_window(state);
    let shown = Arc::new(AtomicUsize::new(0));
    let (st, count) = (Arc::clone(state), Arc::clone(&shown));
    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if let Frame::Control(DaemonMsg::ConfirmationRequested { id, .. }) = frame {
                count.fetch_add(1, Ordering::SeqCst);
                st.answer_confirmation(id, allow);
            }
        }
    });
    shown
}

/// The next prompt a window is shown, within 5 s.
async fn next_prompt(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<Frame<DaemonMsg>>,
) -> (u64, DaemonMsg) {
    let wait = async {
        while let Some(frame) = rx.recv().await {
            if let Frame::Control(msg @ DaemonMsg::ConfirmationRequested { .. }) = frame {
                let DaemonMsg::ConfirmationRequested { id, .. } = &msg else {
                    unreachable!()
                };
                return (*id, msg);
            }
        }
        panic!("window closed");
    };
    tokio::time::timeout(Duration::from_secs(5), wait)
        .await
        .expect("a prompt within 5 s")
}

/// The platform shell's visible screen.
fn visible_text(pty: &micold_daemon::supervisor::PtySession) -> String {
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Line};
    let term = pty.term().lock();
    let grid = term.grid();
    let mut out = String::new();
    for line in 0..grid.screen_lines() {
        for col in 0..grid.columns() {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

async fn screen_shows(
    pty: &micold_daemon::supervisor::PtySession,
    pred: impl Fn(&str) -> bool,
) -> bool {
    for _ in 0..300 {
        if pred(&visible_text(pty)) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

/// A13 (US3 s2; U169, U170): an allowed stop ends the processes, every window shows the session
/// `Idle`, its credential still answers, and it can be started again.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_allowed_stop_session_ends_it_shows_idle_everywhere_and_it_stays_resumable() {
    let f = Fixture::new().await;
    f.start(IDLE).await;
    let key = credential(&f.state, sid(IDLE));
    let asked = answering_window(&f.state, true);
    let mut window = fake_window(&f.state);

    let out = f.ok(CALLER, "stop_session", target(IDLE)).await;

    assert_eq!(out["lifecycle"], "idle", "{out}");
    assert_eq!(asked.load(Ordering::SeqCst), 1, "the user was asked once");
    assert!(f.state.live_session(sid(IDLE)).is_none(), "no process left");
    assert!(
        window_sees(&mut window, WINDOW_BOUND, |c| {
            lifecycle_in(c, sid(IDLE)) == Some(WireLifecycle::Idle)
        })
        .await,
        "every window sees it idle within 2 s (SC-003)"
    );
    assert_eq!(
        f.state.tool_server().unwrap().session_for(&key),
        Some(sid(IDLE)),
        "a stopped session keeps its credential"
    );
    assert_eq!(
        f.start(IDLE).await["lifecycle"],
        "running",
        "it starts again"
    );
}

/// U186 (FR-012a, assumption A-5): stopping an idle session changes nothing and asks nobody.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_session_on_an_idle_session_succeeds_unchanged_without_a_prompt() {
    let f = Fixture::new().await;
    let asked = answering_window(&f.state, true);
    let out = f.ok(CALLER, "stop_session", target(IDLE)).await;
    assert_eq!(out["lifecycle"], "idle", "{out}");
    assert_eq!(asked.load(Ordering::SeqCst), 0, "no prompt for a no-op");
}

/// A14 (US3 s2): an allowed interrupt types `0x03` into the primary terminal and the session keeps
/// running. The shell is put where the byte is readable: no signal characters, no line buffering,
/// and `od` printing the one byte it reads.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_allowed_interrupt_session_types_ctrl_c_and_leaves_it_running() {
    let f = Fixture::new().await;
    f.start(IDLE).await;
    let pty = f.state.primary_pty(sid(IDLE)).expect("running");
    // The user's shell may take a while to read its startup files; wait until it runs commands.
    pty.write_input(b"echo ready-$((40+2))\r").unwrap();
    assert!(
        screen_shows(&pty, |s| s.contains("ready-42")).await,
        "the shell runs commands:\n{}",
        visible_text(&pty)
    );
    pty.write_input(b"stty -isig -icanon; dd bs=1 count=1 2>/dev/null | od -An -tx1\r")
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let asked = answering_window(&f.state, true);

    let out = f.ok(CALLER, "interrupt_session", target(IDLE)).await;

    assert_eq!(out, json!({}));
    assert_eq!(asked.load(Ordering::SeqCst), 1, "the user was asked once");
    assert!(
        screen_shows(&pty, |s| s.lines().any(|l| l.trim_end().ends_with(" 03"))).await,
        "0x03 reached the terminal:\n{}",
        visible_text(&pty)
    );
    let still = f.state.primary_pty(sid(IDLE)).expect("still live");
    assert!(Arc::ptr_eq(&pty, &still), "the same process");
    assert_eq!(f.lifecycle_of(IDLE).await, "running");
}

/// Review A (M5): the approval is for the process the user was asked about. A session stopped and
/// started again while the prompt waits is another process, and the interrupt does not reach it.
// unix-only: the fixture's sessions are `#!/bin/sh` stand-ins.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_interrupt_allowed_after_the_process_was_replaced_is_a_conflict() {
    let f = Fixture::new().await;
    f.start(IDLE).await;
    let asked_about = f.state.primary_pty(sid(IDLE)).expect("running");
    let mut window = fake_window(&f.state);
    let (addr, key) = (f.addr, credential(&f.state, sid(CALLER)));
    let call =
        tokio::spawn(async move { call_err(addr, &key, "interrupt_session", target(IDLE)).await });
    let (id, _) = next_prompt(&mut window).await;

    assert!(f.state.stop_session(sid(IDLE)));
    f.start(IDLE).await;
    let replaced = f.state.primary_pty(sid(IDLE)).expect("running again");
    assert!(!Arc::ptr_eq(&asked_about, &replaced), "another process");
    f.state.answer_confirmation(id, true);

    let error = tokio::time::timeout(Duration::from_secs(5), call)
        .await
        .expect("a reply within 5 s")
        .unwrap();
    assert_eq!(error["category"], "conflict", "{error}");
    assert_eq!(f.lifecycle_of(IDLE).await, "running");
}

/// U187 (FR-012a): interrupting a session that is not running is a conflict, asked of nobody.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupt_session_on_a_session_that_is_not_running_is_a_conflict() {
    let f = Fixture::new().await;
    let asked = answering_window(&f.state, true);
    let error = f.err(CALLER, "interrupt_session", target(IDLE)).await;
    assert_eq!(error["category"], "conflict", "{error}");
    assert_eq!(asked.load(Ordering::SeqCst), 0);
}

/// A15 (US3 s3; FR-009): a worktree with a live session is not deleted unless stopping it was
/// asked for; the refusal names the session, nobody is asked, and nothing changes.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_worktree_with_live_sessions_and_no_stop_sessions_is_a_conflict_naming_them() {
    let f = Fixture::new().await;
    f.start(IN_C).await;
    let asked = answering_window(&f.state, true);

    let error = f
        .err(CALLER, "delete_worktree", json!({"worktree": "c"}))
        .await;

    assert_eq!(error["category"], "conflict", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains(&sid(IN_C).0.to_string()),
        "names the live session: {error}"
    );
    assert_eq!(asked.load(Ordering::SeqCst), 0, "no prompt for a conflict");
    assert!(f.worktree_dir("c").is_dir(), "the worktree is still there");
    assert!(f.has_branch("c"), "its branch is still there");
    assert!(
        f.state.live_session(sid(IN_C)).is_some(),
        "its session runs"
    );
    assert!(f.worktree_refs().await.contains(&"c".to_string()));
}

/// U188, U189 (FR-009, FR-011): an allowed delete with `stop_sessions` stops the worktree's
/// sessions, removes it and, by default, its branch; every window sees it go.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_allowed_delete_worktree_stops_its_sessions_and_removes_it_and_its_branch() {
    let f = Fixture::new().await;
    f.start(IN_C).await;
    let asked = answering_window(&f.state, true);
    let mut window = fake_window(&f.state);

    let out = f
        .ok(
            CALLER,
            "delete_worktree",
            json!({"worktree": "c", "stop_sessions": true}),
        )
        .await;

    assert_eq!(out["removed"], "c", "{out}");
    assert_eq!(out["branch_deleted"], true, "{out}");
    assert_eq!(out["leftovers"], json!([]), "{out}");
    assert_eq!(asked.load(Ordering::SeqCst), 1, "the user was asked once");
    assert!(
        f.state.live_session(sid(IN_C)).is_none(),
        "its session stopped"
    );
    assert!(!f.worktree_dir("c").exists(), "the worktree is gone");
    assert!(!f.has_branch("c"), "its branch is gone");
    assert!(!f.worktree_refs().await.contains(&"c".to_string()));
    assert!(
        window_sees(&mut window, WINDOW_BOUND, |c| {
            c.projects
                .iter()
                .all(|p| p.worktrees.iter().all(|w| w.dir_name != "c"))
        })
        .await,
        "every window sees it gone"
    );
}

/// U190: `delete_branch: false` keeps the branch.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_allowed_delete_worktree_without_delete_branch_keeps_the_branch() {
    let f = Fixture::new().await;
    answering_window(&f.state, true);
    let out = f
        .ok(
            CALLER,
            "delete_worktree",
            json!({"worktree": "d", "delete_branch": false}),
        )
        .await;
    assert_eq!(out["removed"], "d", "{out}");
    assert_eq!(out["branch_deleted"], false, "{out}");
    assert!(!f.worktree_dir("d").exists());
    assert!(f.has_branch("d"), "the branch is kept");
}

/// U191, U192 (FR-006, EC-8): an allowed delete archives the session and its credential stops
/// answering.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_allowed_delete_session_archives_it_and_revokes_its_credential() {
    let f = Fixture::new().await;
    let key = credential(&f.state, sid(FAILED));
    let asked = answering_window(&f.state, true);

    let out = f.ok(CALLER, "delete_session", target(FAILED)).await;

    assert_eq!(out, json!({}));
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    let error = f.err(CALLER, "get_session", target(FAILED)).await;
    assert_eq!(error["category"], "not_found", "archived: {error}");
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).to_string();
    let (status, _) = post_mcp(f.addr, Some(&key), &request).await;
    assert_eq!(status, 401, "the deleted session's credential is refused");
}

/// U193 (EC-4): of two agents deleting one worktree, the one asked second is told it is gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn of_two_agents_deleting_one_worktree_the_second_is_not_found() {
    let f = Fixture::new().await;
    let mut window = fake_window(&f.state);
    let args = json!({"worktree": "d"});
    let first = {
        let (addr, key, args) = (f.addr, credential(&f.state, sid(CALLER)), args.clone());
        tokio::spawn(async move { call_tool(addr, &key, "delete_worktree", args).await })
    };
    let (first_id, _) = next_prompt(&mut window).await;
    let second = {
        let (addr, key, args) = (f.addr, credential(&f.state, sid(IDLE)), args.clone());
        tokio::spawn(async move { call_tool(addr, &key, "delete_worktree", args).await })
    };
    let (_second_id, _) = next_prompt(&mut window).await;

    f.state.answer_confirmation(first_id, true);

    let first = first.await.unwrap();
    assert_eq!(first["isError"], json!(false), "{first}");
    let second = second.await.unwrap();
    assert_eq!(second["isError"], json!(true), "{second}");
    assert_eq!(
        second["structuredContent"]["error"]["category"], "not_found",
        "{second}"
    );
}

/// A16 (US3 s4; FR-014): each destructive tool on another target asks every window first, and
/// nothing changes while it waits or when the user declines.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_destructive_tool_waits_for_the_user_and_a_decline_changes_nothing() {
    let f = Fixture::new().await;
    f.start(IDLE).await;
    let running = f.state.primary_pty(sid(IDLE)).expect("running");
    let cases = [
        ("stop_session", target(IDLE)),
        ("interrupt_session", target(IDLE)),
        ("delete_session", target(FAILED)),
        (
            "delete_worktree",
            json!({"worktree": "d", "stop_sessions": true}),
        ),
    ];
    for (tool, args) in cases {
        let mut window = fake_window(&f.state);
        let call = {
            let (addr, key, args) = (f.addr, credential(&f.state, sid(CALLER)), args.clone());
            tokio::spawn(async move { call_tool(addr, &key, tool, args).await })
        };
        let (id, prompt) = next_prompt(&mut window).await;
        let DaemonMsg::ConfirmationRequested {
            caller,
            target_label,
            ..
        } = &prompt
        else {
            unreachable!()
        };
        assert_eq!(*caller, sid(CALLER), "{tool}: names the caller");
        assert!(!target_label.is_empty(), "{tool}: names the target");
        // Waiting: nothing has changed yet.
        assert!(!call.is_finished(), "{tool} waits for the answer");
        assert!(f.state.live_session(sid(IDLE)).is_some(), "{tool}");
        f.state.answer_confirmation(id, false);
        let result = call.await.unwrap();
        assert_eq!(result["isError"], json!(true), "{tool}: {result}");
        let error = &result["structuredContent"]["error"];
        assert_eq!(error["category"], "refused_by_policy", "{tool}: {error}");
        assert_eq!(error["message"], "declined by the user", "{tool}");
    }
    let still = f.state.primary_pty(sid(IDLE)).expect("still running");
    assert!(Arc::ptr_eq(&running, &still), "the same process");
    assert_eq!(f.lifecycle_of(FAILED).await, "failed", "not deleted");
    assert!(f.worktree_dir("d").is_dir(), "not deleted");
}

/// FR-014: with no window to ask, a destructive request needs confirmation and changes nothing.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_no_window_a_destructive_request_needs_confirmation() {
    let f = Fixture::new().await;
    f.start(IDLE).await;
    let error = f.err(CALLER, "stop_session", target(IDLE)).await;
    assert_eq!(error["category"], "needs_confirmation", "{error}");
    assert!(f.state.live_session(sid(IDLE)).is_some(), "still running");
}

/// A17 (US3 s5; FR-015): a session may not stop or delete itself or delete its own worktree, nor
/// interrupt itself; nobody is asked and nothing changes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn self_targets_are_refused_without_a_prompt() {
    let f = Fixture::new().await;
    let asked = answering_window(&f.state, true);
    for (tool, args, category) in [
        ("stop_session", target(CALLER), "refused_by_policy"),
        ("delete_session", target(CALLER), "refused_by_policy"),
        (
            "delete_worktree",
            json!({"worktree": "b"}),
            "refused_by_policy",
        ),
        (
            "delete_worktree",
            json!({"worktree": "b", "stop_sessions": true}),
            "refused_by_policy",
        ),
        ("interrupt_session", target(CALLER), "invalid_input"),
    ] {
        let error = f.err(CALLER, tool, args.clone()).await;
        assert_eq!(error["category"], category, "{tool} {args}: {error}");
    }
    assert_eq!(asked.load(Ordering::SeqCst), 0, "no prompt for a refusal");
    assert!(f.worktree_dir("b").is_dir());
    assert_eq!(f.lifecycle_of(CALLER).await, "idle");
}

/// A18 (US3 s6; FR-015a): a Default session may not delete a worktree; nobody is asked.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_default_session_is_refused_delete_worktree() {
    let f = Fixture::new().await;
    let asked = answering_window(&f.state, true);
    let error = f
        .err(
            ROOT,
            "delete_worktree",
            json!({"worktree": "d", "stop_sessions": true}),
        )
        .await;
    assert_eq!(error["category"], "refused_by_policy", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("Principle III"),
        "{error}"
    );
    assert_eq!(asked.load(Ordering::SeqCst), 0);
    assert!(f.worktree_dir("d").is_dir(), "nothing changed");
}

/// Scope (FR-010): an unknown target is not found before anybody is asked.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_target_is_not_found_without_a_prompt() {
    let f = Fixture::new().await;
    let asked = answering_window(&f.state, true);
    for (tool, args) in [
        ("stop_session", target(99)),
        ("interrupt_session", target(99)),
        ("delete_session", target(99)),
        ("delete_worktree", json!({"worktree": "nope"})),
    ] {
        let error = f.err(CALLER, tool, args).await;
        assert_eq!(error["category"], "not_found", "{tool}: {error}");
    }
    assert_eq!(asked.load(Ordering::SeqCst), 0);
}

type Client = tokio_util::codec::Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = tokio_util::codec::Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    client
}

/// U168, U171: the protocol's `SessionStop` ends the processes and now tells every window the
/// session is `Idle`.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_protocol_session_stop_ends_the_processes_and_broadcasts_idle() {
    let f = Fixture::new().await;
    f.start(IDLE).await;
    let mut window = fake_window(&f.state);
    let mut client = connect(&f.state).await;

    client
        .send(Frame::Control(ClientMsg::SessionStop {
            session: sid(IDLE),
        }))
        .await
        .unwrap();

    assert!(
        window_sees(&mut window, WINDOW_BOUND, |c| {
            lifecycle_in(c, sid(IDLE)) == Some(WireLifecycle::Idle)
        })
        .await,
        "every window sees it idle"
    );
    assert!(f.state.live_session(sid(IDLE)).is_none(), "no process left");
    drop(client);
}

/// Review A: stopping a `Failed` session keeps it `Failed` with its reason, and stopping a deleted
/// (archived) session reports it unknown instead of reviving its lifecycle.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_keeps_a_failed_session_failed_and_does_not_touch_a_deleted_one() {
    let f = Fixture::new().await;

    assert!(
        f.state.stop_session(sid(FAILED)),
        "a failed session is known"
    );
    assert!(matches!(
        lifecycle_in(&f.state.catalog_snapshot(), sid(FAILED)),
        Some(WireLifecycle::Failed { .. })
    ));

    f.state.delete_session(sid(IDLE)).unwrap();
    assert!(
        !f.state.stop_session(sid(IDLE)),
        "a deleted session is not stopped"
    );
}
