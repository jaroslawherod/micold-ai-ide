//! T011 — extended app base state: defaults + new message wiring (feature 005).

use micold_ai_ide::app::{Message, Overlay, State};
use micold_ai_ide::naming::ConventionalType;
use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::session::Session;
use micold_ai_ide::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

#[test]
fn defaults_are_empty() {
    let state = State::default();
    assert!(state.worktrees.is_empty());
    assert!(state.expanded.is_empty());
    assert!(state.active_session.is_none());
    assert!(state.worktree_form.is_none());
    assert!(state.worktree_error.is_none());
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.active_sessions().is_empty());
}

#[test]
fn opening_the_form_sets_overlay_and_draft() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    assert_eq!(state.overlay, Overlay::AddWorktree);
    assert!(state.worktree_form.is_some());
}

#[test]
fn form_edits_build_a_derived_preview() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    state.update(Message::AddWorktreeTicketChanged("ABC-1".to_string()));
    state.update(Message::AddWorktreeNameChanged("Login".to_string()));

    let form = state.worktree_form.as_ref().unwrap();
    let derived = form.preview().unwrap();
    assert_eq!(derived.dir_name, "feat-abc-1-login");
    assert_eq!(derived.branch, "feat/abc-1-login");
}

#[test]
fn submitting_an_invalid_form_records_the_error() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    // No type selected, no name.
    state.update(Message::AddWorktreeSubmitted);
    assert!(state.worktree_form.as_ref().unwrap().error.is_some());
}

#[test]
fn cancelling_the_form_clears_it() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeCancelled);
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_form.is_none());
}

#[test]
fn created_worktree_is_added_and_form_closed() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    let wt = Worktree {
        dir_name: "feat-x".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        branch: Some("feat/x".to_string()),
        status: WorktreeStatus::Valid,
    };
    state.update(Message::WorktreeCreated(wt));
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_form.is_none());
    assert_eq!(state.worktrees.len(), 1);
}

#[test]
fn session_started_selected_and_closed() {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);

    let session = Session::start_new("feat-x");
    let id = session.id;
    state.update(Message::SessionStarted(session));
    assert_eq!(state.active_session, Some(id));
    assert_eq!(state.active_sessions().len(), 1);
    assert!(state.expanded.contains("feat-x"));

    state.update(Message::SessionRunning(id));
    assert!(state.active_sessions()[0].is_active());

    state.update(Message::SessionTitleUpdated {
        id,
        title: "Titled".to_string(),
    });
    assert_eq!(state.active_sessions()[0].label.display(), "Titled");

    state.update(Message::SessionCloseRequested(id));
    assert!(state.active_sessions().is_empty());
    assert!(state.active_session.is_none());
}
