//! #1459 — a crash restart of a session whose conversation is gone must say so, not crash-loop.
//!
//! `start_session` refuses a resume whose transcript is missing, but the supervisor's respawn
//! (`--resume` again after an exit) never asked: the CLI failed on the missing transcript, was
//! restarted, failed again, and the row ended `Failed` after the whole retry budget with the CLI's
//! own error. A missing transcript is known before the respawn, so it is reported at once and
//! never reaches the crash-loop counter.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::FakeSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

fn state_with_copilot_session(project: &Path) -> (Arc<DaemonState>, SessionId) {
    let id = SessionId::from_uuid(Uuid::from_u128(0x1459));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::Copilot,
    );
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

#[test]
fn a_crash_of_a_session_whose_conversation_is_gone_is_reported_not_retried() {
    // SAFETY: the only test in this binary, so nothing reads the environment concurrently.
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("COPILOT_HOME", home.path());
    let project = tempfile::tempdir().unwrap();
    let (state, id) = state_with_copilot_session(project.path());

    // A `copilot` that fails at once, as the real one does when asked to resume what it lacks.
    let bin = tempfile::tempdir().unwrap();
    let stub = bin.path().join("copilot");
    std::fs::write(&stub, "#!/bin/sh\nexit 1\n").unwrap();
    std::fs::set_permissions(&stub, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin.path().to_path_buf()).chain(
            std::env::var_os("PATH")
                .iter()
                .flat_map(std::env::split_paths),
        ),
    )
    .unwrap();
    std::env::set_var("PATH", path);

    // The record is `Idle`, not offered as resumable, so the start does not refuse it: the CLI is
    // spawned and answers.
    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .unwrap();
    let handle = state.live_session(id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while handle.is_alive() {
        assert!(Instant::now() < deadline, "child did not exit in time");
        std::thread::sleep(Duration::from_millis(20));
    }

    state.supervise_exited_sessions();

    // The wire view, which is where a start failure's reason is laid over the record.
    let lifecycle = state
        .catalog_snapshot()
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id == id)
        .map(|s| s.lifecycle);
    let Some(WireLifecycle::Failed { reason, attempts }) = lifecycle else {
        panic!("expected Failed at once, got {lifecycle:?}");
    };
    assert!(
        reason.contains("no longer has this conversation"),
        "got {reason:?}"
    );
    assert_eq!(attempts, 0, "a missing transcript is not a crash loop");
    assert!(state.live_session(id).is_none(), "nothing is respawned");
}
