//! Phase 6 (US4) — restart supervision runs with no client attached (T058, FR-004/FR-005).
//!
//! These drive [`DaemonState::supervise_exited_sessions`] directly — no socket, no viewer — which is
//! exactly the unattended path. They assert the catalog lifecycle and the live registry move as the
//! policy dictates: a clean exit stops the session and drops it; a crash advances the crash-loop
//! counter and respawns it. The crash-loop *give-up* case lives in `supervision_giveup.rs` (it needs
//! a crashing shell, so it owns its own test binary to keep `SHELL` isolated).

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{Session, SessionId, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{LoadOutcome, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

/// A project store that serves a fixed in-memory workspace. `Session` is not `Serialize`, so this is
/// how a test injects a `Regular`-mode session (whose respawn uses the platform shell we control)
/// without going through disk.
struct FakeStore(Workspace);

impl ProjectStore for FakeStore {
    fn load(&self) -> LoadOutcome {
        LoadOutcome {
            workspace: self.0.clone(),
            status: LoadStatus::Loaded,
        }
    }
    fn save(&self, _workspace: &Workspace) -> std::io::Result<()> {
        Ok(())
    }
}

/// `sh -c "<script>"` — a real, short-lived child whose exit status we choose.
fn sh(script: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(script);
    cmd
}

/// A daemon hosting one `Regular`-mode session at the project root, and its id.
fn state_with_regular_session(
    project: &Path,
    settings_path: &Path,
) -> (Arc<DaemonState>, SessionId) {
    let mut session = Session::start_new(SessionLocation::Default);
    session.set_mode(TerminalMode::Regular);
    let id = session.id;
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), vec![session])]),
        worktree_names: BTreeMap::new(),
    };
    let catalog = Catalog::load(
        Box::new(FakeStore(workspace)),
        Box::new(JsonFileSettingsStore::at(settings_path.to_path_buf())),
    );
    (Arc::new(DaemonState::new(catalog)), id)
}

/// The wire lifecycle the catalog reports for `id` in `project`.
fn lifecycle(state: &DaemonState, project: &Path, id: SessionId) -> Option<WireLifecycle> {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .map(|s| s.lifecycle)
}

/// Block until the child is reaped (bounded).
fn wait_dead(pty: &PtySession) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while pty.is_alive() {
        assert!(Instant::now() < deadline, "child did not exit in time");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn an_unattended_clean_exit_stops_and_drops_the_session() {
    let project = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let (state, id) =
        state_with_regular_session(project.path(), &settings.path().join("settings.json"));

    // A live primary that exits cleanly (status 0), with no client ever attached.
    let handle = state.register_session(PtySession::spawn(id, sh("exit 0"), 100, None).unwrap());
    wait_dead(&handle);

    let changed = state.supervise_exited_sessions();

    assert_eq!(changed, vec![project.path().to_path_buf()]);
    assert_eq!(
        lifecycle(&state, project.path(), id),
        Some(WireLifecycle::Idle),
        "a clean exit leaves the session stopped (Idle), never restarted"
    );
    assert!(
        state.live_session(id).is_none(),
        "a cleanly-exited session's process is dropped"
    );
}

#[test]
fn an_unattended_crash_triggers_a_restart() {
    let project = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let (state, id) =
        state_with_regular_session(project.path(), &settings.path().join("settings.json"));

    // A live primary that crashes (nonzero), with no client attached.
    let handle = state.register_session(PtySession::spawn(id, sh("exit 1"), 100, None).unwrap());
    wait_dead(&handle);

    let changed = state.supervise_exited_sessions();

    assert_eq!(changed, vec![project.path().to_path_buf()]);
    assert_eq!(
        lifecycle(&state, project.path(), id),
        Some(WireLifecycle::Restarting { attempts: 1 }),
        "a crash advances the crash-loop counter"
    );
    assert!(
        state.live_session(id).is_some(),
        "a crashed session is respawned (a fresh live process exists)"
    );
}

#[test]
fn a_restart_that_survives_resets_to_running() {
    // Closes the L5 gap: a respawned process that stays up must return to Running (clearing the
    // crash-loop counter), not read as Restarting forever.
    let project = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let (state, id) =
        state_with_regular_session(project.path(), &settings.path().join("settings.json"));

    // Crash once → the next tick respawns the platform shell, which stays alive on its PTY.
    let handle = state.register_session(PtySession::spawn(id, sh("exit 1"), 100, None).unwrap());
    wait_dead(&handle);
    state.supervise_exited_sessions();
    assert_eq!(
        lifecycle(&state, project.path(), id),
        Some(WireLifecycle::Restarting { attempts: 1 }),
        "the tick that respawns does not itself reset — the survivor is only proven next tick"
    );
    assert!(state.live_session(id).is_some_and(|p| p.is_alive()));

    // A later tick sees the respawn still alive → resets it to Running (crash-loop counter cleared).
    state.supervise_exited_sessions();
    assert_eq!(
        lifecycle(&state, project.path(), id),
        Some(WireLifecycle::Running),
        "a restart that survives a supervision tick is healthy again"
    );
}

/// BUG-003 (`006-real-terminal-emulator` FR-014a, `010` FR-020a/SC-023): a crash respawn must come
/// back at the size the session was last given. Nothing about the viewer changed when the process
/// died, so a respawn at the 100×30 seed silently shrinks a session the user is still looking at —
/// and no `SessionResize` follows, because the pane never changed size.
#[test]
fn a_respawn_comes_back_at_the_sessions_recorded_size() {
    let project = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let (state, id) =
        state_with_regular_session(project.path(), &settings.path().join("settings.json"));

    // A client sized this session, then its process crashed.
    state.resize_session(id, 200, 55);
    let handle =
        state.register_session(PtySession::spawn(id, sh("exit 1"), 100, Some((200, 55))).unwrap());
    wait_dead(&handle);

    state.supervise_exited_sessions();

    let respawned = state.live_session(id).expect("a crash is respawned");
    let (cols, rows) = {
        let term = respawned.term().lock();
        (term.grid().columns(), term.grid().screen_lines())
    };
    assert_eq!(
        (cols, rows),
        (200, 55),
        "the respawn keeps the session's size instead of falling back to the seed"
    );

    respawned.kill().expect("kill");
}
