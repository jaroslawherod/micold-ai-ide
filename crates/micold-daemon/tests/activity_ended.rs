//! `010` BUG-018 — a session that has ended reads *ended*, and one that never ran does not.
//!
//! Quickstart S3's third expectation ("the stopped one reads *ended*") had no producer:
//! `ActivityEvent::Ended` was constructed only by unit tests and the showcase, so a stopped session
//! fell back to `Unknown` and drew nothing — visually identical to a live session whose hooks never
//! fired. These gates drive the real supervision tick over a real process and read the signal off
//! the snapshot the daemon actually publishes.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{ActivitySignal, CatalogSnapshot, SessionSummary};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation, TerminalMode};
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

/// A project with two Regular sessions: one we will run and stop, one we will never start.
fn state_with_two_sessions(project: &Path) -> (DaemonState, SessionId, SessionId) {
    let mut ran = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    ran.set_mode(TerminalMode::Regular);
    let mut never = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    never.set_mode(TerminalMode::Regular);
    let (ran_id, never_id) = (ran.id, never.id);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), vec![ran, never])]),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let catalog = Catalog::load(
        Box::new(FakeProjectStore::loaded(workspace)),
        Box::new(FakeSettingsStore::new()),
    );
    (DaemonState::new(catalog), ran_id, never_id)
}

/// The published snapshot's summary for `id` — the overlaid one, not `sessions_for`'s catalog
/// defaults, since `activity` is exactly the field that is only projected at snapshot time.
fn summary(snapshot: &CatalogSnapshot, id: SessionId) -> SessionSummary {
    snapshot
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id == id)
        .cloned()
        .expect("the session is in the snapshot")
}

fn wait_dead(pty: &PtySession) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while pty.is_alive() {
        assert!(Instant::now() < deadline, "child did not exit in time");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_session_that_exited_cleanly_reads_ended_and_says_why() {
    let project = tempfile::tempdir().unwrap();
    let (state, ran, never) = state_with_two_sessions(project.path());

    let handle = state.register_session(PtySession::spawn(ran, sh("exit 0"), 100, None).unwrap());
    wait_dead(&handle);
    state.supervise_exited_sessions();

    let snapshot = state.catalog_snapshot();
    let ended = summary(&snapshot, ran).activity;
    let ActivitySignal::Ended { reason } = &ended else {
        panic!("a stopped session must read Ended, got {ended:?}");
    };
    // The badge is drawn from the variant; the reason is what a tooltip or a log line can say.
    // An empty one makes `Ended` indistinguishable from "we noticed something, no idea what".
    assert!(
        !reason.trim().is_empty(),
        "an Ended signal with no reason says only that it is over"
    );

    // The over-correction gate. A clean exit leaves the lifecycle `Idle` — which is *also* what a
    // session created and never started reads — so deriving `Ended` from the lifecycle alone would
    // put a hollow ring on a session that has never run.
    assert_eq!(
        summary(&snapshot, never).activity,
        ActivitySignal::Unknown,
        "a session that never ran has not ended; it has never reported (H1)"
    );
}

#[test]
fn a_new_run_does_not_inherit_the_previous_run_s_ending() {
    let project = tempfile::tempdir().unwrap();
    let (state, ran, _never) = state_with_two_sessions(project.path());

    let handle = state.register_session(PtySession::spawn(ran, sh("exit 0"), 100, None).unwrap());
    wait_dead(&handle);
    state.supervise_exited_sessions();
    assert!(
        matches!(
            summary(&state.catalog_snapshot(), ran).activity,
            ActivitySignal::Ended { .. }
        ),
        "precondition: the first run ended"
    );

    // Run it again. `Ended` is absorbing within the run it describes, not across runs.
    let handle = state.register_session(PtySession::spawn(ran, sh("sleep 30"), 100, None).unwrap());
    assert_eq!(
        summary(&state.catalog_snapshot(), ran).activity,
        ActivitySignal::Unknown,
        "a session that is running again has not ended"
    );

    // Now drop the live entry the way an explicit `SessionStop` does — straight out of the
    // registry, with no supervision tick and so no producer. Nothing observed *this* run ending,
    // so the honest answer is `Unknown` (H1). Re-showing the first run's verdict here would be an
    // answer about the wrong run, which is the failure a retained signal invites.
    for pty in state.remove_session(ran) {
        let _ = pty.kill();
    }
    assert_eq!(
        summary(&state.catalog_snapshot(), ran).activity,
        ActivitySignal::Unknown,
        "the previous run's ending must not stand in for this one"
    );
}
