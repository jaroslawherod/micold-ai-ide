//! T040 — session persistence roundtrip (FR-020/021, SC-008, storage option A).

use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::session::{Session, SessionId, SessionLabel, SessionLifecycle};
use micold_ai_ide::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_ai_ide::workspace::Workspace;
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
            Session::restored(id, "feat-x", SessionLabel::Named("Add login".to_string())),
            Session::restored(
                SessionId::from_uuid(Uuid::from_u128(0x5678)),
                "chore-cleanup",
                SessionLabel::Pending,
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
    assert_eq!(named.worktree_dir, "feat-x");
    assert_eq!(named.label, SessionLabel::Named("Add login".to_string()));
    // Lifecycle is not persisted — restored sessions are Idle (FR-021).
    assert_eq!(named.lifecycle, SessionLifecycle::Idle);

    let pending = sessions
        .iter()
        .find(|s| s.worktree_dir == "chore-cleanup")
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
            "feat-x",
            SessionLabel::Pending,
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
