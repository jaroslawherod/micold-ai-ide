//! Feature 488, M1 (US1): a session of Codex or OpenCode starts in its worktree, the Settings
//! default and the MCP `ai_cli` parameter reach it, and the provider is remembered across a
//! restart of the service.
//!
//! Stand-in `codex` and `opencode` on `PATH` record the directory they were started in.

// unix-only: the stand-in CLIs are `#!/bin/sh` scripts.
#![cfg(unix)]

#[path = "support/mcp.rs"]
mod mcp_support;

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mcp_support::*;
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

/// The tests change process-wide variables (`PATH`, `HOME`, …); one at a time.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const PROVIDERS: [(AiCli, &str); 2] = [(AiCli::Codex, "codex"), (AiCli::OpenCode, "opencode")];

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

/// A stand-in CLI that records the directory it was started in and then idles.
fn install_cli(bin: &Path, command: &str) {
    let dir = bin.display();
    let path = bin.join(command);
    std::fs::write(
        &path,
        format!("#!/bin/sh\npwd -P > '{dir}/{command}.cwd'\nexec sleep 600\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

struct Fixture {
    _env: Env,
    bin: tempfile::TempDir,
    project: tempfile::TempDir,
    store: tempfile::TempDir,
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
}

fn open_state(store: &Path) -> Arc<DaemonState> {
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

impl Fixture {
    async fn new() -> Self {
        Self::installed(&["codex", "opencode"]).await
    }

    /// A fixture whose `PATH` holds stand-ins for just the `installed` commands.
    async fn installed(installed: &[&str]) -> Self {
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        for command in installed {
            install_cli(bin.path(), command);
        }
        init_repo(project.path());
        add_worktree(project.path(), "b");
        let mut path = vec![bin.path().to_path_buf()];
        path.extend(["/usr/bin", "/bin"].map(PathBuf::from));
        let env = Env::set(&[
            ("PATH", Some(std::env::join_paths(path).unwrap())),
            ("HOME", Some(home.path().into())),
            (
                "XDG_DATA_HOME",
                Some(home.path().join(".local/share").into()),
            ),
            ("CODEX_HOME", Some(home.path().join(".codex").into())),
            ("MICOLD_IMAGE_REFERENCE", None),
        ]);
        std::mem::forget(home);
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&Workspace {
                projects: vec![Project::new(
                    project.path().to_path_buf(),
                    true,
                    Availability::Available,
                )],
                active: Some(project.path().to_path_buf()),
                sessions: [(
                    project.path().to_path_buf(),
                    vec![session(
                        sid(3),
                        Some("b"),
                        TerminalMode::AiCli,
                        AiCli::ClaudeCode,
                    )],
                )]
                .into(),
                ..Default::default()
            })
            .unwrap();
        let state = open_state(store.path());
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            _env: env,
            bin,
            project,
            store,
            state,
            addr,
        }
    }

    async fn create(&self, args: Value) -> Value {
        call_ok(
            self.addr,
            &credential(&self.state, sid(3)),
            "create_session",
            args,
        )
        .await
    }

    async fn get(&self, id: SessionId) -> Value {
        call_ok(
            self.addr,
            &credential(&self.state, sid(3)),
            "get_session",
            json!({"session": id.0.to_string()}),
        )
        .await
    }

    /// Wait for the stand-in `command` to report the directory it started in.
    async fn started_in(&self, command: &str) -> PathBuf {
        let file = self.bin.path().join(format!("{command}.cwd"));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(text) = std::fs::read_to_string(&file) {
                if !text.trim().is_empty() {
                    return PathBuf::from(text.trim());
                }
            }
            assert!(Instant::now() < deadline, "{command} never started");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn kill_all(&self) {
        for project in self.state.catalog_snapshot().projects {
            for s in project.sessions {
                if let Some(pty) = self.state.live_session(s.id) {
                    let _ = pty.kill();
                }
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.kill_all();
        let _ = (&self.project, &self.store);
    }
}

fn created(out: &Value) -> SessionId {
    SessionId::from_uuid(out["session"].as_str().unwrap().parse().unwrap())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mcp_create_session_starts_each_provider_in_its_worktree() {
    let _guard = ENV.lock().await;
    for (cli, command) in PROVIDERS {
        let f = Fixture::new().await;
        let out = f
            .create(json!({"worktree": "b", "ai_cli": cli.tool_name()}))
            .await;
        let id = created(&out);
        let ran_in = f.started_in(command).await;
        assert!(
            ran_in.ends_with("b"),
            "{command} started in {ran_in:?}, not the worktree b"
        );
        assert_eq!(f.get(id).await["ai_cli"], command);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_settings_default_applies_without_an_override() {
    let _guard = ENV.lock().await;
    for (cli, command) in PROVIDERS {
        let f = Fixture::new().await;
        f.state.set_default_ai_cli(cli).unwrap();
        let out = f.create(json!({"worktree": "b"})).await;
        let id = created(&out);
        assert_eq!(f.get(id).await["ai_cli"], command);
        f.started_in(command).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_provider_survives_a_restart_of_the_service() {
    let _guard = ENV.lock().await;
    for (cli, _) in PROVIDERS {
        let f = Fixture::new().await;
        let out = f
            .create(json!({"worktree": "b", "ai_cli": cli.tool_name()}))
            .await;
        let id = created(&out);
        f.kill_all();

        let restarted = open_state(f.store.path());
        let provider = restarted
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| p.sessions.iter())
            .find(|s| s.id == id)
            .map(|s| s.provider);
        assert_eq!(provider, Some(cli), "the restarted service lists {cli:?}");
    }
}

fn listed(state: &DaemonState, cwd: &Path) -> Vec<AiCli> {
    state.availability_in(cwd).0
}

/// How many session records the catalog holds (the fixture starts with one, session 3).
fn session_count(f: &Fixture) -> usize {
    f.state
        .catalog_snapshot()
        .projects
        .iter()
        .map(|p| p.sessions.len())
        .sum()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_provider_off_the_path_is_listed_unavailable() {
    let _guard = ENV.lock().await;
    let f = Fixture::installed(&[]).await;
    let found = listed(&f.state, f.project.path());
    for (cli, _) in PROVIDERS {
        assert!(
            !found.contains(&cli),
            "{cli:?} offered with nothing on PATH"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn starting_an_unavailable_provider_over_mcp_is_refused_naming_it_and_creates_nothing() {
    let _guard = ENV.lock().await;
    for (cli, command) in PROVIDERS {
        let f = Fixture::installed(&[]).await;
        let before = session_count(&f);
        let err = call_err(
            f.addr,
            &credential(&f.state, sid(3)),
            "create_session",
            json!({"worktree": "b", "ai_cli": cli.tool_name()}),
        )
        .await;
        let message = err["message"].as_str().unwrap();
        assert!(
            message.to_lowercase().contains(command),
            "{command} not named: {err}"
        );
        let app = micold_daemon::ops::cli_unavailable(
            &f.state,
            &f.project.path().join(".claude/worktrees/b"),
            cli,
        )
        .await
        .expect("unavailable");
        assert_eq!(message, app, "MCP and the app give the same reason");
        assert_eq!(session_count(&f), before, "a record was created");
        assert!(
            !f.bin.path().join(format!("{command}.cwd")).exists(),
            "{command} was started"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_app_start_path_gives_the_same_reason_before_any_terminal() {
    let _guard = ENV.lock().await;
    for (cli, command) in PROVIDERS {
        let f = Fixture::installed(&[]).await;
        let why = micold_daemon::ops::cli_unavailable(&f.state, f.project.path(), cli)
            .await
            .expect("unavailable");
        assert!(why.to_lowercase().contains(command), "{command}: {why}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_provider_installed_afterwards_is_available_on_the_next_listing() {
    let _guard = ENV.lock().await;
    for (cli, command) in PROVIDERS {
        let f = Fixture::installed(&[]).await;
        assert!(!listed(&f.state, f.project.path()).contains(&cli));
        install_cli(f.bin.path(), command);
        assert!(
            listed(&f.state, f.project.path()).contains(&cli),
            "{command} not available after install, without a restart"
        );
        let out = f
            .create(json!({"worktree": "b", "ai_cli": cli.tool_name()}))
            .await;
        created(&out);
        f.started_in(command).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_session_whose_provider_was_removed_keeps_its_provider_in_the_listing() {
    let _guard = ENV.lock().await;
    for (cli, _) in PROVIDERS {
        let f = Fixture::new().await;
        let id = created(
            &f.create(json!({"worktree": "b", "ai_cli": cli.tool_name()}))
                .await,
        );
        f.kill_all();
        for (_, command) in PROVIDERS {
            std::fs::remove_file(f.bin.path().join(command)).unwrap();
        }
        let restarted = open_state(f.store.path());
        let kept = restarted
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| p.sessions.iter())
            .find(|s| s.id == id)
            .map(|s| s.provider);
        assert_eq!(kept, Some(cli));
        assert!(!listed(&restarted, f.project.path()).contains(&cli));
    }
}

// ---- M5 (US4): honest activity, tool-server and first-prompt behaviour ----

/// A stand-in that draws a prompt, then copies whatever is typed into it to `<command>.input`.
fn install_typing_cli(bin: &Path, command: &str) {
    let dir = bin.display();
    let path = bin.join(command);
    std::fs::write(
        &path,
        format!("#!/bin/sh\nprintf 'ready> '\nexec cat >> '{dir}/{command}.input'\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn typed(f: &Fixture, command: &str) -> String {
    std::fs::read_to_string(f.bin.path().join(format!("{command}.input"))).unwrap_or_default()
}

fn badge(f: &Fixture, id: SessionId) -> micold_core::protocol::messages::ActivitySignal {
    f.state
        .catalog_snapshot()
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id == id)
        .expect("the session is listed")
        .activity
}

/// FR-009, AS1: Codex and OpenCode have no reliable activity signal, so the badge reads `Unknown`
/// while the CLI prints and after it goes quiet. AS2 (a provider with a source follows busy/idle)
/// is pinned by `activity_pipeline::hooks_drive_the_projected_activity_signal`; here only that
/// Claude Code still has a source while these two have none.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_badge_stays_unknown_through_output_and_silence() {
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::provider::ActivitySource;
    let _guard = ENV.lock().await;
    let (dir, id) = (Path::new("/tmp"), uuid::Uuid::nil());
    assert!(!matches!(
        AiCli::ClaudeCode.provider().activity_source(dir, dir, id),
        ActivitySource::None
    ));
    for (cli, command) in PROVIDERS {
        assert!(matches!(
            cli.provider().activity_source(dir, dir, id),
            ActivitySource::None
        ));
        let f = Fixture::new().await;
        install_typing_cli(f.bin.path(), command);
        let session = created(
            &f.create(json!({"worktree": "b", "ai_cli": cli.tool_name()}))
                .await,
        );
        // Output now, then a stretch of silence longer than the output-settle window.
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until {
            assert_eq!(badge(&f, session), ActivitySignal::Unknown, "{command}");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// FR-011: a provider with no verified tool-server binding starts anyway, unbound, and the reason
/// is logged against the session.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unsupported_tool_server_binding_is_logged_and_the_session_starts() {
    let _guard = ENV.lock().await;
    mcp_support::log();
    for (cli, command) in PROVIDERS {
        let f = Fixture::new().await;
        let session = created(
            &f.create(json!({"worktree": "b", "ai_cli": cli.tool_name()}))
                .await,
        );
        f.started_in(command).await;
        let lines = mcp_support::log_lines_for(session);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("no tool server") && l.contains("no per-launch tool-server")),
            "{command}: the reason is logged: {lines:?}"
        );
    }
}

/// FR-012: Codex in a folder it has not been told to trust gets no first prompt (typing Enter would
/// answer its trust question); in a trusted folder it does. OpenCode asks nothing, so it always does.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_first_prompt_is_never_typed_into_a_trust_question() {
    const PROMPT: &str = "print the branch name";
    let _guard = ENV.lock().await;
    for (cli, command) in PROVIDERS {
        for trusted in [false, true] {
            let f = Fixture::new().await;
            install_typing_cli(f.bin.path(), command);
            if trusted && cli == AiCli::Codex {
                let home = PathBuf::from(std::env::var_os("CODEX_HOME").unwrap());
                std::fs::create_dir_all(&home).unwrap();
                let project = f.project.path().canonicalize().unwrap();
                std::fs::write(
                    home.join("config.toml"),
                    format!(
                        "[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
                        project.display()
                    ),
                )
                .unwrap();
            }
            let out = f
                .create(json!({"worktree": "b", "ai_cli": cli.tool_name(), "prompt": PROMPT}))
                .await;
            let delivered = cli != AiCli::Codex || trusted;
            assert_eq!(out["prompt_delivered"], json!(delivered), "{command}: {out}");
            if delivered {
                let deadline = Instant::now() + Duration::from_secs(10);
                while !typed(&f, command).contains(PROMPT) && Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                assert!(typed(&f, command).contains(PROMPT), "{command} got the prompt");
            } else {
                let reason = out["prompt_reason"].as_str().unwrap_or_default();
                assert!(reason.contains("trust") && reason.contains("Codex"), "{out}");
                tokio::time::sleep(Duration::from_secs(2)).await;
                assert_eq!(typed(&f, command), "", "nothing is typed into {command}");
            }
        }
    }
}
