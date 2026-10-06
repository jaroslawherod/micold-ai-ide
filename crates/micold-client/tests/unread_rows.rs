//! Whether a session's sidebar row carries the unread mark (feature 039, US2; contract
//! `unread-mark.md` U5; FR-016, FR-019, FR-024).
//!
//! Unread state is the service's: it reaches a window in every catalog snapshot. What the window
//! adds is that the session it has in view is read at once, before the service's answer to its
//! view report arrives.

use micold_client::app::{Message, State};
use micold_client::catalog_sync::reconcile_catalog;
use micold_client::features::attention::{in_view, row_unread};
use micold_client::features::session::Msg as SessionMsg;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ProjectSnapshot, SessionSummary, WireLifecycle,
};
use micold_core::session::{AiCli, Session, SessionId, SessionLabel, SessionLocation};
use std::path::PathBuf;

const REPO: &str = "/repo";

fn unread_session() -> Session {
    let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    session.unread = true;
    session
}

/// `REPO` active, with `session` started in it and selected.
fn state_with_selected(session: Session) -> State {
    let mut state = State::default();
    state.workspace.projects.push(Project {
        path: PathBuf::from(REPO),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(PathBuf::from(REPO));
    state.update(Message::Session(SessionMsg::Started(session)));
    state
}

fn catalog(id: SessionId, unread: bool) -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: Some(PathBuf::from(REPO)),
        projects: vec![ProjectSnapshot {
            path: PathBuf::from(REPO),
            display_name: "repo".to_string(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions: vec![SessionSummary {
                id,
                worktree_dir: None,
                title: SessionLabel::Pending,
                lifecycle: WireLifecycle::Running,
                activity: ActivitySignal::AwaitingInput,
                provider: AiCli::ClaudeCode,
                input_serial: 0,
                live_shells: Vec::new(),
                attention_seq: 1,
                unread,
            }],
        }],
        env_include_failures: Vec::new(),
    }
}

/// U125 (U5, FR-018, US2 scenario 1).
#[test]
fn an_unread_session_that_is_not_in_view_has_the_mark() {
    let session = unread_session();
    let other = SessionId::new();

    assert!(
        row_unread(&session, None),
        "no session is in view, so the unread session's row is marked"
    );
    assert!(
        row_unread(&session, Some(other)),
        "another session is in view, so the unread session's row is marked"
    );
}

/// U126 (U5, FR-019, SC-006): the window does not wait for the service to clear the mark.
#[test]
fn the_session_in_view_has_no_mark_before_the_service_answers() {
    let session = unread_session();

    assert!(
        !row_unread(&session, Some(session.id)),
        "the session is in view in this window, so its row is not marked although the catalog \
         still says unread"
    );
}

#[test]
fn a_read_session_has_no_mark() {
    let read = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);

    assert!(
        !row_unread(&read, None),
        "a session that is not unread is not marked"
    );
}

/// U126, A36 (FR-019, US2 scenario 23): the session a row is compared with is the one this window
/// last reported in view, so the mark goes with the report and comes back when the window loses
/// focus.
#[test]
fn the_session_in_view_is_the_one_the_window_last_reported() {
    let session = unread_session();
    let id = session.id;
    let mut state = state_with_selected(session);
    assert_eq!(
        in_view(&state.attention),
        None,
        "before any report this window has nothing in view"
    );

    let _ = state.view_report(true);
    assert_eq!(
        in_view(&state.attention),
        Some(id),
        "the focused window reported its selected session"
    );

    let _ = state.view_report(false);
    assert_eq!(
        in_view(&state.attention),
        None,
        "a window without keyboard focus has no session in view"
    );
}

/// U127 (FR-024): unread state is the service's, and a window learns it from the catalog: for a
/// session it sees for the first time, and for one it already holds.
#[test]
fn reconciling_a_catalog_copies_unread() {
    let project = PathBuf::from(REPO);
    let id = SessionId::new();
    let mut state = State::default();

    reconcile_catalog(&mut state, &catalog(id, true), false);
    assert!(
        state.workspace.sessions[&project][0].unread,
        "a session first seen in a snapshot arrives unread when the snapshot says so"
    );

    reconcile_catalog(&mut state, &catalog(id, false), false);
    assert!(
        !state.workspace.sessions[&project][0].unread,
        "a later snapshot in which the session is read replaces the state held"
    );

    reconcile_catalog(&mut state, &catalog(id, true), false);
    assert!(
        state.workspace.sessions[&project][0].unread,
        "a later snapshot in which the session is unread again replaces the state held"
    );
}
