//! Feature 034, SC-004 (U206): with 50 worktrees and 50 sessions, each read-only tool answers in
//! under 1 second.
//!
//! The fixture is a real git repository with 50 worktrees, each hosting one session, and the tools
//! are called through `POST /mcp` as a bound session calls them. The clock covers the whole call:
//! connect, request, the tool, and the reply. `list_branches` runs git off the state lock and
//! `list_worktrees` reads each worktree's state, so both do real work at this size.
//!
//! The tools timed are the ones `tools/list` marks read-only, not a list kept here. A read-only
//! tool this file has no arguments for fails the test, so a tool added later cannot go untimed.
//!
//! `read_session_output` needs a running session with output to read. On unix one session is
//! started on a stand-in `claude` that prints more lines than a read may return, and the read asks
//! for the maximum. The stand-in is a `#!/bin/sh` script, so on other platforms that one tool is
//! skipped, like the other MCP tests that start a session.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::session::{AiCli, SessionId, TerminalMode};
use serde_json::{json, Value};

const WORKTREES: usize = 50;
const SESSIONS: usize = 50;
/// SC-004's bound.
const BUDGET: Duration = Duration::from_secs(1);
/// Calls per tool. The first is timed like the rest: an agent's first call is a call.
const ROUNDS: usize = 5;

fn worktree_name(i: usize) -> String {
    format!("wt-{i:02}")
}

/// Session `i` lives in worktree `i`; ids start at 1.
fn session_id(i: usize) -> SessionId {
    sid(i as u128 + 1)
}

/// The session `read_session_output` reads: not the caller, and the one that is started.
const READ: usize = 1;
/// The most lines one read returns (contracts/mcp-tools.md).
const MAX_LINES: usize = 2_000;
/// Lines the stand-in prints before its prompt: more than one read returns.
#[cfg(unix)]
const PRINTED: usize = 3_000;

/// The arguments each read-only tool is timed with, or `None` for a tool this file does not know.
fn arguments(tool: &str) -> Option<Value> {
    match tool {
        "whoami"
        | "list_worktrees"
        | "list_branches"
        | "list_sessions"
        | "list_resumable_sessions" => Some(json!({})),
        "get_session" => Some(json!({"session": session_id(SESSIONS - 1).0.to_string()})),
        "read_session_output" => Some(json!({
            "session": session_id(READ).0.to_string(),
            "lines": MAX_LINES,
        })),
        _ => None,
    }
}

/// Whether `tool` can be timed on this platform.
fn timed_here(tool: &str) -> bool {
    cfg!(unix) || tool != "read_session_output"
}

/// A stand-in `claude` on `PATH` that prints `PRINTED` lines and a prompt, then waits. Holds the
/// directories it lives in; this file has one test, so the variables are not restored.
#[cfg(unix)]
struct StandInCli {
    _bin: tempfile::TempDir,
    _home: tempfile::TempDir,
}

#[cfg(unix)]
impl StandInCli {
    fn install() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let path = bin.path().join("claude");
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\n\
                 i=1\n\
                 while [ \"$i\" -le {PRINTED} ]; do printf 'out%s\\n' \"$i\"; i=$((i + 1)); done\n\
                 printf 'ready> '\n\
                 exec cat > /dev/null\n"
            ),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut dirs = vec![bin.path().to_path_buf()];
        dirs.extend(["/usr/bin", "/bin"].map(std::path::PathBuf::from));
        std::env::set_var("PATH", std::env::join_paths(dirs).unwrap());
        std::env::set_var("HOME", home.path());
        std::env::set_var("XDG_DATA_HOME", home.path().join(".local/share"));
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        Self {
            _bin: bin,
            _home: home,
        }
    }
}

/// Start session `READ` and wait until everything it prints is on its terminal.
#[cfg(unix)]
async fn start_the_read_session(
    state: &std::sync::Arc<micold_daemon::state::DaemonState>,
    addr: std::net::SocketAddr,
    bearer: &str,
) {
    let target = session_id(READ);
    call_ok(
        addr,
        bearer,
        "start_session",
        json!({"session": target.0.to_string()}),
    )
    .await;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let (Some(pty), Some(framer)) = (state.primary_pty(target), state.primary_framer(target))
        {
            let last = framer.lock().unwrap().plain_tail(pty.term(), 1).0;
            if last.last().map(String::as_str) == Some("ready>") {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "the stand-in never drew its prompt"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Kills the PTYs of session `READ` when dropped, so a panic still cleans up.
#[cfg(unix)]
struct Reaper(std::sync::Arc<micold_daemon::state::DaemonState>);

#[cfg(unix)]
impl Drop for Reaper {
    fn drop(&mut self) {
        for pty in self.0.session_ptys(session_id(READ)) {
            let _ = pty.kill();
        }
    }
}

/// Every tool `tools/list` marks read-only, in the order it lists them.
async fn read_only_tools(addr: std::net::SocketAddr, bearer: &str) -> Vec<String> {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
    let (status, body) = post_mcp(addr, Some(bearer), &request.to_string()).await;
    assert_eq!(status, 200, "tools/list: {body}");
    let response: Value = serde_json::from_str(&body).expect("JSON response");
    response["result"]["tools"]
        .as_array()
        .expect("a tool list")
        .iter()
        .filter(|tool| tool["annotations"]["readOnlyHint"] == json!(true))
        .map(|tool| tool["name"].as_str().expect("a tool name").to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_read_only_tool_answers_within_a_second_at_fifty_worktrees_and_sessions() {
    #[cfg(unix)]
    let _cli = StandInCli::install();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    for i in 0..WORKTREES {
        add_worktree(project.path(), &worktree_name(i));
    }
    let sessions = (0..SESSIONS)
        .map(|i| {
            session(
                session_id(i),
                Some(&worktree_name(i % WORKTREES)),
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            )
        })
        .collect();
    let state = state_over(
        vec![(project.path().to_path_buf(), true, sessions)],
        store.path(),
    );
    let addr = serve_tool_server(&state, store.path().join("mcp")).await;
    let cred = credential(&state, session_id(0));
    // Kills the started session even if an assertion below panics.
    #[cfg(unix)]
    let _reaper = Reaper(std::sync::Arc::clone(&state));

    let tools = read_only_tools(addr, &cred).await;
    for expected in [
        "whoami",
        "list_worktrees",
        "list_branches",
        "list_sessions",
        "get_session",
        "read_session_output",
    ] {
        assert!(
            tools.iter().any(|t| t == expected),
            "{expected} is not listed as read-only: {tools:?}"
        );
    }
    #[cfg(unix)]
    start_the_read_session(&state, addr, &cred).await;

    let mut slow = Vec::new();
    for tool in tools.iter().filter(|tool| timed_here(tool)) {
        let args = arguments(tool).unwrap_or_else(|| {
            panic!("no arguments for the read-only tool {tool}: time it here (SC-004)")
        });
        let mut times = Vec::with_capacity(ROUNDS);
        let mut last = Value::Null;
        for _ in 0..ROUNDS {
            let started = Instant::now();
            last = call_ok(addr, &cred, tool, args.clone()).await;
            times.push(started.elapsed());
        }
        let worst = *times.iter().max().unwrap();
        println!(
            "SC-004 {tool}: worst {}ms, all {:?}",
            worst.as_millis(),
            times.iter().map(Duration::as_millis).collect::<Vec<_>>()
        );
        if worst >= BUDGET {
            slow.push((tool.clone(), worst));
        }

        // The answer is the full-size one, so the time is the time of the claim.
        match tool.as_str() {
            "list_worktrees" => assert_eq!(
                last["worktrees"].as_array().unwrap().len(),
                WORKTREES + 1,
                "default and every worktree: {last}"
            ),
            "list_sessions" => {
                assert_eq!(last["sessions"].as_array().unwrap().len(), SESSIONS)
            }
            "list_branches" => assert!(
                last["branches"].as_array().unwrap().len() > WORKTREES,
                "main and a branch per worktree: {last}"
            ),
            "read_session_output" => {
                let lines = last["lines"].as_array().unwrap();
                assert_eq!(lines.len(), MAX_LINES, "the most a read returns");
                assert_eq!(lines.last(), Some(&json!("ready>")), "{last}");
            }
            _ => {}
        }
    }
    assert!(
        slow.is_empty(),
        "SC-004: these read-only tools took {BUDGET:?} or longer with {WORKTREES} worktrees and \
         {SESSIONS} sessions: {slow:?}"
    );
}
