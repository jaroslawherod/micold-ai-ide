//! Daemon lifecycle rule (data-model §G4, FR-002, task T023).
//!
//! **The daemon never exits while any session is alive**, regardless of connected clients. Exit is
//! permitted only when live sessions **and** connected clients are *both* zero. This is what makes
//! "close the UI, the work keeps running" true — killing the client changes no session's fate.

use std::sync::atomic::{AtomicUsize, Ordering};

/// The pure exit predicate (G4): exit is permitted only at zero sessions **and** zero clients.
pub fn may_exit(live_sessions: usize, connected_clients: usize) -> bool {
    live_sessions == 0 && connected_clients == 0
}

/// Runtime counters backing [`may_exit`]. Live sessions are process-backed (updated by the
/// supervisor, T031); connected clients are updated by the accept loop on connect/disconnect.
#[derive(Debug, Default)]
pub struct Lifecycle {
    live_sessions: AtomicUsize,
    connected_clients: AtomicUsize,
}

impl Lifecycle {
    /// A fresh tracker at zero/zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a client connection.
    pub fn client_connected(&self) {
        self.connected_clients.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a client disconnection (for any reason, including crash — T2).
    pub fn client_disconnected(&self) {
        self.connected_clients.fetch_sub(1, Ordering::SeqCst);
    }

    /// Record a session becoming process-alive.
    pub fn session_started(&self) {
        self.live_sessions.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a session's process ending.
    pub fn session_ended(&self) {
        self.live_sessions.fetch_sub(1, Ordering::SeqCst);
    }

    /// `(live_sessions, connected_clients)`.
    pub fn counts(&self) -> (usize, usize) {
        (
            self.live_sessions.load(Ordering::SeqCst),
            self.connected_clients.load(Ordering::SeqCst),
        )
    }

    /// Whether the daemon may exit right now (G4).
    pub fn may_exit(&self) -> bool {
        let (sessions, clients) = self.counts();
        may_exit(sessions, clients)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exits_only_at_zero_sessions_and_zero_clients() {
        assert!(may_exit(0, 0));
        assert!(!may_exit(1, 0), "a live session must keep the daemon up");
        assert!(
            !may_exit(0, 1),
            "a connected client must keep the daemon up"
        );
        assert!(!may_exit(3, 2));
    }

    #[test]
    fn a_live_session_keeps_the_daemon_up_with_no_clients() {
        let lc = Lifecycle::new();
        lc.session_started();
        lc.client_connected();
        lc.client_disconnected();
        // Client gone, session still alive → must NOT exit (FR-002, the whole point).
        assert_eq!(lc.counts(), (1, 0));
        assert!(!lc.may_exit());
        lc.session_ended();
        assert!(lc.may_exit());
    }
}
