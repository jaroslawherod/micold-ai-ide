//! A crash loop slower than one supervision tick still reaches the guard (`005` BUG-004, FR-022a).
//!
//! `supervision_giveup.rs` covers the loop where every respawn dies at once. That one was always
//! caught, and this one never was: the tick reset the crash-loop counter for any respawn it saw
//! alive, one 250 ms tick after spawning it, so a CLI that came up, lived about a second and exited
//! non-zero was marked healthy on every cycle and restarted forever — one process a second, for as
//! long as the app was open. A bad `--resume` is exactly that shape.
//!
//! Time is injected (`supervise_exited_sessions_at`): the restart stability window is ten seconds,
//! and the claim is about readings relative to it, not about how long this test is willing to
//! sleep. The processes are real, because the defect lived in the daemon observing a live respawn
//! between two crashes — the seam `supervision.rs`'s unit tests, which drive the policy directly,
//! cannot reach.
//!
//! Its own binary: it points `SHELL` at a script that fails slowly, and that is process-global.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::clock::Uptime;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLocation, TerminalMode, MAX_RESTART_ATTEMPTS,
    RESTART_STABLE_AFTER,
};
use micold_core::settings::FakeSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

/// The daemon's real supervision cadence (`server.rs`'s `SUPERVISION_INTERVAL`).
const TICK: Duration = Duration::from_millis(250);

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
    let deadline = Instant::now() + Duration::from_secs(20);
    while pty.is_alive() {
        assert!(Instant::now() < deadline, "child did not exit in time");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn later(reading: Uptime, by: Duration) -> Uptime {
    let nanos = reading.saturating_sub(Uptime::from_nanos(0)) + by;
    Uptime::from_nanos(nanos.as_nanos() as u64)
}

#[test]
fn a_respawn_that_outlives_a_tick_but_not_the_window_still_counts_toward_failed() {
    // Every respawn is a shell that comes up, lives a second, and exits 1 — BUG-004's `claude
    // --resume` with nothing to resume, minus the CLI.
    let bin = tempfile::tempdir().unwrap();
    let slow_failure = bin.path().join("fails-after-a-second");
    std::fs::write(&slow_failure, "#!/bin/sh\nsleep 1\nexit 1\n").unwrap();
    std::fs::set_permissions(&slow_failure, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: this is the only test in this binary, so nothing reads `SHELL` concurrently.
    std::env::set_var("SHELL", &slow_failure);

    let project = tempfile::tempdir().unwrap();
    let (state, id) = state_with_regular_session(project.path());
    let handle = state.register_session(PtySession::spawn(id, sh("exit 1"), 100, None).unwrap());
    wait_dead(&handle);

    // An arbitrary starting reading; only the differences between readings mean anything.
    let mut now = Uptime::from_nanos(1_000_000_000_000);
    let mut walk: Vec<WireLifecycle> = Vec::new();
    // Bounded by cycles rather than a deadline: before the fix this never terminates, and the
    // bound is what turns "restarts forever" into a failure with the walk attached.
    for _ in 0..(MAX_RESTART_ATTEMPTS as usize * 2) {
        // The tick that observes the crash: counts it, and respawns if still under budget.
        state.supervise_exited_sessions_at(now);
        let after_crash = lifecycle(&state, project.path(), id).expect("the record outlives it");
        walk.push(after_crash.clone());
        if matches!(after_crash, WireLifecycle::Failed { .. }) {
            break;
        }
        let respawn = state
            .live_session(id)
            .expect("a crash under budget is respawned");
        assert!(
            respawn.is_alive(),
            "the respawn lives a second; it has to be alive for the next tick to observe"
        );

        // The next tick, 250 ms later, sees it still up. That is not recovery: it has been up for
        // one tick, not for the stability window.
        now = later(now, TICK);
        state.supervise_exited_sessions_at(now);
        let one_tick_later = lifecycle(&state, project.path(), id).unwrap();
        walk.push(one_tick_later.clone());
        assert!(
            matches!(one_tick_later, WireLifecycle::Restarting { .. }),
            "a respawn alive for one tick ({TICK:?}) is not yet a recovery — the window is \
             {RESTART_STABLE_AFTER:?}. Resetting here is BUG-004: the counter returns to zero on \
             every cycle and the session restarts forever. Walk: {walk:?}"
        );

        // …and then it exits, a second in, well inside the window.
        wait_dead(&respawn);
        now = later(now, Duration::from_secs(1));
    }

    let Some(WireLifecycle::Failed { attempts, .. }) = walk.last() else {
        panic!(
            "a crash loop of one-second failures has to reach the guard (FR-022a), not restart \
             indefinitely. Walk: {walk:?}"
        );
    };
    assert_eq!(
        *attempts, MAX_RESTART_ATTEMPTS,
        "the give-up spent the budget one slow failure at a time"
    );
    assert!(
        state.live_session(id).is_none(),
        "a session that gave up has its process dropped"
    );
}
