//! T038 — project close/switch stops sessions → Idle without auto-restart (FR-023/023a).

use micold_ai_ide::session::{Session, SessionLifecycle, SessionLocation};

#[test]
fn project_change_stops_to_idle_preserving_identity() {
    let mut s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    s.mark_running();
    let id = s.id;
    s.set_title("Work in progress");

    s.stop_for_project_change();

    assert_eq!(s.lifecycle, SessionLifecycle::Idle);
    assert!(!s.is_active());
    // Identity and label are preserved for restore/resume (FR-023a).
    assert_eq!(s.id, id);
    assert_eq!(s.label.display(), "Work in progress");
}

#[test]
fn intentional_stop_does_not_count_as_a_crash() {
    let mut s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    s.mark_running();
    s.stop_for_project_change();

    // Reopen resumes cleanly; no Restarting/Failed leakage from the intentional stop.
    s.start();
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
}
