//! T064 (bugfix BUG-002, completes T054) — session label stays in sync with the AI CLI
//! provider's session name (FR-011a, SC-009).
//!
//! Exercises the real read path (`ClaudeProvider::read_title` over a temp transcript file)
//! wired to the pure reducer (`Message::SessionTitleUpdated` → `set_title`), headlessly — no
//! `claude`, no GUI. This is the behaviour the main-loop terminal poll drives at runtime.

use micold_ai_ide::app::{Message, State};
use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::provider::{AiCliProvider, ClaudeProvider};
use micold_ai_ide::session::Session;
use std::path::{Path, PathBuf};

/// The runtime sync step, mirrored from the main loop: read the provider's current title for a
/// session and, when it differs from the current label, emit `SessionTitleUpdated`.
fn sync_once(state: &mut State, config_dir: &Path, project: &Path) {
    let provider = ClaudeProvider;
    let mut updates = Vec::new();
    for session in state.active_sessions() {
        let cwd = project
            .join(".claude/worktrees")
            .join(&session.worktree_dir);
        if let Some(title) = provider.read_title(config_dir, &cwd, session.id.0) {
            if session.label.display() != title {
                updates.push((session.id, title));
            }
        }
    }
    for (id, title) in updates {
        state.update(Message::SessionTitleUpdated { id, title });
    }
}

fn state_with_active_session(project: &Path, worktree_dir: &str) -> (State, Session) {
    let mut state = State::default();
    state.workspace.projects.push(Project {
        path: project.to_path_buf(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(project.to_path_buf());
    let session = Session::start_new(worktree_dir);
    state.update(Message::SessionStarted(session.clone()));
    (state, session)
}

fn write_title(config_dir: &Path, project: &Path, worktree_dir: &str, id: uuid::Uuid, title: &str) {
    let cwd = project.join(".claude/worktrees").join(worktree_dir);
    let path = ClaudeProvider.transcript_path(config_dir, &cwd, id);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(r#"{{"type":"ai-title","aiTitle":"{title}"}}"#),
    )
    .unwrap();
}

#[test]
fn label_goes_pending_then_named_then_resyncs_on_change() {
    let config = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo");
    let worktree = "feat-x";
    let (mut state, session) = state_with_active_session(&project, worktree);

    // Before the provider records a title, the label is the neutral placeholder.
    assert_eq!(state.active_sessions()[0].label.display(), "New session");

    // Provider assigns a title → a sync reconciles the label from Pending to Named.
    write_title(
        config.path(),
        &project,
        worktree,
        session.id.0,
        "Investigate flaky test",
    );
    sync_once(&mut state, config.path(), &project);
    assert_eq!(
        state.active_sessions()[0].label.display(),
        "Investigate flaky test"
    );

    // Provider later changes the title → the label re-syncs (never stays diverged, SC-009).
    write_title(
        config.path(),
        &project,
        worktree,
        session.id.0,
        "Fix flaky test race",
    );
    sync_once(&mut state, config.path(), &project);
    assert_eq!(
        state.active_sessions()[0].label.display(),
        "Fix flaky test race"
    );
}

#[test]
fn sync_is_noop_when_provider_has_no_title() {
    let config = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo");
    let (mut state, _session) = state_with_active_session(&project, "feat-x");

    // No transcript on disk → read fails silently → label untouched (must not fail the session).
    sync_once(&mut state, config.path(), &project);
    assert_eq!(state.active_sessions()[0].label.display(), "New session");
}
