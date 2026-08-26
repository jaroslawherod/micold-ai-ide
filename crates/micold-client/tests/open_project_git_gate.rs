//! T015 — open-project git-only gate (FR-001a, SC-003a).

use micold_client::app::{Message, State};
use micold_client::features::notifications::Msg as NotificationsMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_core::git::{FakeGit, Git};
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

/// The refusal goes to the global notification surface, which renders unconditionally.
///
/// This assertion previously read `state.worktree_error == Some(..)` and passed green for the
/// entire time the refusal was invisible to users: `worktree_error`'s only render site is
/// inside the Add Worktree modal, which is never open when a folder is refused.
#[test]
fn refusal_message_is_surfaced_to_the_user() {
    let mut state = State::default();
    assert!(state.notifications.queue.visible().is_none());
    state.update(Message::Project(ProjectMsg::OpenRefused(
        "Only git repositories can be opened".to_string(),
    )));
    let visible = state
        .notifications
        .queue
        .visible()
        .expect("the refusal reached the queue");
    assert_eq!(visible.level, micold_core::notify::Level::Error);
    assert_eq!(visible.message, "Only git repositories can be opened");
    assert_eq!(state.notifications.queue.pending(), 0);
    // Not stashed in the modal-owned field that made it unreachable.
    assert!(state.worktree_error.is_none());
}

/// A refusal stays until the user dismisses it — it is not cleared by unrelated activity such
/// as a worktree re-scan, which can fire at any time.
#[test]
fn refusal_persists_until_dismissed() {
    let mut state = State::default();
    state.update(Message::Project(ProjectMsg::OpenRefused(
        "nope".to_string(),
    )));
    state.update(Message::Worktree(WorktreeMsg::Loaded(vec![])));
    assert!(state.notifications.queue.visible().is_some());

    state.update(Message::Notifications(NotificationsMsg::Dismissed));
    assert!(state.notifications.queue.visible().is_none());
}
