//! US1 (feature 008): background-restart marker + return notice. BS-7 / FR-011 / SC-007.

mod support;

use micold_client::app::State;
use std::path::{Path, PathBuf};
use support::{running_session, workspace_with};

fn two_projects_active_b() -> State {
    let mut st = State {
        workspace: workspace_with(vec![
            ("/a", vec![running_session("wa")]),
            ("/b", vec![running_session("wb")]),
        ]),
        ..Default::default()
    };
    st.workspace.active = Some(PathBuf::from("/b")); // /a is inactive
    st.active_session = Some(st.workspace.sessions[Path::new("/b")][0].id);
    st
}

#[test]
fn marks_restart_only_when_owning_project_is_inactive() {
    let mut st = two_projects_active_b();
    let a_id = st.workspace.sessions[Path::new("/a")][0].id;
    let b_id = st.workspace.sessions[Path::new("/b")][0].id;

    st.note_background_restart(a_id); // /a inactive → marked
    st.note_background_restart(b_id); // /b active → NOT marked

    assert!(st.restarted_while_inactive.contains(&a_id));
    assert!(!st.restarted_while_inactive.contains(&b_id));
}

/// FR-011 / SC-007: returning to a project whose session was restarted in the background tells
/// the user, and consumes the marker.
///
/// The notice goes to the global notification surface. It previously had a dedicated `notice`
/// field drawn only by `shell::view` — the *else* branch of `if active_session.is_some()` —
/// while `switch_active` restores the foreground session and therefore sets `active_session`.
/// The banner was unreachable in precisely the situation it existed for, and this test passed
/// green throughout because it asserted on the field rather than on what the user sees.
#[test]
fn returning_to_project_notifies_the_user_and_clears_markers() {
    let mut st = two_projects_active_b();
    let a_id = st.workspace.sessions[Path::new("/a")][0].id;
    st.note_background_restart(a_id);

    assert!(st.notify.visible().is_none());
    assert!(st.switch_active(Path::new("/a")).is_some());

    let visible = st
        .notify
        .visible()
        .expect("the return notice reached the queue");
    assert_eq!(visible.level, micold_core::notify::Level::Info);
    assert_eq!(
        visible.message,
        "A background session was restarted while you were away."
    );
    assert!(st.restarted_while_inactive.is_empty());

    // The restored foreground session is active — the condition that hid the old banner.
    assert!(st.active_session.is_some());
}
