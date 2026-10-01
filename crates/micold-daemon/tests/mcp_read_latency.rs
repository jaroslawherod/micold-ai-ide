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

/// The arguments each read-only tool is timed with, or `None` for a tool this file does not know.
fn arguments(tool: &str) -> Option<Value> {
    match tool {
        "whoami" | "list_worktrees" | "list_branches" | "list_sessions" => Some(json!({})),
        "get_session" => Some(json!({"session": session_id(SESSIONS - 1).0.to_string()})),
        // PHASE B (M6): `read_session_output` is read-only and ships in milestone M6, which was
        // not on main when this was written. Add its arguments here, over a session that has
        // output to read. Until then this test fails on a tree that lists the tool.
        _ => None,
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

#[tokio::test]
async fn each_read_only_tool_answers_within_a_second_at_fifty_worktrees_and_sessions() {
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

    let tools = read_only_tools(addr, &cred).await;
    for expected in [
        "whoami",
        "list_worktrees",
        "list_branches",
        "list_sessions",
        "get_session",
    ] {
        assert!(
            tools.iter().any(|t| t == expected),
            "{expected} is not listed as read-only: {tools:?}"
        );
    }

    let mut slow = Vec::new();
    for tool in &tools {
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
            _ => {}
        }
    }
    assert!(
        slow.is_empty(),
        "SC-004: these read-only tools took {BUDGET:?} or longer with {WORKTREES} worktrees and \
         {SESSIONS} sessions: {slow:?}"
    );
}
