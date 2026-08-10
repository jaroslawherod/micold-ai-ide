//! Session foreground and project switching, in isolation (feature 021, SC-004).
//!
//! Same caveat as `features_worktree.rs`: these are `impl State` methods in Tier 1, so the file
//! builds a `State`. What it holds to is the other half of SC-004 — it names no other feature's
//! types. No sidebar rows, no overlays, no drafts.
//!
//! The switch sequence is what is worth pinning here. Its step order is load-bearing
//! (data-model.md I1) and every step is individually plausible in the wrong place, which is exactly
//! the kind of thing a refactor breaks quietly.

use micold_client::app::State;
use micold_client::features::session::SelectKind;
use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionId, SessionLocation};
use std::path::{Path, PathBuf};

fn project(path: &str) -> Project {
    Project {
        path: PathBuf::from(path),
        display_name: path.trim_start_matches('/').to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    }
}

/// Two open projects, `/a` active, each holding the sessions given. Returns the state plus the
/// session ids, since `Session::start_new` allocates them.
fn two_projects(a: usize, b: usize) -> (State, Vec<SessionId>, Vec<SessionId>) {
    let mut st = State::default();
    st.workspace.projects.push(project("/a"));
    st.workspace.projects.push(project("/b"));

    let sessions_a: Vec<Session> = (0..a)
        .map(|_| Session::start_new(SessionLocation::Default))
        .collect();
    let sessions_b: Vec<Session> = (0..b)
        .map(|_| Session::start_new(SessionLocation::Default))
        .collect();
    let ids_a = sessions_a.iter().map(|s| s.id).collect();
    let ids_b = sessions_b.iter().map(|s| s.id).collect();

    st.workspace
        .sessions
        .insert(PathBuf::from("/a"), sessions_a);
    st.workspace
        .sessions
        .insert(PathBuf::from("/b"), sessions_b);
    st.workspace.active = Some(PathBuf::from("/a"));
    (st, ids_a, ids_b)
}

#[test]
fn switching_to_an_unknown_project_changes_nothing_and_says_so() {
    let (mut st, a, _) = two_projects(1, 1);
    st.active_session = Some(a[0]);

    let switched = st.switch_active(Path::new("/nowhere"));

    assert!(
        !switched,
        "a project that is not open cannot be switched to"
    );
    assert_eq!(
        st.workspace.active.as_deref(),
        Some(Path::new("/a")),
        "a rejected switch must leave the active project alone — half-switching is worse than \
         not switching"
    );
    assert_eq!(
        st.active_session,
        Some(a[0]),
        "and it must leave the foreground alone too"
    );
}

#[test]
fn switching_away_and_back_returns_to_the_session_that_was_in_front() {
    let (mut st, a, _) = two_projects(2, 1);
    st.active_session = Some(a[1]);

    assert!(st.switch_active(Path::new("/b")));
    assert!(st.switch_active(Path::new("/a")));

    assert_eq!(
        st.active_session,
        Some(a[1]),
        "the outgoing foreground is recorded BEFORE activation (data-model.md I1); record it \
         after and you store the incoming project's session under the outgoing project's key, \
         which looks right until you switch back"
    );
}

#[test]
fn entering_a_project_with_no_recorded_foreground_falls_back_to_a_running_session() {
    let (mut st, _, b) = two_projects(1, 1);

    assert!(st.switch_active(Path::new("/b")));

    assert_eq!(
        st.active_session,
        Some(b[0]),
        "a first visit has nothing stored, so the project shows its first running session \
         rather than an empty shell"
    );
}

#[test]
fn a_switch_lands_on_a_terminal_ready_to_type() {
    // Reversed by feature 023 (FR-011). This asserted the opposite until then, on the reasoning
    // that arriving in a project is not the same as asking to type in it — true of arriving
    // somewhere by accident, but a project switch is deliberate, and what fills the pane afterwards
    // is a restored session's terminal. An explicit release made before the switch goes with it
    // (FR-021a): it was about the moment, not about the session.
    let (mut st, _, _) = two_projects(1, 1);
    st.update(micold_client::app::Message::TerminalFocusReleased);

    assert!(st.switch_active(Path::new("/b")));

    assert!(
        st.terminal_focused(),
        "switching to a project with a restored session must leave its terminal holding the \
         keyboard, with no press (FR-011)"
    );
}

#[test]
fn a_restart_in_the_active_project_raises_no_return_notice() {
    let (mut st, a, _) = two_projects(1, 1);

    st.note_background_restart(a[0]);

    assert!(
        !st.restarted_while_inactive.contains(&a[0]),
        "the user watched it happen — telling them about it on return would be noise"
    );
}

#[test]
fn a_restart_in_an_inactive_project_is_remembered_until_the_user_returns() {
    let (mut st, _, b) = two_projects(1, 1);

    st.note_background_restart(b[0]);
    assert!(
        st.restarted_while_inactive.contains(&b[0]),
        "it happened out of sight, so it is owed a notice"
    );

    assert!(st.switch_active(Path::new("/b")));

    assert!(
        !st.restarted_while_inactive.contains(&b[0]),
        "the marker is consumed on arrival, or the same notice reappears on every later visit"
    );
}

#[test]
fn sessions_are_located_by_worktree_regardless_of_whether_they_are_visible() {
    let mut st = State::default();
    st.workspace.projects.push(project("/a"));
    st.workspace.active = Some(PathBuf::from("/a"));

    let mut archived = Session::start_new(SessionLocation::Worktree("feat-a".into()));
    archived.archived = true;
    let id = archived.id;
    st.workspace
        .sessions
        .insert(PathBuf::from("/a"), vec![archived]);

    assert_eq!(
        st.sessions_in_worktree("feat-a"),
        vec![id],
        "deleting a worktree must terminate every session it hosts, and an archived session is \
         still a process — filtering by visibility here would leak one"
    );
}

#[test]
fn the_three_selection_kinds_stay_distinct() {
    assert_ne!(SelectKind::Simple, SelectKind::Semantic);
    assert_ne!(SelectKind::Semantic, SelectKind::Lines);
}
