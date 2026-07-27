//! `ActivityBadge` — the per-session activity indicator (Constitution Principle VIII, feat 010 US2
//! FR-016d).
//!
//! A small status dot rendered beside a session's name in the sidebar, derived from the daemon's
//! [`ActivitySignal`]. It encodes invariant **H2**: `Working` is ambient, `AwaitingInput` and `Ended`
//! are notification-grade, and `Unknown` shows **nothing** — the app never renders a "needs you" cue
//! it cannot justify (FR-016c). The signal→emphasis decision is a pure function ([`emphasis`]) so it
//! is unit-testable independent of theming; the builder maps emphasis to a glyph + role colour.
//!
//! Exposed as a chainable builder terminating in `.into()` (Principle VIII builder-API rule).

use crate::icons::Icon;
use crate::tokens::{sidebar, Roles};
use crate::ui::icon;
use iced::widget::Space;
use iced::{Element, Length};
use micold_core::protocol::messages::ActivitySignal;
use std::marker::PhantomData;

/// The visual emphasis a signal deserves in the list. `None` from [`emphasis`] means "render nothing"
/// — the ambient `Unknown` state (H2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeEmphasis {
    /// Actively working — ambient positive activity.
    Working,
    /// Blocked awaiting the user — notification-grade (a strong hint, not a guarantee; H4).
    Attention,
    /// The session ended.
    Ended,
}

/// Map an [`ActivitySignal`] to its badge emphasis, or `None` when nothing should be shown.
///
/// `Unknown` deliberately yields `None`: absent hooks must never be dressed up as an attention cue
/// (H1/H2). This is the whole decision the badge makes, kept pure so it is tested without a renderer.
pub fn emphasis(signal: &ActivitySignal) -> Option<BadgeEmphasis> {
    match signal {
        ActivitySignal::Working => Some(BadgeEmphasis::Working),
        ActivitySignal::AwaitingInput => Some(BadgeEmphasis::Attention),
        ActivitySignal::Ended { .. } => Some(BadgeEmphasis::Ended),
        ActivitySignal::Unknown => None,
    }
}

/// A session activity dot. Construct with the signal + theme roles; it renders a filled dot for
/// `Working`/`AwaitingInput`, a hollow dot for `Ended`, and an empty (zero-width) placeholder for
/// `Unknown` so callers can render it uniformly for every session (FR-016d).
pub struct ActivityBadge<'a, M> {
    signal: ActivitySignal,
    roles: Roles,
    size: u16,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> ActivityBadge<'a, M> {
    /// A badge for `signal`, themed by `roles`.
    pub fn new(signal: ActivitySignal, roles: Roles) -> Self {
        Self {
            signal,
            roles,
            size: sidebar::TAG,
            _marker: PhantomData,
        }
    }

    /// Override the dot size (defaults to the sidebar tag size).
    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }
}

impl<'a, M: 'a> From<ActivityBadge<'a, M>> for Element<'a, M> {
    fn from(badge: ActivityBadge<'a, M>) -> Self {
        let r = badge.roles;
        // Drawn from the shared `Icon` vocabulary, never a raw literal (FR-016e): only glyphs in
        // `Icon::ALL` are proven present in the shipped font at build time, and only `icon(..)`
        // draws in that font — a plain `text(..)` uses the default text font, which maps neither
        // `●` nor `○`, so the badge rendered as tofu (BUG-004).
        //
        // Filled centre for live states, empty ring for a spent one: the states stay distinct by
        // *shape*, not by tint alone, so the distinction survives for a colour-blind user.
        let (glyph, color) = match emphasis(&badge.signal) {
            Some(BadgeEmphasis::Working) => (Icon::ActivityWorking, r.primary),
            Some(BadgeEmphasis::Attention) => (Icon::ActivityWorking, r.error),
            Some(BadgeEmphasis::Ended) => (Icon::ActivityEnded, r.on_surface_variant),
            // Unknown is ambient — nothing is drawn (H2), but the slot is still occupied so rows
            // stay uniform whether or not a session has a signal yet.
            None => return Space::with_width(Length::Shrink).into(),
        };
        icon(glyph, badge.size, color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_and_awaiting_input_are_shown_but_unknown_is_not() {
        assert_eq!(
            emphasis(&ActivitySignal::Working),
            Some(BadgeEmphasis::Working)
        );
        assert_eq!(
            emphasis(&ActivitySignal::AwaitingInput),
            Some(BadgeEmphasis::Attention)
        );
        assert_eq!(
            emphasis(&ActivitySignal::Ended {
                reason: "exit 0".into()
            }),
            Some(BadgeEmphasis::Ended)
        );
        // H1/H2: Unknown must never render an attention cue.
        assert_eq!(emphasis(&ActivitySignal::Unknown), None);
    }
}
