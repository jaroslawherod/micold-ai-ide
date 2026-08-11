//! Phase 6 (US4) — a crash loop gives up in a durable `Failed` state, unattended (T059, FR-005).
//!
//! This owns its own test binary because it sets `SHELL` to a crashing command (`/bin/false`) so
//! every *respawn* also crashes — the only way to exercise the give-up path end to end. A separate
//! binary keeps that process-global env off every other test.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    Session, SessionId, SessionLocation, TerminalMode, MAX_RESTART_ATTEMPTS,
};
use micold_core::settings::FakeSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

fn sh(script: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(script);
    cmd
}

fn state_with_regular_session(project: &Path) -> (Arc<DaemonState>, SessionId) {
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
        Box::new(FakeProjectStore::loaded(workspace)),
        Box::new(FakeSettingsStore::new()),
    );
    (Arc::new(DaemonState::new(catalog)), id)
}

fn lifecycle(state: &DaemonState, project: &Path, id: SessionId) -> Option<WireLifecycle> {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .map(|s| s.lifecycle)
}

fn wait_dead(pty: &PtySession) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while pty.is_alive() {
        assert!(Instant::now() < deadline, "child did not exit in time");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_crash_loop_settles_failed_and_drops_the_session() {
    // Force every respawn to crash immediately: the platform shell is `/bin/false` (exits nonzero).
    // SAFETY: this is the only test in this binary, so nothing reads `SHELL` concurrently.
    std::env::set_var("SHELL", "/bin/false");

    let project = tempfile::tempdir().unwrap();
    let (state, id) = state_with_regular_session(project.path());

    // The initial primary crashes; every respawn (a `/bin/false` shell) crashes again.
    let handle = state.register_session(PtySession::spawn(id, sh("exit 1"), 100, None).unwrap());
    wait_dead(&handle);

    // Drive supervision cycles until it gives up; each cycle detects the dead child, advances the
    // counter, and respawns another crashing shell (which we let exit before the next cycle).
    let deadline = Instant::now() + Duration::from_secs(20);
    let attempts = loop {
        state.supervise_exited_sessions();
        if let Some(WireLifecycle::Failed { attempts, .. }) = lifecycle(&state, project.path(), id)
        {
            break attempts;
        }
        assert!(
            Instant::now() < deadline,
            "supervision never settled Failed: {:?}",
            lifecycle(&state, project.path(), id)
        );
        // Let the just-respawned /bin/false exit before the next cycle observes it.
        std::thread::sleep(Duration::from_millis(60));
    };

    assert_eq!(
        attempts, MAX_RESTART_ATTEMPTS,
        "give-up records the full retry budget"
    );
    assert!(
        state.live_session(id).is_none(),
        "a session that gave up has its process dropped (no restart pending)"
    );

    std::env::remove_var("SHELL");
}
