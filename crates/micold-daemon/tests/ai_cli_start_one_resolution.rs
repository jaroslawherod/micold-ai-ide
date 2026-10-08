//! An AI-CLI session start resolves its directory's environment once, and both the launch gate's
//! `PATH` check and the spawn use that one result (#441; feature 029 BUG-001, FR-003b; feature 011
//! FR-021).
//!
//! The start used to look the environment up twice: once for the availability check, once to build
//! the spawn's environment. An invalidation (a Settings save, a worktree deletion) that landed in
//! between took effect for the second lookup only, so the CLI was found on one `PATH` and spawned
//! with another, where it could be missing. The second lookup also re-ran the script.
//!
//! The include script here blocks until the test releases it, so the overlap is decided by the
//! test, not by timing: the invalidation lands while the start's resolve is in progress. That
//! resolve may still answer the start (FR-021(a)) but is not cached, so a second lookup in the same
//! start would run the script again and get the environment as it is after the change.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

/// Bounds a wait for the script to reach a point. It bounds a failure, not a pass.
const PATIENCE: Duration = Duration::from_secs(30);

/// The CLI the session runs. Its stand-in is on `PATH` only when the script's run read `new`.
const CLI: AiCli = AiCli::ClaudeCode;

fn session_id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x0441))
}

/// The files a gated script reads and writes, and the directory a `new` run puts on `PATH`.
struct Gated {
    script: PathBuf,
    /// One line per run of the script, written once the run has read the marker.
    runs: PathBuf,
    /// Read by each run as it starts: `new` makes the run put `bin` in front of `PATH`.
    marker: PathBuf,
    /// Every run waits until this file exists.
    gate: PathBuf,
    _bin: tempfile::TempDir,
}

fn gated_script(dir: &Path) -> Gated {
    let runs = dir.join("runs");
    let marker = dir.join("marker");
    let gate = dir.join("gate");
    let bin = tempfile::tempdir().unwrap();
    write_cli_stand_in(bin.path(), CLI.provider().command());
    let (name, body) = if cfg!(windows) {
        (
            "env-include.ps1",
            format!(
                "$v = (Get-Content -Path '{marker}' -Raw).Trim()\r\n\
                 Add-Content -Path '{runs}' -Value run\r\n\
                 while (-not (Test-Path '{gate}')) {{ Start-Sleep -Milliseconds 20 }}\r\n\
                 if ($v -eq 'new') {{ $env:PATH = '{bin};' + $env:PATH }}\r\n",
                runs = runs.display(),
                marker = marker.display(),
                gate = gate.display(),
                bin = bin.path().display(),
            ),
        )
    } else {
        (
            "env-include.sh",
            format!(
                "v=$(cat '{marker}')\n\
                 echo run >> '{runs}'\n\
                 while [ ! -e '{gate}' ]; do sleep 0.02; done\n\
                 if [ \"$v\" = new ]; then export PATH=\"{bin}:$PATH\"; fi\n",
                runs = runs.display(),
                marker = marker.display(),
                gate = gate.display(),
                bin = bin.path().display(),
            ),
        )
    };
    let script = dir.join(name);
    std::fs::write(&script, body).unwrap();
    Gated {
        script,
        runs,
        marker,
        gate,
        _bin: bin,
    }
}

#[cfg(unix)]
fn write_cli_stand_in(dir: &Path, command: &str) {
    use std::os::unix::fs::PermissionsExt;
    let stub = dir.join(command);
    std::fs::write(&stub, "#!/bin/sh\nexec sleep 600\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(windows)]
fn write_cli_stand_in(dir: &Path, command: &str) {
    std::fs::write(
        dir.join(format!("{command}.cmd")),
        "@echo off\r\nping -n 600 127.0.0.1 >nul\r\n",
    )
    .unwrap();
}

/// A service with environment-include on and `script` configured, holding one AI-CLI session at
/// the root of `project_dir`.
fn service(project_dir: &Path, store: &Path, script: &Path) -> Arc<DaemonState> {
    JsonFileSettingsStore::at(store.join("settings.json"))
        .save(&Settings {
            env_include_enabled: true,
            env_include_script_path: script.to_string_lossy().into_owned(),
            env_include_timeout_secs: PATIENCE.as_secs(),
            ..Settings::default()
        })
        .unwrap();
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_dir.to_path_buf(),
        vec![Session::restored(
            session_id(),
            SessionLocation::Default,
            SessionLabel::Named("Start me".into()),
            TerminalMode::AiCli,
            CLI,
        )],
    );
    let projects_path = store.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(
                project_dir.to_path_buf(),
                false,
                Availability::Available,
            )],
            active: Some(project_dir.to_path_buf()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

fn runs(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// Opens the script's gate when dropped, so a test that fails before it releases the script does
/// not leave the script, and the start blocked on it, running past the test.
struct OpenOnDrop<'a>(&'a Path);

impl Drop for OpenOnDrop<'_> {
    fn drop(&mut self) {
        let _ = std::fs::write(self.0, "");
    }
}

fn wait_for_runs(log: &Path, n: usize) {
    let deadline = Instant::now() + PATIENCE;
    while runs(log) < n {
        assert!(
            Instant::now() < deadline,
            "fixture check: the script never reached run {n}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// #441: a worktree deletion that lands while the start's resolve is in progress. The launch gate
/// found the CLI on the `PATH` that resolve gave; the spawn must use that same environment, not
/// look again and get one without the CLI.
#[test]
fn an_ai_cli_start_spawns_with_the_environment_its_path_check_saw() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let gated = gated_script(store.path());
    std::fs::write(&gated.marker, "new").unwrap();
    let state = service(project.path(), store.path(), &gated.script);

    let _release = OpenOnDrop(&gated.gate);
    let starter = Arc::clone(&state);
    let start = std::thread::spawn(move || starter.start_session(session_id(), LaunchMode::Fresh));
    wait_for_runs(&gated.runs, 1); // the start's resolve is in progress and has read "new"
    std::fs::write(&gated.marker, "old").unwrap();
    state.invalidate_env_include(project.path());
    std::fs::write(&gated.gate, "").unwrap();
    let started = start.join().unwrap();

    let n = runs(&gated.runs);
    for pty in state.remove_session(session_id()) {
        let _ = pty.kill();
    }
    assert_eq!(
        n, 1,
        "one start resolved its directory's environment twice: the launch gate's PATH check and \
         the spawn each looked it up, so they can disagree (#441)"
    );
    assert!(
        started.is_ok(),
        "the launch gate found the CLI, so the start must spawn it with that same environment; \
         got {started:?}"
    );
}
