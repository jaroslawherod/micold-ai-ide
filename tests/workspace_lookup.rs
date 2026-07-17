//! T002/T004 (feature 008): cross-project session lookup + per-project running count.

mod support;

use micold_ai_ide::session::{SessionId, SessionLifecycle};
use std::path::Path;
use support::{failed_session, idle_session, running_session, workspace_with};

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
