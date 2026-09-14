//! Phase 6 (US4) — restart supervision runs with no client attached (T058, FR-004/FR-005).
//!
//! These drive [`DaemonState::supervise_exited_sessions`] directly — no socket, no viewer — which is
//! exactly the unattended path. They assert the catalog lifecycle and the live registry move as the
//! policy dictates: a clean exit stops the session and drops it; a crash advances the crash-loop
//! counter and respawns it. The crash-loop *give-up* case lives in `supervision_giveup.rs` (it needs
//! a crashing shell, so it owns its own test binary to keep `SHELL` isolated).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use micold_core::clock::Uptime;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLocation, TerminalMode, RESTART_STABLE_AFTER,
};
use micold_core::settings::FakeSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

/// `sh -c "<script>"` — a real, short-lived child whose exit status we choose.
#[cfg(unix)]
fn sh(script: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(script);
    cmd
}
/// `cmd /c` reads `exit <status>` the same way, which is all these scripts do on Windows.
#[cfg(windows)]
fn sh(script: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("cmd");
    cmd.arg("/c");
    cmd.arg(script);
    cmd
}

/// A daemon hosting one `Regular`-mode session at the project root, and its id.
fn state_with_regular_session(project: &Path) -> (Arc<DaemonState>, SessionId) {
    let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
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
        ..Default::default()
    };
    let catalog = Catalog::load(
        Box::new(FakeProjectStore::loaded(workspace)),
        Box::new(FakeSettingsStore::new()),
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
    let (state, id) = state_with_regular_session(project.path());

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
    let (state, id) = state_with_regular_session(project.path());

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
fn a_restart_that_survives_the_stability_window_resets_to_running() {
    // Closes the L5 gap: a respawned process that stays up must return to Running (clearing the
    // crash-loop counter), not read as Restarting forever — so crashes far apart never accumulate.
    // But only once it has stayed up for the window (`005` BUG-004): surviving a single tick is not
    // recovery, or a process that fails a second in restarts forever. Readings are injected; the
    // window is ten seconds and this is about the readings, not about sleeping through it.
    let project = tempfile::tempdir().unwrap();
    let (state, id) = state_with_regular_session(project.path());

    // Crash once → the tick respawns the platform shell, which stays alive on its PTY.
    let handle = state.register_session(PtySession::spawn(id, sh("exit 1"), 100, None).unwrap());
    wait_dead(&handle);
    let respawned_at = Uptime::from_nanos(1_000_000_000_000);
    state.supervise_exited_sessions_at(respawned_at);
    assert_eq!(
        lifecycle(&state, project.path(), id),
        Some(WireLifecycle::Restarting { attempts: 1 }),
        "the tick that respawns does not itself reset"
    );
    assert!(state.live_session(id).is_some_and(|p| p.is_alive()));

    // One tick later it is alive, and not yet proven.
    state.supervise_exited_sessions_at(later(respawned_at, Duration::from_millis(250)));
    assert_eq!(
        lifecycle(&state, project.path(), id),
        Some(WireLifecycle::Restarting { attempts: 1 }),
        "a respawn alive for one tick is still inside the stability window"
    );

    // Still alive once the window has passed → Running, crash-loop counter cleared.
    state.supervise_exited_sessions_at(later(respawned_at, RESTART_STABLE_AFTER));
    assert_eq!(
        lifecycle(&state, project.path(), id),
        Some(WireLifecycle::Running),
        "a restart that survives the stability window is healthy again"
    );

    if let Some(live) = state.live_session(id) {
        live.kill().expect("kill");
    }
}

fn later(reading: Uptime, by: Duration) -> Uptime {
    let nanos = reading.saturating_sub(Uptime::from_nanos(0)) + by;
    Uptime::from_nanos(nanos.as_nanos() as u64)
}

/// BUG-003 (`006-real-terminal-emulator` FR-014a, `010` FR-020a/SC-023): a crash respawn must come
/// back at the size the session was last given. Nothing about the viewer changed when the process
/// died, so a respawn at the 100×30 seed silently shrinks a session the user is still looking at —
/// and no `SessionResize` follows, because the pane never changed size.
#[test]
fn a_respawn_comes_back_at_the_sessions_recorded_size() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = state_with_regular_session(project.path());

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
