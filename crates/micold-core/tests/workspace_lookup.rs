//! T002/T004 (feature 008): cross-project session lookup + per-project running count.

mod support;

use micold_core::session::{SessionId, SessionLifecycle, SessionLocation};
use std::path::Path;
use support::{failed_session, idle_session, running_session, workspace_with};

// T006 (010-root-dir-session): `find_session`/`running_session_count` still correctly
// attribute sessions after `worktree_dir: String` became `location: SessionLocation`.
#[test]
fn find_session_matches_on_session_location_not_a_bare_string() {
    let ws = workspace_with(vec![("/a", vec![running_session("wt-a")])]);
    let id = ws.sessions[Path::new("/a")][0].id;
    let (_, sess) = ws.find_session(id).expect("resolved");
    assert_eq!(sess.location, SessionLocation::Worktree("wt-a".to_string()));
}

#[test]
fn find_session_resolves_in_non_active_project() {
    let mut ws = workspace_with(vec![
        ("/a", vec![running_session("wt-a")]),
        ("/b", vec![running_session("wt-b")]),
    ]);
    ws.active = Some(Path::new("/b").into()); // /a is NOT active

    let id_a = ws.sessions[Path::new("/a")][0].id;
    let (path, sess) = ws.find_session(id_a).expect("resolved across projects");
    assert_eq!(path, Path::new("/a"));
    assert_eq!(sess.id, id_a);
}

#[test]
fn find_session_none_for_unknown_id() {
    let ws = workspace_with(vec![("/a", vec![running_session("wt")])]);
    assert!(ws.find_session(SessionId::new()).is_none());
}

#[test]
fn find_session_mut_allows_lifecycle_mutation() {
    let mut ws = workspace_with(vec![("/a", vec![running_session("wt")])]);
    let id = ws.sessions[Path::new("/a")][0].id;

    let (owner, s) = ws.find_session_mut(id).expect("resolved mutably");
    assert_eq!(owner, Path::new("/a"));
    let _ = s.on_unexpected_exit();

    assert!(matches!(
        ws.sessions[Path::new("/a")][0].lifecycle,
        SessionLifecycle::Restarting { .. }
    ));
}

// Feature 014 (forget project): the binary needs every recorded session id for a project — of
// any lifecycle — to stop its live processes before forgetting it (FR-010).
#[test]
fn session_ids_of_project_returns_all_recorded_ids_regardless_of_lifecycle() {
    let ws = workspace_with(vec![
        (
            "/a",
            vec![
                running_session("w1"),
                idle_session("w2"),
                failed_session("w3"),
            ],
        ),
        ("/b", vec![running_session("wb")]),
    ]);

    let ids_a = ws.session_ids_of_project(Path::new("/a"));
    let expected: Vec<_> = ws.sessions[Path::new("/a")].iter().map(|s| s.id).collect();
    assert_eq!(ids_a, expected, "all three /a sessions, in order");
    assert_eq!(ids_a.len(), 3);

    assert_eq!(ws.session_ids_of_project(Path::new("/b")).len(), 1);
    assert!(
        ws.session_ids_of_project(Path::new("/unknown")).is_empty(),
        "unknown project has no session ids"
    );
}

#[test]
fn running_session_count_counts_active_only() {
    let ws = workspace_with(vec![
        (
            "/a",
            vec![
                running_session("w1"),
                idle_session("w2"),
                failed_session("w3"),
            ],
        ),
        ("/b", vec![]),
    ]);

    assert_eq!(ws.running_session_count(Path::new("/a")), 1);
    assert_eq!(ws.running_session_count(Path::new("/b")), 0);
    assert_eq!(ws.running_session_count(Path::new("/unknown")), 0);
}
