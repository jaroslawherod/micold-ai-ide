//! The dialing side of the link (#1725), as pure rules: the status a daemon reports and the delay
//! before its next attempt. No clock, no randomness (the jitter unit is passed in) and no socket,
//! so every rule is tested exactly; `micold-daemon::orchestrator_link` runs them.

use std::time::Duration;

use super::announce::RefuseReason;
use crate::ssh::reconnect::{Backoff, MAX_DELAY};

/// The most a jittered delay grows: up to 20 % (FR-006).
pub const MAX_JITTER: f64 = 0.2;

/// Where a daemon stands with its orchestrator link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkStatus {
    /// No link: the start, a lost link or an endpoint nothing listens on.
    NotLinked,
    /// The endpoint accepted and the handshake is under way.
    Linking,
    /// `Welcome` was read.
    Linked,
    /// The orchestrator refused the announcement.
    Refused(RefuseReason),
    /// The orchestrator accepted the announcement and then refused `Hello`; the detail says why.
    HelloRefused(String),
}

impl LinkStatus {
    /// The state's name, as logged.
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotLinked => "not linked",
            Self::Linking => "linking",
            Self::Linked => "linked",
            Self::Refused(RefuseReason::VersionMismatch) => "refused (version mismatch)",
            Self::Refused(RefuseReason::AlreadyLinked) => "refused (already linked)",
            Self::HelloRefused(_) => "refused (hello)",
        }
    }

    /// The state's word in `status.json` (FR-012).
    pub fn word(&self) -> &'static str {
        match self {
            Self::NotLinked => "not-linked",
            Self::Linking => "linking",
            Self::Linked => "linked",
            Self::Refused(RefuseReason::VersionMismatch) => "refused-version-mismatch",
            Self::Refused(RefuseReason::AlreadyLinked) => "refused-already-linked",
            Self::HelloRefused(_) => "refused-hello",
        }
    }
}

/// `delay` grown by `unit` of [`MAX_JITTER`]: `unit` in `0.0..=1.0` gives `1.0..=1.2` times
/// `delay`. A `unit` outside the range (or NaN) is clamped, so a bad source cannot shrink or
/// double a delay.
pub fn jittered(delay: Duration, unit: f64) -> Duration {
    let unit = if unit.is_nan() {
        0.0
    } else {
        unit.clamp(0.0, 1.0)
    };
    delay.mul_f64(1.0 + MAX_JITTER * unit)
}

/// The delay before the next dial, over [`Backoff`]. Only [`Self::after_failure`], the connect
/// delay, is never jittered, so the capped delay stays 15 s and SC-002/SC-003 keep their budget.
#[derive(Debug, Clone, Copy, Default)]
pub struct RetryPolicy {
    backoff: Backoff,
}

impl RetryPolicy {
    /// A policy at its first delay.
    pub fn new() -> Self {
        Self::default()
    }

    /// The endpoint is absent or the dial failed: 1, 2, 4, 8, 15, 15 s, no jitter.
    pub fn after_failure(&mut self) -> Duration {
        self.backoff.next_delay()
    }

    /// The orchestrator refused: always the cap, and the connect backoff does not grow. The caller
    /// applies [`jittered`], so a crowd of refused daemons does not redial in step.
    pub fn after_refusal(&self) -> Duration {
        MAX_DELAY
    }

    /// `Welcome` was read: the next failure starts at one second.
    pub fn on_linked(&mut self) {
        self.backoff.reset();
    }

    /// A link that was up ended: start again at one second (the next failure gives two). The
    /// caller applies [`jittered`] to this first delay.
    pub fn after_loss(&mut self) -> Duration {
        self.backoff.reset();
        self.backoff.next_delay()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(policy: &mut RetryPolicy, n: usize) -> Vec<u64> {
        (0..n).map(|_| policy.after_failure().as_secs()).collect()
    }

    #[test]
    fn backoff_delays_are_one_two_four_eight_then_the_cap() {
        assert_eq!(secs(&mut RetryPolicy::new(), 6), [1, 2, 4, 8, 15, 15]);
    }

    #[test]
    fn a_refusal_waits_the_cap_and_does_not_grow_the_connect_backoff() {
        let mut policy = RetryPolicy::new();
        for _ in 0..4 {
            assert_eq!(policy.after_refusal(), MAX_DELAY);
        }
        assert_eq!(secs(&mut policy, 3), [1, 2, 4]);
    }

    #[test]
    fn a_link_resets_the_backoff() {
        let mut policy = RetryPolicy::new();
        secs(&mut policy, 5);
        policy.on_linked();
        assert_eq!(secs(&mut policy, 2), [1, 2]);
    }

    #[test]
    fn a_loss_gives_one_second_then_two() {
        let mut policy = RetryPolicy::new();
        secs(&mut policy, 5);
        assert_eq!(policy.after_loss(), Duration::from_secs(1));
        assert_eq!(policy.after_failure(), Duration::from_secs(2));
    }

    #[test]
    fn jitter_stays_within_one_to_one_point_two_times_the_delay() {
        let delay = Duration::from_secs(10);
        assert_eq!(jittered(delay, 0.0), delay);
        assert_eq!(jittered(delay, 1.0), Duration::from_secs(12));
        let mid = jittered(delay, 0.5);
        assert!(mid > delay && mid < Duration::from_secs(12), "{mid:?}");
        for bad in [-3.0, 7.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let j = jittered(delay, bad);
            assert!(j >= delay && j <= Duration::from_secs(12), "{bad}: {j:?}");
        }
    }

    #[test]
    fn the_capped_connect_delay_carries_no_jitter() {
        let mut policy = RetryPolicy::new();
        secs(&mut policy, 4);
        for _ in 0..3 {
            assert_eq!(policy.after_failure(), MAX_DELAY);
        }
    }

    #[test]
    fn every_status_has_a_distinct_name() {
        let all = [
            LinkStatus::NotLinked,
            LinkStatus::Linking,
            LinkStatus::Linked,
            LinkStatus::Refused(RefuseReason::VersionMismatch),
            LinkStatus::Refused(RefuseReason::AlreadyLinked),
            LinkStatus::HelloRefused("no".into()),
        ];
        let names: std::collections::BTreeSet<_> = all.iter().map(LinkStatus::name).collect();
        assert_eq!(names.len(), all.len());
    }
}
