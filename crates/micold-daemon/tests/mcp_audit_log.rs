//! Feature 034 (FR-018, SC-010; U202–U205): every mutating tool call writes exactly one `info` line
//! naming the caller, the operation, its target and the outcome, and never the text an agent sends.
//!
//! The mutating tools are read from `tools/list` (`readOnlyHint: false`), so a tool a later
//! milestone ships is covered as soon as it is listed: [`good_call`] and [`failing_call`] then name
//! the arguments for it, or the test fails saying which tool is missing.
//!
//! Each test calls as its own session, so the lines it counts are its own even though every test in
//! this binary shares one log.

// unix-only: `create_session` starts a stand-in `#!/bin/sh` CLI (Windows port is a follow-up).
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

/// What `send_session_input` types into a sibling; it must never reach the log either.
const TEXT_MARKER: &str = "text-marker-9e02b5";

/// The second session of a fixture: a sibling of `caller` in the same worktree, for the tools
/// that target another session.
fn sibling_of(caller: SessionId) -> SessionId {
    sid(caller.0.as_u128() + 1_000)
}

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
        // Copilot's own record that it trusts every fixture's project (an ancestor counts):
        // `send_session_input` types nothing into a CLI that would ask about its folder.
        std::fs::create_dir_all(home.path().join(".copilot")).unwrap();
        let trusted = json!({"trustedFolders": [std::env::temp_dir()]});
        std::fs::write(
            home.path().join(".copilot/config.json"),
            trusted.to_string(),
        )
        .unwrap();
        (bin, home)
    });
}

struct Fixture {
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
    _project: tempfile::TempDir,
    _store: tempfile::TempDir,
}

/// Sibling sessions of `caller` in worktree `b`: a shell the destructive tools act on, and one to
/// delete.
fn sibling(caller: SessionId, n: u128) -> SessionId {
    sid(caller.0.as_u128() + n)
}
const SHELL: u128 = 100;
const DOOMED: u128 = 200;

/// A repository with worktrees `b` and `c` and a free branch `taken`; `caller` is a session in
/// `b` beside its [`sibling`]s and the AI sibling [`sibling_of`] names. A window answers every prompt with Allow.
async fn fixture(caller: SessionId) -> Fixture {
    log();
    sandbox_env();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "b");
    add_worktree(project.path(), "c");
    git(project.path(), &["branch", "taken"]);
    let state = state_over(
        vec![(
            project.path().to_path_buf(),
            true,
            vec![
                session(caller, Some("b"), TerminalMode::AiCli, AiCli::Copilot),
                session(
                    sibling(caller, SHELL),
                    Some("b"),
                    TerminalMode::Regular,
                    AiCli::Copilot,
                ),
                session(
                    sibling(caller, DOOMED),
                    Some("b"),
                    TerminalMode::Regular,
                    AiCli::Copilot,
                ),
                session(
                    sibling_of(caller),
                    Some("b"),
                    TerminalMode::AiCli,
                    AiCli::Copilot,
                ),
            ],
        )],
        store.path(),
    );
    let addr = serve_tool_server(&state, store.path().join("mcp")).await;
    allow_every_prompt(&state);
    Fixture {
        state,
        addr,
        _project: project,
        _store: store,
    }
}

/// A window that allows every prompt it is shown (FR-014), so a destructive call can succeed.
fn allow_every_prompt(state: &Arc<DaemonState>) {
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::DaemonMsg;
    let mut rx = fake_window(state);
    let state = Arc::clone(state);
    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if let Frame::Control(DaemonMsg::ConfirmationRequested { id, .. }) = frame {
                state.answer_confirmation(id, true);
            }
        }
    });
}

impl Fixture {
    async fn call(&self, caller: SessionId, tool: &str, args: Value) -> Value {
        call_tool(self.addr, &credential(&self.state, caller), tool, args).await
    }

    /// What `good_call` for `tool` needs to be true first. `send_session_input` types into a
    /// running sibling and only a running session can be interrupted, so the target is started
    /// here, before the caller's lines are counted.
    async fn prepare(&self, tool: &str, caller: SessionId) {
        let target = match tool {
            "send_session_input" => sibling_of(caller),
            "interrupt_session" => sibling(caller, SHELL),
            _ => return,
        };
        let started = self
            .call(
                caller,
                "start_session",
                json!({"session": target.0.to_string()}),
            )
            .await;
        assert_eq!(started["isError"], json!(false), "{started}");
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

/// Arguments with which `tool` succeeds for `caller`, a session in worktree `b`.
fn good_call(tool: &str, caller: SessionId) -> Value {
    match tool {
        "create_worktree" => json!({"branch": "audit-ok"}),
        "rename_worktree" => json!({"worktree": "b", "display_name": "Audited"}),
        "create_session" => json!({"worktree": "b", "ai_cli": "copilot"}),
        "start_session" => json!({"session": caller.0.to_string()}),
        // An idle session: a no-op success, asked of nobody.
        "stop_session" => json!({"session": sibling(caller, SHELL).0.to_string()}),
        // Running: `prepare` starts it.
        "interrupt_session" => json!({"session": sibling(caller, SHELL).0.to_string()}),
        "delete_session" => json!({"session": sibling(caller, DOOMED).0.to_string()}),
        "delete_worktree" => json!({"worktree": "c"}),
        "send_session_input" => {
            json!({"session": sibling_of(caller).0.to_string(), "text": TEXT_MARKER})
        }
        other => panic!("no successful call for the mutating tool {other}: add one here"),
    }
}

/// The target an audit line names for a call with `args`: the session, else the worktree, else
/// the branch a new worktree is made from.
fn expected_target(args: &Value) -> String {
    ["session", "worktree", "branch"]
        .iter()
        .find_map(|k| args[*k].as_str())
        .expect("every call names a session, a worktree or a branch")
        .to_string()
}

/// Arguments with which `tool` fails for `caller`, and the category it fails with.
fn failing_call(tool: &str, caller: SessionId) -> Vec<(Value, ErrorCategory)> {
    match tool {
        "create_worktree" => vec![
            (json!({}), ErrorCategory::InvalidInput),
            (json!({"branch": "taken"}), ErrorCategory::Conflict),
        ],
        "rename_worktree" => vec![
            (json!({"worktree": "b"}), ErrorCategory::InvalidInput),
            (
                json!({"worktree": "nope", "display_name": "X"}),
                ErrorCategory::NotFound,
            ),
        ],
        "create_session" => vec![(json!({"worktree": "nope"}), ErrorCategory::NotFound)],
        "start_session" => vec![(
            json!({"session": sid(99).0.to_string()}),
            ErrorCategory::NotFound,
        )],
        "delete_worktree" => vec![
            (json!({"worktree": "nope"}), ErrorCategory::NotFound),
            // The caller's own worktree (FR-015).
            (json!({"worktree": "b"}), ErrorCategory::RefusedByPolicy),
        ],
        "stop_session" | "delete_session" => vec![
            (
                json!({"session": sid(99).0.to_string()}),
                ErrorCategory::NotFound,
            ),
            (
                json!({"session": caller.0.to_string()}),
                ErrorCategory::RefusedByPolicy,
            ),
        ],
        "interrupt_session" => vec![(
            json!({"session": caller.0.to_string()}),
            ErrorCategory::InvalidInput,
        )],
        "send_session_input" => vec![
            (
                json!({"session": sid(99).0.to_string()}),
                ErrorCategory::InvalidInput,
            ),
            (
                json!({"session": sid(99).0.to_string(), "text": TEXT_MARKER}),
                ErrorCategory::NotFound,
            ),
        ],
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
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(key.as_str()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_successful_mutating_call_writes_one_info_line() {
    let caller = sid(71);
    let f = fixture(caller).await;
    let tools = f.mutating_tools(caller).await;
    assert!(!tools.is_empty(), "M3 ships mutating tools");
    for tool in &tools {
        f.prepare(tool, caller).await;
        let before = audit_lines(caller).len();
        let args = good_call(tool, caller);
        let result = f.call(caller, tool, args.clone()).await;
        assert_eq!(result["isError"], json!(false), "{tool}: {result}");
        let lines = audit_lines(caller);
        assert_eq!(
            lines.len(),
            before + 1,
            "exactly one line for {tool}: {lines:#?}"
        );
        let line = lines.last().unwrap();
        assert!(line.contains(" INFO "), "logged at info: {line}");
        assert_eq!(field(line, "op"), Some(tool.as_str()), "{line}");
        assert_eq!(field(line, "outcome"), Some("ok"), "{line}");
        assert_eq!(
            field(line, "target"),
            Some(expected_target(&args).as_str()),
            "the line names what {tool} acted on: {line}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_failed_mutating_call_writes_one_line_with_its_category() {
    let caller = sid(72);
    let f = fixture(caller).await;
    let tools = f.mutating_tools(caller).await;
    assert!(!tools.is_empty(), "M3 ships mutating tools");
    for tool in tools {
        for (args, category) in failing_call(&tool, caller) {
            let before = audit_lines(caller).len();
            let result = f.call(caller, &tool, args.clone()).await;
            assert_eq!(result["isError"], json!(true), "{tool} {args}: {result}");
            let lines = audit_lines(caller);
            assert_eq!(
                lines.len(),
                before + 1,
                "exactly one line for {tool} {args}"
            );
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

/// FR-018 for `send_session_input`: one line naming the sibling as the target, and the text in no
/// line of the log, whether the send is delivered or refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_sent_text_never_reaches_the_log() {
    use micold_core::mcp::policy::CrossSessionAccess;

    let caller = sid(75);
    let f = fixture(caller).await;
    f.prepare("send_session_input", caller).await;
    let sibling = sibling_of(caller).0.to_string();
    let send = json!({"session": sibling, "text": TEXT_MARKER});

    let before = audit_lines(caller).len();
    let result = f.call(caller, "send_session_input", send.clone()).await;
    assert_eq!(result["isError"], json!(false), "{result}");
    f.state
        .set_cross_session_access(CrossSessionAccess::Off)
        .unwrap();
    let result = f.call(caller, "send_session_input", send).await;
    assert_eq!(result["isError"], json!(true), "{result}");

    let lines = audit_lines(caller);
    assert_eq!(lines.len(), before + 2, "one line per send: {lines:#?}");
    let [delivered, refused] = &lines[before..] else {
        unreachable!("two lines were just counted")
    };
    assert_eq!(field(delivered, "outcome"), Some("ok"), "{delivered}");
    assert_eq!(
        field(refused, "outcome"),
        Some("refused_by_policy"),
        "{refused}"
    );
    for line in [delivered, refused] {
        assert_eq!(field(line, "op"), Some("send_session_input"), "{line}");
        assert_eq!(field(line, "target"), Some(sibling.as_str()), "{line}");
    }
    let logged = String::from_utf8_lossy(&log().lock().unwrap()).into_owned();
    assert!(
        !logged.contains(TEXT_MARKER),
        "a sent text reached the log (FR-018)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_logged_failure_carries_one_of_the_six_categories() {
    let caller = sid(74);
    let f = fixture(caller).await;
    let tools = f.mutating_tools(caller).await;
    assert!(!tools.is_empty(), "M3 ships mutating tools");
    // Each call, in order, with the category its failure must be logged under.
    let mut expected = Vec::new();
    for tool in tools {
        for (args, category) in failing_call(&tool, caller) {
            let result = f.call(caller, &tool, args.clone()).await;
            assert_eq!(result["isError"], json!(true), "{tool} {args}: {result}");
            expected.push((tool.clone(), category));
        }
        // Arguments of the wrong type.
        let result = f
            .call(caller, &tool, json!({"worktree": 7, "branch": 7}))
            .await;
        assert_eq!(result["isError"], json!(true), "{tool}: {result}");
        expected.push((tool.clone(), ErrorCategory::InvalidInput));
    }
    let lines = audit_lines(caller);
    assert_eq!(lines.len(), expected.len(), "one line per call: {lines:#?}");
    for (line, (tool, category)) in lines.iter().zip(&expected) {
        let outcome = field(line, "outcome").unwrap_or_default();
        assert!(
            CATEGORIES.iter().any(|c| c.as_str() == outcome),
            "an outcome outside the six categories: {line}"
        );
        assert_eq!(field(line, "op"), Some(tool.as_str()), "{line}");
        assert_eq!(outcome, category.as_str(), "{tool}: {line}");
    }
}
