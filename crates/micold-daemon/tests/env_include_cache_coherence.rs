//! The daemon's per-directory environment-include cache stays coherent when asks overlap
//! (feature 011, FR-021; BUG-005).
//!
//! Resolving a directory's environment runs the user's script, off the state lock, for up to the
//! configured timeout. Two things may happen while it runs, and both are tested here with a script
//! that blocks until the test releases it, so the overlap is decided by the test and not by timing:
//!
//! - **An invalidation lands** (a Settings save, a worktree deletion). It must win: the resolve in
//!   progress may still answer the callers that asked before it, but it must not be cached, and the
//!   next ask resolves again (FR-021(a), FR-007, FR-016).
//! - **A second first ask arrives** for the same directory. It shares the run in progress instead of
//!   starting another (FR-021(b), FR-020).
//!
//! Observations go through the public `ai_clis_available_in`, which reads the same cache every
//! spawn site does.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::session::AiCli;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::JsonFileStore;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

/// How long a wait for the script to reach a point may take before the test gives up. Generous:
/// it bounds a failure, not a pass.
const PATIENCE: Duration = Duration::from_secs(30);

/// The files a gated script reads and writes, and the directory a `new` run puts on `PATH`.
struct Gated {
    script: PathBuf,
    /// One line per run of the script, written once the run has read the marker.
    runs: PathBuf,
    /// Read by each run once it has started: `new` makes the run put `bin` in front of `PATH`.
    marker: PathBuf,
    /// Every run waits until this file exists.
    gate: PathBuf,
    _bin: tempfile::TempDir,
}

/// An include script that reads the marker, logs its run, waits for the gate, and then — when the
/// marker said `new` — puts a directory holding `pi` in front of `PATH`. Which environment a later
/// ask was served is then visible in its answer.
fn gated_script(dir: &Path) -> Gated {
    let runs = dir.join("runs");
    let marker = dir.join("marker");
    let gate = dir.join("gate");
    let bin = tempfile::tempdir().unwrap();
    std::fs::write(
        bin.path().join(AiCli::Pi.provider().command()),
        b"#!/bin/sh\n",
    )
    .unwrap();
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

/// A service with environment-include on and `script` configured.
fn service(store: &Path, script: &Path) -> Arc<DaemonState> {
    JsonFileSettingsStore::at(store.join("settings.json"))
        .save(&Settings {
            env_include_enabled: true,
            env_include_script_path: script.to_string_lossy().into_owned(),
            env_include_timeout_secs: PATIENCE.as_secs(),
            ..Settings::default()
        })
        .unwrap();
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

fn runs(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// Waits until the script has started `n` runs.
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

/// Asks for `cwd` on a thread of its own, so the test can act while the resolve is in progress.
fn ask_on_a_thread(state: &Arc<DaemonState>, cwd: &Path) -> std::thread::JoinHandle<Vec<AiCli>> {
    let (state, cwd) = (Arc::clone(state), cwd.to_path_buf());
    std::thread::spawn(move || state.ai_clis_available_in(&cwd))
}

/// Starts a resolve for a directory, runs `invalidate` while the script is blocked in it, releases
/// the script, and asks again. The later ask must not be served the value resolved before the
/// invalidation: the script runs a second time, and its answer reflects the environment as it is
/// now.
fn an_invalidation_racing_a_resolve_wins(invalidate: impl FnOnce(&DaemonState, &Path)) {
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let gated = gated_script(store.path());
    std::fs::write(&gated.marker, "old").unwrap();
    let state = service(store.path(), &gated.script);

    let first = ask_on_a_thread(&state, project.path());
    wait_for_runs(&gated.runs, 1); // the resolve is in progress and has read "old"
    std::fs::write(&gated.marker, "new").unwrap();
    invalidate(&state, project.path());
    std::fs::write(&gated.gate, "").unwrap();
    first.join().unwrap();

    let later = state.ai_clis_available_in(project.path());
    assert_eq!(
        runs(&gated.runs),
        2,
        "the invalidation landed while a resolve was in progress, and the resolve's stale result \
         was cached over it: the later ask was served the environment from before the change \
         (FR-021(a))"
    );
    assert!(
        later.contains(&AiCli::Pi),
        "the later ask must be answered from the environment as it is after the change, got \
         {later:?}"
    );
}

/// T037 Case 1: a Settings save (`SettingsSet`) that lands during a resolve wins.
#[test]
fn a_settings_save_during_a_resolve_is_not_lost() {
    an_invalidation_racing_a_resolve_wins(|state, _| {
        state.set_env_include(Some(true), None, None).unwrap();
    });
}

/// T037 Case 2: a worktree deletion (`WorktreeDelete`) that lands during a resolve for its
/// directory wins.
#[test]
fn a_worktree_delete_during_a_resolve_is_not_lost() {
    an_invalidation_racing_a_resolve_wins(|state, cwd| state.invalidate_env_include(cwd));
}

/// T038: two first asks for one directory share one run of the script, and both get its result.
#[test]
fn concurrent_first_asks_for_a_directory_run_the_script_once() {
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let gated = gated_script(store.path());
    std::fs::write(&gated.marker, "new").unwrap();
    let state = service(store.path(), &gated.script);

    let first = ask_on_a_thread(&state, project.path());
    wait_for_runs(&gated.runs, 1);
    let second = ask_on_a_thread(&state, project.path());

    // The second ask has to reach the cache while the first run is still blocked, or it would find
    // the finished entry and pass for the wrong reason. Nothing public shows an ask parked on a run
    // in progress, so this waits: it ends early when a second run starts (the defect, which then
    // fails below), and otherwise gives the thread ample time to reach the cache. A slow machine
    // can only make this pass without having tested the overlap, never fail spuriously.
    let deadline = Instant::now() + Duration::from_millis(800);
    while runs(&gated.runs) < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    std::fs::write(&gated.gate, "").unwrap();
    let (first, second) = (first.join().unwrap(), second.join().unwrap());

    assert_eq!(
        runs(&gated.runs),
        1,
        "two first asks for one directory each ran the script; the second must share the run in \
         progress (FR-021(b), FR-020)"
    );
    assert!(
        first.contains(&AiCli::Pi) && second.contains(&AiCli::Pi),
        "both asks must get the environment the one run resolved, got {first:?} and {second:?}"
    );
}
