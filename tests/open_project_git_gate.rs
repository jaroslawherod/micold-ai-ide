//! T015 — open-project git-only gate (FR-001a, SC-003a).

use micold_ai_ide::app::{Message, State};
use micold_ai_ide::git::{FakeGit, Git};
use std::path::Path;

#[test]
fn git_repo_passes_the_gate() {
    let git = FakeGit::new().with_repo("/repo");
    assert!(git.is_repo_root(Path::new("/repo")));
}

#[test]
fn non_git_directory_fails_the_gate() {
    let git = FakeGit::new().with_repo("/repo");
    assert!(!git.is_repo_root(Path::new("/plain/dir")));
}

#[test]
fn refusal_message_is_surfaced_in_state() {
    let mut state = State::default();
    assert!(state.worktree_error.is_none());
    state.update(Message::ProjectOpenRefused(
        "Only git repositories can be opened".to_string(),
    ));
    assert_eq!(
        state.worktree_error.as_deref(),
        Some("Only git repositories can be opened")
    );
}

#[test]
fn loading_worktrees_clears_the_error() {
    let mut state = State::default();
    state.update(Message::ProjectOpenRefused("nope".to_string()));
    state.update(Message::WorktreesLoaded(vec![]));
    assert!(state.worktree_error.is_none());
}
