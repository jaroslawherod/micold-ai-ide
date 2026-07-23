//! US2 tests: the JSON project store round-trips and degrades gracefully. Uses `tempfile`
//! — never the real user data directory (research R7/R8; storage-schema contract).

use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_ai_ide::workspace::Workspace;
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
fn save_then_load_roundtrips() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a/one", "one", true));
    ws.projects.push(project("/b/two", "Two", false));
    ws.active = Some(PathBuf::from("/b/two"));
    store.save(&ws).unwrap();

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);
    assert_eq!(out.workspace.projects.len(), 2);
    assert_eq!(out.workspace.projects[0].path, PathBuf::from("/a/one"));
    assert_eq!(out.workspace.projects[0].display_name, "one");
    assert!(out.workspace.projects[0].is_git_repo);
    assert!(!out.workspace.projects[1].is_git_repo);
    assert_eq!(out.workspace.active, Some(PathBuf::from("/b/two")));
}

#[test]
fn missing_file_loads_empty() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("does-not-exist.json"));

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Missing);
    assert!(out.workspace.projects.is_empty());
    assert_eq!(out.workspace.active, None);
}

#[test]
fn corrupt_file_recovers_to_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    std::fs::write(&path, "this is not json").unwrap();
    let store = JsonFileStore::at(path);

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Recovered);
    assert!(out.workspace.projects.is_empty());
    assert_eq!(out.workspace.active, None);
}

#[test]
fn dangling_last_active_loads_as_no_active() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    let json = r#"{"schema_version":1,"last_active":"/gone",
                   "projects":[{"path":"/a","display_name":"a","is_git_repo":false}]}"#;
    std::fs::write(&path, json).unwrap();
    let store = JsonFileStore::at(path);

    let out = store.load();
    assert_eq!(out.workspace.projects.len(), 1);
    assert_eq!(out.workspace.active, None);
}

#[test]
fn unknown_fields_are_ignored() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    let json = r#"{"schema_version":1,"extra":"ignored",
                   "projects":[{"path":"/a","display_name":"a","is_git_repo":true,"future":42}]}"#;
    std::fs::write(&path, json).unwrap();
    let store = JsonFileStore::at(path);

    let out = store.load();
    assert_eq!(out.workspace.projects.len(), 1);
    assert!(out.workspace.projects[0].is_git_repo);
}

#[test]
fn renamed_display_name_persists_across_save_and_load() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a", "old-name", false));
    ws.rename(Path::new("/a"), "new-name").unwrap();
    store.save(&ws).unwrap();

    let out = store.load();
    assert_eq!(out.workspace.projects[0].display_name, "new-name");
}

#[test]
fn corrupt_file_is_preserved_as_backup() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    std::fs::write(&path, "definitely not json").unwrap();
    let store = JsonFileStore::at(path.clone());

    let _ = store.load();

    // The corrupt file is moved aside (best-effort) before recovery (research R8).
    assert!(path.with_extension("json.bak").exists());
}

// --- Feature 008 US3: per-worktree display-name override persistence ---

#[test]
fn worktree_display_name_override_roundtrips() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/repo", "repo", true));
    ws.active = Some(PathBuf::from("/repo"));
    ws.set_worktree_name("feat-abc-123-login-page", "My Login")
        .unwrap();
    store.save(&ws).unwrap();

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);
    assert_eq!(out.workspace.active, Some(PathBuf::from("/repo")));
    assert_eq!(
        out.workspace.worktree_name("feat-abc-123-login-page"),
        Some("My Login")
    );
}

#[test]
fn missing_worktree_names_field_loads_without_override() {
    // An older file (no `worktree_display_names`) loads fine — no schema bump (FR-015).
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    let json = r#"{"schema_version":1,"last_active":"/repo",
                   "projects":[{"path":"/repo","display_name":"repo","is_git_repo":true}]}"#;
    std::fs::write(&path, json).unwrap();
    let store = JsonFileStore::at(path);

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);
    assert_eq!(out.workspace.active, Some(PathBuf::from("/repo")));
    assert_eq!(out.workspace.worktree_name("feat-x"), None);
}

// T026 (010-root-dir-session, contracts/storage-schema.md backward-compatibility guarantee):
// a `projects.json` shaped exactly as it was before this feature — `worktree_dir` always a
// plain JSON string, across multiple sessions and multiple projects — loads unchanged, with
// zero `SessionLocation::Default` sessions inferred anywhere.
#[test]
fn pre_feature_010_catalog_with_multiple_sessions_loads_as_all_worktree_located() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    let json = r#"{"schema_version":1,"last_active":"/repo-a","projects":[
        {"path":"/repo-a","display_name":"repo-a","is_git_repo":true,"sessions":[
            {"id":"11111111-1111-1111-1111-111111111111","worktree_dir":"feat-one","title":"One"},
            {"id":"22222222-2222-2222-2222-222222222222","worktree_dir":"feat-two","title":null}
        ]},
        {"path":"/repo-b","display_name":"repo-b","is_git_repo":true,"sessions":[
            {"id":"33333333-3333-3333-3333-333333333333","worktree_dir":"chore-cleanup","title":null}
        ]}
    ]}"#;
    std::fs::write(&path, json).unwrap();
    let store = JsonFileStore::at(path);

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);

    use micold_ai_ide::session::SessionLocation;
    let all_sessions = out.workspace.sessions.values().flat_map(|list| list.iter());
    let mut count = 0;
    for session in all_sessions {
        count += 1;
        assert!(
            matches!(session.location, SessionLocation::Worktree(_)),
            "a pre-feature-010 session must never be misread as Default: {:?}",
            session.location
        );
    }
    assert_eq!(count, 3, "all three legacy sessions loaded");
}

// --- Feature 014: forgetting removes the catalog entry AND its per-project state file ---

#[test]
fn forgotten_project_does_not_reappear_and_survivors_stay_intact() {
    use micold_ai_ide::session::{Session, SessionLocation};
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/keep", "keep", true));
    ws.projects.push(project("/drop", "drop", true));
    ws.active = Some(PathBuf::from("/keep"));
    ws.sessions.insert(
        PathBuf::from("/drop"),
        vec![Session::start_new(SessionLocation::Worktree(
            "feat-x".to_string(),
        ))],
    );
    store.save(&ws).unwrap();
    // The dropped project's per-project state file exists after the initial save.
    assert!(store.project_state_path(Path::new("/drop")).exists());

    // Forget /drop: prune the catalog entry + metadata, then delete its state file.
    ws.forget(Path::new("/drop"));
    store.remove_project_state(Path::new("/drop")).unwrap();
    store.save(&ws).unwrap();

    let out = store.load();
    assert_eq!(out.workspace.projects.len(), 1);
    assert_eq!(out.workspace.projects[0].path, PathBuf::from("/keep"));
    assert_eq!(out.workspace.active, Some(PathBuf::from("/keep")));
    // FR-005/FR-012: no lingering per-project state, so no old sessions can be resurrected.
    assert!(!store.project_state_path(Path::new("/drop")).exists());
    assert!(out.workspace.sessions.get(Path::new("/drop")).is_none());
}

#[test]
fn remove_project_state_is_idempotent_and_ok_when_absent() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    // Removing a never-written project's state file is success, not an error (idempotent).
    store
        .remove_project_state(Path::new("/never-existed"))
        .unwrap();
    store
        .remove_project_state(Path::new("/never-existed"))
        .unwrap();
}

#[test]
fn save_is_atomic_and_leaves_no_temp_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");
    let store = JsonFileStore::at(path.clone());

    store.save(&Workspace::empty()).unwrap();

    assert!(path.exists());
    // The temp file used for the atomic write must be renamed away, not left behind.
    assert!(!path.with_extension("json.tmp").exists());
}
