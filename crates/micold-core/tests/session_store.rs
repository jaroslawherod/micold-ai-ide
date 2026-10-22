//! T040 — session persistence roundtrip (FR-020/021, SC-008, storage option A).
//! T004 (010-root-dir-session) — StoredSession.worktree_dir widened to Option<String>;
//! `None`/`null` round-trips to `SessionLocation::Default`.

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, PermissionMode, Session, SessionId, SessionKind, SessionLabel, SessionLifecycle,
    SessionLocation, TerminalMode,
};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::structured::model::{ModelChoice, ModelName};
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
                AiCli::ClaudeCode,
            ),
            Session::restored(
                SessionId::from_uuid(Uuid::from_u128(0x5678)),
                SessionLocation::Worktree("chore-cleanup".to_string()),
                SessionLabel::Pending,
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            ),
        ],
    );
    (
        Workspace {
            projects: vec![project],
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
            AiCli::ClaudeCode,
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
        ..Default::default()
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
            AiCli::ClaudeCode,
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
        ..Default::default()
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

/// #942 (FR-013, FR-015): a structured session of `mode`, saved and loaded back; also the project
/// state file as written.
fn round_trip_mode(mode: PermissionMode) -> (Session, String) {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let path = PathBuf::from("/home/dev/proj");
    let mut session = Session::restored_of_kind(
        SessionId::new(),
        SessionLocation::Worktree("feat-x".to_string()),
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
        SessionKind::Structured,
    );
    session.permission_mode = mode;
    let ws = Workspace {
        projects: vec![Project {
            path: path.clone(),
            display_name: "proj".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        }],
        active: None,
        sessions: BTreeMap::from([(path.clone(), vec![session])]),
        worktree_names: Default::default(),
        ..Default::default()
    };
    store.save(&ws).unwrap();
    let written = std::fs::read_to_string(store.project_state_path(&path)).unwrap();
    let loaded = store.load();
    let back = loaded.workspace.sessions.get(&path).unwrap()[0].clone();
    (back, written)
}

#[test]
fn each_permission_mode_round_trips_through_the_store() {
    for mode in PermissionMode::ALL {
        let (back, _) = round_trip_mode(mode);
        assert_eq!(
            back.permission_mode, mode,
            "{mode:?} reloads as itself (FR-015)"
        );
    }
}

#[test]
fn a_default_mode_record_is_written_with_no_permission_mode_key() {
    let (_, written) = round_trip_mode(PermissionMode::Default);
    assert!(
        !written.contains("permission_mode"),
        "a Default session is written exactly as before the feature: {written}"
    );
    let (_, written) = round_trip_mode(PermissionMode::Plan);
    assert!(
        written.contains("permission_mode"),
        "a chosen mode is what the record carries: {written}"
    );
}

#[test]
fn a_record_written_before_permission_modes_loads_as_default() {
    // A structured session recorded before #942: no "permission_mode" key.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("projects.json");
    std::fs::write(
        &file,
        r#"{"schema_version":1,"last_active":null,"projects":[{"path":"/p","display_name":"p","is_git_repo":true,"sessions":[{"id":"00000000-0000-0000-0000-000000000942","worktree_dir":"feat-x","title":null,"kind":"Structured"}]}]}"#,
    )
    .unwrap();
    let loaded = JsonFileStore::at(file).load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    let session = &loaded.workspace.sessions.get(&PathBuf::from("/p")).unwrap()[0];
    assert_eq!(session.kind, SessionKind::Structured);
    assert_eq!(session.permission_mode, PermissionMode::Default, "FR-013");
}

/// #943 T032 (FR-017): a session's model choice, saved and loaded back; also the state file.
fn round_trip_model(choice: ModelChoice) -> (Session, String) {
    round_trip_structured(|session| session.model = choice)
}

/// A structured session, changed by `edit`, saved and loaded back: the loaded session and the
/// project's state file as written.
fn round_trip_structured(edit: impl FnOnce(&mut Session)) -> (Session, String) {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let path = PathBuf::from("/home/dev/proj");
    let mut session = Session::restored_of_kind(
        SessionId::new(),
        SessionLocation::Worktree("feat-x".to_string()),
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
        SessionKind::Structured,
    );
    edit(&mut session);
    let ws = Workspace {
        projects: vec![Project {
            path: path.clone(),
            display_name: "proj".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        }],
        active: None,
        sessions: BTreeMap::from([(path.clone(), vec![session])]),
        worktree_names: Default::default(),
        ..Default::default()
    };
    store.save(&ws).unwrap();
    let written = std::fs::read_to_string(store.project_state_path(&path)).unwrap();
    let back = store.load().workspace.sessions.get(&path).unwrap()[0].clone();
    (back, written)
}

#[test]
fn a_named_model_round_trips_and_default_is_not_written() {
    let haiku = ModelChoice::Named(ModelName::parse("haiku").unwrap());
    let (back, written) = round_trip_model(haiku.clone());
    assert_eq!(back.model, haiku, "FR-017");
    assert!(written.contains("haiku"), "{written}");
    let (back, written) = round_trip_model(ModelChoice::Default);
    assert_eq!(back.model, ModelChoice::Default);
    assert!(
        !written.contains("\"model\""),
        "a Default session is written exactly as before the feature: {written}"
    );
}

#[test]
fn a_record_written_before_model_choices_loads_default() {
    // A structured session recorded before #943 M4: no "model" key.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("projects.json");
    std::fs::write(
        &file,
        r#"{"schema_version":1,"last_active":null,"projects":[{"path":"/p","display_name":"p","is_git_repo":true,"sessions":[{"id":"00000000-0000-0000-0000-000000000943","worktree_dir":"feat-x","title":null,"kind":"Structured"}]}]}"#,
    )
    .unwrap();
    let loaded = JsonFileStore::at(file).load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    let session = &loaded.workspace.sessions.get(&PathBuf::from("/p")).unwrap()[0];
    assert_eq!(session.model, ModelChoice::Default, "FR-017");
}

/// #943 T039 (FR-005a, provisional M5): the conversation a `/clear` started is saved with the
/// session, and a session that never cleared is written exactly as before.
#[test]
fn a_conversation_handle_round_trips_and_none_is_not_written() {
    let handle = Uuid::from_u128(0x943_0005);
    let (back, written) = round_trip_structured(|session| session.conversation = Some(handle));
    assert_eq!(back.conversation, Some(handle), "FR-005a");
    assert_eq!(back.resume_handle(), handle, "a restart resumes the cleared conversation");
    assert!(written.contains(&handle.to_string()), "{written}");
    let (back, written) = round_trip_structured(|_| {});
    assert_eq!(back.conversation, None);
    assert_eq!(back.resume_handle(), back.id.0);
    assert!(
        !written.contains("\"conversation\""),
        "a session that never cleared is written exactly as before: {written}"
    );
}

#[test]
fn a_record_written_before_conversation_handles_resumes_its_own_id() {
    // A structured session recorded before #943 M5: no "conversation" key.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("projects.json");
    std::fs::write(
        &file,
        r#"{"schema_version":1,"last_active":null,"projects":[{"path":"/p","display_name":"p","is_git_repo":true,"sessions":[{"id":"00000000-0000-0000-0000-000000000943","worktree_dir":"feat-x","title":null,"kind":"Structured"}]}]}"#,
    )
    .unwrap();
    let loaded = JsonFileStore::at(file).load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    let session = &loaded.workspace.sessions.get(&PathBuf::from("/p")).unwrap()[0];
    assert_eq!(session.conversation, None, "FR-005a");
    assert_eq!(session.resume_handle(), session.id.0);
}
