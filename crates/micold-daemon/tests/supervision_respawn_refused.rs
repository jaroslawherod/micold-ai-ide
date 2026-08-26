//! A respawn that cannot spawn still reaches a terminus (BUG-024, FR-005).
//!
//! `supervision_giveup.rs` covers the loop where every respawn *succeeds* and then crashes. This is
//! the other shape, and the one BUG-024 is about: the respawn itself fails, so there is no new
//! process to observe. The daemon used to count a further crash, throw away the answer, and drop
//! the live entry — and since `supervise_exited_sessions` walks `inner.sessions`, nothing could
//! ever observe the session again. It sat at `Restarting` for the life of the daemon: no process,
//! no further supervision, no report.
//!
//! The refusal is this project's own guard rather than a contrived failure. `ensure_cwd_exists`
//! (010 BUG-012) refuses to spawn into a directory that is gone, which is what a worktree deleted
//! from outside the application leaves behind — so this drives exactly the route that report names.
//!
//! Its own binary, like `supervision_giveup.rs`: it deletes the project directory out from under a
//! live session, and that is not something to do beside tests sharing a process.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLocation, TerminalMode, MAX_RESTART_ATTEMPTS,
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

/// A regular-terminal session at the project root, so the spawn under test is `spawn_shell` and no
/// AI CLI has to be on `PATH` for this to be about supervision.
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
fn a_respawn_that_cannot_spawn_still_reaches_failed_and_says_so() {
    let project = tempfile::tempdir().unwrap();
    let project: PathBuf = project.keep();
    let (state, id) = state_with_regular_session(&project);

    // A live primary that crashes, exactly as the give-up test sets one up…
    let handle = state.register_session(PtySession::spawn(id, sh("exit 1"), 100, None).unwrap());
    wait_dead(&handle);
    // …and then the working directory goes away, which is what a worktree deleted from outside
    // leaves behind. Every respawn from here on is refused before a process exists (BUG-012).
    std::fs::remove_dir_all(&project).expect("remove the session's working directory");

    // No sleep between cycles, unlike the give-up test: nothing is ever respawned, so there is no
    // fresh child to wait for. Each cycle observes the same dead primary and advances the budget.
    // Every distinct lifecycle the record passes through is recorded, because *how* it gets to the
    // terminus is half the claim — see the walk assertion below.
    let mut walk: Vec<WireLifecycle> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    let changed = loop {
        let changed = state.supervise_exited_sessions();
        let now = lifecycle(&state, &project, id).expect("the record outlives the process");
        if walk.last() != Some(&now) {
            walk.push(now.clone());
        }
        if matches!(now, WireLifecycle::Failed { .. }) {
            break changed;
        }
        assert!(
            Instant::now() < deadline,
            "a session whose respawn cannot spawn has to reach a terminus. It is not running — \
             there is no process — and if it is never `Failed` it is reported as `restarting` for \
             the life of the daemon, with nothing left in the live registry for a later tick to \
             observe (BUG-024). Walk so far: {walk:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    };

    // One attempt per tick, counted where every other exit is counted. A failed respawn that also
    // counted itself would advance the budget twice per tick and skip a rung of this walk — and the
    // wire `Failed` cannot show that, since it reports `attempts` as the constant `MAX_RESTART_
    // ATTEMPTS` regardless of what was actually spent (`catalog.rs`, `wire_lifecycle`). The rungs
    // are where the count is observable, so this asserts the whole walk rather than the terminus.
    let expected: Vec<WireLifecycle> = (1..MAX_RESTART_ATTEMPTS)
        .map(|attempts| WireLifecycle::Restarting { attempts })
        .chain([WireLifecycle::Failed {
            reason: String::new(),
            attempts: MAX_RESTART_ATTEMPTS,
        }])
        .collect();
    assert_eq!(walk, expected, "budget spent one attempt at a time");
    assert!(
        changed.contains(&project),
        "and the give-up is announced: `Failed` is the state that tells the user to look at this, \
         and a transition no `CatalogChanged` follows is one no client ever draws. Reported: \
         {changed:?}"
    );
    assert!(
        state.live_session(id).is_none(),
        "a session that gave up has its process dropped (no restart pending)"
    );
}
