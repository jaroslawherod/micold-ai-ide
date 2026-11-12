//! One link's heartbeat (#1724, FR-005): the protocol's [`Keepalive`] with its constants, plus the
//! memory that expiry is reported once and that a silent link is still probed, so a daemon that
//! was only delayed has something to answer. Pure: every instant is passed in.

use std::time::Instant;

use crate::protocol::keepalive::{Keepalive, KeepaliveAction, PING_INTERVAL};

/// The heartbeat of one live link.
#[derive(Debug, Clone, Copy)]
pub struct Heartbeat {
    keepalive: Keepalive,
    last_probe: Instant,
    expired: bool,
}

impl Heartbeat {
    /// Start counting from `now`, the instant the link was confirmed.
    pub fn new(now: Instant) -> Self {
        Self {
            keepalive: Keepalive::new(now),
            last_probe: now,
            expired: false,
        }
    }

    /// Any frame from the daemon, `LinkPong` included, at `now`. Returns whether the link had
    /// been reported expired (it is live again).
    pub fn answered(&mut self, now: Instant) -> bool {
        self.keepalive.on_daemon_frame(now);
        std::mem::take(&mut self.expired)
    }

    /// What the link loop does at `now`. `Expired` is returned once per silence; after it the link
    /// is still probed every [`PING_INTERVAL`], so a late daemon can answer.
    pub fn tick(&mut self, now: Instant) -> KeepaliveAction {
        match self.keepalive.poll(now) {
            KeepaliveAction::SendPing => {
                self.last_probe = now;
                KeepaliveAction::SendPing
            }
            KeepaliveAction::Expired if !self.expired => {
                self.expired = true;
                KeepaliveAction::Expired
            }
            KeepaliveAction::Expired => {
                if now.duration_since(self.last_probe) >= PING_INTERVAL {
                    self.last_probe = now;
                    KeepaliveAction::SendPing
                } else {
                    KeepaliveAction::Idle
                }
            }
            idle => idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::protocol::keepalive::LIVENESS_DEADLINE;

    #[test]
    fn pings_after_the_interval_and_is_idle_between() {
        let t0 = Instant::now();
        let mut beat = Heartbeat::new(t0);
        assert_eq!(beat.tick(t0 + Duration::from_secs(1)), KeepaliveAction::Idle);
        assert_eq!(beat.tick(t0 + PING_INTERVAL), KeepaliveAction::SendPing);
        assert_eq!(
            beat.tick(t0 + PING_INTERVAL + Duration::from_secs(1)),
            KeepaliveAction::Idle
        );
    }

    #[test]
    fn expires_once_at_the_deadline_then_keeps_probing() {
        let t0 = Instant::now();
        let mut beat = Heartbeat::new(t0);
        assert_eq!(beat.tick(t0 + LIVENESS_DEADLINE), KeepaliveAction::Expired);
        let after = t0 + LIVENESS_DEADLINE;
        assert_eq!(beat.tick(after + Duration::from_secs(1)), KeepaliveAction::Idle);
        assert_eq!(beat.tick(after + PING_INTERVAL), KeepaliveAction::SendPing);
        assert_eq!(
            beat.tick(after + PING_INTERVAL + Duration::from_secs(1)),
            KeepaliveAction::Idle
        );
    }

    #[test]
    fn an_answer_reports_the_return_once_and_restarts_the_deadline() {
        let t0 = Instant::now();
        let mut beat = Heartbeat::new(t0);
        assert!(!beat.answered(t0 + Duration::from_secs(2)));
        assert_eq!(
            beat.tick(t0 + LIVENESS_DEADLINE),
            KeepaliveAction::SendPing,
            "an answer at t+2 pushes the deadline past t+9"
        );
        let silent = t0 + LIVENESS_DEADLINE + Duration::from_secs(5);
        assert_eq!(beat.tick(silent), KeepaliveAction::Expired);
        assert!(beat.answered(silent));
        assert!(!beat.answered(silent));
    }
}
