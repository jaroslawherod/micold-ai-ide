//! Feature 034 (contracts/mcp-tools.md `create_session`; US2 scenarios 3–5; FR-017; A9–A11,
//! U152–U162): an agent creates and starts a session, optionally typing its first prompt once the
//! new session's CLI is ready for it.
//!
//! Stand-in `claude`, `copilot` and `pi` on `PATH` record their launch, and append what is typed
//! into their terminal to `<bin>/<command>.input`. `claude` and `copilot` draw a prompt; `pi` draws
//! nothing and instead reports `session_start` through the activity component's log after
//! `<bin>/pi.delay` seconds, so only the component's event can make it ready.

// unix-only: the stand-in CLIs are `#!/bin/sh` scripts (Windows port is a recorded follow-up).
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::cli_reason::{self, AttemptDir, Place, SpawnEnv};
use micold_core::mcp::submission::SETTLE_AFTER;
use micold_core::protocol::messages::{ActivitySignal, CatalogSnapshot};
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_daemon::hooks::{self, HookReceiver};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// The tests change process-wide variables (`PATH`, `HOME`, …); one at a time.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// SC-003: a change reaches every window within this bound.
const WINDOW_BOUND: Duration = Duration::from_secs(2);

const PROMPT: &str = "print the branch name";

/// Every variable a test changes, restored on drop.
struct Env {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl Env {
    fn set(vars: &[(&'static str, Option<OsString>)]) -> Self {
        let saved = vars
            .iter()
            .map(|(name, value)| {
                let previous = std::env::var_os(name);
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// Stand-in CLIs on a `PATH` that holds nothing else of the user's, a fake home whose Claude and
/// Copilot records trust the project, a project with worktrees `b` and `gone` (whose directory has
/// been removed), S1 in the project root and S3 in `b`.
struct Sandbox {
    _env: Env,
    bin: tempfile::TempDir,
    home: tempfile::TempDir,
    project: tempfile::TempDir,
    store: tempfile::TempDir,
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
}

impl Sandbox {
    async fn new() -> Self {
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        install_cli(bin.path(), "claude", true);
        install_cli(bin.path(), "copilot", true);
        install_cli(bin.path(), "pi", false);
        init_repo(project.path());
        add_worktree(project.path(), "b");
        let gone = add_worktree(project.path(), "gone");
        std::fs::remove_dir_all(gone).unwrap();
        trust_project(home.path(), project.path());
        let mut path = vec![bin.path().to_path_buf()];
        path.extend(["/usr/bin", "/bin"].map(PathBuf::from));
        let env = Env::set(&[
            ("PATH", Some(std::env::join_paths(path).unwrap())),
            ("HOME", Some(home.path().into())),
            (
                "XDG_DATA_HOME",
                Some(home.path().join(".local/share").into()),
            ),
            ("CLAUDE_CONFIG_DIR", None),
            ("COPILOT_HOME", None),
            ("PI_CODING_AGENT_DIR", Some(home.path().join(".pi").into())),
            // The service runs on this computer, whatever started the test run (037, A14).
            ("MICOLD_IMAGE_REFERENCE", None),
        ]);
        let state = state_over(
            vec![(
                project.path().to_path_buf(),
                true,
                vec![
                    session(sid(1), None, TerminalMode::AiCli, AiCli::ClaudeCode),
                    session(sid(3), Some("b"), TerminalMode::AiCli, AiCli::ClaudeCode),
                ],
            )],
            store.path(),
        );
        state.set_pi_activity_component(true).unwrap();
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            _env: env,
            bin,
            home,
            project,
            store,
            state,
            addr,
        }
    }

    async fn call(&self, tool: &str, args: Value) -> Value {
        call_tool(self.addr, &credential(&self.state, sid(3)), tool, args).await
    }

    async fn ok(&self, args: Value) -> Value {
        call_ok(
            self.addr,
            &credential(&self.state, sid(3)),
            "create_session",
            args,
        )
        .await
    }

    async fn err(&self, args: Value) -> Value {
        call_err(
            self.addr,
            &credential(&self.state, sid(3)),
            "create_session",
            args,
        )
        .await
    }

    /// What was typed into `command`'s terminal so far.
    fn typed(&self, command: &str) -> String {
        std::fs::read_to_string(self.bin.path().join(format!("{command}.input")))
            .unwrap_or_default()
    }

    /// How many times `command` was launched.
    fn launches(&self, command: &str) -> usize {
        std::fs::read_to_string(self.bin.path().join(format!("{command}.launches")))
            .map(|body| body.lines().count())
            .unwrap_or(0)
    }

    /// Wait until `command` was launched `expected` times. `create_session` returns once the
    /// process is spawned; the stand-in writes its launch line a moment later, as its first act.
    async fn launched(&self, command: &str, expected: usize) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.launches(command) != expected {
            assert!(
                Instant::now() < deadline,
                "{command} was launched {} times, expected {expected}",
                self.launches(command)
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Remove the Claude and Copilot trust records, so both would ask about every folder.
    fn distrust(&self) {
        std::fs::remove_file(self.home.path().join(".claude.json")).unwrap();
        std::fs::remove_file(self.home.path().join(".copilot/config.json")).unwrap();
    }

    fn set_pi_delay(&self, seconds: f64) {
        std::fs::write(self.bin.path().join("pi.delay"), seconds.to_string()).unwrap();
    }

    fn session_ids(&self) -> Vec<SessionId> {
        self.state
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| p.sessions.iter().map(|s| s.id))
            .collect()
    }

    /// The session `create_session` answered with.
    fn created(out: &Value) -> SessionId {
        SessionId::from_uuid(out["session"].as_str().unwrap().parse().unwrap())
    }

    fn kill_all(&self) {
        for id in self.session_ids() {
            if let Some(pty) = self.state.live_session(id) {
                let _ = pty.kill();
            }
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        self.kill_all();
        let _ = &self.store;
    }
}

/// Each CLI's own record that it trusts `project` (and so every worktree below it), as the real
/// ones write it: `~/.claude.json` and `~/.copilot/config.json` with its leading comment.
fn trust_project(home: &Path, project: &Path) {
    let claude = json!({"projects": {project.to_str().unwrap(): {"hasTrustDialogAccepted": true}}});
    std::fs::write(home.join(".claude.json"), claude.to_string()).unwrap();
    std::fs::create_dir_all(home.join(".copilot")).unwrap();
    let copilot = json!({"trustedFolders": [project]});
    std::fs::write(
        home.join(".copilot/config.json"),
        format!("// This file is managed automatically.\n{copilot}"),
    )
    .unwrap();
}

/// A stand-in CLI: records its launch; `draws` makes it print a prompt; Pi reports `session_start`
/// through the component's log after `<bin>/pi.delay` seconds (0.3 by default). Everything typed
/// into it is appended to `<bin>/<command>.input`.
fn install_cli(bin: &Path, command: &str, draws: bool) {
    let dir = bin.display();
    let draw = if draws { "printf 'ready> '\n" } else { "" };
    let report = if command == "pi" {
        format!(
            "( sleep \"$(cat '{dir}/pi.delay' 2>/dev/null || echo 0.3)\"; \
             printf '{{\"type\":\"session_start\"}}\\n' >> \"$MICOLD_PI_ACTIVITY_LOG\" ) &\n"
        )
    } else {
        String::new()
    };
    let path = bin.join(command);
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{dir}/{command}.launches'\n{draw}{report}\
             exec cat >> '{dir}/{command}.input'\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn session_in(
    catalog: &CatalogSnapshot,
    id: SessionId,
) -> Option<&micold_core::protocol::messages::SessionSummary> {
    catalog
        .projects
        .iter()
        .flat_map(|p| p.sessions.iter())
        .find(|s| s.id == id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_worktree_then_create_session_types_the_first_prompt() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let mut window = fake_window(&s.state);
    let row = s.call("create_worktree", json!({"branch": "feat-x"})).await;
    assert_eq!(row["isError"], json!(false), "{row}");

    let out = s
        .ok(json!({"worktree": "feat-x", "ai_cli": "claude_code", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    let id = Sandbox::created(&out);
    assert!(
        window_sees(&mut window, WINDOW_BOUND, |c| session_in(c, id)
            .is_some_and(|s| s.worktree_dir.as_deref() == Some("feat-x")))
        .await,
        "every window sees the new session (SC-003)"
    );
    assert_eq!(
        s.typed("claude"),
        format!("{PROMPT}\n"),
        "the prompt is the session's first input, submitted once"
    );
    let rows = s.call("list_sessions", json!({"worktree": "feat-x"})).await;
    assert_eq!(
        rows["structuredContent"]["sessions"][0]["ai_cli"],
        "claude_code"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_ai_cli_the_session_runs_the_default_cli_from_settings() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.state.set_default_ai_cli(AiCli::Copilot).unwrap();
    let out = s.ok(json!({"worktree": "b"})).await;
    let id = Sandbox::created(&out);
    let row = s
        .call("get_session", json!({"session": id.0.to_string()}))
        .await;
    assert_eq!(row["structuredContent"]["ai_cli"], "copilot", "{row}");
    s.launched("copilot", 1).await;
    assert_eq!(s.launches("claude"), 0);
}

/// The `{reason} {action}` an agent is told when `cli` would not be found in `cwd` (037, W4 U5).
fn refusal(cli: AiCli, env: SpawnEnv, cwd: &Path) -> String {
    let said = cli_reason::explain(&[cli], env, Place::ThisComputer, AttemptDir::Dir(cwd))
        .expect("one CLI is missing, so there is an explanation");
    format!("{} {}", said.reason, said.action)
}

/// A14 (037, FR-009a): `create_session` for a CLI the target directory's environment lacks is
/// refused with the reason for the state that environment is in, and leaves no session record.
///
/// It said `Pi is not installed where this session would run`, whatever had kept the CLI off the
/// session's `PATH`. With environment-include off, or with a script that failed, nothing has
/// established that, and an agent told so cannot tell the user what to change (FR-002).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cli_the_directorys_environment_lacks_is_refused_with_the_reason_and_no_record() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    std::fs::remove_file(s.bin.path().join("pi")).unwrap();
    let before = s.session_ids();

    // Environment-include off: sessions get only the service's own PATH.
    s.state.set_env_include(Some(false), None, None).unwrap();
    let worktree = s.project.path().join(".claude/worktrees/b");
    let error = s.err(json!({"worktree": "b", "ai_cli": "pi"})).await;
    assert_eq!(error["category"], "service_error", "{error}");
    let message = error["message"].as_str().unwrap();
    assert_eq!(
        message,
        refusal(AiCli::Pi, SpawnEnv::IncludeOff, &worktree),
        "{error}"
    );
    assert!(
        !message.contains("is not installed"),
        "nothing established that it is not installed (FR-009a): {error}"
    );
    assert_eq!(s.session_ids(), before, "no session record is left behind");

    // On, with a script that exits with an error: the reason is the directory's own attempt.
    let script = s.store.path().join("env-include.sh");
    std::fs::write(&script, "exit 3\n").unwrap();
    s.state
        .set_env_include(
            Some(true),
            Some(script.to_string_lossy().into_owned()),
            None,
        )
        .unwrap();
    let error = s.err(json!({"worktree": "default", "ai_cli": "pi"})).await;
    assert_eq!(error["category"], "service_error", "{error}");
    let message = error["message"].as_str().unwrap();
    assert_eq!(
        message,
        refusal(AiCli::Pi, SpawnEnv::ScriptFailed, s.project.path()),
        "{error}"
    );
    assert!(
        message.contains(&s.project.path().display().to_string()),
        "names the directory the script failed for (FR-004a): {error}"
    );
    assert_eq!(s.session_ids(), before, "no session record is left behind");
}

/// 037 M2 review B, F1 (D18): the refusal tells the agent what to fix, so the call made after the
/// fix has to look again.
///
/// The refusal was written from the directory's cached resolution and left it there. An agent
/// told to fix the script that fixed it and asked again was refused in the same words, until an
/// environment-include field was saved. A refused start in the pane already dropped it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_session_after_the_script_is_fixed_sources_it_again_and_starts() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    std::fs::remove_file(s.bin.path().join("pi")).unwrap();
    let before = s.session_ids();

    let script = s.store.path().join("env-include.sh");
    std::fs::write(&script, "exit 3\n").unwrap();
    s.state
        .set_env_include(
            Some(true),
            Some(script.to_string_lossy().into_owned()),
            None,
        )
        .unwrap();
    let error = s.err(json!({"worktree": "default", "ai_cli": "pi"})).await;
    assert_eq!(
        error["message"].as_str().unwrap(),
        refusal(AiCli::Pi, SpawnEnv::ScriptFailed, s.project.path()),
        "{error}"
    );

    // The fix the refusal asked for: the script now runs, and puts Pi on the PATH. No setting is
    // saved, so nothing but the refusal itself can have dropped the failed attempt.
    let extra = tempfile::tempdir().unwrap();
    install_cli(extra.path(), "pi", false);
    std::fs::write(
        &script,
        format!("export PATH='{}':\"$PATH\"\n", extra.path().display()),
    )
    .unwrap();

    let out = s.ok(json!({"worktree": "default", "ai_cli": "pi"})).await;
    let id = Sandbox::created(&out);
    assert!(!before.contains(&id), "a new session was created: {out}");
    assert!(s.session_ids().contains(&id), "{out}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_session_in_default_runs_in_the_project_root() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let out = s
        .ok(json!({"worktree": "default", "ai_cli": "copilot"}))
        .await;
    let id = Sandbox::created(&out);
    let summary = session_in(&s.state.catalog_snapshot(), id)
        .cloned()
        .unwrap();
    assert_eq!(summary.worktree_dir, None, "the project root");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unknown_worktree_is_not_found_and_creates_no_record() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let before = s.session_ids();
    let error = s
        .err(json!({"worktree": "nope", "ai_cli": "copilot"}))
        .await;
    assert_eq!(error["category"], "not_found", "{error}");
    assert_eq!(s.session_ids(), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_a_prompt_the_call_does_not_wait_for_readiness() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let asked = Instant::now();
    let out = s.ok(json!({"worktree": "b", "ai_cli": "copilot"})).await;
    assert_eq!(out["prompt_delivered"], Value::Null, "{out}");
    assert!(
        asked.elapsed() < SETTLE_AFTER,
        "returned before the CLI could have settled: {:?}",
        asked.elapsed()
    );
    assert!(out["lifecycle"].is_string(), "{out}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_ready_signal_inside_the_bound_delivers_the_prompt() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.state.set_first_prompt_bound(Duration::from_secs(3));
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "copilot", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert_eq!(s.typed("copilot"), format!("{PROMPT}\n"));
    assert_eq!(
        out.get("prompt_reason"),
        None,
        "no reason when delivered: {out}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_ready_signal_by_the_bound_is_not_delivered_and_a_late_one_types_nothing() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.state.set_first_prompt_bound(Duration::from_secs(1));
    s.set_pi_delay(2.0);
    let asked = Instant::now();
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "pi", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(false), "{out}");
    assert!(
        out["prompt_reason"].as_str().unwrap().contains("ready"),
        "says why: {out}"
    );
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "answers at the bound, not at the late signal"
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(
        s.typed("pi"),
        "",
        "a signal after the bound delivers nothing (FR-017)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_session_that_fails_to_start_reports_the_prompt_undelivered() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let out = s
        .ok(json!({"worktree": "gone", "ai_cli": "copilot", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(false), "{out}");
    assert!(
        out["prompt_reason"]
            .as_str()
            .unwrap()
            .contains("did not start"),
        "says why: {out}"
    );
    assert_eq!(out["lifecycle"], "failed", "{out}");
}

/// Claude Code posts no `SessionStart` over an HTTP hook (quickstart §B3,
/// `evidence/m3-real-cli.md`), so with the hook receiver running it is still ready by the
/// output-settled rule, and a `SessionStart` post, should one come, moves nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn with_the_hook_receiver_running_claude_is_ready_once_its_output_settles() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let (receiver, listener) = HookReceiver::bind(s.store.path().join("hooks"))
        .await
        .unwrap();
    let registry = receiver.tokens();
    let hook_addr = listener.local_addr().unwrap();
    tokio::spawn(hooks::serve(
        listener,
        receiver.tokens(),
        Arc::clone(&s.state),
    ));
    s.state.set_hooks(receiver);

    let asked = Instant::now();
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "claude_code", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert!(
        asked.elapsed() >= SETTLE_AFTER,
        "not before its output settled"
    );
    assert_eq!(s.typed("claude"), format!("{PROMPT}\n"));

    let id = Sandbox::created(&out);
    let token = registry
        .lock()
        .unwrap()
        .get(&id.0)
        .cloned()
        .expect("a claude session started with hooks has a token");
    let body = r#"{"hook_event_name":"SessionStart"}"#;
    let request = format!(
        "POST /hook/{} HTTP/1.1\r\nHost: {hook_addr}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        id.0,
        body.len()
    );
    let (status, _) = raw_request(hook_addr, request.as_bytes()).await;
    assert_eq!(status, 200);
    let summary = session_in(&s.state.catalog_snapshot(), id)
        .cloned()
        .unwrap();
    assert_eq!(
        summary.activity,
        ActivitySignal::Unknown,
        "SessionStart does not move the activity badge"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_pi_session_start_event_makes_pi_ready() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.set_pi_delay(0.5);
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "pi", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert_eq!(s.typed("pi"), format!("{PROMPT}\n"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn copilot_is_ready_once_its_output_settles() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let asked = Instant::now();
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "copilot", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert!(
        asked.elapsed() >= SETTLE_AFTER,
        "not before its output has settled"
    );
    assert_eq!(s.typed("copilot"), format!("{PROMPT}\n"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn claude_without_a_hook_receiver_is_ready_once_its_output_settles() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let asked = Instant::now();
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "claude_code", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert!(asked.elapsed() >= SETTLE_AFTER);
    assert_eq!(s.typed("claude"), format!("{PROMPT}\n"));
}

/// A CLI that would first ask whether to trust the folder gets nothing typed: the prompt's Enter
/// would answer that question for the user (quickstart §B3, `evidence/m3-real-cli.md` finding 4).
/// The call says so at once instead of waiting, and the session keeps running at the question.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cli_that_would_ask_to_trust_the_folder_gets_no_first_prompt() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.distrust();
    for (cli, command, name) in [
        ("claude_code", "claude", "Claude Code"),
        ("copilot", "copilot", "GitHub Copilot"),
    ] {
        let asked = Instant::now();
        let out = s
            .ok(json!({"worktree": "b", "ai_cli": cli, "prompt": PROMPT}))
            .await;
        assert_eq!(out["prompt_delivered"], json!(false), "{out}");
        let reason = out["prompt_reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("trust") && reason.contains(name),
            "names the CLI and the trust question: {out}"
        );
        assert!(
            asked.elapsed() < SETTLE_AFTER,
            "does not wait for a ready signal: {:?}",
            asked.elapsed()
        );
        assert_ne!(out["lifecycle"], "failed", "{out}");
        tokio::time::sleep(SETTLE_AFTER + Duration::from_millis(500)).await;
        assert_eq!(s.typed(command), "", "nothing is typed into {command}");
    }
}

/// Pi asks no trust question, so it gets its prompt without any trust record.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pi_gets_its_first_prompt_without_a_trust_record() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.distrust();
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "pi", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert_eq!(s.typed("pi"), format!("{PROMPT}\n"));
}

/// The prompt goes to the session's primary process, and readiness is judged on its output, even
/// when the user opens a shell in the new session and attaches it while the call waits (U230).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_first_prompt_goes_to_the_primary_process_even_with_a_shell_attached() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    s.state.set_first_prompt_bound(Duration::from_secs(5));
    let before = s.session_ids();
    let addr = s.addr;
    let credential = credential(&s.state, sid(3));
    let call = tokio::spawn(async move {
        call_ok(
            addr,
            &credential,
            "create_session",
            json!({"worktree": "b", "ai_cli": "copilot", "prompt": PROMPT}),
        )
        .await
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let id = loop {
        let new = s.session_ids().into_iter().find(|id| !before.contains(id));
        if let Some(id) = new.filter(|id| s.state.live_session(*id).is_some()) {
            break id;
        }
        assert!(Instant::now() < deadline, "the new session never went live");
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let shell = micold_core::session::ShellInstanceId(1);
    s.state.open_shell(id, shell).unwrap();
    s.state
        .attach_process(
            id,
            micold_core::protocol::messages::SessionProcess::Shell(shell),
        )
        .unwrap();
    let out = call.await.unwrap();
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert_eq!(s.typed("copilot"), format!("{PROMPT}\n"));
}

/// Every window's connection learns that a session an agent created has gone live, so a window
/// already viewing it builds its stream then (U231).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_connection_hears_that_an_agent_created_session_went_live() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    let mut started = s.state.subscribe_session_started();
    let out = s.ok(json!({"worktree": "b", "ai_cli": "copilot"})).await;
    let id = Sandbox::created(&out);
    let heard = tokio::time::timeout(Duration::from_secs(2), started.recv())
        .await
        .expect("announced")
        .unwrap();
    assert_eq!(heard, id);
}

/// A Pi session whose `session_start` never reaches the service is still ready once its output
/// settles, rather than waiting out the bound (U232).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pi_without_its_event_is_ready_once_its_output_settles() {
    let _guard = ENV.lock().await;
    let s = Sandbox::new().await;
    install_cli(s.bin.path(), "pi", true);
    s.set_pi_delay(30.0);
    s.state.set_first_prompt_bound(Duration::from_secs(5));
    let out = s
        .ok(json!({"worktree": "b", "ai_cli": "pi", "prompt": PROMPT}))
        .await;
    assert_eq!(out["prompt_delivered"], json!(true), "{out}");
    assert_eq!(s.typed("pi"), format!("{PROMPT}\n"));
}
