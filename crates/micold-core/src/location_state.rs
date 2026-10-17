//! A location's state (feature 905): Running, Blocked, Idle / waiting or Unknown, derived from the
//! sessions at a worktree or the project root (data-model *LocationState*). Nothing is stored: the
//! sidebar recomputes it on every render from the sessions the last catalog snapshot gave.

use crate::protocol::messages::ActivitySignal;
use crate::session::{Session, SessionLifecycle};

/// Why a location is Blocked. `Ord` is derived in ascending precedence, so among blocked sessions
/// the location reports the most urgent reason (FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlockedReason {
    /// The service restarted and the session waits for an explicit resume.
    Resumable,
    /// The session's start failed or its restarts ran out.
    Failed,
    /// The session stopped mid-turn to ask the user for a permission or an answer.
    NeedsPermission,
}

/// The state a worktree row (or the Default row) shows. `Ord` is derived in ascending precedence,
/// so a location's state is the `max` of its sessions' states (FR-004, research R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LocationState {
    /// Nothing in progress, or waiting for the next prompt.
    Idle,
    /// The app cannot read the session's activity (FR-005): the icon slot stays empty.
    Unknown,
    /// A session is working, starting or restarting.
    Running,
    /// A session cannot proceed without the user.
    Blocked(BlockedReason),
}

impl LocationState {
    /// The state in words, for the row's tooltip (FR-009).
    pub fn tooltip_line(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Blocked(BlockedReason::NeedsPermission) => "Blocked: needs permission",
            Self::Blocked(BlockedReason::Failed) => "Blocked: failed",
            Self::Blocked(BlockedReason::Resumable) => "Blocked: resumable",
            Self::Idle => "Idle",
            Self::Unknown => "State unknown",
        }
    }
}

/// One session's state, by the spec's ordered per-session rules (first match wins, FR-003).
pub fn session_state(session: &Session) -> LocationState {
    use LocationState::*;
    match &session.lifecycle {
        SessionLifecycle::Failed { .. } => Blocked(BlockedReason::Failed),
        SessionLifecycle::InterruptedResumable => Blocked(BlockedReason::Resumable),
        SessionLifecycle::Starting | SessionLifecycle::Restarting { .. } => Running,
        SessionLifecycle::Idle => Idle,
        SessionLifecycle::Running => match session.activity {
            ActivitySignal::Working => Running,
            ActivitySignal::AwaitingInput if session.paused_on_user => {
                Blocked(BlockedReason::NeedsPermission)
            }
            ActivitySignal::AwaitingInput | ActivitySignal::Ended { .. } => Idle,
            // Rule 7: an unreadable activity never makes a row Blocked or Idle (FR-005).
            ActivitySignal::Unknown => Unknown,
        },
    }
}

/// A location's state from all of its sessions: archived ones skipped, none left → `Idle`
/// (FR-003), otherwise the most urgent session state (FR-004).
pub fn location_state<'a>(sessions: impl IntoIterator<Item = &'a Session>) -> LocationState {
    sessions
        .into_iter()
        .filter(|s| !s.archived)
        .map(session_state)
        .max()
        .unwrap_or(LocationState::Idle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{AiCli, SessionLocation};

    fn session(lifecycle: SessionLifecycle, activity: ActivitySignal, paused: bool) -> Session {
        let mut s = Session::start_new(SessionLocation::Worktree("wt".into()), AiCli::ClaudeCode);
        s.lifecycle = lifecycle;
        s.activity = activity;
        s.paused_on_user = paused;
        s
    }

    fn lifecycles() -> Vec<SessionLifecycle> {
        vec![
            SessionLifecycle::Idle,
            SessionLifecycle::Starting,
            SessionLifecycle::Running,
            SessionLifecycle::Restarting { attempts: 1 },
            SessionLifecycle::Failed {
                reason: "gave up".into(),
                attempts: 3,
            },
            SessionLifecycle::InterruptedResumable,
        ]
    }

    fn activities() -> Vec<ActivitySignal> {
        vec![
            ActivitySignal::Unknown,
            ActivitySignal::Working,
            ActivitySignal::AwaitingInput,
            ActivitySignal::Ended {
                reason: "exit".into(),
            },
        ]
    }

    /// The data-model rule table, written out row by row, independent of the implementation's
    /// `match` (SC-004).
    fn expected(l: &SessionLifecycle, a: &ActivitySignal, paused: bool) -> LocationState {
        use LocationState::*;
        if matches!(l, SessionLifecycle::Failed { .. }) {
            return Blocked(BlockedReason::Failed);
        }
        if *l == SessionLifecycle::InterruptedResumable {
            return Blocked(BlockedReason::Resumable);
        }
        if matches!(
            l,
            SessionLifecycle::Starting | SessionLifecycle::Restarting { .. }
        ) {
            return Running;
        }
        if *l == SessionLifecycle::Idle {
            return Idle;
        }
        match (a, paused) {
            (ActivitySignal::Working, _) => Running,
            (ActivitySignal::AwaitingInput, true) => Blocked(BlockedReason::NeedsPermission),
            (ActivitySignal::AwaitingInput, false) => Idle,
            (ActivitySignal::Ended { .. }, _) => Idle,
            (ActivitySignal::Unknown, _) => Unknown,
        }
    }

    #[test]
    fn every_lifecycle_activity_and_pause_combination_follows_the_rule_table() {
        let mut checked = 0;
        for l in lifecycles() {
            for a in activities() {
                for paused in [false, true] {
                    let s = session(l.clone(), a.clone(), paused);
                    assert_eq!(
                        session_state(&s),
                        expected(&l, &a, paused),
                        "{l:?} × {a:?} × paused={paused}"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 6 * 4 * 2);
    }

    #[test]
    fn running_with_unknown_activity_is_unknown_never_blocked() {
        for paused in [false, true] {
            let s = session(SessionLifecycle::Running, ActivitySignal::Unknown, paused);
            assert_eq!(session_state(&s), LocationState::Unknown);
        }
    }

    #[test]
    fn a_location_with_no_session_is_idle() {
        assert_eq!(location_state(std::iter::empty()), LocationState::Idle);
    }

    #[test]
    fn archived_sessions_do_not_count() {
        let mut blocked = of(LocationState::Blocked(BlockedReason::Failed));
        blocked.archive();
        let idle = of(LocationState::Idle);
        assert_eq!(location_state([&blocked, &idle]), LocationState::Idle);
        assert_eq!(location_state([&blocked]), LocationState::Idle);
    }

    /// A session whose own state is `state`.
    fn of(state: LocationState) -> Session {
        use LocationState::*;
        match state {
            Idle => session(SessionLifecycle::Idle, ActivitySignal::Unknown, false),
            Unknown => session(SessionLifecycle::Running, ActivitySignal::Unknown, false),
            Running => session(SessionLifecycle::Running, ActivitySignal::Working, false),
            Blocked(BlockedReason::NeedsPermission) => session(
                SessionLifecycle::Running,
                ActivitySignal::AwaitingInput,
                true,
            ),
            Blocked(BlockedReason::Failed) => session(
                SessionLifecycle::Failed {
                    reason: "x".into(),
                    attempts: 3,
                },
                ActivitySignal::Unknown,
                false,
            ),
            Blocked(BlockedReason::Resumable) => session(
                SessionLifecycle::InterruptedResumable,
                ActivitySignal::Unknown,
                false,
            ),
        }
    }

    #[test]
    fn the_most_urgent_session_wins_in_either_order() {
        use BlockedReason::*;
        use LocationState::*;
        let pairs = [
            (Blocked(Resumable), Running),
            (Running, Unknown),
            (Unknown, Idle),
            (Blocked(Failed), Idle),
            (Blocked(NeedsPermission), Blocked(Failed)),
            (Blocked(Failed), Blocked(Resumable)),
            (Blocked(Resumable), Unknown),
        ];
        for (high, low) in pairs {
            let (h, l) = (of(high), of(low));
            assert_eq!(location_state([&h, &l]), high, "{high:?} over {low:?}");
            assert_eq!(location_state([&l, &h]), high, "{high:?} over {low:?}");
        }
    }

    #[test]
    fn the_tooltip_line_names_each_state() {
        use BlockedReason::*;
        use LocationState::*;
        assert_eq!(Running.tooltip_line(), "Running");
        assert_eq!(
            Blocked(NeedsPermission).tooltip_line(),
            "Blocked: needs permission"
        );
        assert_eq!(Blocked(Failed).tooltip_line(), "Blocked: failed");
        assert_eq!(Blocked(Resumable).tooltip_line(), "Blocked: resumable");
        assert_eq!(Idle.tooltip_line(), "Idle");
        assert_eq!(Unknown.tooltip_line(), "State unknown");
    }
}
