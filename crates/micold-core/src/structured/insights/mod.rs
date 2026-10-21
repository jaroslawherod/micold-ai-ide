//! Session insights (feature 944): the context reading, each turn's usage and the session totals
//! of one structured session, derived from #938's `StructuredEvents` messages.
//!
//! Render-free (research R1): it takes the same messages as #939's
//! [`Transcript`](super::transcript::Transcript), under the same replace-or-continue rule and the
//! same [`MAX_EVENTS`] bound, and keeps a digest per event that matters rather than the events
//! (research R8). Rules: `specs/944-session-usage-subagents/contracts/insights-model.md`.

mod context;
mod digest;
pub mod figures;
mod subagents;
mod tasks;
mod usage;

pub use context::{ContextReading, ContextState};
pub use usage::{ModelFigures, SessionTotals, Tokens, TurnUsage};

use super::sync::Continuity;
use super::{SequencedEvent, MAX_EVENTS};
use context::ContextFold;
use digest::{digest, Digest};
use std::collections::VecDeque;
use usage::UsageState;

/// One structured session's insights, as one window holds them (data-model.md
/// *SessionInsights*).
///
/// Equality compares what the session's events decide, not the receiver's bookkeeping: a value
/// built message by message equals one built from the held events in one history message with the
/// same `omitted` (guarantee I1, FR-023).
#[derive(Debug, Clone)]
pub struct SessionInsights {
    sync: Continuity,
    /// Events before the first held one: the service's `omitted` plus those dropped here.
    omitted: u64,
    /// How many events are held (at most [`MAX_EVENTS`]).
    held: usize,
    /// The first held `seq`; seqs are contiguous (#938 wire).
    first_held: u64,
    /// The held events that matter here, by `seq`, oldest first.
    digests: VecDeque<(u64, Digest)>,
    usage: UsageState,
    context: ContextFold,
}

impl PartialEq for SessionInsights {
    fn eq(&self, other: &Self) -> bool {
        self.omitted == other.omitted
            && self.held == other.held
            && (self.held == 0 || self.first_held == other.first_held)
            && self.digests == other.digests
            && self.usage == other.usage
            && self.context == other.context
    }
}

impl Default for SessionInsights {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionInsights {
    pub fn new() -> Self {
        Self {
            sync: Continuity::default(),
            omitted: 0,
            held: 0,
            first_held: 0,
            digests: VecDeque::new(),
            usage: UsageState::default(),
            context: ContextFold::default(),
        }
    }

    /// Takes one `StructuredEvents` message (#938 wire.md), as `Transcript::apply` does: a
    /// `live: false` message replaces what is held unless it continues it; a `live: true` one
    /// appends. Past [`MAX_EVENTS`] the oldest events leave and the figures that depended on them
    /// are folded again, at most once per message (research R8).
    pub fn apply(&mut self, omitted: u64, events: &[SequencedEvent], live: bool) {
        if self.sync.take(omitted, events, live) {
            *self = Self {
                sync: self.sync.clone(),
                omitted,
                ..Self::new()
            };
        }
        let mut refold = false;
        for event in events {
            if self.held == 0 {
                self.first_held = event.seq;
            }
            self.held += 1;
            if let Some(digest) = digest(&event.event) {
                self.fold(event.seq, &digest);
                self.digests.push_back((event.seq, digest));
            }
            refold |= self.bound();
        }
        if refold {
            self.refold();
        }
        self.usage.totals.partial = self.omitted > 0;
    }

    /// Drops the events past [`MAX_EVENTS`]; whether the figures must be folded again.
    fn bound(&mut self) -> bool {
        let Some(excess) = self.held.checked_sub(MAX_EVENTS).filter(|n| *n > 0) else {
            return false;
        };
        let was_complete = self.omitted == 0;
        self.held = MAX_EVENTS;
        self.omitted += excess as u64;
        self.first_held += excess as u64;
        let before = self.digests.len();
        while self
            .digests
            .front()
            .is_some_and(|(seq, _)| *seq < self.first_held)
        {
            self.digests.pop_front();
        }
        // FR-009 reads `omitted` for the first held result, so the first drop refolds too.
        was_complete || self.digests.len() != before
    }

    /// The derived figures folded again from the digests still held.
    fn refold(&mut self) {
        self.usage = UsageState::default();
        self.context = ContextFold::default();
        let digests = std::mem::take(&mut self.digests);
        for (seq, digest) in &digests {
            self.fold(*seq, digest);
        }
        self.digests = digests;
    }

    fn fold(&mut self, seq: u64, digest: &Digest) {
        match digest {
            Digest::AssistantUsage { model, used } => {
                self.context.response(model.as_deref(), *used)
            }
            Digest::Result {
                tokens,
                cost_cum,
                models_cum,
                windows,
            } => {
                self.usage
                    .result(seq, *tokens, *cost_cum, models_cum.as_ref(), self.omitted);
                self.context.windows(windows);
            }
            Digest::TurnEndedWithoutResult => self.usage.ended_without_result(),
            Digest::Autocompact { threshold } => self.context.threshold(*threshold),
        }
    }

    /// The context reading (FR-001–FR-005).
    pub fn context(&self) -> &ContextReading {
        &self.context.reading
    }

    /// The usage of the turn whose result is the event `result_seq`, while it is held.
    pub fn turn(&self, result_seq: u64) -> Option<&TurnUsage> {
        self.usage.turn(result_seq)
    }

    /// The session totals over the counted turns held (FR-007, FR-008).
    pub fn totals(&self) -> &SessionTotals {
        &self.usage.totals
    }

    /// Events before the first held one.
    pub fn omitted(&self) -> u64 {
        self.omitted
    }

    /// How many events are held.
    pub fn held(&self) -> usize {
        self.held
    }
}
