//! T036 — session lifecycle transitions: start / running / select-neutral / close (FR-010/015/015a).
//! T002 (010-root-dir-session) — SessionLocation replaces the bare worktree_dir string.

use micold_ai_ide::session::{Session, SessionLifecycle, SessionLocation};

#[test]
fn new_session_starts() {
    let s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
    assert_eq!(s.location, SessionLocation::Worktree("feat-x".to_string()));
    assert!(s.is_active());
}

#[test]
fn new_default_session_has_no_worktree_identity() {
    let s = Session::start_new(SessionLocation::Default);
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
    assert_eq!(s.location, SessionLocation::Default);
    assert_ne!(s.location, SessionLocation::Worktree(String::new()));
}

#[test]
fn restored_default_session_is_idle_and_inactive() {
    use micold_ai_ide::session::{SessionId, SessionLabel};
    let s = Session::restored(
        SessionId::new(),
        SessionLocation::Default,
        SessionLabel::Pending,
    );
    assert_eq!(s.lifecycle, SessionLifecycle::Idle);
    assert_eq!(s.location, SessionLocation::Default);
    assert!(!s.is_active());
}

#[test]
fn mark_running_transitions_from_starting() {
    let mut s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    s.mark_running();
    assert_eq!(s.lifecycle, SessionLifecycle::Running);
    assert!(s.is_active());
}

#[test]
fn restored_session_is_idle_and_inactive() {
    use micold_ai_ide::session::{SessionId, SessionLabel};
    let s = Session::restored(
        SessionId::new(),
        SessionLocation::Worktree("feat-x".to_string()),
        SessionLabel::Pending,
    );
    assert_eq!(s.lifecycle, SessionLifecycle::Idle);
    assert!(!s.is_active());
}

#[test]
fn idle_session_can_start_again() {
    use micold_ai_ide::session::{SessionId, SessionLabel};
    let mut s = Session::restored(
        SessionId::new(),
        SessionLocation::Worktree("feat-x".to_string()),
        SessionLabel::Pending,
    );
    s.start();
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
}

#[test]
fn title_updates_label() {
    use micold_ai_ide::session::SessionLabel;
    let mut s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    assert_eq!(s.label, SessionLabel::Pending);
    assert_eq!(s.label.display(), "New session");
    s.set_title("Add login page");
    assert_eq!(s.label, SessionLabel::Named("Add login page".to_string()));
    assert_eq!(s.label.display(), "Add login page");
}
