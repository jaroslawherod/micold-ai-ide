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
use crate::ui::icon;
use crate::ui::material::TypeRole;
use iced::widget::{container, Space};
use iced::{Element, Length};
use micold_core::protocol::messages::ActivitySignal;
use micold_core::tokens::Roles;
use std::marker::PhantomData;

/// The dot's diameter, and the width of the slot reserved for it whether or not one is drawn.
///
/// Deliberately the sidebar tag role's size: the badge sits in the same row as the tag chips and
/// has to line up with them optically, so it follows the role rather than restating a number that
/// would drift the moment the sidebar's density is re-valued (FR-011).
fn dot_size() -> f32 {
    TypeRole::SidebarTag.size()
}

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
    size: f32,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> ActivityBadge<'a, M> {
    /// A badge for `signal`, themed by `roles`.
    pub fn new(signal: ActivitySignal, roles: Roles) -> Self {
        Self {
            signal,
            roles,
            size: dot_size(),
            _marker: PhantomData,
        }
    }

    /// Override the dot size (defaults to the sidebar tag size).
    pub fn size(mut self, size: f32) -> Self {
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
        let inner: Element<'a, M> = match emphasis(&badge.signal) {
            Some(BadgeEmphasis::Working) => icon(Icon::ActivityWorking, badge.size, r.primary),
            Some(BadgeEmphasis::Attention) => icon(Icon::ActivityWorking, badge.size, r.error),
            Some(BadgeEmphasis::Ended) => {
                icon(Icon::ActivityEnded, badge.size, r.on_surface_variant)
            }
            // Unknown is ambient — nothing is drawn (H2). The slot is still reserved below.
            None => Space::new().into(),
        };

        // The slot is a fixed `size`-wide box in *every* state, drawn or not (FR-016f, SC-019).
        // Since BUG-005 removed the constant `check_circle` that used to anchor the row, this badge
        // is a session row's only leading element: a `Shrink` slot would let a hook-less session's
        // name sit left of its siblings, and would make a row shift horizontally as its signal
        // moved Unknown → Working → Ended. Centring keeps the glyph in the box if a future icon's
        // advance differs from the nominal 1em.
        container(inner)
            .center_x(Length::Fixed(badge.size))
            .center_y(Length::Shrink)
            .into()
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

    /// FR-016f / SC-019: the slot is constant-width in **every** state, including `Unknown` where
    /// nothing is drawn. Without this the badge is the row's only leading element (BUG-005 removed
    /// the icon that used to anchor it), so a `Shrink` slot would let a hook-less session's name sit
    /// left of its siblings and make a row shift horizontally as its signal changes.
    #[test]
    fn the_slot_is_constant_width_in_every_state_including_unknown() {
        let r = micold_core::tokens::roles(micold_core::theme::ColorScheme::Dark);
        for signal in [
            ActivitySignal::Unknown,
            ActivitySignal::Working,
            ActivitySignal::AwaitingInput,
            ActivitySignal::Ended {
                reason: "exit 0".into(),
            },
        ] {
            let element: Element<'_, ()> = ActivityBadge::new(signal.clone(), r).into();
            assert_eq!(
                element.as_widget().size().width,
                Length::Fixed(dot_size()),
                "the badge slot for {signal:?} is not the reserved width"
            );
        }
    }
}
