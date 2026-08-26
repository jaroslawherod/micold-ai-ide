//! Feature 014 (forget a project): reducer-flow integration tests.
//!
//! Drives the pure `State` reducer through the forget request → confirm/cancel flow. Process
//! termination and per-project state-file deletion are binary/IO glue validated by
//! `quickstart.md` (Principle I GUI-wiring exception); here we assert the pure state transitions.

use micold_client::app::{on_escape, Message, State};
use micold_client::features::project::Msg as ProjectMsg;
use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionLocation};
use std::path::{Path, PathBuf};

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

/// A `State` with the given project paths, all `Available`, the last one active.
fn state_with_projects(paths: &[&str]) -> State {
    let mut state = State::default();
    for p in paths {
        state.workspace.projects.push(Project {
            path: PathBuf::from(p),
            display_name: p.trim_start_matches('/').to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
    }
    state.workspace.active = paths.last().map(PathBuf::from);
    state
}

// --- US1: request / cancel / confirm on a non-active project ---

#[test]
fn forget_requested_opens_confirmation_and_sets_target() {
    let mut state = state_with_projects(&["/a", "/b"]);

    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/a"),
    )));

    assert_eq!(open_dialog(&state), Some("confirm_forget_project"));
    assert_eq!(
        state.project.forget_target.as_deref(),
        Some(std::path::Path::new("/a"))
    );
    // Requesting does not yet remove anything.
    assert_eq!(state.workspace.projects.len(), 2);
}

#[test]
fn forget_cancelled_closes_and_changes_nothing() {
    let mut state = state_with_projects(&["/a", "/b"]);
    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/a"),
    )));

    state.update(Message::Project(ProjectMsg::ForgetCancelled));

    assert_eq!(open_dialog(&state), None);
    assert!(state.project.forget_target.is_none());
    assert_eq!(
        state.workspace.projects.len(),
        2,
        "nothing removed on cancel"
    );
    assert_eq!(state.workspace.active, Some(PathBuf::from("/b")));
}

#[test]
fn forget_confirmed_removes_the_nonactive_target_others_remain() {
    let mut state = state_with_projects(&["/a", "/b"]); // active = /b
    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/a"),
    )));

    state.update(Message::Project(ProjectMsg::ForgetConfirmed));

    assert_eq!(open_dialog(&state), None);
    assert!(state.project.forget_target.is_none());
    assert_eq!(state.workspace.projects.len(), 1);
    assert_eq!(state.workspace.projects[0].path, PathBuf::from("/b"));
    assert_eq!(
        state.workspace.active,
        Some(PathBuf::from("/b")),
        "active untouched"
    );
}

#[test]
fn escape_cancels_the_forget_confirmation() {
    let mut state = state_with_projects(&["/a", "/b"]);
    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/a"),
    )));
    assert_eq!(
        on_escape(&state),
        Some(Message::Project(ProjectMsg::ForgetCancelled))
    );
}

// --- US2: forgetting the active project clears the active working space + active session ---

#[test]
fn forget_confirmed_on_active_project_clears_active_and_active_session() {
    let mut state = state_with_projects(&["/a", "/b"]); // active = /b
                                                        // Give the active project a running foreground session.
    let session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    let id = session.id;
    state
        .workspace
        .sessions
        .insert(PathBuf::from("/b"), vec![session]);
    state.active_session = Some(id);

    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/b"),
    )));
    state.update(Message::Project(ProjectMsg::ForgetConfirmed));

    assert!(!state
        .workspace
        .projects
        .iter()
        .any(|p| p.path == *Path::new("/b")));
    assert_eq!(
        state.workspace.active, None,
        "active working space cleared (FR-008)"
    );
    assert!(state.active_session.is_none(), "active session cleared");
}

#[test]
fn forgetting_the_last_project_leaves_an_empty_list() {
    let mut state = state_with_projects(&["/only"]); // active = /only
    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/only"),
    )));
    state.update(Message::Project(ProjectMsg::ForgetConfirmed));

    assert!(
        state.workspace.projects.is_empty(),
        "empty-state precondition (FR-009)"
    );
    assert_eq!(state.workspace.active, None);
}

#[test]
fn forgetting_a_background_project_leaves_active_untouched() {
    let mut state = state_with_projects(&["/bg", "/fg"]); // active = /fg
    let session = Session::start_new(SessionLocation::Default);
    let id = session.id;
    state
        .workspace
        .sessions
        .insert(PathBuf::from("/fg"), vec![session]);
    state.active_session = Some(id);

    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/bg"),
    )));
    state.update(Message::Project(ProjectMsg::ForgetConfirmed));

    assert_eq!(state.workspace.active, Some(PathBuf::from("/fg")));
    assert_eq!(
        state.active_session,
        Some(id),
        "foreground session untouched"
    );
}

// --- US3: forgetting an unavailable project ---

#[test]
fn forget_confirmed_removes_an_unavailable_project() {
    let mut state = state_with_projects(&["/gone", "/here"]);
    // Mark /gone unavailable (folder deleted on disk).
    if let Some(p) = state
        .workspace
        .projects
        .iter_mut()
        .find(|p| p.path == *Path::new("/gone"))
    {
        p.availability = Availability::Unavailable;
    }

    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/gone"),
    )));
    assert_eq!(open_dialog(&state), Some("confirm_forget_project"));
    state.update(Message::Project(ProjectMsg::ForgetConfirmed));

    assert!(!state
        .workspace
        .projects
        .iter()
        .any(|p| p.path == *Path::new("/gone")));
    assert_eq!(state.workspace.projects.len(), 1);
}

// --- Feature 024: forgetting the active project clears the pointer like anything else ---------

#[test]
fn forgetting_the_active_project_clears_the_current_session_through_the_one_path() {
    let mut state = state_with_projects(&["/a", "/b"]);
    let session = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let id = session.id;
    state
        .workspace
        .sessions
        .insert(PathBuf::from("/b"), vec![session]);
    state.active_session = Some(id);
    state.sidebar.pending_reveal_scroll = false;

    state.update(Message::Project(ProjectMsg::ForgetRequested(
        PathBuf::from("/b"),
    )));
    state.update(Message::Project(ProjectMsg::ForgetConfirmed));

    assert!(
        state.active_session.is_none(),
        "the project holding it is gone, so no session is current"
    );
    assert!(
        !state.sidebar.pending_reveal_scroll,
        "and nothing is armed to scroll to. This is an app-initiated clear like the close and \
         remove arms, and it goes through the same function for the same reason — a scroll armed \
         with no target stays armed, then fires against whatever row appears next (invariant I5)"
    );
}
