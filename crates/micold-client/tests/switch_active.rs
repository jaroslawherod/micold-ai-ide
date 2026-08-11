//! US1 (feature 008): `State::switch_active` is non-destructive, restores the prior
//! foreground (recording the outgoing one BEFORE activation — I1), and rejects unavailable
//! projects. BS-1, BS-3, BS-10.

mod support;

use micold_client::app::State;
use micold_core::project::Availability;
use micold_core::session::{SessionLifecycle, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
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
    assert_eq!(
        st.workspace.foreground_by_project.get(Path::new("/a")),
        Some(&fg_a)
    );
    assert!(st.workspace.foreground_by_project.get(Path::new("/b")) != Some(&fg_a));
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

// --- Feature 024: the switch reveals where you landed -----------------------------------------
//
// US1, the reported bug. `switch_active` is where a project switch decides which session is in
// front of you; this is that decision reaching the panel.

/// The two-project state above, with each project's worktrees discovered — the panel can only
/// open a location it knows about (contract §1.2).
fn two_projects_with_worktrees() -> State {
    let mut st = two_projects_active_a();
    st.worktrees = vec![Worktree {
        dir_name: "wa2".to_string(),
        path: PathBuf::from("/a/.claude/worktrees/wa2"),
        branch: Some("feat/wa2".to_string()),
        status: WorktreeStatus::Valid,
    }];
    st
}

/// The worktree list the app would discover on arriving at `/b`.
fn b_worktrees() -> Vec<Worktree> {
    vec![Worktree {
        dir_name: "wb".to_string(),
        path: PathBuf::from("/b/.claude/worktrees/wb"),
        branch: Some("feat/wb".to_string()),
        status: WorktreeStatus::Valid,
    }]
}

#[test]
fn switching_opens_the_row_holding_the_session_you_land_on() {
    let mut st = two_projects_with_worktrees();

    assert!(st.switch_active(Path::new("/b")));
    st.set_worktrees(b_worktrees());

    assert!(
        st.location_open(&SessionLocation::Worktree("wb".to_string())),
        "the whole feature: after a switch the panel already lists the session the main area is \
         showing, with no clicks and no guessing which row holds it (FR-001, SC-001)"
    );
}

#[test]
fn the_reveal_survives_a_worktree_list_that_arrives_after_the_switch() {
    let mut st = two_projects_with_worktrees();

    // The switch happens first and the incoming project's worktrees arrive afterwards — the
    // ordinary case, since discovery is asynchronous (FR-001b).
    assert!(st.switch_active(Path::new("/b")));
    assert!(
        !st.location_open(&SessionLocation::Worktree("wb".to_string())),
        "before the list arrives the location is not yet known, and nothing is opened on a guess"
    );

    st.set_worktrees(b_worktrees());

    assert!(
        st.location_open(&SessionLocation::Worktree("wb".to_string())),
        "and the row opens the moment its location becomes known, however late — there is no \
         one-shot reveal to have missed (FR-001b)"
    );
}

#[test]
fn view_state_does_not_carry_from_the_project_you_left() {
    let mut st = two_projects_with_worktrees();
    // Open a row by hand in /a, on top of the one its current session reveals.
    st.expanded.insert("wa1".to_string());

    assert!(st.switch_active(Path::new("/b")));
    st.set_worktrees(b_worktrees());

    assert!(
        !st.expanded.contains("wa1"),
        "/a's expansion is pruned by /b's worktree names, so a row opened in one project cannot \
         render in another (FR-007)"
    );
    assert!(
        !st.default_expanded,
        "and the Default row, which has no name to prune by, is reset outright"
    );
}

#[test]
fn switching_arms_a_scroll_and_clears_a_stale_suppression() {
    let mut st = two_projects_with_worktrees();
    st.reveal_suppressed_for = st.active_session;
    st.pending_reveal_scroll = false;

    assert!(st.switch_active(Path::new("/b")));

    assert!(
        st.pending_reveal_scroll,
        "the revealed row is no use below the fold, so a switch arms the scroll that brings it \
         into view (FR-008)"
    );
    assert!(
        st.reveal_suppressed_for.is_none(),
        "and a row closed against the session you were on does not keep the next project's \
         reveal closed (invariant I2)"
    );
}

#[test]
fn switching_to_a_project_with_no_session_reveals_nothing() {
    let mut st = State {
        workspace: workspace_with(vec![("/a", vec![running_session("wa1")]), ("/b", vec![])]),
        ..Default::default()
    };
    st.workspace.active = Some(PathBuf::from("/a"));
    st.active_session = Some(st.workspace.sessions[Path::new("/a")][0].id);

    assert!(st.switch_active(Path::new("/b")));
    st.set_worktrees(b_worktrees());

    assert!(st.active_session.is_none());
    assert!(
        !st.location_open(&SessionLocation::Worktree("wb".to_string())),
        "no session is current, so no row is opened and the panel does not claim otherwise \
         (US1 scenario 4, FR-013)"
    );
    assert!(
        !st.pending_reveal_scroll,
        "and nothing is armed to scroll to, which is what stops a scroll firing later against \
         an unrelated row (invariant I5)"
    );
}

// --- Feature 025: switching uses the memory that came from disk -------------------------------

#[test]
fn switching_to_a_project_not_yet_visited_uses_its_stored_memory() {
    let mut st = two_projects_active_a();
    // /b's memory as it would arrive from the store at boot: a session nothing in this run has
    // selected, so only the persisted value can account for it.
    let b_second = st.workspace.sessions[Path::new("/b")]
        .last()
        .map(|s| s.id)
        .unwrap();
    st.workspace
        .foreground_by_project
        .insert(PathBuf::from("/b"), b_second);

    assert!(st.switch_active(Path::new("/b")));

    assert_eq!(
        st.active_session,
        Some(b_second),
        "the memory loaded from disk is the same map `record_foreground` writes, so a project you \
         have not visited this run behaves exactly like one you have. Falling back to the first \
         running session here would mean the persisted memory only worked at launch"
    );
}

#[test]
fn each_project_keeps_the_session_it_was_last_on_not_the_last_one_overall() {
    let mut st = two_projects_active_a();
    let fg_a = st.active_session.unwrap();

    assert!(st.switch_active(Path::new("/b")));
    let fg_b = st.active_session.unwrap();
    assert!(st.switch_active(Path::new("/a")));

    assert_eq!(
        st.workspace.foreground_by_project.get(Path::new("/a")),
        Some(&fg_a)
    );
    assert_eq!(
        st.workspace.foreground_by_project.get(Path::new("/b")),
        Some(&fg_b),
        "switching several times leaves each project remembering its own session, not whichever \
         one happened to be current when the user stopped (US2 scenario 2)"
    );
}
