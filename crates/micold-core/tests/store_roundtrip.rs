//! US2 tests: the JSON project store round-trips and degrades gracefully. Uses `tempfile`
//! — never the real user data directory (research R7/R8; storage-schema contract).

use micold_core::project::{Availability, Project};
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

    use micold_core::session::SessionLocation;
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
    use micold_core::session::{Session, SessionLocation};
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
    assert!(!out.workspace.sessions.contains_key(Path::new("/drop")));
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

// --- Feature 025: the last-session memory ------------------------------------------------------
//
// The memory is one id per project, stored in that project's own state file beside its sessions —
// the same file, the same writer, the same deletion when a project is forgotten.

#[test]
fn the_last_session_memory_survives_save_and_load() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a/one", "one", true));
    let session =
        micold_core::session::Session::start_new(micold_core::session::SessionLocation::Default);
    let id = session.id;
    ws.sessions.insert(PathBuf::from("/a/one"), vec![session]);
    ws.foreground_by_project.insert(PathBuf::from("/a/one"), id);

    store.save(&ws).unwrap();
    let loaded = store.load().workspace;

    assert_eq!(
        loaded.foreground_by_project.get(Path::new("/a/one")),
        Some(&id),
        "the whole feature: the session you were last on is still known after a restart"
    );
}

#[test]
fn a_project_with_no_memory_round_trips_as_none() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a/one", "one", true));

    store.save(&ws).unwrap();
    let loaded = store.load().workspace;

    assert!(
        loaded.foreground_by_project.is_empty(),
        "absence is a real answer, not an id that happens to match nothing — a project nobody has \
         used a session in must load as having no memory at all"
    );
}

#[test]
fn a_state_file_written_before_this_feature_still_loads() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("projects.json");
    let store = JsonFileStore::at(root.clone());

    // Save a project the ordinary way, then rewrite its state file without the new field —
    // exactly the shape every existing installation has on disk today.
    let mut ws = Workspace::empty();
    ws.projects.push(project("/a/one", "one", true));
    ws.sessions.insert(
        PathBuf::from("/a/one"),
        vec![micold_core::session::Session::start_new(
            micold_core::session::SessionLocation::Default,
        )],
    );
    store.save(&ws).unwrap();

    let state_dir = root.parent().unwrap().join("projects");
    let state_file = std::fs::read_dir(&state_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .expect("the project's state file");
    let text = std::fs::read_to_string(&state_file).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value.as_object_mut().unwrap().remove("last_session");
    std::fs::write(&state_file, serde_json::to_string(&value).unwrap()).unwrap();

    let loaded = store.load();

    assert_eq!(
        loaded.status,
        LoadStatus::Loaded,
        "a file written by a build that predates this feature must load normally, not as corrupt \
         — this is the claim that lets the field ship without a schema_version bump"
    );
    assert!(loaded.workspace.foreground_by_project.is_empty());
    assert_eq!(
        loaded
            .workspace
            .sessions
            .get(Path::new("/a/one"))
            .map(Vec::len),
        Some(1),
        "and the rest of the file is unaffected"
    );
}

#[test]
fn two_projects_remember_independently() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a/one", "one", true));
    ws.projects.push(project("/a/two", "two", true));
    let a =
        micold_core::session::Session::start_new(micold_core::session::SessionLocation::Default);
    let b =
        micold_core::session::Session::start_new(micold_core::session::SessionLocation::Default);
    let (a_id, b_id) = (a.id, b.id);
    ws.sessions.insert(PathBuf::from("/a/one"), vec![a]);
    ws.sessions.insert(PathBuf::from("/a/two"), vec![b]);
    ws.foreground_by_project
        .insert(PathBuf::from("/a/one"), a_id);
    ws.foreground_by_project
        .insert(PathBuf::from("/a/two"), b_id);

    store.save(&ws).unwrap();
    let loaded = store.load().workspace;

    assert_eq!(
        loaded.foreground_by_project.get(Path::new("/a/one")),
        Some(&a_id)
    );
    assert_eq!(
        loaded.foreground_by_project.get(Path::new("/a/two")),
        Some(&b_id),
        "each project's memory lives in its own file, so one cannot overwrite the other"
    );
}

#[test]
fn closing_the_remembered_session_does_not_erase_the_memory() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    let mut ws = Workspace::empty();
    ws.projects.push(project("/a/one", "one", true));
    let mut session =
        micold_core::session::Session::start_new(micold_core::session::SessionLocation::Default);
    let id = session.id;
    session.archive();
    ws.sessions.insert(PathBuf::from("/a/one"), vec![session]);
    ws.foreground_by_project.insert(PathBuf::from("/a/one"), id);

    store.save(&ws).unwrap();
    let loaded = store.load().workspace;

    assert_eq!(
        loaded.foreground_by_project.get(Path::new("/a/one")),
        Some(&id),
        "the memory still names it. Nothing is restored from it — a closed session cannot be — but \
         the memory is replaced only by another session becoming current, never erased by the \
         pointer going away (FR-005a, invariant I0)"
    );
}
