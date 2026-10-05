//! The unread counts the project switcher shows (feature 039, US2; contract `unread-mark.md` U6,
//! U7; FR-019, FR-021, FR-022, FR-023).
//!
//! A project's row carries the number of its unread sessions, the active project's as the others'.
//! The switcher's button carries the total of the *other* projects. Both leave out the session this
//! window has in view before the service's answer to its view report arrives.

use micold_client::app::{Message, State};
use micold_client::features::session::Msg as SessionMsg;
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionLocation};
use std::path::{Path, PathBuf};

const P: &str = "/p";
const Q: &str = "/q";
const R: &str = "/r";

fn unread_session() -> Session {
    let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    session.unread = true;
    session
}

fn unread_sessions(count: usize) -> Vec<Session> {
    (0..count).map(|_| unread_session()).collect()
}

/// The known projects P, Q and R with `unread` unread sessions each, in that order, and P active.
fn state_with_unread(unread: [usize; 3]) -> State {
    let mut state = State::default();
    for (path, count) in [P, Q, R].into_iter().zip(unread) {
        state.workspace.projects.push(Project {
            path: PathBuf::from(path),
            display_name: path.trim_start_matches('/').to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        state
            .workspace
            .sessions
            .insert(PathBuf::from(path), unread_sessions(count));
    }
    state.workspace.active = Some(PathBuf::from(P));
    state
}

fn unread_counts(state: &State) -> Vec<usize> {
    state
        .switcher_entries()
        .iter()
        .map(|entry| entry.unread_count)
        .collect()
}

/// U128, A17, A18 (FR-021, US2 scenarios 4 and 5).
#[test]
fn every_switcher_entry_carries_its_projects_unread_count() {
    let state = state_with_unread([1, 2, 0]);

    assert_eq!(
        unread_counts(&state),
        [1, 2, 0],
        "P is active and holds one unread session, Q holds two, R none"
    );
}

/// U129, A16 (FR-019, US2 scenario 3): the count does not wait for the service.
#[test]
fn a_projects_count_falls_at_once_for_the_session_in_view() {
    let mut state = state_with_unread([1, 0, 0]);
    let selected = unread_session();
    let id = selected.id;
    state.update(Message::Session(SessionMsg::Started(selected)));
    assert_eq!(
        unread_counts(&state),
        [2, 0, 0],
        "before this window reports a session in view, both sessions count"
    );

    let _ = state.view_report(true);

    assert!(
        state
            .workspace
            .find_session(id)
            .is_some_and(|(_, s)| s.unread),
        "the catalog still says unread: the service has not answered"
    );
    assert_eq!(
        unread_counts(&state),
        [1, 0, 0],
        "the session this window has in view is no longer counted"
    );
}

/// U130, A28 (FR-023, US2 scenario 15, SC-010).
#[test]
fn the_buttons_total_is_the_unread_sessions_of_the_other_projects() {
    let state = state_with_unread([1, 2, 1]);

    assert_eq!(
        state.other_projects_unread(),
        3,
        "two in Q and one in R; P's own unread session is not in the button's total"
    );
    assert_eq!(
        state.other_projects_unread(),
        state
            .workspace
            .other_projects_unread(state.workspace.active.as_deref()),
        "the total is the workspace's, asked for the active project"
    );
}

/// A29 (FR-023, US2 scenario 16).
#[test]
fn the_buttons_total_is_zero_when_only_the_active_project_has_unread_sessions() {
    let state = state_with_unread([1, 0, 0]);

    assert_eq!(state.other_projects_unread(), 0);
}

/// A30 (FR-023, US2 scenario 17): whether or not Q's sessions were read.
#[test]
fn after_a_switch_the_buttons_total_counts_the_remaining_projects() {
    let mut state = state_with_unread([0, 2, 1]);
    assert_eq!(state.other_projects_unread(), 3);

    assert!(state.workspace.activate(Path::new(Q)), "Q is switched to");

    assert_eq!(
        state.other_projects_unread(),
        1,
        "Q's two unread sessions leave the total and R's one stays"
    );
}

/// FR-010, US1 scenario 5 (switcher half): a closed unread session leaves the project's count and
/// the button's total; the other projects' counts stay.
#[test]
fn a_closed_unread_session_leaves_the_switcher_counts() {
    let mut state = state_with_unread([1, 2, 0]);
    let mut closed = unread_session();
    closed.archived = true;
    for path in [P, Q] {
        state
            .workspace
            .sessions
            .get_mut(Path::new(path))
            .unwrap()
            .push(closed.clone());
    }

    assert_eq!(unread_counts(&state), [1, 2, 0]);
    assert_eq!(
        state.other_projects_unread(),
        2,
        "the button totals Q and R without Q's closed session"
    );
}
