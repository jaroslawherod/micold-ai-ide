//! T011 (feature 008): cross-project session isolation + concurrency with no cap.
//!
//! This is the Constitution "Isolation & lifecycle gate" (Principle II) for feature 008 — an
//! integration test over the full multi-project switch flow. Runtime PTY-output isolation is
//! validated by quickstart.md (BS-4 at the process level); here we assert the isolation the
//! core guarantees: distinct identities, correct owner resolution, independent per-project
//! counts, and non-destructive switching across THREE concurrently-running projects (BS-1/4/5).

mod support;

use micold_client::app::State;
use micold_core::session::{SessionLifecycle, SessionLocation};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use support::{running_default_session, running_session, workspace_with};

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
        assert!(st.switch_active(Path::new(p)).is_some());
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
        let micold_core::session::SessionLocation::Worktree(dir) = &s.location else {
            panic!("expected a worktree-located session in this fixture");
        };
        assert!(dirs.insert(dir.clone()), "worktree dirs distinct");
        // Each id resolves to its OWN project — never another's (identity isolation).
        let (owner, found) = st.workspace.find_session(s.id).expect("owner resolved");
        assert_eq!(owner, Path::new(p));
        assert_eq!(found.id, s.id);
    }
    assert_eq!(ids.len(), 3);
}

// T023 (010-root-dir-session, US3, SC-005): two concurrent Default sessions for the SAME
// project are independently listed, and stopping/closing one leaves the other's lifecycle
// untouched. This already passes once the Foundational `SessionLocation` model lands (no new
// coupling was added for it) — it's a regression lock, not new implementation.
#[test]
fn two_concurrent_default_sessions_are_independent() {
    let mut st = State {
        workspace: workspace_with(vec![(
            "/p1",
            vec![running_default_session(), running_default_session()],
        )]),
        ..Default::default()
    };
    st.workspace.active = Some(PathBuf::from("/p1"));

    let sessions = &st.workspace.sessions[Path::new("/p1")];
    assert_eq!(sessions.len(), 2, "both Default sessions coexist");
    assert!(
        sessions
            .iter()
            .all(|s| s.location == SessionLocation::Default),
        "both are genuinely Default-located, not accidentally sharing a worktree identity"
    );
    let (first, second) = (sessions[0].id, sessions[1].id);
    assert_ne!(first, second, "distinct identities");

    // Stop the first (e.g. project close/session-close path) and confirm the second is
    // untouched.
    st.workspace
        .find_session_mut(first)
        .unwrap()
        .1
        .stop_for_project_change();
    let sessions = &st.workspace.sessions[Path::new("/p1")];
    assert_eq!(
        sessions.iter().find(|s| s.id == first).unwrap().lifecycle,
        SessionLifecycle::Idle
    );
    assert_eq!(
        sessions.iter().find(|s| s.id == second).unwrap().lifecycle,
        SessionLifecycle::Running,
        "closing one Default session must not affect the other"
    );
}

#[test]
fn displayed_session_always_belongs_to_the_active_project() {
    let mut st = three_projects();
    for p in ["/p2", "/p3", "/p1"] {
        assert!(st.switch_active(Path::new(p)).is_some());
        let expected = st.workspace.sessions[Path::new(p)][0].id;
        assert_eq!(
            st.active_session,
            Some(expected),
            "no cross-project leak of foreground"
        );
    }
}

// ---------------------------------------------------------------------------------------
// Feature 026 (T025) — the provider is per session, not per project
// ---------------------------------------------------------------------------------------

/// Two sessions in one project, on different CLIs, both running (FR-009, US1 scenario 4).
///
/// `provider` is independent of `location`: nothing groups or constrains sessions by it
/// (data-model invariant 2). The isolation this file is about — distinct identities, correct owner
/// resolution, independent counts — is unaffected by the two rows being backed by different CLIs,
/// and that is what makes a mixed project ordinary rather than a special case.
#[test]
fn two_sessions_in_one_project_can_run_different_clis_at_once() {
    use micold_core::session::{AiCli, Session};

    let claude = Session::start_new(
        SessionLocation::Worktree("wt1".to_string()),
        AiCli::ClaudeCode,
    );
    let mut copilot =
        Session::start_new(SessionLocation::Worktree("wt1".to_string()), AiCli::Copilot);
    copilot.mark_running();
    let mut claude = claude;
    claude.mark_running();

    let mut st = State {
        workspace: workspace_with(vec![("/p1", vec![claude.clone(), copilot.clone()])]),
        ..State::default()
    };
    st.workspace.active = Some(PathBuf::from("/p1"));

    let sessions = &st.workspace.sessions[Path::new("/p1")];
    assert_eq!(sessions.len(), 2, "both are live in the same project");
    assert_ne!(sessions[0].id, sessions[1].id);
    assert_eq!(sessions[0].provider, AiCli::ClaudeCode);
    assert_eq!(
        sessions[1].provider,
        AiCli::Copilot,
        "the second session's CLI is its own — sharing a worktree does not share a provider"
    );
    assert!(sessions
        .iter()
        .all(|s| s.lifecycle == SessionLifecycle::Running));
}
