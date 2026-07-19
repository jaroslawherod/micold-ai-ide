//! T011 — extended app base state: defaults + new message wiring (feature 005).

use micold_ai_ide::app::{on_escape, Message, Overlay, State};
use micold_ai_ide::naming::ConventionalType;
use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::session::{Session, SessionLocation};
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

    let session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
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

// T015 (010-root-dir-session): a Default-located session enters `Workspace.sessions` exactly
// like a worktree session. Note: `Message::SessionStartRequested` itself has no pure-reducer
// effect for ANY location (it's an I/O trigger the binary consumes to spawn a PTY before
// dispatching `SessionStarted` — see `src/app.rs`'s `on_escape`-adjacent no-op arm list); the
// pure-core assertion point is `SessionStarted`, exercised identically to the existing
// `session_started_selected_and_closed` test above.
#[test]
fn default_session_started_enters_workspace_sessions() {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);

    let session = Session::start_new(SessionLocation::Default);
    let id = session.id;
    state.update(Message::SessionStarted(session));

    assert_eq!(state.active_session, Some(id));
    assert_eq!(state.active_sessions().len(), 1);
    assert_eq!(
        state.active_sessions()[0].location,
        SessionLocation::Default
    );
    // The Default row's own expansion flag opens, not the worktree `expanded` set.
    assert!(state.default_expanded);
}

// --- Feature 008 US2: worktree delete reducer ---

fn state_with_worktree_and_session(dir: &str) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state.worktrees.push(Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("feat/{dir}")),
        status: WorktreeStatus::Valid,
    });
    let session = Session::start_new(SessionLocation::Worktree(dir.to_string()));
    state.update(Message::SessionStarted(session));
    state
}

#[test]
fn delete_requested_opens_confirm_then_confirmed_drops_records() {
    let mut state = state_with_worktree_and_session("feat-x");
    assert_eq!(state.active_sessions().len(), 1);

    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    assert_eq!(state.overlay, Overlay::ConfirmWorktreeDelete);
    assert_eq!(state.worktree_delete_target.as_deref(), Some("feat-x"));
    assert!(state.worktree_menu_open.is_none());

    state.update(Message::WorktreeDeleteConfirmed);
    assert!(state.active_sessions().is_empty(), "sessions dropped");
    assert!(state.active_session.is_none(), "active cleared");
    assert!(!state.worktrees.iter().any(|w| w.dir_name == "feat-x"));
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_delete_target.is_none());
}

#[test]
fn delete_cancelled_changes_nothing() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    state.update(Message::WorktreeDeleteCancelled);
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_delete_target.is_none());
    assert_eq!(state.active_sessions().len(), 1, "session untouched");
    assert!(state.worktrees.iter().any(|w| w.dir_name == "feat-x"));
}

#[test]
fn escape_cancels_confirm_delete() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    assert_eq!(on_escape(&state), Some(Message::WorktreeDeleteCancelled));
}

// --- Feature 008 US3: worktree rename changes display only ---

#[test]
fn worktree_rename_changes_display_only_not_branch_or_path() {
    let mut state = state_with_worktree_and_session("feat-x");
    let before = state
        .worktrees
        .iter()
        .find(|w| w.dir_name == "feat-x")
        .unwrap()
        .clone();
    let tags_before = state.worktree_tree()[0].tags.clone();

    state.update(Message::WorktreeRenameStarted("feat-x".to_string()));
    state.update(Message::WorktreeRenameTextChanged("Renamed".to_string()));
    state.update(Message::WorktreeRenameConfirmed);

    assert_eq!(state.worktree_display_name("feat-x"), "Renamed");
    let after = state
        .worktrees
        .iter()
        .find(|w| w.dir_name == "feat-x")
        .unwrap();
    // FR-007/FR-014: the on-disk identity is untouched.
    assert_eq!(after.dir_name, "feat-x");
    assert_eq!(after.path, before.path);
    assert_eq!(after.branch, before.branch);
    // FR-016: tags still derive from the branch/dir, unaffected by the rename.
    assert_eq!(state.worktree_tree()[0].tags, tags_before);
}

#[test]
fn escape_cancels_worktree_rename() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeRenameStarted("feat-x".to_string()));
    assert_eq!(on_escape(&state), Some(Message::WorktreeRenameCancelled));
}
