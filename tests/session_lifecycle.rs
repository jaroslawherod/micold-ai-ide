//! T036 — session lifecycle transitions: start / running / select-neutral / close (FR-010/015/015a).

use micold_ai_ide::session::{Session, SessionLifecycle};

#[test]
fn new_session_starts() {
    let s = Session::start_new("feat-x");
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
    assert_eq!(s.worktree_dir, "feat-x");
    assert!(s.is_active());
}

#[test]
fn mark_running_transitions_from_starting() {
    let mut s = Session::start_new("feat-x");
    s.mark_running();
    assert_eq!(s.lifecycle, SessionLifecycle::Running);
    assert!(s.is_active());
}

#[test]
fn restored_session_is_idle_and_inactive() {
    use micold_ai_ide::session::{SessionId, SessionLabel};
    let s = Session::restored(SessionId::new(), "feat-x", SessionLabel::Pending);
    assert_eq!(s.lifecycle, SessionLifecycle::Idle);
    assert!(!s.is_active());
}

#[test]
fn idle_session_can_start_again() {
    use micold_ai_ide::session::{SessionId, SessionLabel};
    let mut s = Session::restored(SessionId::new(), "feat-x", SessionLabel::Pending);
    s.start();
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
}

#[test]
fn title_updates_label() {
    use micold_ai_ide::session::SessionLabel;
    let mut s = Session::start_new("feat-x");
    assert_eq!(s.label, SessionLabel::Pending);
    assert_eq!(s.label.display(), "New session");
    s.set_title("Add login page");
    assert_eq!(s.label, SessionLabel::Named("Add login page".to_string()));
    assert_eq!(s.label.display(), "Add login page");
}
