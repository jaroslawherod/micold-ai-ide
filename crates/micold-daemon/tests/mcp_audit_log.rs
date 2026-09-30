//! Feature 034 (FR-018, SC-010; U202–U205): every mutating tool call writes exactly one `info` line
//! naming the caller, the operation, its target and the outcome, and never the text an agent sends.
//!
//! The mutating tools are read from `tools/list` (`readOnlyHint: false`), so a tool a later
//! milestone ships is covered as soon as it is listed: [`good_call`] and [`failing_call`] then name
//! the arguments for it, or the test fails saying which tool is missing.
//!
//! Each test calls as its own session, so the lines it counts are its own even though every test in
//! this binary shares one log.

// unix-only: `create_session` starts a stand-in `#!/bin/sh` CLI (Windows port is a recorded
// follow-up).
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mcp_support::*;
use micold_core::mcp::errors::ErrorCategory;
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// What a first prompt says; it must never reach the log.
const PROMPT_MARKER: &str = "prompt-marker-4c1d77";

/// The six categories a failure may carry (contracts/mcp-tools.md).
const CATEGORIES: [ErrorCategory; 6] = [
    ErrorCategory::NotFound,
    ErrorCategory::InvalidInput,
    ErrorCategory::Conflict,
    ErrorCategory::RefusedByPolicy,
    ErrorCategory::NeedsConfirmation,
    ErrorCategory::ServiceError,
];

/// A stand-in `copilot` on `PATH` and a throwaway home, set once for the whole binary: the audit
/// tests start real sessions, and nothing they start may touch the user's own configuration.
fn sandbox_env() {
    static DIRS: OnceLock<(tempfile::TempDir, tempfile::TempDir)> = OnceLock::new();
    DIRS.get_or_init(|| {
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let cli = bin.path().join("copilot");
        std::fs::write(&cli, "#!/bin/sh\nprintf 'ready> '\nexec cat > /dev/null\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut path = vec![bin.path().to_path_buf()];
        path.extend(["/usr/bin", "/bin"].map(PathBuf::from));
        std::env::set_var("PATH", std::env::join_paths(path).unwrap());
        std::env::set_var("HOME", home.path());
        std::env::set_var("XDG_DATA_HOME", home.path().join(".local/share"));
        std::env::remove_var("COPILOT_HOME");
        (bin, home)
    });
}

struct Fixture {
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
    _project: tempfile::TempDir,
    _store: tempfile::TempDir,
}

/// A repository with worktree `b` and a free branch `taken`; `caller` is a session in `b`.
async fn fixture(caller: SessionId) -> Fixture {
    log();
    sandbox_env();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "b");
    git(project.path(), &["branch", "taken"]);
    let state = state_over(
        vec![(
            project.path().to_path_buf(),
            true,
            vec![session(caller, Some("b"), TerminalMode::AiCli, AiCli::Copilot)],
        )],
        store.path(),
    );
    let addr = serve_tool_server(&state, store.path().join("mcp")).await;
    Fixture {
        state,
        addr,
        _project: project,
        _store: store,
    }
}

impl Fixture {
    async fn call(&self, caller: SessionId, tool: &str, args: Value) -> Value {
        call_tool(self.addr, &credential(&self.state, caller), tool, args).await
    }

    /// The names `tools/list` marks as not read-only.
    async fn mutating_tools(&self, caller: SessionId) -> Vec<String> {
        let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
        let (status, body) = post_mcp(
            self.addr,
            Some(&credential(&self.state, caller)),
            &request.to_string(),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        let list: Value = serde_json::from_str(&body).unwrap();
        list["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["annotations"]["readOnlyHint"] == json!(false))
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    }
}

/// Arguments with which `tool` succeeds for a caller in worktree `b`.
fn good_call(tool: &str) -> Value {
    match tool {
        "create_worktree" => json!({"branch": "audit-ok"}),
        "create_session" => json!({"worktree": "b", "ai_cli": "copilot"}),
        other => panic!("no successful call for the mutating tool {other}: add one here"),
    }
}

/// Arguments with which `tool` fails, and the category it fails with.
fn failing_call(tool: &str) -> Vec<(Value, ErrorCategory)> {
    match tool {
        "create_worktree" => vec![
            (json!({}), ErrorCategory::InvalidInput),
            (json!({"branch": "taken"}), ErrorCategory::Conflict),
        ],
        "create_session" => vec![(json!({"worktree": "nope"}), ErrorCategory::NotFound)],
        other => panic!("no failing call for the mutating tool {other}: add one here"),
    }
}

/// The audit lines `caller`'s calls wrote.
fn audit_lines(caller: SessionId) -> Vec<String> {
    log_lines_for(caller)
        .into_iter()
        .filter(|line| line.contains(" micold::mcp:"))
        .collect()
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=");
    line.split_whitespace().find_map(|w| w.strip_prefix(key.as_str()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_successful_mutating_call_writes_one_info_line() {
    let caller = sid(71);
    let f = fixture(caller).await;
    let tools = f.mutating_tools(caller).await;
    assert!(!tools.is_empty(), "M3 ships mutating tools");
    for tool in &tools {
        let before = audit_lines(caller).len();
        let result = f.call(caller, tool, good_call(tool)).await;
        assert_eq!(result["isError"], json!(false), "{tool}: {result}");
        let lines = audit_lines(caller);
        assert_eq!(lines.len(), before + 1, "exactly one line for {tool}: {lines:#?}");
        let line = lines.last().unwrap();
        assert!(line.contains(" INFO "), "logged at info: {line}");
        assert_eq!(field(line, "op"), Some(tool.as_str()), "{line}");
        assert_eq!(field(line, "outcome"), Some("ok"), "{line}");
        assert!(field(line, "target").is_some(), "{line}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_failed_mutating_call_writes_one_line_with_its_category() {
    let caller = sid(72);
    let f = fixture(caller).await;
    for tool in f.mutating_tools(caller).await {
        for (args, category) in failing_call(&tool) {
            let before = audit_lines(caller).len();
            let result = f.call(caller, &tool, args.clone()).await;
            assert_eq!(result["isError"], json!(true), "{tool} {args}: {result}");
            let lines = audit_lines(caller);
            assert_eq!(lines.len(), before + 1, "exactly one line for {tool} {args}");
            let line = lines.last().unwrap();
            assert!(line.contains(" INFO "), "logged at info: {line}");
            assert_eq!(field(line, "op"), Some(tool.as_str()), "{line}");
            assert_eq!(field(line, "outcome"), Some(category.as_str()), "{line}");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_prompt_never_reaches_the_log() {
    let caller = sid(73);
    let f = fixture(caller).await;
    let result = f
        .call(
            caller,
            "create_session",
            json!({"worktree": "b", "ai_cli": "copilot", "prompt": PROMPT_MARKER}),
        )
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(audit_lines(caller).len(), 1);
    let logged = String::from_utf8_lossy(&log().lock().unwrap()).into_owned();
    assert!(
        !logged.contains(PROMPT_MARKER),
        "a prompt reached the log (FR-018)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_logged_failure_carries_one_of_the_six_categories() {
    let caller = sid(74);
    let f = fixture(caller).await;
    for tool in f.mutating_tools(caller).await {
        for (args, _) in failing_call(&tool) {
            f.call(caller, &tool, args).await;
        }
        // Arguments of the wrong type.
        f.call(caller, &tool, json!({"worktree": 7, "branch": 7}))
            .await;
    }
    let lines = audit_lines(caller);
    assert!(!lines.is_empty());
    for line in lines {
        let outcome = field(&line, "outcome").unwrap_or_default();
        assert!(
            outcome == "ok" || CATEGORIES.iter().any(|c| c.as_str() == outcome),
            "an outcome outside the six categories: {line}"
        );
    }
}
