//! T040 — session persistence roundtrip (FR-020/021, SC-008, storage option A).
//! T004 (010-root-dir-session) — StoredSession.worktree_dir widened to Option<String>;
//! `None`/`null` round-trips to `SessionLocation::Default`.

use micold_core::project::{Availability, Project};
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, TerminalMode,
};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

fn workspace_with_sessions() -> (Workspace, PathBuf, SessionId) {
    let path = PathBuf::from("/home/dev/proj");
    let project = Project {
        path: path.clone(),
        display_name: "proj".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    };
    let id = SessionId::from_uuid(Uuid::from_u128(0x1234));
    let mut sessions = BTreeMap::new();
    sessions.insert(
        path.clone(),
        vec![
            Session::restored(
                id,
                SessionLocation::Worktree("feat-x".to_string()),
                SessionLabel::Named("Add login".to_string()),
                TerminalMode::AiCli,
            ),
            Session::restored(
                SessionId::from_uuid(Uuid::from_u128(0x5678)),
                SessionLocation::Worktree("chore-cleanup".to_string()),
                SessionLabel::Pending,
                TerminalMode::AiCli,
            ),
        ],
    );
    (
        Workspace {
            projects: vec![project],
            active: Some(path.clone()),
            sessions,
            worktree_names: Default::default(),
        },
        path,
        id,
    )
}

#[test]
fn sessions_roundtrip_through_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let (ws, path, id) = workspace_with_sessions();

    store.save(&ws).unwrap();
    let loaded = store.load();

    assert_eq!(loaded.status, LoadStatus::Loaded);
    let sessions = loaded
        .workspace
        .sessions
        .get(&path)
        .expect("sessions present");
    assert_eq!(sessions.len(), 2);

    let named = sessions.iter().find(|s| s.id == id).unwrap();
    assert_eq!(
        named.location,
        SessionLocation::Worktree("feat-x".to_string())
    );
    assert_eq!(named.label, SessionLabel::Named("Add login".to_string()));
    // Lifecycle is not persisted — restored sessions are Idle (FR-021).
    assert_eq!(named.lifecycle, SessionLifecycle::Idle);

    let pending = sessions
        .iter()
        .find(|s| s.location == SessionLocation::Worktree("chore-cleanup".to_string()))
        .unwrap();
    assert_eq!(pending.label, SessionLabel::Pending);
}

#[test]
fn null_title_restores_as_pending() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let path = PathBuf::from("/home/dev/proj");
    let mut sessions = BTreeMap::new();
    sessions.insert(
        path.clone(),
        vec![Session::restored(
            SessionId::new(),
            SessionLocation::Worktree("feat-x".to_string()),
            SessionLabel::Pending,
            TerminalMode::AiCli,
        )],
    );
    let ws = Workspace {
        projects: vec![Project {
            path: path.clone(),
            display_name: "proj".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        }],
        active: None,
        sessions,
        worktree_names: Default::default(),
    };

    store.save(&ws).unwrap();
    let loaded = store.load();
    let restored = &loaded.workspace.sessions.get(&path).unwrap()[0];
    assert_eq!(restored.label, SessionLabel::Pending);
}

#[test]
fn default_session_persists_as_null_worktree_dir_and_roundtrips() {
    // contracts/storage-schema.md: SessionLocation::Default -> StoredSession { worktree_dir: None }.
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let path = PathBuf::from("/home/dev/proj");
    let mut sessions = BTreeMap::new();
    sessions.insert(
        path.clone(),
        vec![Session::restored(
            SessionId::new(),
            SessionLocation::Default,
            SessionLabel::Pending,
            TerminalMode::AiCli,
        )],
    );
    let ws = Workspace {
        projects: vec![Project {
            path: path.clone(),
            display_name: "proj".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        }],
        active: None,
        sessions,
        worktree_names: Default::default(),
    };

    store.save(&ws).unwrap();
    // Bugfix 002/BUG-001: session data lives in the project's own state file, not the catalog.
    let raw = std::fs::read_to_string(store.project_state_path(&path)).unwrap();
    assert!(
        raw.contains("\"worktree_dir\": null") || raw.contains("\"worktree_dir\":null"),
        "expected a null worktree_dir for a Default session, got: {raw}"
    );

    let loaded = store.load();
    let restored = &loaded.workspace.sessions.get(&path).unwrap()[0];
    assert_eq!(restored.location, SessionLocation::Default);
}

#[test]
fn legacy_string_worktree_dir_loads_as_worktree_location() {
    // Backward compatibility (contracts/storage-schema.md): a file written before feature
    // 010 has worktree_dir as a plain JSON string, never null. It must still load correctly
    // as SessionLocation::Worktree, not be misread as Default.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("projects.json");
    std::fs::write(
        &file,
        r#"{"schema_version":1,"last_active":null,"projects":[{"path":"/p","display_name":"p","is_git_repo":true,"sessions":[{"id":"11111111-1111-1111-1111-111111111111","worktree_dir":"feat-legacy","title":null}]}]}"#,
    )
    .unwrap();
    let store = JsonFileStore::at(file);
    let loaded = store.load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    let sessions = loaded
        .workspace
        .sessions
        .get(&PathBuf::from("/p"))
        .expect("sessions present");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].location,
        SessionLocation::Worktree("feat-legacy".to_string())
    );
}

#[test]
fn projects_without_sessions_load_unchanged() {
    // Forward-compat: a legacy catalog with no `sessions` field still loads.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("projects.json");
    std::fs::write(
        &file,
        r#"{"schema_version":1,"last_active":null,"projects":[{"path":"/p","display_name":"p","is_git_repo":true}]}"#,
    )
    .unwrap();
    let store = JsonFileStore::at(file);
    let loaded = store.load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    assert_eq!(loaded.workspace.projects.len(), 1);
    assert!(loaded.workspace.sessions.is_empty());
}
