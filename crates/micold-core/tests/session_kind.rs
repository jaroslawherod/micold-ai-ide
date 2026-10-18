//! Feature 938 T004 — a session's kind (FR-001, FR-002, FR-003, FR-004; SC-004): terminal by
//! default and for every session recorded before the feature, chosen at construction and never
//! changed, structured only for Claude Code, persisted without a schema bump.

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, KindRefusal, Session, SessionId, SessionKind, SessionLabel, SessionLocation,
    TerminalMode,
};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const PROJECT: &str = "/home/dev/proj";

fn restored_of(kind: SessionKind) -> Session {
    Session::restored_of_kind(
        SessionId::new(),
        SessionLocation::Worktree("feat-x".to_string()),
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
        kind,
    )
}

fn workspace_with(session: Session) -> Workspace {
    let path = PathBuf::from(PROJECT);
    Workspace {
        projects: vec![Project {
            path: path.clone(),
            display_name: "proj".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        }],
        active: Some(path.clone()),
        sessions: BTreeMap::from([(path, vec![session])]),
        ..Default::default()
    }
}

/// Every file the store wrote under `dir`, concatenated: the catalog and the per-project state.
fn written_text(dir: &Path) -> String {
    let mut text = String::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).expect("the store directory reads") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                text.push_str(&std::fs::read_to_string(&path).expect("a written file reads"));
                text.push('\n');
            }
        }
    }
    text
}

fn round_trip(session: Session) -> (Session, String) {
    let dir = tempfile::tempdir().expect("a store directory");
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let id = session.id;
    store
        .save(&workspace_with(session))
        .expect("the catalog saves");
    let loaded = store.load();
    assert_eq!(loaded.status, LoadStatus::Loaded, "the saved catalog loads");
    let back = loaded
        .workspace
        .sessions
        .get(&PathBuf::from(PROJECT))
        .and_then(|list| list.iter().find(|s| s.id == id))
        .cloned()
        .expect("the saved session is listed again");
    (back, written_text(dir.path()))
}

#[test]
fn the_default_kind_is_terminal() {
    assert_eq!(
        SessionKind::default(),
        SessionKind::Terminal,
        "every new session is a terminal session unless asked otherwise (FR-001)"
    );
}

#[test]
fn the_kind_constructors_keep_the_kind_they_are_given() {
    for kind in [SessionKind::Terminal, SessionKind::Structured] {
        let fresh = Session::start_new_of_kind(SessionLocation::Default, AiCli::ClaudeCode, kind);
        assert_eq!(
            fresh.kind, kind,
            "a new session has the kind it was created with"
        );
        assert_eq!(
            restored_of(kind).kind,
            kind,
            "a restored session has the kind it was recorded with"
        );
    }
}

#[test]
fn the_constructors_without_a_kind_make_terminal_sessions() {
    let fresh = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let restored = Session::restored(
        SessionId::new(),
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    );
    assert_eq!(
        fresh.kind,
        SessionKind::Terminal,
        "a session created without a kind behaves as before (SC-004)"
    );
    assert_eq!(
        restored.kind,
        SessionKind::Terminal,
        "a session restored without a kind behaves as before (SC-004)"
    );
}

#[test]
fn a_session_has_no_kind_setter() {
    let source = include_str!("../src/session.rs");
    assert!(
        !source.contains("fn set_kind"),
        "the kind is chosen at creation and never changes for the session's life (FR-002)"
    );
}

#[test]
fn structured_is_offered_only_for_claude_code() {
    assert_eq!(
        SessionKind::Structured.check(AiCli::ClaudeCode),
        Ok(()),
        "Claude Code sessions can be structured (FR-003)"
    );
    for provider in [AiCli::Copilot, AiCli::Pi, AiCli::Codex, AiCli::OpenCode] {
        let refusal = SessionKind::Structured.check(provider);
        assert_eq!(
            refusal,
            Err(KindRefusal::NotSupported(provider)),
            "a structured session of another CLI is refused before anything is written (FR-003)"
        );
        let sentence = refusal.expect_err("refused").to_string();
        assert!(
            sentence.contains(provider.provider().display_name()),
            "the refusal names the CLI it refuses: {sentence}"
        );
        assert!(
            sentence.contains("only Claude Code sessions can be structured"),
            "the refusal says which CLI can be structured: {sentence}"
        );
    }
}

#[test]
fn every_cli_can_run_a_terminal_session() {
    for provider in AiCli::ALL {
        assert_eq!(
            SessionKind::Terminal.check(provider),
            Ok(()),
            "a terminal session is what every CLI already runs"
        );
    }
}

#[test]
fn a_record_without_a_kind_loads_as_terminal() {
    // A catalog written before feature 938: no "kind" key anywhere.
    let dir = tempfile::tempdir().expect("a store directory");
    let file = dir.path().join("projects.json");
    std::fs::write(
        &file,
        r#"{"schema_version":1,"last_active":null,"projects":[
            {"path":"/p","display_name":"p","is_git_repo":true,
             "sessions":[{"id":"00000000-0000-0000-0000-000000000938",
                          "worktree_dir":"feat-x","title":"Old session"}]}
        ]}"#,
    )
    .expect("the old catalog is written");
    let loaded = JsonFileStore::at(file).load();
    assert_eq!(loaded.status, LoadStatus::Loaded, "the old catalog loads");
    let sessions = loaded
        .workspace
        .sessions
        .get(&PathBuf::from("/p"))
        .expect("its session is listed");
    assert_eq!(
        sessions[0].kind,
        SessionKind::Terminal,
        "a session recorded before the feature is a terminal session (FR-001, FR-004)"
    );
}

#[test]
fn a_terminal_record_is_written_with_no_kind_key() {
    let (back, written) = round_trip(restored_of(SessionKind::Terminal));
    assert_eq!(
        back.kind,
        SessionKind::Terminal,
        "a terminal session reloads terminal"
    );
    assert!(
        !written.contains("\"kind\""),
        "a terminal session is written exactly as before the feature: {written}"
    );
}

#[test]
fn a_structured_record_round_trips() {
    let (back, written) = round_trip(restored_of(SessionKind::Structured));
    assert_eq!(
        back.kind,
        SessionKind::Structured,
        "a structured session reloads structured (FR-004)"
    );
    assert!(
        written.contains("\"kind\""),
        "the structured kind is what the record carries: {written}"
    );
}

#[test]
fn the_catalog_schema_version_is_unchanged() {
    let (_, written) = round_trip(restored_of(SessionKind::Structured));
    assert!(
        written.contains("\"schema_version\": 1") || written.contains("\"schema_version\":1"),
        "the kind is additive and defaulted, so the schema version stays 1: {written}"
    );
}
