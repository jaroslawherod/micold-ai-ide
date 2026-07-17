//! T011 (feature 008): cross-project session isolation + concurrency with no cap.
//!
//! This is the Constitution "Isolation & lifecycle gate" (Principle II) for feature 008 — an
//! integration test over the full multi-project switch flow. Runtime PTY-output isolation is
//! validated by quickstart.md (BS-4 at the process level); here we assert the isolation the
//! core guarantees: distinct identities, correct owner resolution, independent per-project
//! counts, and non-destructive switching across THREE concurrently-running projects (BS-1/4/5).

mod support;

use micold_ai_ide::app::State;
use micold_ai_ide::session::SessionLifecycle;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use support::{running_session, workspace_with};

fn three_projects() -> State {
    let mut st = State {
        workspace: workspace_with(vec![
            ("/p1", vec![running_session("wt1")]),
            ("/p2", vec![running_session("wt2")]),
            ("/p3", vec![running_session("wt3")]),
        ]),
        ..Default::default()
    };
    st.workspace.active = Some(PathBuf::from("/p1"));
    st.active_session = Some(st.workspace.sessions[Path::new("/p1")][0].id);
    st
}

#[test]
fn three_projects_run_concurrently_with_no_cap() {
    let st = three_projects();
    for p in ["/p1", "/p2", "/p3"] {
        assert_eq!(st.workspace.running_session_count(Path::new(p)), 1);
    }
}

#[test]
fn switching_among_projects_never_stops_a_session() {
    let mut st = three_projects();
    for p in ["/p2", "/p3", "/p1", "/p2"] {
        assert!(st.switch_active(Path::new(p)));
        for q in ["/p1", "/p2", "/p3"] {
            let list = &st.workspace.sessions[Path::new(q)];
            assert_eq!(list.len(), 1, "no session dropped from {q}");
            assert_eq!(
                list[0].lifecycle,
                SessionLifecycle::Running,
                "{q} still running"
            );
        }
    }
}

#[test]
fn sessions_are_isolated_by_owner_and_worktree() {
    let st = three_projects();
    let mut ids = BTreeSet::new();
    let mut dirs = BTreeSet::new();
    for p in ["/p1", "/p2", "/p3"] {
        let s = &st.workspace.sessions[Path::new(p)][0];
        assert!(ids.insert(s.id), "session ids unique across projects");
        assert!(
            dirs.insert(s.worktree_dir.clone()),
            "worktree dirs distinct"
        );
        // Each id resolves to its OWN project — never another's (identity isolation).
        let (owner, found) = st.workspace.find_session(s.id).expect("owner resolved");
        assert_eq!(owner, Path::new(p));
        assert_eq!(found.id, s.id);
    }
    assert_eq!(ids.len(), 3);
}

#[test]
fn displayed_session_always_belongs_to_the_active_project() {
    let mut st = three_projects();
    for p in ["/p2", "/p3", "/p1"] {
        assert!(st.switch_active(Path::new(p)));
        let expected = st.workspace.sessions[Path::new(p)][0].id;
        assert_eq!(
            st.active_session,
            Some(expected),
            "no cross-project leak of foreground"
        );
    }
}
