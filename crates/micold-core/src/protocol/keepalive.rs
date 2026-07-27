//! Client-side liveness keepalive (US5, FR-026, SC-011).
//!
//! A TCP/local-socket connection can go *half-open*: the peer vanishes (power loss, a severed
//! network, a wedged process) without ever sending a FIN, so the client's reader simply blocks
//! forever and would keep presenting stale content as if it were live. FR-026 forbids that — the
//! client MUST notice within a bounded time and transition to a visibly disconnected state.
//!
//! The mechanism is a probe cadence plus a silence deadline, both measured off *any* inbound frame
//! (a `Pong`, a grid delta, a catalog push — anything proves the socket carries). This type is the
//! pure, time-injected core of it: the connection loop feeds it a monotonic clock and acts on the
//! [`KeepaliveAction`] it returns, which is what lets the deadline logic be unit-tested exactly,
//! with no real timers (the connection wiring that calls it lives in the client).

use std::time::{Duration, Instant};

/// How often to send a `Ping` while the link is otherwise idle. Frequent enough that three missed
/// probes still fit inside the SC-011 budget.
pub const PING_INTERVAL: Duration = Duration::from_secs(3);

/// How long the daemon may be silent before the connection is declared dead. Nine seconds is three
/// missed [`PING_INTERVAL`] probes; the resulting worst-case detection (< 10 s) satisfies SC-011.
pub const LIVENESS_DEADLINE: Duration = Duration::from_secs(9);

/// The cadence at which the connection loop should call [`Keepalive::poll`]. It is finer than
/// [`PING_INTERVAL`] on purpose: the deadline is only *noticed* on a poll, so a coarse poll could
/// push expiry well past the deadline. At this cadence worst-case detection is
/// `LIVENESS_DEADLINE + CHECK_INTERVAL` = 10 s, inside the SC-011 budget. Sending an actual `Ping` is
/// still gated by [`PING_INTERVAL`] inside `poll`, so the finer tick costs nothing on the wire.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// What the keepalive wants the connection loop to do at a given instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepaliveAction {
    /// Nothing due yet.
    Idle,
    /// Send a `Ping` now — the ping interval has elapsed since the last probe.
    SendPing,
    /// The deadline lapsed with no inbound frame: treat the connection as dead (FR-026).
    Expired,
}

/// A monotonic liveness tracker for one connection.
///
/// `last_seen` advances on every inbound frame; `last_ping` advances each time the loop is told to
/// probe. [`Self::poll`] compares both against `now`, so the caller only needs to tick it on a timer
/// and forward daemon frames to [`Self::on_daemon_frame`].
#[derive(Debug, Clone, Copy)]
pub struct Keepalive {
    last_seen: Instant,
    last_ping: Instant,
}

impl Keepalive {
    /// Start tracking from `now` (typically the instant the connection was established — the welcome
    /// frame counts as the first sign of life).
    pub fn new(now: Instant) -> Self {
        Self {
            last_seen: now,
            last_ping: now,
        }
    }

    /// Record that a frame arrived from the daemon at `now`. Any frame resets the silence deadline —
    /// a busy connection never needs an explicit `Pong` to stay alive.
    pub fn on_daemon_frame(&mut self, now: Instant) {
        self.last_seen = now;
    }

    /// Decide what to do at `now`. `Expired` takes precedence over `SendPing`: once the deadline has
    /// lapsed there is no point probing a dead link. When this returns `SendPing` it has already
    /// recorded the probe, so the caller must actually send the `Ping`.
    pub fn poll(&mut self, now: Instant) -> KeepaliveAction {
        if now.duration_since(self.last_seen) >= LIVENESS_DEADLINE {
            return KeepaliveAction::Expired;
        }
        if now.duration_since(self.last_ping) >= PING_INTERVAL {
            self.last_ping = now;
            return KeepaliveAction::SendPing;
        }
        KeepaliveAction::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_on_the_ping_interval_while_idle() {
        let t0 = Instant::now();
        let mut ka = Keepalive::new(t0);

        // Nothing due before the interval.
        assert_eq!(ka.poll(t0 + Duration::from_secs(1)), KeepaliveAction::Idle);
        // First probe at the interval boundary.
        assert_eq!(ka.poll(t0 + PING_INTERVAL), KeepaliveAction::SendPing);
        // Having just probed, the next tick is idle again.
        assert_eq!(
            ka.poll(t0 + PING_INTERVAL + Duration::from_millis(1)),
            KeepaliveAction::Idle
        );
        // A second probe one interval later.
        assert_eq!(ka.poll(t0 + PING_INTERVAL * 2), KeepaliveAction::SendPing);
    }

    #[test]
    fn an_inbound_frame_keeps_the_connection_alive() {
        let t0 = Instant::now();
        let mut ka = Keepalive::new(t0);

        // Traffic just before the deadline resets it — a busy link never expires.
        ka.on_daemon_frame(t0 + Duration::from_secs(8));
        assert_ne!(
            ka.poll(t0 + Duration::from_secs(10)),
            KeepaliveAction::Expired,
            "a frame at t+8 must push the deadline past t+10"
        );
    }

    #[test]
    fn a_silent_connection_expires_within_the_sc011_budget() {
        let t0 = Instant::now();
        let mut ka = Keepalive::new(t0);

        // Just under the deadline: not yet dead (still probing).
        assert_ne!(
            ka.poll(t0 + LIVENESS_DEADLINE - Duration::from_millis(1)),
            KeepaliveAction::Expired
        );
        // At the deadline: dead. SC-011 requires this to be < 10 s.
        assert_eq!(ka.poll(t0 + LIVENESS_DEADLINE), KeepaliveAction::Expired);
        assert!(LIVENESS_DEADLINE < Duration::from_secs(10));
        // The bound that actually matters is deadline + one poll cadence — that must fit in 10 s.
        assert!(LIVENESS_DEADLINE + CHECK_INTERVAL <= Duration::from_secs(10));
    }

    #[test]
    fn expiry_takes_precedence_over_a_due_probe() {
        let t0 = Instant::now();
        let mut ka = Keepalive::new(t0);
        // Well past the deadline (a probe is also "due", but the link is already dead).
        assert_eq!(
            ka.poll(t0 + LIVENESS_DEADLINE + PING_INTERVAL),
            KeepaliveAction::Expired
        );
    }
}
