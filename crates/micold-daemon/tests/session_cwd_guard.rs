//! BUG-012 — a session whose working directory no longer exists must be **refused**, not started
//! somewhere else.
//!
//! `portable-pty`'s `CommandBuilder` filters a non-existent `cwd` out and silently substitutes the
//! user's home directory (`cmdbuilder.rs`'s `.filter(|dir| Path::new(dir).is_dir())
//! .unwrap_or(home)`), which also changes how the command binary itself is resolved. Nothing in this
//! project chose that: neither `start_session` nor `respawn_primary` ever checked the directory, and
//! the `WorktreeStatus::Missing` the daemon computes for the row badge is never read on the spawn
//! path. The result was an AI-CLI session — a thing the user gives instructions to — running against
//! `$HOME` with nothing on screen saying so.
//!
//! Uses Regular (shell) sessions so no `claude` binary is needed; both modes go through the same
//! guard.

#![cfg(unix)]

use std::collections::BTreeMap;

use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use uuid::Uuid;

const SESSION: u128 = 0x5E55_C0DE;

/// A catalog holding one Regular-mode session at `location` within `project_dir`.
fn catalog_with_session_at(
    project_dir: &std::path::Path,
    store_dir: &std::path::Path,
    location: SessionLocation,
) -> Catalog {
    let session = Session::restored(
        SessionId::from_uuid(Uuid::from_u128(SESSION)),
        location,
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
        ..Default::default()
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

/// The guard at the spawn itself. `respawn_primary` reaches the same function after a crash, so
/// covering it here covers the restart path too — that one has no client request behind it and so no
/// other place to be refused.
#[test]
fn spawning_a_shell_in_a_directory_that_does_not_exist_is_refused() {
    let project = tempfile::tempdir().unwrap();
    let gone = project
        .path()
        .join(".claude/worktrees/deleted-outside-the-app");
    assert!(!gone.exists(), "the fixture directory must not exist");

    // `PtySession` has no `Debug`, so the success arm is unwrapped by hand rather than `expect_err`.
    let Err(err) = PtySession::spawn_shell(
        SessionId::from_uuid(Uuid::from_u128(SESSION)),
        &gone,
        &[],
        1_000,
        None,
    ) else {
        panic!("a spawn into a missing directory must fail rather than fall back to $HOME");
    };

    assert_eq!(
        err.kind(),
        std::io::ErrorKind::NotFound,
        "the refusal must name a missing directory, not some other failure"
    );
}

/// The reported path: a worktree deleted from outside the application, then the session started —
/// by clicking it, or by feature 025's restore, which since that feature's BUG-002 makes every
/// reopen a resume.
#[test]
fn starting_a_session_whose_worktree_was_deleted_registers_no_process() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION));
    let state = DaemonState::new(catalog_with_session_at(
        project.path(),
        store.path(),
        // `SessionLocation::cwd` joins this under `.claude/worktrees`; it is never created.
        SessionLocation::Worktree("deleted-outside-the-app".into()),
    ));

    let err = state
        .start_session(id, LaunchMode::Resume)
        .expect_err("starting a session whose directory is gone must be refused");

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(
        state.live_session(id).is_none(),
        "a refused start must leave nothing in the live registry"
    );
}

/// The other half of the guard: it must refuse only what is actually missing. A session whose
/// directory exists still starts, so the fix cannot be satisfied by refusing everything.
#[test]
fn a_session_whose_directory_exists_still_starts() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION));
    let state = DaemonState::new(catalog_with_session_at(
        project.path(),
        store.path(),
        SessionLocation::Default,
    ));

    state
        .start_session(id, LaunchMode::Resume)
        .expect("a session at an existing directory must still start");
    let live = state.live_session(id).expect("registered");
    live.kill().expect("kill");
}
