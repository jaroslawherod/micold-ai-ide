//! T048/T049 (bugfix 002/BUG-001, Phase 8): the per-project storage split isolates a storage
//! fault to a single project (FR-012a), and a pre-split `projects.json` migrates its embedded
//! session/worktree-name data into the new per-project state file on next save.

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn project(path: &str, name: &str, git: bool) -> Project {
    Project {
        path: PathBuf::from(path),
        display_name: name.to_string(),
        is_git_repo: git,
        availability: Availability::Available,
    }
}

#[test]
fn corrupt_one_project_state_file_does_not_affect_others() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a", "a", true));
    ws.projects.push(project("/b", "b", true));
    ws.sessions.insert(
        PathBuf::from("/a"),
        vec![Session::restored(
            SessionId::new(),
            SessionLocation::Default,
            SessionLabel::Named("A session".to_string()),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        )],
    );
    ws.sessions.insert(
        PathBuf::from("/b"),
        vec![Session::restored(
            SessionId::new(),
            SessionLocation::Default,
            SessionLabel::Named("B session".to_string()),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        )],
    );
    store.save(&ws).unwrap();

    // Corrupt only `/a`'s own state file.
    let a_state_path = store.project_state_path(Path::new("/a"));
    std::fs::write(&a_state_path, "not json").unwrap();

    let out = store.load();
    assert_eq!(
        out.status,
        LoadStatus::Loaded,
        "the catalog itself is untouched by a fault in one project's state file"
    );
    assert_eq!(
        out.workspace.projects.len(),
        2,
        "both projects remain known"
    );
    assert!(
        !out.workspace.sessions.contains_key(&PathBuf::from("/a")),
        "a's sessions degrade to empty, isolated to this project only"
    );
    let b_sessions = out
        .workspace
        .sessions
        .get(&PathBuf::from("/b"))
        .expect("b's sessions survive a's corruption");
    assert_eq!(b_sessions.len(), 1);
    assert_eq!(
        b_sessions[0].label,
        SessionLabel::Named("B session".to_string())
    );

    // The corrupt file is left exactly where it is (029 FR-011): renaming it aside preserved the
    // bytes but let the next reader mistake the damage for a project that never had a state file.
    assert_eq!(std::fs::read_to_string(&a_state_path).unwrap(), "not json");
}

#[test]
fn removed_project_state_file_degrades_only_that_project() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a", "a", true));
    ws.projects.push(project("/b", "b", true));
    ws.sessions.insert(
        PathBuf::from("/b"),
        vec![Session::restored(
            SessionId::new(),
            SessionLocation::Default,
            SessionLabel::Named("B session".to_string()),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        )],
    );
    store.save(&ws).unwrap();

    // Delete `/a`'s state file outright (e.g. lost mid-write, wiped by another process).
    let a_state_path = store.project_state_path(Path::new("/a"));
    std::fs::remove_file(&a_state_path).unwrap();

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);
    assert_eq!(out.workspace.projects.len(), 2, "the catalog is unaffected");
    assert!(!out.workspace.sessions.contains_key(&PathBuf::from("/a")));
    assert_eq!(
        out.workspace
            .sessions
            .get(&PathBuf::from("/b"))
            .expect("b's sessions survive a's missing file")
            .len(),
        1
    );
}

#[test]
fn pre_split_embedded_sessions_migrate_to_per_project_file_on_next_save() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    let json = r#"{"schema_version":1,"last_active":"/a","projects":[
        {"path":"/a","display_name":"a","is_git_repo":true,"sessions":[
            {"id":"11111111-1111-1111-1111-111111111111","worktree_dir":null,"title":"Old session"}
        ],"worktree_display_names":{"feat-x":"My Feature"}}
    ]}"#;
    std::fs::write(&path, json).unwrap();
    let store = JsonFileStore::at(path.clone());

    // First load: no per-project file exists yet, so the embedded legacy data is the fallback.
    let loaded = store.load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    let sessions = loaded
        .workspace
        .sessions
        .get(&PathBuf::from("/a"))
        .expect("legacy embedded sessions are recovered");
    assert_eq!(sessions.len(), 1);
    assert_eq!(loaded.workspace.worktree_name("feat-x"), Some("My Feature"));

    // Re-saving (as the app does after every mutating load/boot) is the migration write.
    store.save(&loaded.workspace).unwrap();

    let raw_catalog = std::fs::read_to_string(&path).unwrap();
    assert!(
        !raw_catalog.contains("Old session"),
        "catalog must not re-embed session data after migration: {raw_catalog}"
    );
    assert!(
        !raw_catalog.contains("My Feature"),
        "catalog must not re-embed worktree names after migration: {raw_catalog}"
    );

    let state_path = store.project_state_path(Path::new("/a"));
    let raw_state = std::fs::read_to_string(&state_path).unwrap();
    assert!(raw_state.contains("Old session"));
    assert!(raw_state.contains("My Feature"));

    // A subsequent load still recovers everything, now sourced from the per-project file.
    let reloaded = store.load();
    let sessions = reloaded
        .workspace
        .sessions
        .get(&PathBuf::from("/a"))
        .expect("sessions still present after migration");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        reloaded.workspace.worktree_name("feat-x"),
        Some("My Feature")
    );
}

/// Regression test found by code review: writing the catalog before attempting a migrating
/// project's own state-file write meant that if that write then failed, the project's only copy
/// of its session/worktree-name data (previously embedded in the catalog, just stripped by the
/// write that already completed) was gone from both files — permanent loss, not a fault isolated
/// to that project. `save()` now writes per-project state first and keeps the catalog's legacy
/// fields as a fallback for exactly the projects whose write failed.
#[test]
fn migrating_project_whose_state_write_fails_keeps_a_catalog_fallback() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    // Pre-split projects.json: "/a"'s only copy of its data is these embedded fields — no
    // per-project state file exists for it yet.
    let json = r#"{"schema_version":1,"last_active":"/a","projects":[
        {"path":"/a","display_name":"a","is_git_repo":true,"sessions":[
            {"id":"11111111-1111-1111-1111-111111111111","worktree_dir":null,"title":"Old session"}
        ],"worktree_display_names":{"feat-x":"My Feature"}}
    ]}"#;
    std::fs::write(&path, json).unwrap();
    let store = JsonFileStore::at(path.clone());

    let loaded = store.load();
    assert_eq!(
        loaded
            .workspace
            .sessions
            .get(&PathBuf::from("/a"))
            .unwrap()
            .len(),
        1
    );

    // Sabotage: a plain file where the per-project state directory needs to be, so writing any
    // project's own state file fails (simulates a disk/permission fault during migration).
    let state_dir = dir.path().join("projects");
    std::fs::write(&state_dir, b"not a directory").unwrap();

    store
        .save(&loaded.workspace)
        .expect_err("the sabotaged write must fail, not silently drop data");

    let raw_catalog = std::fs::read_to_string(&path).unwrap();
    assert!(
        raw_catalog.contains("Old session"),
        "the catalog must keep a fallback copy when the project's own state-file write failed: \
         {raw_catalog}"
    );
    assert!(raw_catalog.contains("My Feature"));

    // Clear the sabotage and confirm the fallback is dropped once the write can succeed again —
    // the catalog stays slim in the normal case.
    std::fs::remove_file(&state_dir).unwrap();
    store.save(&loaded.workspace).unwrap();
    let raw_catalog_after = std::fs::read_to_string(&path).unwrap();
    assert!(
        !raw_catalog_after.contains("Old session"),
        "the fallback must clear once the state-file write succeeds: {raw_catalog_after}"
    );
}

/// Feature 025: a project whose state cannot be read has no memory either.
///
/// The memory lives in the same file as that project's sessions, so a fault that loses the sessions
/// must lose the memory with them — restoring against sessions that failed to load would name an id
/// nothing can resolve. A launch must still start normally: FR-010 says an unreadable memory is
/// treated as no memory, never as an error.
#[test]
fn a_corrupt_project_state_file_leaves_that_project_with_no_memory() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("projects.json");
    let store = JsonFileStore::at(root.clone());

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a/one", "one", true));
    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let id = session.id;
    ws.sessions.insert(PathBuf::from("/a/one"), vec![session]);
    ws.foreground_by_project.insert(PathBuf::from("/a/one"), id);
    store.save(&ws).unwrap();

    // Corrupt that project's own state file, leaving the catalog itself intact.
    let state_dir = root.parent().unwrap().join("projects");
    let state_file = std::fs::read_dir(&state_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .expect("the project's state file");
    std::fs::write(&state_file, "{ not json").unwrap();

    let loaded = store.load();

    assert_eq!(
        loaded.status,
        LoadStatus::Loaded,
        "the catalog is fine, so the launch proceeds — a fault in one project's file never fails \
         the whole load (FR-012a), and never fails a launch (FR-010)"
    );
    assert!(
        loaded.workspace.foreground_by_project.is_empty(),
        "and the memory goes with the sessions it referred to, rather than surviving to name an id \
         that nothing loaded can resolve"
    );
    assert!(!loaded.workspace.sessions.contains_key(Path::new("/a/one")));
}

// ---------------------------------------------------------------------------------------
// Feature 029 (T005): a project whose state file cannot be read is *named* as such.
//
// Under 014 an unreadable file degrading to empty was safe — classification came from names, so
// losing the records lost only rename overrides. Under 029 an empty record set means "the app
// created none of these", i.e. hide everything. The load must therefore distinguish "no records"
// from "records unknown", which is what `unreadable_projects` is for (FR-011, data-model §4).
// ---------------------------------------------------------------------------------------

#[test]
fn a_corrupt_project_state_file_marks_the_project_unreadable() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a", "a", true));
    ws.projects.push(project("/b", "b", true));
    ws.record_user_created(Path::new("/a"), "feat-x");
    ws.record_user_created(Path::new("/b"), "feat-y");
    store.save(&ws).unwrap();

    std::fs::write(store.project_state_path(Path::new("/a")), "not json").unwrap();

    let out = store.load();
    assert!(
        out.workspace.unreadable_projects.contains(Path::new("/a")),
        "the project whose file failed to parse is named"
    );
    assert!(
        !out.workspace.unreadable_projects.contains(Path::new("/b")),
        "and no other project is — the fault stays isolated (FR-012a)"
    );
    assert!(
        !out.workspace
            .worktree_provenance
            .contains_key(Path::new("/a")),
        "its records are still dropped; it is the *not knowing* that is now recorded"
    );
    assert!(
        out.workspace.is_user_created(Path::new("/b"), "feat-y"),
        "/b's own records survive intact"
    );
}

#[test]
fn a_project_with_no_state_file_is_not_unreadable() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/fresh", "fresh", true));
    store.save(&ws).unwrap();
    std::fs::remove_file(store.project_state_path(Path::new("/fresh"))).unwrap();

    let out = store.load();
    assert!(
        out.workspace.unreadable_projects.is_empty(),
        "a project that has never been saved has nothing to fail reading — it is simply new, \
         and a new project genuinely has created nothing"
    );
}

#[test]
fn unreadable_is_not_persisted() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a", "a", true));
    ws.unreadable_projects.insert(PathBuf::from("/a"));
    store.save(&ws).unwrap();

    let out = store.load();
    assert!(
        out.workspace.unreadable_projects.is_empty(),
        "it describes this run's reading of the disk, not a fact about the project — a later \
         run that reads the file successfully must not inherit the failure"
    );
}

/// 029 FR-011, found walking quickstart Part 3 step 6: the corrupt file used to be renamed aside at
/// load, so the *second* reader of the same store in the same launch — the daemon the client just
/// spawned — saw a merely missing file, did not mark the project unreadable, and ran the one-time
/// backfill against evidence that had just been discarded. Every reader must reach the same verdict.
#[test]
fn a_corrupt_project_state_file_stays_unreadable_for_every_reader() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a", "a", true));
    ws.record_user_created(Path::new("/a"), "feat-x");
    store.save(&ws).unwrap();

    std::fs::write(store.project_state_path(Path::new("/a")), "not json").unwrap();

    for reader in 1..=2 {
        let out = store.load();
        assert!(
            out.workspace.unreadable_projects.contains(Path::new("/a")),
            "reader {reader} must see the same failure — a load that hides the damage from the \
             next reader is how the FR-006 migration gets consumed by a failed read"
        );
    }
}

/// 029 FR-011: "a transient failure cannot overwrite the true record set". A save during a run that
/// could not read a project's state would write that project's file from the empty state the
/// failure degraded to, destroying records the next run could otherwise have read back.
#[test]
fn saving_never_overwrites_an_unreadable_projects_state_file() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a", "a", true));
    ws.projects.push(project("/b", "b", true));
    ws.record_user_created(Path::new("/a"), "feat-x");
    ws.record_user_created(Path::new("/b"), "feat-y");
    store.save(&ws).unwrap();

    let a_state_path = store.project_state_path(Path::new("/a"));
    std::fs::write(&a_state_path, "not json").unwrap();
    let damaged = std::fs::read(&a_state_path).unwrap();

    let out = store.load();
    assert!(out.workspace.unreadable_projects.contains(Path::new("/a")));
    store.save(&out.workspace).unwrap();

    assert_eq!(
        std::fs::read(&a_state_path).unwrap(),
        damaged,
        "the unreadable project's file is left exactly as found"
    );
    let after = store.load();
    assert!(
        after
            .workspace
            .unreadable_projects
            .contains(Path::new("/a")),
        "so the next run fails visible too, rather than inheriting an empty record set"
    );
    assert!(
        after.workspace.is_user_created(Path::new("/b"), "feat-y"),
        "and the readable projects are still saved as usual"
    );
}
