//! US1 (feature 008): background-restart marker + return notice. BS-7 / FR-011 / SC-007.

mod support;

use micold_ai_ide::app::State;
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

#[test]
fn returning_to_project_arms_notice_and_clears_markers() {
    let mut st = two_projects_active_b();
    let a_id = st.workspace.sessions[Path::new("/a")][0].id;
    st.note_background_restart(a_id);

    assert!(st.notice.is_none());
    assert!(st.switch_active(Path::new("/a")));

    // SC-007: the change is surfaced, never silent; the marker is consumed.
    assert!(st.notice.is_some());
    assert!(st.restarted_while_inactive.is_empty());
}
