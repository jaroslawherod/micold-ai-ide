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
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        for (_, command) in PROVIDERS {
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
