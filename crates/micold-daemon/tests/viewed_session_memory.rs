//! T035 (feature 025, convergence) — `DaemonState::set_viewed` is where the last-session memory is
//! actually decided, and until now nothing drove it.
//!
//! `catalog_adoption.rs` covers `Catalog::remember_foreground` directly: that it persists, that a
//! repeat writes nothing, that each project is independent. What none of those reach is the caller
//! — the guard in `set_viewed` that decides *whether to record at all*. A change there that started
//! clearing the memory whenever the client reports no session would pass every one of those tests
//! while costing the user the place they would have returned to (FR-005a, contract §2.6).
//!
//! So these assert against the **file on disk** rather than in-memory state. That is the durable
//! claim the feature makes, and `DaemonState` exposes no workspace accessor to shortcut it.

use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionId, SessionLocation};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tempfile::TempDir;

/// A daemon holding one project with two sessions, backed by a real store on disk.
fn daemon_with_two_sessions() -> (TempDir, DaemonState, PathBuf, SessionId, SessionId) {
    let dir = tempfile::tempdir().unwrap();
    let projects_path = dir.path().join("projects.json");
    let project = PathBuf::from("/repo");

    let first = Session::start_new(
        SessionLocation::Default,
        micold_core::session::AiCli::ClaudeCode,
    );
    let second = Session::start_new(
        SessionLocation::Default,
        micold_core::session::AiCli::ClaudeCode,
    );
    let (first_id, second_id) = (first.id, second.id);

    let mut workspace = Workspace {
        projects: vec![Project::new(project.clone(), true, Availability::Available)],
        active: Some(project.clone()),
        ..Default::default()
    };
    workspace
        .sessions
        .insert(project.clone(), vec![first, second]);
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();

    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(dir.path().join("settings.json"))),
    );
    (dir, DaemonState::new(catalog), project, first_id, second_id)
}

/// What the next launch would find for `project`.
fn remembered_on_disk(dir: &TempDir, project: &Path) -> Option<SessionId> {
    JsonFileStore::at(dir.path().join("projects.json"))
        .load()
        .workspace
        .foreground_by_project
        .get(project)
        .copied()
}

#[test]
fn viewing_a_session_records_it_for_the_next_launch() {
    let (dir, state, project, first, _) = daemon_with_two_sessions();
    let (client, _rx) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));

    state.set_viewed(client, project.clone(), Some(first));

    assert_eq!(
        remembered_on_disk(&dir, &project),
        Some(first),
        "the client reports which session it is showing on every path that changes it, and that \
         report is what the next launch reads back"
    );
}

#[test]
fn reporting_no_session_does_not_erase_the_memory() {
    let (dir, state, project, first, _) = daemon_with_two_sessions();
    let (client, _rx) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));
    state.set_viewed(client, project.clone(), Some(first));

    state.set_viewed(client, project.clone(), None);

    assert_eq!(
        remembered_on_disk(&dir, &project),
        Some(first),
        "the pointer goes to nothing for reasons the user never took — closing a session, an \
         internal cleanup after a reconcile. Erasing the memory on those would silently cost them \
         the place they would have returned to, and the restore already declines a session it \
         cannot show, so a stale memory is harmless where a lost one is not (FR-005a, §2.6)"
    );
}

#[test]
fn moving_to_another_session_replaces_the_memory() {
    let (dir, state, project, first, second) = daemon_with_two_sessions();
    let (client, _rx) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));
    state.set_viewed(client, project.clone(), Some(first));

    state.set_viewed(client, project.clone(), Some(second));

    assert_eq!(
        remembered_on_disk(&dir, &project),
        Some(second),
        "the one thing that does replace it: another session becoming current in that project \
         (FR-005a). Without this the previous test would also pass on code that simply never \
         recorded anything"
    );
}

#[test]
fn a_no_session_report_for_a_project_with_no_memory_stays_no_memory() {
    let (dir, state, project, _, _) = daemon_with_two_sessions();
    let (client, _rx) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));

    state.set_viewed(client, project.clone(), None);

    assert_eq!(
        remembered_on_disk(&dir, &project),
        None,
        "ignoring a no-session report must not invent one either — a project nobody has used a \
         session in still has no memory, and its next launch shows the project overview (FR-007)"
    );
}

#[test]
fn a_second_client_viewing_elsewhere_does_not_disturb_the_first_projects_memory() {
    let (dir, state, project, first, _) = daemon_with_two_sessions();
    let (client_a, _rx_a) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test-a",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test-a".into(),
        },
    ));
    let (client_b, _rx_b) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test-b",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test-b".into(),
        },
    ));
    state.set_viewed(client_a, project.clone(), Some(first));

    // Two windows, as the spec's edge cases allow. The second is looking at a different project.
    let elsewhere = PathBuf::from("/elsewhere");
    state.set_viewed(client_b, elsewhere.clone(), None);

    assert_eq!(
        remembered_on_disk(&dir, &project),
        Some(first),
        "both windows report through the same daemon, and one project's memory is not another's \
         (FR-008)"
    );
    assert_eq!(remembered_on_disk(&dir, &elsewhere), None);
}
