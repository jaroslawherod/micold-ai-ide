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

use std::collections::HashMap;

use micold_core::input::InputSeq;
use micold_core::protocol::messages::ClientMsg;
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
