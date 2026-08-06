//! Rate-limiting for a long operation's progress reports (BUG-009, T123).
//!
//! A worktree create reports two different kinds of thing on one channel: *stage transitions*,
//! which are few and each worth a frame, and *live output lines*, of which a submodule fetch emits
//! thousands. Feature 013's rule was to forward transitions and drop everything else, which left
//! the create form frozen on "Setting up submodules" for the whole fetch — the state US2 of
//! `010-submodule-worktree-support` exists to prevent.
//!
//! This is the decision that fixes it, kept out of `server.rs`'s connection loop so it can be
//! tested against an injected clock: a stage transition always reports, and within a stage the
//! latest line reports at most once per `min_gap`.
//!
//! **This is a display signal, not a liveness signal.** The connection stays alive because the loop
//! that serves it is never parked (FR-026a, T120), not because these frames keep arriving — a fetch
//! that goes quiet for a minute is still a live one, and nothing may come to depend on this traffic
//! to decide otherwise.

use std::time::{Duration, Instant};

use micold_core::worktree::{CreateProgressEvent, CreateStage};

/// Decides which of an operation's progress events reach the client.
pub struct ProgressThrottle {
    last_stage: Option<CreateStage>,
    last_detail_at: Option<Instant>,
    min_gap: Duration,
}

impl ProgressThrottle {
    /// A throttle that forwards at most one same-stage line per `min_gap`.
    pub fn new(min_gap: Duration) -> Self {
        Self {
            last_stage: None,
            last_detail_at: None,
            min_gap,
        }
    }

    /// What to send for `event`, observed at `now`:
    ///
    /// - `Some(None)` — the operation entered a new stage; report the stage alone.
    /// - `Some(Some(line))` — same stage, and enough time has passed to report where it is.
    /// - `None` — same stage, too soon; drop it.
    pub fn admit(&mut self, event: CreateProgressEvent, now: Instant) -> Option<Option<String>> {
        if self.last_stage != Some(event.stage) {
            self.last_stage = Some(event.stage);
            // A fresh stage owes the user its first line promptly rather than a gap's silence, so
            // the detail clock starts unset: the next line of this stage is immediately due.
            self.last_detail_at = None;
            return Some(None);
        }
        let due = self
            .last_detail_at
            .is_none_or(|t| now.duration_since(t) >= self.min_gap);
        if !due {
            return None;
        }
        self.last_detail_at = Some(now);
        Some(Some(event.line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAP: Duration = Duration::from_millis(400);

    fn event(stage: CreateStage, line: &str) -> CreateProgressEvent {
        CreateProgressEvent {
            stage,
            line: line.to_string(),
        }
    }

    #[test]
    fn entering_a_stage_reports_the_stage_alone() {
        let t0 = Instant::now();
        let mut th = ProgressThrottle::new(GAP);
        assert_eq!(
            th.admit(event(CreateStage::PreflightCheck, "checking…"), t0),
            Some(None)
        );
    }

    #[test]
    fn the_first_line_of_a_stage_is_reported_without_waiting_out_the_gap() {
        // Otherwise every stage would open with a gap's worth of silence — the frozen-label
        // behaviour this exists to remove, just shorter.
        let t0 = Instant::now();
        let mut th = ProgressThrottle::new(GAP);
        th.admit(
            event(CreateStage::SettingUpSubmodules, "$ git submodule…"),
            t0,
        );
        assert_eq!(
            th.admit(
                event(CreateStage::SettingUpSubmodules, "Cloning into 'vendor/a'…"),
                t0 + Duration::from_millis(1)
            ),
            Some(Some("Cloning into 'vendor/a'…".to_string()))
        );
    }

    #[test]
    fn same_stage_lines_inside_the_gap_are_dropped() {
        let t0 = Instant::now();
        let mut th = ProgressThrottle::new(GAP);
        th.admit(
            event(CreateStage::SettingUpSubmodules, "$ git submodule…"),
            t0,
        );
        th.admit(event(CreateStage::SettingUpSubmodules, "first"), t0);
        // A fetch emits these in a burst; only the clock decides, never the count.
        for ms in [1, 50, 399] {
            assert_eq!(
                th.admit(
                    event(CreateStage::SettingUpSubmodules, "burst"),
                    t0 + Duration::from_millis(ms)
                ),
                None
            );
        }
    }

    #[test]
    fn a_same_stage_line_reports_again_once_the_gap_has_passed() {
        let t0 = Instant::now();
        let mut th = ProgressThrottle::new(GAP);
        th.admit(
            event(CreateStage::SettingUpSubmodules, "$ git submodule…"),
            t0,
        );
        th.admit(event(CreateStage::SettingUpSubmodules, "first"), t0);
        assert_eq!(
            th.admit(event(CreateStage::SettingUpSubmodules, "later"), t0 + GAP),
            Some(Some("later".to_string()))
        );
    }

    #[test]
    fn a_stage_transition_is_never_throttled() {
        // The stage is the thing the user reads; a transition arriving mid-gap must not be dropped
        // for having arrived too soon after a line. Rollback is the case that matters: a failure
        // right after a fetch line must still be able to say it is rolling back.
        let t0 = Instant::now();
        let mut th = ProgressThrottle::new(GAP);
        th.admit(
            event(CreateStage::SettingUpSubmodules, "$ git submodule…"),
            t0,
        );
        th.admit(event(CreateStage::SettingUpSubmodules, "first"), t0);
        assert_eq!(
            th.admit(
                event(CreateStage::RollingBack, "Rolling back…"),
                t0 + Duration::from_millis(1)
            ),
            Some(None)
        );
    }

    #[test]
    fn returning_to_a_stage_reports_it_again() {
        // Stages are not monotonic in general (a retry, a rollback that re-enters), so "new stage"
        // means "different from the last one", not "never seen".
        let t0 = Instant::now();
        let mut th = ProgressThrottle::new(GAP);
        th.admit(
            event(CreateStage::CreatingWorktree, "$ git worktree add"),
            t0,
        );
        th.admit(event(CreateStage::RollingBack, "Rolling back…"), t0);
        assert_eq!(
            th.admit(
                event(CreateStage::CreatingWorktree, "$ git worktree add"),
                t0
            ),
            Some(None)
        );
    }
}
