//! The session-input ordering contract (protocol.md §7, data-model G2, task T039).
//!
//! **Input is a lossless, append-only, ordered log — never coalesced, dropped, or reordered,
//! including across a detach/reattach boundary.** Screen state is the opposite (lossy, convergent);
//! getting the two backwards loses user keystrokes. This module is the single source of truth for
//! that contract, shared by both ends so they cannot drift:
//!
//! - the client stamps every [`ClientMsg::SessionInput`](crate::protocol::messages::ClientMsg)
//!   with a monotonic per-session serial via [`InputSeq`];
//! - the daemon feeds each arriving serial through [`InputReceiver`], which decides — with no I/O and
//!   no policy of its own — whether the bytes are applied, reveal a loss, or are a stale duplicate.
//!
//! The serial's *only* purpose is to detect loss and reordering; it is never used to coalesce. A
//! single connection is ordered and lossless (TCP), so on the happy path every serial is exactly the
//! expected next one. The serial earns its keep across a **reconnect**: the client counter is never
//! reset, and the daemon's expectation is per-session (not per-connection), so a gap opened by a
//! reattach is caught loudly instead of silently swallowing the keystrokes typed just before a drop
//! (spec Edge: clock/ordering).
//!
//! **The two counters do not have the same lifetime, and the daemon's is authoritative** (FR-028a,
//! BUG-006). [`InputReceiver`] lives with the session, which by design outlives the UI; [`InputSeq`]
//! lives in the client process, which does not. Across a *reconnect* the client counter survives and
//! continuity is its own; across a **client restart** it is gone, and a counter rebuilt from `0`
//! would have every keystroke classified [`InputOutcome::Stale`] and silently dropped. A client that
//! has no counter for a session must therefore adopt the daemon's position — published as
//! `SessionSummary::input_serial` and resumed via [`InputSeq::resume_from`] — rather than assume its
//! own process lifetime bounds the session's.

/// The client's monotonic per-session input serial. Construct one per session and **never reset it**
/// — not on detach, not on reattach — so the daemon can prove across a reconnect that no keystroke
/// was lost (protocol.md §7).
#[derive(Debug, Clone, Default)]
pub struct InputSeq {
    next: u64,
}

impl InputSeq {
    /// A fresh counter starting at serial `0`. Correct only for a session this client process is
    /// itself starting; for one that already exists, use [`Self::resume_from`].
    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// A counter resumed at the daemon's expected next serial, for a session this client process did
    /// not start (FR-028a, BUG-006).
    ///
    /// `next` is the session's authoritative `SessionSummary::input_serial`. Resuming there — rather
    /// than at `0` — is what lets a restarted UI drive a surviving session from its first keystroke
    /// instead of having the daemon discard input as [`InputOutcome::Stale`]. Loss detection is
    /// unaffected: from here on the stream is dense and monotonic exactly as [`Self::new`]'s is, so a
    /// keystroke severed in flight is still reported as [`InputOutcome::Lost`].
    ///
    /// Only ever use this to *seed* a counter the client does not yet have. Overwriting a live
    /// counter would move it backwards past input still in flight and manufacture the duplicate the
    /// `Stale` rule exists to reject.
    pub fn resume_from(next: u64) -> Self {
        Self { next }
    }

    /// Take the next serial for an outgoing keystroke batch, advancing the counter. Serials are
    /// dense (0, 1, 2, …) so the receiver can detect a gap as loss.
    pub fn stamp(&mut self) -> u64 {
        let serial = self.next;
        self.next += 1;
        serial
    }

    /// The serial the next [`Self::stamp`] will return, without advancing (diagnostics/tests).
    pub fn peek(&self) -> u64 {
        self.next
    }
}

/// What the daemon must do with an arriving input serial. The receiver decides; the caller acts —
/// this type carries no bytes and does no I/O, so the ordering rule is testable in isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOutcome {
    /// The expected next serial: apply these bytes to the PTY. The log advanced by one.
    Apply,
    /// A serial **beyond** the expected next: `missing` serials were lost in transit before it
    /// (only possible across a reconnect — a single connection cannot lose input). This is a loud
    /// failure, never silently absorbed: the caller MUST surface the loss. The current bytes are
    /// still applied and the log resynced past them, because dropping *arrived* input would compound
    /// the loss.
    Lost {
        /// How many serials were skipped before this one.
        missing: u64,
    },
    /// A serial at or below what has already been applied — a duplicate or a reordering. Dropped and
    /// **never** applied: re-applying it would reorder/duplicate the append-only log (G2).
    Stale,
}

/// The daemon's per-session view of the input log. Tracks the high-water mark and classifies each
/// arriving serial against the contract. Lives with the session (not the connection), so it survives
/// a client detach/reattach and keeps verifying continuity across it.
#[derive(Debug, Clone, Default)]
pub struct InputReceiver {
    expected: u64,
}

impl InputReceiver {
    /// A receiver expecting the first serial, `0`.
    pub fn new() -> Self {
        Self { expected: 0 }
    }

    /// Classify `serial` against the expected next one, advancing the high-water mark for anything
    /// that is not stale. See [`InputOutcome`] for the three cases.
    pub fn accept(&mut self, serial: u64) -> InputOutcome {
        use std::cmp::Ordering;
        match serial.cmp(&self.expected) {
            Ordering::Equal => {
                self.expected += 1;
                InputOutcome::Apply
            }
            Ordering::Greater => {
                let missing = serial - self.expected;
                // Resync past the arrived serial so subsequent input is measured from here rather
                // than re-reporting the same gap on every following keystroke.
                self.expected = serial + 1;
                InputOutcome::Lost { missing }
            }
            Ordering::Less => InputOutcome::Stale,
        }
    }

    /// The serial the receiver next expects (equivalently, the count of serials it has accepted).
    pub fn expected(&self) -> u64 {
        self.expected
    }
}
