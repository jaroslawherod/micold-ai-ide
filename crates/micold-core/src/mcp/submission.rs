//! Typing a submission into a session, and knowing when a new one is ready for it (feature 034,
//! FR-017, research R12).
//!
//! The service writes a submission straight to the session's primary terminal: the text, wrapped
//! in bracketed-paste markers when the terminal has asked for bracketed paste (so a multi-line
//! prompt is one paste rather than a submission per line), then one carriage return.
//!
//! A CLI that reports no ready-for-input signal of its own is ready once its terminal has produced
//! output and then none for [`SETTLE_AFTER`]. That evidence decides readiness only; it never moves
//! the activity badge.

use std::time::Duration;

use crate::clock::Uptime;

/// How long a terminal must stay quiet after output to count as settled.
pub const SETTLE_AFTER: Duration = Duration::from_millis(1500);

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

/// The bytes that type `text` and submit it once. Trailing line breaks are dropped, so the single
/// closing carriage return is the only submission.
pub fn encode_submission(text: &str, bracketed: bool) -> Vec<u8> {
    let body = text.trim_end_matches(['\r', '\n']).as_bytes();
    let mut out = Vec::with_capacity(body.len() + PASTE_START.len() + PASTE_END.len() + 1);
    if bracketed {
        out.extend_from_slice(PASTE_START);
        out.extend_from_slice(body);
        out.extend_from_slice(PASTE_END);
    } else {
        out.extend_from_slice(body);
    }
    out.push(b'\r');
    out
}

/// The output-settled rule, fed with readings of a clock the caller owns.
#[derive(Debug, Clone, Default)]
pub struct OutputSettled {
    last_output: Option<Uptime>,
}

impl OutputSettled {
    pub fn new() -> Self {
        Self::default()
    }

    /// The terminal produced output at `at`.
    pub fn output(&mut self, at: Uptime) {
        self.last_output = Some(at);
    }

    /// Whether the terminal has produced output and then none for [`SETTLE_AFTER`] by `now`.
    pub fn is_ready(&self, now: Uptime) -> bool {
        self.last_output
            .is_some_and(|last| now.saturating_sub(last) >= SETTLE_AFTER)
    }
}
