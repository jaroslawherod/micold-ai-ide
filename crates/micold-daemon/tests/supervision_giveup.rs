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

use micold_client::app::State;
use micold_client::catalog_sync::reconcile_catalog;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{ActivitySignal, WireLifecycle};
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
    let (reason, attempts) = loop {
        state.supervise_exited_sessions();
        if let Some(WireLifecycle::Failed { reason, attempts }) =
            lifecycle(&state, project.path(), id)
        {
            break (reason, attempts);
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
    // The give-up has to say *why*, and the reason is the only place it can (010 BUG-017). This is
    // not decoration: the client announces a failure exactly when the wire reason is non-empty
    // (`catalog_sync::announce_start_failures`, feature 026 T088), so an empty one here means a
    // crash loop that happened while nobody was watching is never announced at all — it settles
    // into a `failed` word in the status bar and nothing else.
    assert!(
        !reason.trim().is_empty(),
        "a give-up with no reason tells the user only that it failed"
    );
    // And it has to name the exit it gave up on. `/bin/false` exits 1, so a reason that does not
    // mention that status is a sentence about the budget with the diagnosis left out.
    assert!(
        reason.contains('1'),
        "the reason should name the exit the session kept dying on; got {reason:?}"
    );
    assert!(
        reason.contains(&MAX_RESTART_ATTEMPTS.to_string()),
        "and how many times it tried; got {reason:?}"
    );
    assert!(
        state.live_session(id).is_none(),
        "a session that gave up has its process dropped (no restart pending)"
    );
    // And the row says it is over (`010` BUG-018). The give-up drops the live entry, and the FSM
    // went with it, so before this the badge fell back to `Unknown` — the same nothing a live
    // session whose hooks never fired draws. The reason is the tick's own word for the outcome, not
    // the sentence above: that one belongs to `Failed`, which the client reads separately.
    let activity = state
        .catalog_snapshot()
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id == id)
        .map(|s| s.activity.clone());
    assert_eq!(
        activity,
        Some(ActivitySignal::Ended {
            reason: "crash loop".to_string()
        }),
        "a session that gave up reads Ended"
    );

    // And the join: the snapshot the daemon would really publish, fed to the real client. This is
    // the half neither side can fail on its own — `announce_start_failures` reads the wire reason,
    // so a give-up the daemon words correctly and the wire flattens reaches the user as nothing.
    let mut core = State::default();
    reconcile_catalog(&mut core, &state.welcome_payload().0, false);
    assert_eq!(
        core.notifications
            .queue
            .visible()
            .map(|n| n.message.clone()),
        Some(reason.clone()),
        "the give-up the daemon recorded is what the user is told"
    );

    std::env::remove_var("SHELL");
}
