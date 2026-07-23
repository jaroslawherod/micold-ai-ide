//! Phase 4 (US2) — `ClientMsg::SessionStart` brings a durable session to life: the daemon spawns its
//! process from the catalog (cwd from the session's location, mode = which process) and adopts it
//! into the live registry, so a client can then view and drive it (FR-006, data-model §Session).
//!
//! Uses a Regular (shell) session so the test spawns the platform shell — no `claude` binary needed.
//! The AI-CLI spawn path is compile-covered by the same `start_session` code.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}

/// A catalog holding one Regular-mode session at the project root, whose path is a real directory so
/// the spawned shell can `cwd` into it.
fn catalog_with_shell_session(
    project_dir: &std::path::Path,
    store_dir: &std::path::Path,
) -> Catalog {
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project_dir.to_path_buf(), vec![session]);

    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
    };

    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

#[test]
fn session_start_spawns_and_registers_a_durable_session() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));

    assert!(
        state.live_session(id).is_none(),
        "the session is not live until started"
    );

    state
        .start_session(id)
        .expect("start must spawn the shell session");

    let live = state
        .live_session(id)
        .expect("the session is now in the registry");
    assert!(
        wait_until(Duration::from_secs(5), || live.is_alive()),
        "the spawned shell process must be running"
    );

    // Idempotent: a second Start is a no-op, not a double-spawn.
    state
        .start_session(id)
        .expect("a redundant start is a no-op");
    assert!(state.live_session(id).is_some());

    // Test-owned process: stop it so nothing leaks.
    live.kill().expect("kill");
}

#[test]
fn starting_an_unknown_session_is_an_error_not_a_panic() {
    let store = tempfile::tempdir().unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.path().join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    ));
    let err = state
        .start_session(SessionId::new())
        .expect_err("an unknown session cannot be started");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn create_session_adds_a_daemon_owned_session_to_the_catalog() {
    use micold_core::project::{Availability, Project};
    use micold_core::store::ProjectStore;
    use micold_core::workspace::Workspace;

    let project = std::path::PathBuf::from("/repo/alpha");
    let store = tempfile::tempdir().unwrap();
    // Seed a catalog with a project but no sessions.
    let workspace = Workspace {
        projects: vec![Project::new(project.clone(), true, Availability::Available)],
        active: Some(project.clone()),
        sessions: BTreeMap::new(),
        worktree_names: BTreeMap::new(),
    };
    let projects_path = store.path().join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    ));

    // The daemon assigns the id and records the session at the project root (empty worktree_dir).
    let id = state
        .create_session(&project, "")
        .expect("create must succeed");

    let snapshot = state.welcome_payload().0;
    let proj = snapshot
        .projects
        .iter()
        .find(|p| p.path == project)
        .expect("project in snapshot");
    assert_eq!(proj.sessions.len(), 1, "the created session appears");
    assert_eq!(proj.sessions[0].id, id);
    assert_eq!(
        proj.sessions[0].worktree_dir, None,
        "an empty worktree_dir is the Default (root) location"
    );
}
