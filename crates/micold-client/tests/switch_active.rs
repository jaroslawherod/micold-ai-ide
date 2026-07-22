//! US1 (feature 008): `State::switch_active` is non-destructive, restores the prior
//! foreground (recording the outgoing one BEFORE activation — I1), and rejects unavailable
//! projects. BS-1, BS-3, BS-10.

mod support;

use micold_client::app::State;
use micold_core::project::Availability;
use micold_core::session::SessionLifecycle;
use std::path::{Path, PathBuf};
use support::{running_session, workspace_with};

fn two_projects_active_a() -> State {
    let mut st = State {
        workspace: workspace_with(vec![
            ("/a", vec![running_session("wa1"), running_session("wa2")]),
            ("/b", vec![running_session("wb")]),
        ]),
        ..Default::default()
    };
    st.workspace.active = Some(PathBuf::from("/a"));
    // Foreground on /a = the SECOND session, to prove exact restore.
    st.active_session = Some(st.workspace.sessions[Path::new("/a")][1].id);
    st
}

#[test]
fn switch_keeps_outgoing_sessions_running() {
    let mut st = two_projects_active_a();

    assert!(st.switch_active(Path::new("/b")));

    // BS-1: /a's sessions are untouched (still Running, none dropped).
    let a = &st.workspace.sessions[Path::new("/a")];
    assert_eq!(a.len(), 2);
    assert!(a.iter().all(|s| s.lifecycle == SessionLifecycle::Running));
    assert_eq!(st.workspace.active, Some(PathBuf::from("/b")));
}

#[test]
fn records_outgoing_foreground_before_activating() {
    let mut st = two_projects_active_a();
    let fg_a = st.active_session.unwrap();

    assert!(st.switch_active(Path::new("/b")));

    // I1: the OUTGOING project (/a) is recorded, not the incoming (/b).
    assert_eq!(st.foreground_by_project.get(Path::new("/a")), Some(&fg_a));
    assert!(st.foreground_by_project.get(Path::new("/b")) != Some(&fg_a));
}

#[test]
fn foreground_restored_on_return() {
    let mut st = two_projects_active_a();
    let fg_a = st.active_session.unwrap();

    assert!(st.switch_active(Path::new("/b")));
    // On /b, foreground falls to its first running session.
    assert_eq!(
        st.active_session,
        Some(st.workspace.sessions[Path::new("/b")][0].id)
    );

    assert!(st.switch_active(Path::new("/a")));
    // BS-3: the exact prior foreground of /a is restored.
    assert_eq!(st.active_session, Some(fg_a));
}

#[test]
fn switch_to_unavailable_is_rejected_and_leaves_state_unchanged() {
    let mut st = two_projects_active_a();
    let fg_a = st.active_session.unwrap();
    for p in &mut st.workspace.projects {
        if p.path.as_path() == Path::new("/b") {
            p.availability = Availability::Unavailable;
        }
    }

    assert!(!st.switch_active(Path::new("/b")));

    // BS-10: nothing changed.
    assert_eq!(st.workspace.active, Some(PathBuf::from("/a")));
    assert_eq!(st.active_session, Some(fg_a));
    assert!(st.workspace.sessions[Path::new("/b")]
        .iter()
        .all(|s| s.lifecycle == SessionLifecycle::Running));
}
