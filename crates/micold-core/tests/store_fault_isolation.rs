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

    // The corrupt file is preserved as a backup (mirrors the catalog's own corrupt-file handling).
    assert!(a_state_path.with_extension("json.bak").exists());
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
