//! When nobody is using this daemon, and what it does about it (feature 028, data-model G1/G2/G4).
//!
//! Three things live here: [`Presence`] — how many clients are connected and, when none are, since
//! when; [`IdleWindow`] — the 30-minute rule read against that count; and [`StopReason`] — why the
//! daemon is unwinding, so the last line in the log says which of the ways out this was.
//!
//! Deliberately not `state.rs`: the rule is a pure function of a count and a clock reading, and
//! keeping it away from the lock-guarded session catalogue is what makes it testable without a
//! daemon.
//!
//! # Sessions are not an input
//!
//! This replaces `lifecycle::may_exit(live_sessions, connected_clients)`, and the difference is the
//! feature: a live session no longer holds the service up (FR-006a). It cannot, once the service
//! stops itself — a machine left with an agent running and no window open would keep a daemon alive
//! forever, which is the situation this feature exists to end. What protects the work is not the
//! daemon staying up but the handoff on the way down: every live session is marked
//! `InterruptedResumable` and persisted before anything is killed (G5), which is the same durable
//! situation as any other service restart.

use std::time::Duration;

use micold_core::clock::Uptime;

/// How long the service tolerates having nobody connected (FR-008, FR-017).
///
/// One value for both placements and all three platforms. Not configurable: a knob here would be a
/// knob on how long a machine keeps a service running for nobody, which is not a decision a user
/// has the information to make, and every value anyone would pick is this one.
pub const IDLE_WINDOW: Duration = Duration::from_secs(30 * 60);

/// How many clients are connected, and — when none are — since when (G1).
///
/// A count *and* its armed deadline in one value, updated only through the two transitions below,
/// because the failure this shape prevents is the two disagreeing: a separate count and a
/// separately-armed timer drift apart under a reconnect storm, and the drift is invisible until a
/// daemon stops with a window open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presence {
    connected: usize,
    alone_since: Option<Uptime>,
}

impl Presence {
    /// A daemon with nobody connected, idle **from now**.
    ///
    /// Not `alone_since: None`. A daemon spawned by a client that then died before completing a
    /// handshake would otherwise never arm its window and would live until the machine rebooted —
    /// and that client is exactly the one most likely to have died, since spawning the daemon is
    /// the first thing it does.
    pub fn new(now: Uptime) -> Self {
        Self {
            connected: 0,
            alone_since: Some(now),
        }
    }

    /// A client completed its handshake.
    pub fn client_connected(&mut self) {
        self.connected += 1;
        self.alone_since = None;
    }

    /// A client's connection ended, for any reason — clean close, EOF, or error (§2.5).
    pub fn client_disconnected(&mut self, now: Uptime) {
        // An unbalanced disconnect leaves the armed deadline where it is rather than re-arming it
        // to `now`. Re-arming would push a pending stop half an hour further out every time one
        // arrived — and the count is already zero, so nothing about the daemon's situation changed.
        if self.connected == 0 {
            return;
        }
        self.connected -= 1;
        if self.connected == 0 {
            self.alone_since = Some(now);
        }
    }

    /// How many are connected right now.
    pub fn connected(&self) -> usize {
        self.connected
    }

    /// When the count last fell to zero, or `None` while anyone is connected.
    pub fn alone_since(&self) -> Option<Uptime> {
        self.alone_since
    }
}

/// The idle rule (G2): a pure predicate over a [`Presence`] and a clock reading.
///
/// A type rather than a free function only so that the window can be shortened in a test. The rule
/// itself takes no configuration — see [`IDLE_WINDOW`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleWindow {
    window: Duration,
}

impl Default for IdleWindow {
    fn default() -> Self {
        Self::new(IDLE_WINDOW)
    }
}

impl IdleWindow {
    /// A rule with an explicit window. Production uses [`Self::default`]; tests use this, because
    /// the alternative is a suite that takes thirty minutes to say anything.
    pub const fn new(window: Duration) -> Self {
        Self { window }
    }

    /// Whether the service should stop now.
    ///
    /// Live sessions are not an argument, and that is the clarified rule rather than an omission —
    /// see the module docs.
    pub fn expired(&self, presence: &Presence, now: Uptime) -> bool {
        // Reading the count as well as the deadline, rather than trusting `alone_since` alone. The
        // invariant says they agree, and this is where believing that without checking would turn a
        // bug in `Presence` into a daemon that stops on a connected user.
        presence.connected == 0
            && presence
                .alone_since
                .is_some_and(|since| now.saturating_sub(since) >= self.window)
    }
}

/// Why the service is unwinding (G4).
///
/// The absence of a reason is itself a reason: a crash, an OOM kill or a `SIGKILL` produces no
/// variant and no line, which is what makes the line's *presence* evidence that the stop was
/// deliberate. Nothing external can forge one, because it is written before teardown begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The window in [`IdleWindow`] expired (FR-024).
    Idle,
    /// Something asked the service to stop — `spawn::stop_running_daemon`, or a signal it handles.
    Requested,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Readings are nanoseconds since boot; a test only needs them to be ordered.
    fn at(secs: u64) -> Uptime {
        Uptime::from_nanos(secs * 1_000_000_000)
    }

    // -----------------------------------------------------------------------------------------
    // G1 — Presence
    // -----------------------------------------------------------------------------------------

    /// G1's invariant, exercised over a sequence rather than asserted once: the armed deadline and
    /// the count must agree after *every* transition, which is where a separate timer would drift.
    #[test]
    fn the_armed_deadline_and_the_count_never_disagree() {
        let mut presence = Presence::new(at(0));
        let check = |p: &Presence| {
            assert_eq!(
                p.alone_since().is_some(),
                p.connected() == 0,
                "alone_since must be armed exactly when nobody is connected: {p:?}"
            );
        };
        check(&presence);
        for step in 1..20u64 {
            presence.client_connected();
            check(&presence);
            presence.client_connected();
            check(&presence);
            presence.client_disconnected(at(step));
            check(&presence);
            presence.client_disconnected(at(step));
            check(&presence);
        }
    }

    /// A reconnect disarms the window, rather than leaving a stale deadline that a later
    /// disconnect would inherit — the difference between "idle for 30 minutes" and "idle at some
    /// point in the last 30 minutes".
    #[test]
    fn a_reconnect_clears_the_armed_deadline() {
        let mut presence = Presence::new(at(0));
        presence.client_connected();
        presence.client_disconnected(at(100));
        assert_eq!(presence.alone_since(), Some(at(100)));

        presence.client_connected();
        assert_eq!(presence.alone_since(), None);

        // …and the *next* time it empties, the clock starts from then, not from 100.
        presence.client_disconnected(at(900));
        assert_eq!(presence.alone_since(), Some(at(900)));
    }

    /// G1's construction rule: a daemon that has never seen a client is idle from startup.
    ///
    /// This is the case that keeps a daemon spawned by a client that died mid-launch from becoming
    /// a permanent resident.
    #[test]
    fn a_daemon_that_never_had_a_client_is_idle_from_construction() {
        let presence = Presence::new(at(42));
        assert_eq!(presence.connected(), 0);
        assert_eq!(presence.alone_since(), Some(at(42)));
    }

    /// Only the *last* client leaving arms the window. Two connected and one leaving is not idle.
    #[test]
    fn the_window_arms_only_when_the_last_client_leaves() {
        let mut presence = Presence::new(at(0));
        presence.client_connected();
        presence.client_connected();
        presence.client_disconnected(at(10));
        assert_eq!(presence.connected(), 1);
        assert_eq!(presence.alone_since(), None);
    }

    // -----------------------------------------------------------------------------------------
    // G2 — IdleWindow
    // -----------------------------------------------------------------------------------------

    /// FR-008/FR-017: the constant is thirty minutes, and it is the same one everywhere.
    ///
    /// Asserted as a literal because it is a promise made in the documentation and in the spec's
    /// success criteria; a change to it is a change to what was published, not a tuning decision.
    #[test]
    fn the_window_is_thirty_minutes() {
        assert_eq!(IDLE_WINDOW, Duration::from_secs(1_800));
        assert_eq!(IdleWindow::default(), IdleWindow::new(IDLE_WINDOW));
    }

    /// Zero connections plus the full window ⇒ expired. Checked at the boundary in both directions,
    /// because "30 continuous minutes" is `>=`, and an off-by-one here is a service that stops a
    /// tick early or never.
    #[test]
    fn zero_connections_for_the_whole_window_expires() {
        let rule = IdleWindow::default();
        let presence = Presence::new(at(0));

        assert!(!rule.expired(&presence, at(0)));
        assert!(!rule.expired(&presence, at(1_799)));
        assert!(rule.expired(&presence, at(1_800)), "the window is inclusive");
        assert!(rule.expired(&presence, at(100_000)));
    }

    /// One connection ⇒ never, however long it has been there. This is the half that makes the
    /// service safe to leave open.
    #[test]
    fn a_connected_client_never_expires_the_window() {
        let rule = IdleWindow::default();
        let mut presence = Presence::new(at(0));
        presence.client_connected();

        for secs in [0, 1_800, 86_400, 86_400 * 30] {
            assert!(
                !rule.expired(&presence, at(secs)),
                "expired after {secs}s with a client connected"
            );
        }
    }

    /// FR-006a, the clarified rule, stated where it can be seen: the predicate has no session
    /// input at all. A live session cannot hold the service up because there is nowhere to say so.
    ///
    /// The old `may_exit(live_sessions, connected_clients)` took two counts; this takes one, and
    /// the test that would have proved a session holds the daemon up cannot be written.
    #[test]
    fn a_live_session_is_not_an_input_to_the_rule() {
        let rule = IdleWindow::default();
        let presence = Presence::new(at(0));
        // Whatever the daemon is running, this is the whole state the rule reads.
        assert!(rule.expired(&presence, at(1_800)));
    }

    /// The clock never runs backwards, but if a reading somehow did, the answer must be "not yet"
    /// rather than a panic or an instant stop (G3's saturating subtraction, seen from here).
    #[test]
    fn a_reading_before_the_deadline_does_not_expire_the_window() {
        let rule = IdleWindow::default();
        let presence = Presence::new(at(1_000));
        assert!(!rule.expired(&presence, at(0)));
    }

    /// A short window is honoured, or every integration test in this feature would take half an
    /// hour (T034).
    #[test]
    fn a_shortened_window_is_the_one_that_applies() {
        let rule = IdleWindow::new(Duration::from_millis(50));
        let presence = Presence::new(at(0));
        assert!(!rule.expired(&presence, Uptime::from_nanos(49_000_000)));
        assert!(rule.expired(&presence, Uptime::from_nanos(50_000_000)));
    }
}
