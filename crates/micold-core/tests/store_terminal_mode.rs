//! T005 — `StoredSession.mode` serde default/roundtrip (feature 010, FR-011,
//! contracts/persistence-schema.md).

use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn workspace_with(mode: TerminalMode) -> (Workspace, PathBuf, SessionId) {
    let path = PathBuf::from("/home/dev/proj");
    let id = SessionId::new();
    let mut sessions = BTreeMap::new();
    sessions.insert(
        path.clone(),
        vec![Session::restored(
            id,
            SessionLocation::Worktree("feat-x".to_string()),
            SessionLabel::Pending,
            mode,
        )],
    );
    (
        Workspace {
            projects: vec![Project {
                path: path.clone(),
                display_name: "proj".to_string(),
                is_git_repo: true,
                availability: Availability::Available,
            }],
            active: Some(path.clone()),
            sessions,
            worktree_names: Default::default(),
            ..Default::default()
        },
        path,
        id,
    )
}

#[test]
fn mode_round_trips_ai_cli() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let (ws, path, id) = workspace_with(TerminalMode::AiCli);

    store.save(&ws).unwrap();
    let loaded = store.load();

    let session = loaded
        .workspace
        .sessions
        .get(&path)
        .and_then(|s| s.iter().find(|s| s.id == id))
        .expect("session present");
    assert_eq!(session.mode, TerminalMode::AiCli);
}

#[test]
fn mode_round_trips_regular() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let (ws, path, id) = workspace_with(TerminalMode::Regular);

    store.save(&ws).unwrap();
    let loaded = store.load();

    let session = loaded
        .workspace
        .sessions
        .get(&path)
        .and_then(|s| s.iter().find(|s| s.id == id))
        .expect("session present");
    assert_eq!(session.mode, TerminalMode::Regular);
}

#[test]
fn a_stored_session_with_no_mode_key_deserializes_as_ai_cli() {
    // Back-compat: a catalog written before feature 010 has no "mode" key at all.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("projects.json");
    std::fs::write(
        &file,
        r#"{"schema_version":1,"last_active":null,"projects":[
            {"path":"/p","display_name":"p","is_git_repo":true,
             "sessions":[{"id":"00000000-0000-0000-0000-000000000001",
                          "worktree_dir":"feat-x","title":"Add login"}]}
        ]}"#,
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
    assert_eq!(sessions[0].mode, TerminalMode::AiCli, "back-compat default");
}
