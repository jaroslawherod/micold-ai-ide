//! Feature 488, M3 (US3, T020): a restarted Codex session resumes its own conversation, and only
//! its own.
//!
//! A stand-in `codex` on `PATH` writes a rollout file in Codex's store layout and records its
//! arguments; the daemon binds the conversation after the spawn.

// unix-only: the stand-in CLI is a `#!/bin/sh` script.
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
use micold_core::session::{AiCli, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

/// The tests change process-wide variables (`PATH`, `HOME`, …); one at a time.
static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

/// A `codex` that logs its arguments and, when not resuming, records a conversation for its
/// working directory the way Codex does: `sessions/Y/M/D/rollout-<ts>-<id>.jsonl`.
fn install_codex(bin: &Path, home: &Path) {
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> '{bin}/args.log'
if [ "$1" != resume ]; then
  id=$(printf '%08d-0000-4000-8000-%012d' $$ $$)
  dir='{home}/sessions/2026/10/09'
  mkdir -p "$dir"
  file="$dir/rollout-2026-10-09T10-00-00-$id.jsonl"
  printf '{{"type":"session_meta","payload":{{"id":"%s","cwd":"%s"}}}}\n' "$id" "$(pwd -P)" > "$file"
  printf '{{"type":"event_msg","payload":{{"type":"user_message","message":"hello codex"}}}}\n' >> "$file"
fi
exec sleep 600
"#,
        bin = bin.display(),
        home = home.display()
    );
    let path = bin.join("codex");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

struct Fixture {
    _env: Env,
    bin: PathBuf,
    codex_home: PathBuf,
    store: PathBuf,
    state: Arc<DaemonState>,
    _dirs: Vec<tempfile::TempDir>,
}

fn open_state(store: &Path) -> Arc<DaemonState> {
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

impl Fixture {
    /// A project with worktree `b` holding one Codex session per id in `ids`.
    async fn new(ids: &[u128]) -> Self {
        Self::with_modes(
            &ids.iter()
                .map(|n| (*n, TerminalMode::AiCli))
                .collect::<Vec<_>>(),
        )
        .await
    }

    async fn with_modes(ids: &[(u128, TerminalMode)]) -> Self {
        let dirs: Vec<tempfile::TempDir> = (0..4).map(|_| tempfile::tempdir().unwrap()).collect();
        let canonical = |n: usize| dirs[n].path().canonicalize().unwrap();
        let (bin, home, project, store) = (canonical(0), canonical(1), canonical(2), canonical(3));
        let codex_home = home.join(".codex");
        install_codex(&bin, &codex_home);
        init_repo(&project);
        add_worktree(&project, "b");
        let mut path = vec![bin.clone()];
        path.extend(["/usr/bin", "/bin"].map(PathBuf::from));
        let env = Env::set(&[
            ("PATH", Some(std::env::join_paths(path).unwrap())),
            ("HOME", Some(home.clone().into())),
            ("CODEX_HOME", Some(codex_home.clone().into())),
            ("MICOLD_IMAGE_REFERENCE", None),
        ]);
        let sessions = ids
            .iter()
            .map(|(n, mode)| session(sid(*n), Some("b"), *mode, AiCli::Codex))
            .collect();
        JsonFileStore::at(store.join("projects.json"))
            .save(&Workspace {
                projects: vec![Project::new(project.clone(), true, Availability::Available)],
                active: Some(project.clone()),
                sessions: [(project, sessions)].into(),
                ..Default::default()
            })
            .unwrap();
        let state = open_state(&store);
        Self {
            _env: env,
            bin,
            codex_home,
            store,
            state,
            _dirs: dirs,
        }
    }

    async fn start(&self, state: &Arc<DaemonState>, n: u128, mode: LaunchMode) {
        assert!(micold_daemon::ops::start_session(state, sid(n), mode)
            .await
            .unwrap());
    }

    fn binding(&self, n: u128) -> PathBuf {
        self.codex_home
            .join("micold-bindings")
            .join(sid(n).0.to_string())
    }

    fn argument_lines(&self) -> Vec<String> {
        std::fs::read_to_string(self.bin.join("args.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    async fn until(&self, what: &str, mut done: impl FnMut(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !done(self) {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn stop(&self, state: &DaemonState, ids: &[u128]) {
        for n in ids {
            for pty in state.remove_session(sid(*n)) {
                let _ = pty.kill();
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for n in 0..8 {
            if let Some(pty) = self.state.live_session(sid(n)) {
                let _ = pty.kill();
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_daemon_binds_after_spawn_and_a_restart_resumes_the_bound_conversation() {
    let _guard = ENV.lock().await;
    let f = Fixture::new(&[1]).await;
    f.start(&f.state, 1, LaunchMode::Fresh).await;
    f.until("the binding", |f| f.binding(1).exists()).await;
    let bound = std::fs::read_to_string(f.binding(1)).unwrap();
    assert!(!bound.trim().is_empty());
    assert_eq!(
        f.argument_lines(),
        vec![String::new()],
        "the first start is fresh"
    );

    // The service restarts: a new daemon over the same files resumes it.
    f.stop(&f.state, &[1]);
    let restarted = open_state(&f.store);
    f.start(&restarted, 1, LaunchMode::Resume).await;
    f.until("the resume", |f| f.argument_lines().len() == 2)
        .await;
    assert_eq!(f.argument_lines()[1], format!("resume {}", bound.trim()));
    f.stop(&restarted, &[1]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_sessions_in_one_directory_never_resume_each_others_conversation() {
    let _guard = ENV.lock().await;
    let f = Fixture::new(&[1, 2]).await;
    f.start(&f.state, 1, LaunchMode::Fresh).await;
    f.start(&f.state, 2, LaunchMode::Fresh).await;
    f.until("both conversations", |f| f.argument_lines().len() == 2)
        .await;
    // Two conversations, two running sessions: nothing says which is whose, so nothing is bound.
    tokio::time::sleep(Duration::from_secs(6)).await;
    assert!(!f.binding(1).exists() && !f.binding(2).exists());

    // Restarting an unbound session starts fresh, and is not refused as "no longer has it".
    f.state.mark_live_sessions_interrupted();
    f.stop(&f.state, &[1, 2]);
    f.start(&f.state, 1, LaunchMode::Resume).await;
    f.start(&f.state, 2, LaunchMode::Resume).await;
    f.until("both restarts", |f| f.argument_lines().len() == 4)
        .await;
    assert!(
        f.argument_lines().iter().all(|line| line.is_empty()),
        "a session resumed some conversation: {:?}",
        f.argument_lines()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_shell_session_beside_it_does_not_hold_the_binding_back() {
    let _guard = ENV.lock().await;
    let f = Fixture::with_modes(&[(1, TerminalMode::AiCli), (2, TerminalMode::Regular)]).await;
    f.start(&f.state, 2, LaunchMode::Fresh).await;
    f.start(&f.state, 1, LaunchMode::Fresh).await;
    f.until("the binding", |f| f.binding(1).exists()).await;
    f.stop(&f.state, &[1, 2]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn closing_a_session_leaves_the_archive_marker_beside_the_binding() {
    let _guard = ENV.lock().await;
    let f = Fixture::new(&[1]).await;
    let mut catalog = Catalog::load(
        Box::new(JsonFileStore::at(f.store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(f.store.join("settings.json"))),
    );
    catalog.archive_session(sid(1)).unwrap();
    assert!(f
        .codex_home
        .join("micold-bindings")
        .join(format!("{}.archived", sid(1).0))
        .exists());
    assert!(
        !f.codex_home.join("sessions").exists(),
        "the CLI store is untouched"
    );
}
