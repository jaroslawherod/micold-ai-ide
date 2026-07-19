//! T037 — crash auto-restart with crash-loop guard (FR-022/022a).

use micold_ai_ide::session::{
    RestartDecision, Session, SessionLifecycle, SessionLocation, MAX_RESTART_ATTEMPTS,
};

#[test]
fn unexpected_exit_schedules_resume_then_gives_up() {
    let mut s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    s.mark_running();

    // First failures resume (Restarting), incrementing the attempt counter.
    let mut decisions = Vec::new();
    for _ in 0..MAX_RESTART_ATTEMPTS {
        decisions.push(s.on_unexpected_exit());
    }

    // The final attempt gives up → Failed (FR-022a guard).
    assert_eq!(*decisions.last().unwrap(), RestartDecision::GiveUp);
    assert_eq!(s.lifecycle, SessionLifecycle::Failed);
    assert!(decisions[..decisions.len() - 1]
        .iter()
        .all(|d| *d == RestartDecision::Resume));
}

#[test]
fn running_again_resets_the_guard() {
    let mut s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    s.mark_running();
    assert_eq!(s.on_unexpected_exit(), RestartDecision::Resume);
    assert_eq!(s.lifecycle, SessionLifecycle::Restarting { attempts: 1 });

    // A successful restart resets the counter.
    s.mark_running();
    assert_eq!(s.on_unexpected_exit(), RestartDecision::Resume);
    assert_eq!(s.lifecycle, SessionLifecycle::Restarting { attempts: 1 });
}

#[test]
fn failed_session_can_be_manually_restarted() {
    let mut s = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    s.mark_running();
    for _ in 0..MAX_RESTART_ATTEMPTS {
        s.on_unexpected_exit();
    }
    assert_eq!(s.lifecycle, SessionLifecycle::Failed);
    s.start();
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
}
