//! The assistant message being streamed (research R3).
//!
//! Milestone M1 models finished events only: a partial-message event is absorbed here without an
//! entry. M2 replaces this with the open stream, its streamed blocks and their supersede.

use crate::structured::StructuredEvent;

/// Takes one top-level `StreamDelta`. Absorbed: no entry, no change.
pub(super) fn absorb(_delta: &StructuredEvent) {}
