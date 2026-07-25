//! Restart supervision policy (plan W5, US4, tasks T058–T060).
//!
//! This is the pure decision between a child's exit and what the daemon does about it. Detecting an
//! exit lives in [`crate::supervisor`] (the reader thread that already absorbs the blocking PTY read
//! reports it); respawning lives in [`crate::state`]. Keeping the policy here — one function both the
//! attended and unattended paths call — is what makes "supervised anyway" true: there is no second
//! code path that could apply a different retry budget when no client is watching (FR-005).
//!
//! The crash-loop guard itself is the core FSM ([`Session::on_unexpected_exit`]); this module only
//! classifies the exit and translates the core's [`RestartDecision`] into an action the daemon can
//! carry out.

use micold_core::session::{RestartDecision, Session};

/// How a supervised child ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitOutcome {
    /// Exited successfully (status 0) — a normal end: the user quit `claude`, a shell `exit`.
    Clean,
    /// Exited nonzero, or was terminated by a signal — an unexpected end that the crash-loop guard
    /// governs (FR-005).
    Crashed,
}

impl ExitOutcome {
    /// Classify from an OS exit's success bit: success is [`Clean`](Self::Clean); a nonzero code or
    /// a signal is [`Crashed`](Self::Crashed).
    pub fn from_success(success: bool) -> Self {
        if success {
            ExitOutcome::Clean
        } else {
            ExitOutcome::Crashed
        }
    }
}

/// What the daemon must do with a session after its child exited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisionAction {
    /// Normal exit: the session is left stopped (`Idle`); drop the live process, no restart
    /// (FR-004 scenario 3).
    Stop,
    /// Unexpected exit, still under the retry budget: respawn the process (FR-005).
    Restart,
    /// Unexpected exit past the retry budget: settle `Failed`, surfaced on the next attach
    /// (FR-005); drop the live process.
    GiveUp,
}

/// Apply the supervision policy to `session`, mutating its lifecycle, and return the action the
/// daemon must carry out.
///
/// This is the single decision point shared by attended and unattended sessions — the identity that
/// US4 guarantees (FR-005). A clean exit stops the session; a crash defers to the core crash-loop
/// FSM, which increments the attempt counter and eventually settles `Failed`.
pub fn supervise_exit(session: &mut Session, outcome: ExitOutcome) -> SupervisionAction {
    match outcome {
        ExitOutcome::Clean => {
            session.record_clean_exit();
            SupervisionAction::Stop
        }
        ExitOutcome::Crashed => match session.on_unexpected_exit() {
            RestartDecision::Resume => SupervisionAction::Restart,
            RestartDecision::GiveUp => SupervisionAction::GiveUp,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::session::{Session, SessionLifecycle, SessionLocation, MAX_RESTART_ATTEMPTS};

    fn running_session() -> Session {
        let mut s = Session::start_new(SessionLocation::Default);
        s.mark_running();
        s
    }

    #[test]
    fn a_clean_exit_stops_the_session_and_never_restarts() {
        let mut s = running_session();
        assert_eq!(
            supervise_exit(&mut s, ExitOutcome::Clean),
            SupervisionAction::Stop
        );
        assert_eq!(s.lifecycle, SessionLifecycle::Idle);
    }

    #[test]
    fn a_crash_under_budget_asks_for_a_restart() {
        let mut s = running_session();
        assert_eq!(
            supervise_exit(&mut s, ExitOutcome::Crashed),
            SupervisionAction::Restart
        );
        assert_eq!(s.lifecycle, SessionLifecycle::Restarting { attempts: 1 });
    }

    #[test]
    fn repeated_crashes_settle_failed_after_the_retry_budget() {
        let mut s = running_session();
        // The first MAX-1 crashes each ask for a restart, bumping the attempt counter.
        for attempt in 1..MAX_RESTART_ATTEMPTS {
            assert_eq!(
                supervise_exit(&mut s, ExitOutcome::Crashed),
                SupervisionAction::Restart,
                "crash #{attempt} should still restart"
            );
            assert_eq!(
                s.lifecycle,
                SessionLifecycle::Restarting { attempts: attempt }
            );
        }
        // The crash that reaches the budget gives up and settles Failed (durable, surfaced on
        // next attach — FR-005).
        assert_eq!(
            supervise_exit(&mut s, ExitOutcome::Crashed),
            SupervisionAction::GiveUp
        );
        assert_eq!(s.lifecycle, SessionLifecycle::Failed);
    }

    #[test]
    fn a_clean_exit_after_crashes_clears_the_crash_loop() {
        // A crash arms the counter; a subsequent clean exit is not a failure and resets to Idle,
        // so a later crash starts counting from one again (no stale budget carried across a clean
        // run).
        let mut s = running_session();
        assert_eq!(
            supervise_exit(&mut s, ExitOutcome::Crashed),
            SupervisionAction::Restart
        );
        assert_eq!(
            supervise_exit(&mut s, ExitOutcome::Clean),
            SupervisionAction::Stop
        );
        assert_eq!(s.lifecycle, SessionLifecycle::Idle);

        s.mark_running();
        assert_eq!(
            supervise_exit(&mut s, ExitOutcome::Crashed),
            SupervisionAction::Restart
        );
        assert_eq!(s.lifecycle, SessionLifecycle::Restarting { attempts: 1 });
    }

    #[test]
    fn exit_outcome_classifies_success_and_failure() {
        assert_eq!(ExitOutcome::from_success(true), ExitOutcome::Clean);
        assert_eq!(ExitOutcome::from_success(false), ExitOutcome::Crashed);
    }
}
