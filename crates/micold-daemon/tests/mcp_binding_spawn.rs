//! Feature 034 (contracts/binding.md §4–§6; US1 scenarios 1, 4, 5; A1, A4, A5, U59–U70): what a
//! session is actually started with.
//!
//! Stand-in `claude`, `copilot` and `pi` on `PATH` record their arguments, and the test reads that
//! record — then uses the binding file it names to call the tool server exactly as the CLI would.

// The owner-only check reads Unix modes, and the user's configuration is redirected through
// `HOME`/`XDG_DATA_HOME`, which the Windows known folders ignore.
// unix-only: the stand-in CLIs are `#!/bin/sh` scripts (Windows port is a recorded follow-up).
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::session::{AiCli, Session, SessionId, SessionLifecycle, TerminalMode};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// The tests change process-wide variables (`PATH`, `HOME`, …); one at a time.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

/// A sandbox: stand-in CLIs on `PATH`, a fake home holding the user's configuration, a project
/// repository with worktree `b`, and a store directory for the binding files.
struct Sandbox {
    _env: Env,
    bin: tempfile::TempDir,
    home: tempfile::TempDir,
    project: tempfile::TempDir,
    store: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        log();
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        for cli in [AiCli::ClaudeCode, AiCli::Copilot, AiCli::Pi] {
            install_recording_cli(bin.path(), cli.provider().command());
        }
        init_repo(project.path());
        add_worktree(project.path(), "b");
        let mut path = vec![bin.path().to_path_buf()];
        path.extend(
            std::env::var_os("PATH")
                .iter()
                .flat_map(std::env::split_paths),
        );
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
        ]);
        Self {
            _env: env,
            bin,
            home,
            project,
            store,
        }
    }

    fn repo(&self) -> &Path {
        self.project.path()
    }

    fn binding_dir(&self) -> PathBuf {
        self.store.path().join("mcp")
    }

    fn state(&self, sessions: Vec<Session>) -> Arc<DaemonState> {
        let state = state_over(
            vec![(self.repo().to_path_buf(), true, sessions)],
            self.store.path(),
        );
        state.set_pi_activity_component(false).unwrap();
        state
    }

    /// The `n`th launch of `cli`, as its arguments.
    async fn launch(&self, cli: AiCli, n: usize) -> Vec<String> {
        let record = self
            .bin
            .path()
            .join(format!("{}.launches", cli.provider().command()));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(line) = std::fs::read_to_string(&record)
                .ok()
                .and_then(|body| body.lines().nth(n).map(str::to_string))
            {
                return line.split('\t').map(str::to_string).collect();
            }
            assert!(
                Instant::now() < deadline,
                "launch {n} never reached {cli:?}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

/// A CLI that appends its arguments, tab separated, to `<bin>/<command>.launches`, then waits on
/// its terminal so it is still a live session when the record is read.
fn install_recording_cli(bin: &Path, command: &str) {
    let path = bin.join(command);
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nIFS=\"$(printf '\\t')\"\nprintf '%s\\n' \"$*\" >> '{}'\ncat > /dev/null\n",
            bin.join(format!("{command}.launches")).display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn kill(state: &DaemonState, id: SessionId) {
    if let Some(session) = state.live_session(id) {
        let _ = session.kill();
    }
}

/// The skip lines logged for `id`.
fn skip_lines(id: SessionId) -> Vec<String> {
    log_lines_for(id)
        .into_iter()
        .filter(|line| line.contains("no tool server"))
        .collect()
}

/// The URL and bearer credential a binding file names.
fn binding_of(file: &Path) -> (String, String) {
    let doc: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let entry = &doc["mcpServers"]["micold"];
    let bearer = entry["headers"]["Authorization"]
        .as_str()
        .unwrap()
        .strip_prefix("Bearer ")
        .unwrap()
        .to_string();
    (entry["url"].as_str().unwrap().to_string(), bearer)
}

fn claude(n: u128, worktree: Option<&str>) -> Session {
    session(sid(n), worktree, TerminalMode::AiCli, AiCli::ClaudeCode)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_claude_session_is_bound_and_answers_whoami_with_its_own_credential() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let state = sandbox.state(vec![claude(1, None), claude(3, Some("b"))]);
    let addr = serve_tool_server(&state, sandbox.binding_dir()).await;

    state.start_session(sid(3), LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::ClaudeCode, 0).await;
    kill(&state, sid(3));

    let file = sandbox.binding_dir().join(format!("{}.json", sid(3).0));
    let file_arg = file.display().to_string();
    assert_eq!(
        args[args.len() - 4..],
        [
            "--mcp-config",
            file_arg.as_str(),
            "--allowedTools",
            "mcp__micold"
        ],
        "the binding comes last: {args:?}"
    );
    assert_eq!(args[..2], ["--session-id", sid(3).0.to_string().as_str()]);
    assert_eq!(
        std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(sandbox.binding_dir())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );

    let (url, bearer) = binding_of(&file);
    assert_eq!(url, format!("http://{addr}/mcp"));
    let list = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string();
    let (status, body) = post_mcp(addr, Some(&bearer), &list).await;
    assert_eq!(status, 200);
    let tools: Value = serde_json::from_str(&body).unwrap();
    assert!(!tools["result"]["tools"].as_array().unwrap().is_empty());
    let who = call_ok(addr, &bearer, "whoami", json!({})).await;
    assert_eq!(who["session"], sid(3).0.to_string());
    assert_eq!(who["worktree"], "b");
    assert!(skip_lines(sid(3)).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_service_keeps_binding_files_under_its_own_data_directory() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let dir = micold_daemon::mcp::server::default_binding_dir();
    assert!(
        dir.starts_with(sandbox.home.path()),
        "under the data directory: {dir:?}"
    );
    assert_eq!(dir.file_name().unwrap(), "mcp");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_copilot_session_ends_with_the_additional_mcp_config_arguments() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let state = sandbox.state(vec![session(
        sid(2),
        None,
        TerminalMode::AiCli,
        AiCli::Copilot,
    )]);
    let addr = serve_tool_server(&state, sandbox.binding_dir()).await;

    state.start_session(sid(2), LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::Copilot, 0).await;
    kill(&state, sid(2));

    let file = sandbox.binding_dir().join(format!("{}.json", sid(2).0));
    let file_arg = format!("@{}", file.display());
    assert_eq!(
        args[args.len() - 4..],
        [
            "--additional-mcp-config",
            file_arg.as_str(),
            "--allow-tool",
            "micold"
        ]
    );
    let (_, bearer) = binding_of(&file);
    let who = call_ok(addr, &bearer, "whoami", json!({})).await;
    assert_eq!(who["ai_cli"], "copilot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pi_session_spawns_with_its_pre_feature_argv_and_one_skip_line() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let pi = sid(0x34_0005);
    let state = sandbox.state(vec![session(pi, None, TerminalMode::AiCli, AiCli::Pi)]);
    serve_tool_server(&state, sandbox.binding_dir()).await;

    state.start_session(pi, LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::Pi, 0).await;
    kill(&state, pi);

    assert_eq!(args, ["--session-id", pi.0.to_string().as_str()]);
    let lines = skip_lines(pi);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("INFO") && lines[0].contains("no tool server: Pi has no MCP support"),
        "{lines:?}"
    );
    assert!(!sandbox
        .binding_dir()
        .join(format!("{}.json", pi.0))
        .exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bound_spawns_leave_every_user_configuration_file_byte_identical() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let home = sandbox.home.path();
    let other = r#"{"mcpServers":{"other":{"type":"http","url":"http://127.0.0.1:1/mcp"}}}"#;
    let fixtures: Vec<PathBuf> = vec![
        home.join(".claude.json"),
        home.join(".claude/settings.json"),
        sandbox.repo().join(".mcp.json"),
        home.join(".copilot/mcp-config.json"),
    ];
    for file in &fixtures {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, other).unwrap();
    }
    let before: BTreeMap<PathBuf, Vec<u8>> = fixtures
        .iter()
        .map(|f| (f.clone(), std::fs::read(f).unwrap()))
        .collect();

    let ids: Vec<(SessionId, AiCli)> = (0..20u128)
        .map(|n| {
            let cli = if n % 2 == 0 {
                AiCli::ClaudeCode
            } else {
                AiCli::Copilot
            };
            (sid(0x34_0100 + n), cli)
        })
        .collect();
    let state = sandbox.state(
        ids.iter()
            .map(|(id, cli)| session(*id, None, TerminalMode::AiCli, *cli))
            .collect(),
    );
    serve_tool_server(&state, sandbox.binding_dir()).await;
    for (n, (id, cli)) in ids.iter().enumerate() {
        state.start_session(*id, LaunchMode::Fresh).unwrap();
        let args = sandbox.launch(*cli, n / 2).await;
        kill(&state, *id);
        assert!(
            args.iter()
                .any(|a| a == "--mcp-config" || a == "--additional-mcp-config"),
            "spawn {n} was bound: {args:?}"
        );
    }

    for (file, bytes) in before {
        assert_eq!(std::fs::read(&file).unwrap(), bytes, "{file:?} changed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bound_spawns_create_no_user_configuration_file_that_was_absent() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let home = sandbox.home.path();
    let absent = [
        home.join(".claude.json"),
        home.join(".claude/settings.json"),
        sandbox.repo().join(".mcp.json"),
        home.join(".copilot/mcp-config.json"),
    ];
    let claude = sid(0x34_0200);
    let copilot = sid(0x34_0201);
    let state = sandbox.state(vec![
        session(claude, None, TerminalMode::AiCli, AiCli::ClaudeCode),
        session(copilot, None, TerminalMode::AiCli, AiCli::Copilot),
    ]);
    serve_tool_server(&state, sandbox.binding_dir()).await;
    for (id, cli) in [(claude, AiCli::ClaudeCode), (copilot, AiCli::Copilot)] {
        state.start_session(id, LaunchMode::Fresh).unwrap();
        let args = sandbox.launch(cli, 0).await;
        kill(&state, id);
        assert!(
            args.iter()
                .any(|a| a == "--mcp-config" || a == "--additional-mcp-config"),
            "{cli:?} was bound: {args:?}"
        );
    }

    for file in absent {
        assert!(!file.exists(), "{file:?} was created");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_collision_spawns_unbound_and_logs_the_colliding_file() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let taken = sandbox.home.path().join(".claude.json");
    std::fs::write(&taken, r#"{"mcpServers":{"micold":{"command":"mine"}}}"#).unwrap();
    let id = sid(0x34_0063);
    let state = sandbox.state(vec![claude(0x34_0063, None)]);
    serve_tool_server(&state, sandbox.binding_dir()).await;

    state.start_session(id, LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::ClaudeCode, 0).await;
    kill(&state, id);

    assert_eq!(args, ["--session-id", id.0.to_string().as_str()]);
    let lines = skip_lines(id);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let expected = format!(
        "no tool server: a server named \"micold\" is already configured in {}",
        taken.display()
    );
    assert!(lines[0].contains(&expected), "{lines:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_collision_in_the_copilot_home_the_session_environment_sets_is_found() {
    // The environment-include script relocates Copilot's store for this session only; the
    // service's own environment does not name it, so only the session's view finds the collision.
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let relocated = sandbox.home.path().join("relocated-copilot");
    std::fs::create_dir_all(&relocated).unwrap();
    let taken = relocated.join("mcp-config.json");
    std::fs::write(&taken, r#"{"mcpServers":{"micold":{"command":"mine"}}}"#).unwrap();
    let script = sandbox.home.path().join("env-include.sh");
    std::fs::write(
        &script,
        format!("export COPILOT_HOME='{}'\n", relocated.display()),
    )
    .unwrap();
    JsonFileSettingsStore::at(sandbox.store.path().join("settings.json"))
        .save(&Settings {
            env_include_enabled: true,
            env_include_script_path: script.to_string_lossy().into_owned(),
            ..Settings::default()
        })
        .unwrap();
    let id = sid(0x34_0064);
    let state = sandbox.state(vec![session(id, None, TerminalMode::AiCli, AiCli::Copilot)]);
    serve_tool_server(&state, sandbox.binding_dir()).await;

    state.start_session(id, LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::Copilot, 0).await;
    kill(&state, id);

    assert!(
        !args.iter().any(|a| a == "--additional-mcp-config"),
        "{args:?}"
    );
    let lines = skip_lines(id);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(&taken.display().to_string()), "{lines:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_crash_respawn_reuses_the_credential_and_rewrites_the_file() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let id = sid(0x34_0064);
    let state = sandbox.state(vec![claude(0x34_0064, None)]);
    let addr = serve_tool_server(&state, sandbox.binding_dir()).await;
    let file = sandbox.binding_dir().join(format!("{}.json", id.0));

    state.start_session(id, LaunchMode::Fresh).unwrap();
    sandbox.launch(AiCli::ClaudeCode, 0).await;
    let (_, first) = binding_of(&file);
    std::fs::remove_file(&file).unwrap();

    // The CLI dies under it; supervision respawns it (`Restarting`).
    let pty = state.live_session(id).unwrap();
    pty.kill().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while pty.is_alive() {
        assert!(Instant::now() < deadline, "the stand-in never exited");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    state.supervise_exited_sessions();
    let args = sandbox.launch(AiCli::ClaudeCode, 1).await;
    kill(&state, id);

    assert_eq!(args[..2], ["--resume", id.0.to_string().as_str()]);
    assert_eq!(args[args.len() - 4], "--mcp-config");
    assert!(file.exists(), "the respawn rewrote the binding file");
    let (_, second) = binding_of(&file);
    assert_eq!(second, first, "the same credential");
    let session = state
        .catalog_snapshot()
        .projects
        .remove(0)
        .sessions
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    assert!(
        matches!(
            session.lifecycle,
            micold_core::protocol::messages::WireLifecycle::Restarting { .. }
        ),
        "{:?}",
        session.lifecycle
    );
    let who = call_ok(addr, &second, "whoami", json!({})).await;
    assert_eq!(who["session"], id.0.to_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_started_after_a_service_restart_gets_a_new_credential() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let id = sid(0x34_0066);
    let file = sandbox.binding_dir().join(format!("{}.json", id.0));

    let before = sandbox.state(vec![claude(0x34_0066, None)]);
    serve_tool_server(&before, sandbox.binding_dir()).await;
    before.start_session(id, LaunchMode::Fresh).unwrap();
    sandbox.launch(AiCli::ClaudeCode, 0).await;
    kill(&before, id);
    let (_, old) = binding_of(&file);

    let mut interrupted = claude(0x34_0066, None);
    interrupted.lifecycle = SessionLifecycle::InterruptedResumable;
    let after = sandbox.state(vec![interrupted]);
    let addr = serve_tool_server(&after, sandbox.binding_dir()).await;
    after.start_session(id, LaunchMode::Fresh).unwrap();
    sandbox.launch(AiCli::ClaudeCode, 1).await;
    kill(&after, id);
    let (_, new) = binding_of(&file);

    assert_ne!(new, old);
    let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    assert_eq!(post_mcp(addr, Some(&old), ping).await.0, 401);
    assert_eq!(post_mcp(addr, Some(&new), ping).await.0, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_regular_terminal_session_gets_no_binding_and_no_log_line() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let id = sid(0x34_0067);
    let state = sandbox.state(vec![session(
        id,
        None,
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    )]);
    serve_tool_server(&state, sandbox.binding_dir()).await;

    state.start_session(id, LaunchMode::Fresh).unwrap();
    kill(&state, id);

    assert!(!sandbox
        .binding_dir()
        .join(format!("{}.json", id.0))
        .exists());
    assert!(skip_lines(id).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_tool_server_a_session_starts_unbound_and_says_so_once() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let id = sid(0x34_0068);
    let state = sandbox.state(vec![claude(0x34_0068, None)]);

    state.start_session(id, LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::ClaudeCode, 0).await;
    kill(&state, id);

    assert_eq!(args, ["--session-id", id.0.to_string().as_str()]);
    let lines = skip_lines(id);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("no tool server: tool server unavailable"),
        "{lines:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unwritable_binding_starts_the_session_unbound_and_says_so_once() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let id = sid(0x34_0069);
    let state = sandbox.state(vec![claude(0x34_0069, None)]);
    // A regular file where the binding directory's parent should be: nothing can be created in it.
    let blocker = sandbox.store.path().join("blocker");
    std::fs::write(&blocker, b"").unwrap();
    serve_tool_server(&state, blocker.join("mcp")).await;

    state.start_session(id, LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::ClaudeCode, 0).await;
    kill(&state, id);

    assert_eq!(args, ["--session-id", id.0.to_string().as_str()]);
    let lines = skip_lines(id);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("no tool server: could not write the binding: "),
        "{lines:?}"
    );
}

/// A6 (US1-AS6, FR-004, FR-005). With the toggle off, a new session starts unbound and the log says
/// why; a session bound before the change keeps its binding and still answers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_the_toggle_off_a_new_session_starts_unbound_and_a_running_one_keeps_answering() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let (running, new) = (sid(0x34_0070), sid(0x34_0071));
    let state = sandbox.state(vec![claude(0x34_0070, None), claude(0x34_0071, Some("b"))]);
    let addr = serve_tool_server(&state, sandbox.binding_dir()).await;

    state.start_session(running, LaunchMode::Fresh).unwrap();
    sandbox.launch(AiCli::ClaudeCode, 0).await;
    let (_, bearer) = binding_of(&sandbox.binding_dir().join(format!("{}.json", running.0)));

    state.set_tool_server_enabled(false).unwrap();
    state.start_session(new, LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::ClaudeCode, 1).await;
    kill(&state, new);

    assert_eq!(
        args,
        ["--session-id", new.0.to_string().as_str()],
        "a session started with the toggle off carries no binding arguments"
    );
    let lines = skip_lines(new);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("INFO") && lines[0].contains("no tool server: disabled in settings"),
        "the log says the binding was skipped and why: {lines:?}"
    );
    let who = call_ok(addr, &bearer, "whoami", json!({})).await;
    kill(&state, running);
    assert_eq!(
        who["session"],
        running.0.to_string(),
        "turning the toggle off affects sessions started afterwards only"
    );
}

/// U71 (FR-004). Turning the toggle back on binds the next session again: off is a setting, not a
/// latch.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turning_the_toggle_back_on_binds_the_next_session_again() {
    let _guard = ENV.lock().await;
    let sandbox = Sandbox::new();
    let id = sid(0x34_0072);
    let state = sandbox.state(vec![claude(0x34_0072, None)]);
    serve_tool_server(&state, sandbox.binding_dir()).await;

    state.set_tool_server_enabled(false).unwrap();
    state.set_tool_server_enabled(true).unwrap();
    state.start_session(id, LaunchMode::Fresh).unwrap();
    let args = sandbox.launch(AiCli::ClaudeCode, 0).await;
    kill(&state, id);

    assert_eq!(
        args.get(2).map(String::as_str),
        Some("--mcp-config"),
        "re-enabled, the next session is bound again: {args:?}"
    );
    assert!(skip_lines(id).is_empty());
}
