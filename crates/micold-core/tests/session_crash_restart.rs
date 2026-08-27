//! T037 — crash auto-restart with crash-loop guard (FR-022/022a).

use micold_core::session::{
    AiCli, RestartDecision, Session, SessionLifecycle, SessionLocation, MAX_RESTART_ATTEMPTS,
};

#[test]
fn unexpected_exit_schedules_resume_then_gives_up() {
    let mut s = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    );
    s.mark_running();

    // First failures resume (Restarting), incrementing the attempt counter.
    let mut decisions = Vec::new();
    for _ in 0..MAX_RESTART_ATTEMPTS {
        decisions.push(s.on_unexpected_exit("exit status 1"));
    }

    // The final attempt gives up → Failed (FR-022a guard).
    assert_eq!(*decisions.last().unwrap(), RestartDecision::GiveUp);
    let SessionLifecycle::Failed { reason, attempts } = &s.lifecycle else {
        panic!("expected a give-up, got {:?}", s.lifecycle);
    };
    assert_eq!(
        *attempts, MAX_RESTART_ATTEMPTS,
        "the budget it actually spent"
    );
    // The give-up says what it gave up on, and does it once, here — so the attended and unattended
    // paths cannot word it differently (010 BUG-017, FR-005). The caller supplies only the exit
    // (`exit status 1` above); the count is this FSM's own, because the budget is.
    assert!(
        reason.contains("exit status 1"),
        "the reason names the last exit; got {reason:?}"
    );
    assert!(
        reason.contains(&MAX_RESTART_ATTEMPTS.to_string()),
        "and how many attempts it spent; got {reason:?}"
    );
    assert!(decisions[..decisions.len() - 1]
        .iter()
        .all(|d| *d == RestartDecision::Resume));
}

#[test]
fn running_again_resets_the_guard() {
    let mut s = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    );
    s.mark_running();
    assert_eq!(
        s.on_unexpected_exit("exit status 1"),
        RestartDecision::Resume
    );
    assert_eq!(s.lifecycle, SessionLifecycle::Restarting { attempts: 1 });

    // A successful restart resets the counter.
    s.mark_running();
    assert_eq!(
        s.on_unexpected_exit("exit status 1"),
        RestartDecision::Resume
    );
    assert_eq!(s.lifecycle, SessionLifecycle::Restarting { attempts: 1 });
}

#[test]
fn failed_session_can_be_manually_restarted() {
    let mut s = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    );
    s.mark_running();
    for _ in 0..MAX_RESTART_ATTEMPTS {
        s.on_unexpected_exit("exit status 1");
    }
    assert!(matches!(s.lifecycle, SessionLifecycle::Failed { .. }));
    s.start();
    assert_eq!(s.lifecycle, SessionLifecycle::Starting);
}
