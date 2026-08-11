//! T021 — the Catalog adopts the existing durable state in place and is its single writer
//! (data-model §Catalog C1–C4, FR-008, FR-012).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::{JsonFileSettingsStore, SettingsStore, MIN_SCROLLBACK_LINES};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use uuid::Uuid;

fn seeded_workspace(project_path: &Path) -> Workspace {
    let session = Session::restored(
        SessionId::from_uuid(Uuid::from_u128(0x42)),
        SessionLocation::Worktree("feat-login".into()),
        SessionLabel::Named("Fix login".into()),
        TerminalMode::AiCli,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project_path.to_path_buf(), vec![session]);

    let mut worktree_names = BTreeMap::new();
    let mut names = BTreeMap::new();
    names.insert("feat-login".to_string(), "Login work".to_string());
    worktree_names.insert(project_path.to_path_buf(), names);

    Workspace {
        projects: vec![Project::new(
            project_path.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project_path.to_path_buf()),
        sessions,
        worktree_names,
        foreground_by_project: BTreeMap::new(),
    }
}

#[test]
fn adopts_an_existing_projects_json_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let projects_path = dir.path().join("projects.json");
    let settings_path = dir.path().join("settings.json");
    let project_path = PathBuf::from("/repo/alpha");

    // Seed the store exactly as the existing app writes it — the shape is unchanged.
    let store = JsonFileStore::at(projects_path.clone());
    store.save(&seeded_workspace(&project_path)).unwrap();

    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(settings_path)),
    );
    assert_eq!(catalog.load_status(), LoadStatus::Loaded);

    let snapshot = catalog.snapshot();
    assert_eq!(snapshot.schema_version, 1);
    assert_eq!(snapshot.last_active, Some(project_path.clone()));
    assert_eq!(snapshot.projects.len(), 1);

    let project = &snapshot.projects[0];
    assert_eq!(project.path, project_path);
    assert!(project.is_git_repo);
    assert!(project.available);

    // The persisted session survives with its identity and title.
    assert_eq!(project.sessions.len(), 1);
    let session = &project.sessions[0];
    assert_eq!(session.id, SessionId::from_uuid(Uuid::from_u128(0x42)));
    assert_eq!(session.worktree_dir.as_deref(), Some("feat-login"));
    assert_eq!(session.title, SessionLabel::Named("Fix login".into()));
    // Restored sessions are Idle; activity is never persisted (data-model S3/A4).
    assert_eq!(session.lifecycle, WireLifecycle::Idle);

    // The worktree is known from its display-name override + the session binding.
    assert_eq!(project.worktrees.len(), 1);
    assert_eq!(project.worktrees[0].dir_name, "feat-login");
    assert_eq!(project.worktrees[0].display_name, "Login work");
}

#[test]
fn archived_sessions_are_excluded_from_the_snapshot() {
    // Anti-resurrection (main 93a0a08/7dc9c8a): a session marked `archived` in durable state — its
    // worktree was deleted, or it was removed — must never reappear via the daemon catalog, which is
    // the single source clients render.
    let dir = tempfile::tempdir().unwrap();
    let projects_path = dir.path().join("projects.json");
    let settings_path = dir.path().join("settings.json");
    let project_path = PathBuf::from("/repo/alpha");

    let live = Session::restored(
        SessionId::from_uuid(Uuid::from_u128(0x1)),
        SessionLocation::Worktree("feat-live".into()),
        SessionLabel::Named("Live".into()),
        TerminalMode::AiCli,
    );
    let mut archived = Session::restored(
        SessionId::from_uuid(Uuid::from_u128(0x2)),
        SessionLocation::Worktree("feat-gone".into()),
        SessionLabel::Named("Gone".into()),
        TerminalMode::AiCli,
    );
    archived.archive();

    let mut sessions = BTreeMap::new();
    sessions.insert(project_path.to_path_buf(), vec![live, archived]);
    let workspace = Workspace {
        projects: vec![Project::new(
            project_path.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project_path.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        foreground_by_project: BTreeMap::new(),
    };
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();

    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(settings_path)),
    );
    let snapshot = catalog.snapshot();
    let project = &snapshot.projects[0];

    // Only the live session surfaces; the archived one is gone.
    assert_eq!(
        project.sessions.len(),
        1,
        "archived session must be filtered out"
    );
    assert_eq!(
        project.sessions[0].id,
        SessionId::from_uuid(Uuid::from_u128(0x1))
    );
    assert!(
        project
            .sessions
            .iter()
            .all(|s| s.title != SessionLabel::Named("Gone".into())),
        "the archived session must not appear"
    );
}

#[test]
fn a_corrupt_catalog_is_preserved_and_recovered_to_empty() {
    // C4: an unparseable file is kept as `.json.bak` and an empty catalog is loaded with a status
    // the daemon can surface, rather than crashing or silently discarding the user's data.
    let dir = tempfile::tempdir().unwrap();
    let projects_path = dir.path().join("projects.json");
    std::fs::write(&projects_path, "{ this is not json").unwrap();

    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path.clone())),
        Box::new(JsonFileSettingsStore::at(dir.path().join("settings.json"))),
    );

    assert_eq!(catalog.load_status(), LoadStatus::Recovered);
    assert!(catalog.snapshot().projects.is_empty());
    assert!(
        projects_path.with_extension("json.bak").exists(),
        "the corrupt file must be preserved, not discarded"
    );
}

#[test]
fn a_missing_catalog_is_a_clean_first_run() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(dir.path().join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(dir.path().join("settings.json"))),
    );
    assert_eq!(catalog.load_status(), LoadStatus::Missing);
    assert!(catalog.snapshot().projects.is_empty());
}

#[test]
fn scrollback_is_clamped_and_persisted_by_the_daemon_as_single_writer() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");

    let mut catalog = Catalog::load(
        Box::new(JsonFileStore::at(dir.path().join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(settings_path.clone())),
    );

    // Below the supported minimum: clamped, not rejected (FR-012a).
    let applied = catalog.set_scrollback(5).unwrap();
    assert_eq!(applied, MIN_SCROLLBACK_LINES);
    assert_eq!(
        catalog.settings_wire().scrollback_lines,
        MIN_SCROLLBACK_LINES
    );

    // It reached disk — the daemon is the writer.
    let reloaded = JsonFileSettingsStore::at(settings_path).load();
    assert_eq!(reloaded.settings.scrollback_lines, MIN_SCROLLBACK_LINES);
}

#[test]
fn persist_round_trips_the_workspace_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let projects_path = dir.path().join("projects.json");
    let project_path = PathBuf::from("/repo/beta");

    let store = JsonFileStore::at(projects_path.clone());
    store.save(&seeded_workspace(&project_path)).unwrap();

    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path.clone())),
        Box::new(JsonFileSettingsStore::at(dir.path().join("settings.json"))),
    );
    catalog.persist().unwrap();

    // No temp file left behind, and the catalog still reads back identically.
    assert!(!projects_path.with_extension("json.tmp").exists());
    let again = JsonFileStore::at(projects_path).load();
    assert_eq!(again.status, LoadStatus::Loaded);
    assert_eq!(again.workspace.projects.len(), 1);
}

// --- Feature 025: the catalog remembers which session a project was last showing ---------------
//
// The daemon is the store's single writer, so it is the only thing that may persist this. The
// client keeps the same value in memory for its own run and never writes it — `store.rs` has no
// locking, and a client-side save would clobber whatever the daemon wrote since the client loaded.

/// A catalog holding one project with one session, plus the temp dir keeping its files alive.
fn catalog_with_one_session() -> (tempfile::TempDir, Catalog, PathBuf, SessionId) {
    let dir = tempfile::tempdir().unwrap();
    let projects_path = dir.path().join("projects.json");
    let settings_path = dir.path().join("settings.json");
    let project_path = PathBuf::from("/repo");

    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: project_path.clone(),
        display_name: "repo".into(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    let session = Session::start_new(SessionLocation::Default);
    let id = session.id;
    ws.sessions.insert(project_path.clone(), vec![session]);
    JsonFileStore::at(projects_path.clone()).save(&ws).unwrap();

    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(settings_path)),
    );
    (dir, catalog, project_path, id)
}

#[test]
fn recording_the_viewed_session_persists_it() {
    let (dir, mut catalog, project, id) = catalog_with_one_session();

    let wrote = catalog.remember_foreground(&project, id).unwrap();

    assert!(
        wrote,
        "a session that was not remembered before is a change, and is written"
    );
    let reloaded = JsonFileStore::at(dir.path().join("projects.json"))
        .load()
        .workspace;
    assert_eq!(
        reloaded.foreground_by_project.get(&project),
        Some(&id),
        "it has to survive the process, which is the whole point — an in-memory record is what \
         this feature already had"
    );
}

#[test]
fn recording_the_same_session_again_writes_nothing() {
    let (_dir, mut catalog, project, id) = catalog_with_one_session();
    assert!(catalog.remember_foreground(&project, id).unwrap());

    let wrote = catalog.remember_foreground(&project, id).unwrap();

    assert!(
        !wrote,
        "attach re-sends the current session id and a session start may name the session already \
         in front of the user. Writing on those rewrites a file holding every session record with \
         identical content, on every reconnect (FR-001a)"
    );
}

#[test]
fn each_project_is_remembered_independently() {
    let (dir, mut catalog, project, id) = catalog_with_one_session();
    let other = PathBuf::from("/elsewhere");

    catalog.remember_foreground(&project, id).unwrap();

    let reloaded = JsonFileStore::at(dir.path().join("projects.json"))
        .load()
        .workspace;
    assert_eq!(reloaded.foreground_by_project.get(&project), Some(&id));
    assert_eq!(
        reloaded.foreground_by_project.get(&other),
        None,
        "one project's memory is not another's (FR-008)"
    );
}

#[test]
fn nothing_in_the_catalog_erases_a_projects_memory() {
    let (dir, mut catalog, project, id) = catalog_with_one_session();
    catalog.remember_foreground(&project, id).unwrap();

    // Archiving the very session the memory names — the durable half of closing it.
    catalog.archive_worktree_sessions(&project, "").ok();

    let reloaded = JsonFileStore::at(dir.path().join("projects.json"))
        .load()
        .workspace;
    assert_eq!(
        reloaded.foreground_by_project.get(&project),
        Some(&id),
        "the memory is replaced by another session becoming current and by nothing else. Closing \
         the session it names leaves it in place — restoring already declines a closed session, so \
         a stale memory costs nothing, where an erased one costs the user their place (FR-005a)"
    );
}
