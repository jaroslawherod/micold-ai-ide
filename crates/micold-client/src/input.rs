//! Client-side session input: turn key-encoded VT bytes into ordered `SessionInput` wire messages,
//! one monotonic serial stream per session (protocol.md §7, data-model G2, task T044).
//!
//! This is the client end of the input contract whose daemon end is `micold_daemon`'s
//! `InputReceiver`; both are stamped/verified against the same `micold_core::input` primitive so
//! they cannot drift. The flow is: `keymap::encode` produces [`KeyOutput::Bytes`], the client hands
//! those bytes to [`SessionInputStamper::stamp`], and the resulting [`ClientMsg::SessionInput`] is
//! sent to the daemon, which writes them to the PTY in order.
//!
//! The stamper lives in the client's long-lived runtime state — **not** in any per-connection
//! object — so a session's serial counter is never reset by a daemon detach/reattach. That is the
//! whole point of the serial: continuity across a reconnect is provable, and a keystroke lost to a
//! drop is detected rather than silently swallowed.
//!
//! "Long-lived" means *this process*, though, and the daemon is designed to outlive it (FR-002).
//! A restarted UI therefore has no counter at all for a session that predates it, and starting one
//! at `0` would put it behind the daemon's per-session expectation — which discards every keystroke
//! as stale, silently (BUG-006). So the daemon's position is authoritative and travels in the
//! catalog snapshot as `SessionSummary::input_serial`; [`SessionInputStamper::seed`] adopts it on
//! connect, for sessions this client has not driven itself.

use std::collections::HashMap;

use micold_core::input::InputSeq;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg};
use micold_core::session::SessionId;

/// Per-session monotonic input stamping for the client. Holds one [`InputSeq`] per session and
/// mints the next [`ClientMsg::SessionInput`] for a batch of encoded bytes.
#[derive(Debug, Default)]
pub struct SessionInputStamper {
    seqs: HashMap<SessionId, InputSeq>,
}

impl SessionInputStamper {
    /// An empty stamper (no sessions seen yet).
    pub fn new() -> Self {
        Self::default()
    }

    /// Adopt the daemon's expected next serial for `session` — **only if this client has no counter
    /// for it yet** (FR-028a, BUG-006).
    ///
    /// Call this for every session in an authoritative catalog snapshot, on connect and on each
    /// later push. Seeding is what lets a freshly started client drive a session it did not create:
    /// without it the counter starts at `0`, behind a daemon already expecting `N`, and every
    /// keystroke is dropped as stale until the client has burned through `N` batches.
    ///
    /// The absent-only rule is load-bearing, not an optimisation. A counter this client already
    /// holds is *ahead* of the snapshot by whatever input is still in flight, so overwriting it
    /// would rewind the stream and re-mint serials the daemon has already applied — manufacturing
    /// exactly the duplicate that `Stale` exists to reject.
    pub fn seed(&mut self, session: SessionId, serial: u64) {
        self.seqs
            .entry(session)
            .or_insert_with(|| InputSeq::resume_from(serial));
    }

    /// Seed every session in an authoritative catalog snapshot (FR-028a, T111).
    ///
    /// The bulk form of [`Self::seed`], and the one the client actually calls — on connect and on
    /// every later catalog push. Seed-only, never pruning: a snapshot that predates an in-flight
    /// local mutation, or an ephemeral daemon reporting an empty catalog, is not evidence that a
    /// session ended, and dropping a counter on that evidence would rebuild it at `0` on the next
    /// keystroke — the very bug this seeding exists to prevent. Counters are released explicitly
    /// instead, by [`Self::forget`].
    pub fn seed_from_catalog(&mut self, catalog: &CatalogSnapshot) {
        for project in &catalog.projects {
            for session in &project.sessions {
                self.seed(session.id, session.input_serial);
            }
        }
    }

    /// Stamp `bytes` as the next input for `session`, advancing that session's serial. The returned
    /// message is ready to send over the daemon connection. Each session's serials are independent
    /// and dense (0, 1, 2, …) so the daemon can detect loss as a gap.
    pub fn stamp(&mut self, session: SessionId, bytes: Vec<u8>) -> ClientMsg {
        let serial = self.seqs.entry(session).or_default().stamp();
        ClientMsg::SessionInput {
            session,
            serial,
            bytes,
        }
    }

    /// Drop a session's counter once the session has ended (hygiene; ids are unique UUIDs so a
    /// counter is never reused). Never call this on a mere detach — the counter must survive a
    /// reconnect for the loss-detection contract to hold.
    pub fn forget(&mut self, session: SessionId) {
        self.seqs.remove(&session);
    }
}
