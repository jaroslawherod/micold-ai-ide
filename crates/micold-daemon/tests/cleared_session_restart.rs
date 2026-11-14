#![cfg(unix)] // the test writes a `#!/bin/sh` stub and chmods it: Unix-only

//! #1853 (root cause of #1795): a session that was `/clear`ed and restarted before anything was
//! typed in the new conversation must not become a dead row.
//!
//! `claude` writes a conversation's transcript on its first user message, so right after a
//! `/clear` the conversation the `SessionStart` hook names (#1794) has none, while the one from
//! before the clear still has. Each case ends with a reload of the catalog from disk (the restart).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

fn id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

fn boot(store: &Path) -> DaemonState {
    DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    ))
}

/// `claude`'s transcript of `conversation` for `project`, as `ClaudeProvider` lays it out.
fn transcript(config: &Path, project: &Path, conversation: SessionId) -> PathBuf {
    let encoded: String = project
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    config
        .join("projects")
        .join(encoded)
        .join(format!("{}.jsonl", conversation.0))
}

#[test]
fn a_cleared_session_restarted_before_a_new_message_resumes_its_earlier_conversation() {
    let config = tempfile::tempdir().unwrap();
    std::env::set_var("CLAUDE_CONFIG_DIR", config.path());
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let project = project.path().to_path_buf();
    let (session_id, cleared) = (id(1), id(2));

    // Before the clear the conversation was the session's own id, and it has a transcript.
    let before = transcript(config.path(), &project, session_id);
    std::fs::create_dir_all(before.parent().unwrap()).unwrap();
    std::fs::write(&before, "{\"type\":\"user\"}\n").unwrap();

    // After `/clear` the hook named a new conversation (#1794); nothing was typed, so no transcript.
    let mut session = Session::restored(
        session_id,
        SessionLocation::Default,
        SessionLabel::Named("cleared".into()),
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    );
    session.conversation = Some(cleared.0);
    JsonFileStore::at(store.path().join("projects.json"))
        .save(&Workspace {
            projects: vec![Project::new(project.clone(), true, Availability::Available)],
            active: Some(project.clone()),
            sessions: BTreeMap::from([(project.clone(), vec![session])]),
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();

    let state = boot(store.path());
    state.present_interrupted_resumable_at_startup();
    let listed = state.sessions_for(&project);
    let row = listed.iter().find(|s| s.id == session_id).expect("listed");
    assert_eq!(
        row.lifecycle,
        WireLifecycle::InterruptedResumable,
        "its earlier conversation is on disk, so the restart offers it for resume; got {:?}",
        row.lifecycle
    );
}
