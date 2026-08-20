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
    /// A process that is **not running** and can be started again — feature 026's stopped mark
    /// (FR-012c). Its own variant rather than a reuse of [`Self::Attention`] or [`Self::Ended`],
    /// for the reason `for_emphasis` exists at all: `Attention` means "blocked awaiting the user"
    /// and already draws exactly this ring's opposite (a filled dot in the error role), so reusing
    /// it would put two different meanings behind one appearance and leave the gallery posing them
    /// as one. `Ended` is the right *shape* and the wrong role — muted, where FR-012c asks for
    /// error or warning, because a stopped process is a thing to act on rather than a thing that
    /// merely finished.
    Stopped,
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

/// A small status dot. Construct with an emphasis (or with a signal, which maps to one) plus the
/// theme roles; it renders a filled dot for `Working`/`Attention`, a hollow one for `Ended` and
/// `Stopped`, and an empty placeholder for no emphasis at all — so callers can render it uniformly
/// for every state (FR-016d). The slot is the same width in each case.
pub struct ActivityBadge<'a, M> {
    emphasis: Option<BadgeEmphasis>,
    roles: Roles,
    size: f32,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> ActivityBadge<'a, M> {
    /// A badge for `signal`, themed by `roles`. Sugar over [`Self::for_emphasis`] and the
    /// [`emphasis`] mapping, which is the only decision either form makes.
    pub fn new(signal: ActivitySignal, roles: Roles) -> Self {
        Self::for_emphasis(emphasis(&signal), roles)
    }

    /// A badge at a given emphasis, or — for `None` — a reserved slot with nothing in it.
    ///
    /// The constructor for a caller whose state is **not** daemon activity. Feature 026's stopped
    /// mark is a process lifecycle, and reaching this dot through a contrived [`ActivitySignal`]
    /// would put that lie somewhere it is read as truth: `tests/showcase_completeness.rs` poses
    /// variants by name, so the gallery would carry a session-activity heading over a terminal
    /// tab's mark. The emphasis is the thing both callers actually share; the signal is one way to
    /// arrive at it.
    pub fn for_emphasis(emphasis: Option<BadgeEmphasis>, roles: Roles) -> Self {
        Self {
            emphasis,
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
        let inner: Element<'a, M> = match badge.emphasis {
            Some(BadgeEmphasis::Working) => icon(Icon::ActivityWorking, badge.size, r.primary),
            Some(BadgeEmphasis::Attention) => icon(Icon::ActivityWorking, badge.size, r.error),
            Some(BadgeEmphasis::Ended) => {
                icon(Icon::ActivityEnded, badge.size, r.on_surface_variant)
            }
            // The spent ring in the error role: distinct from `Ended` by colour and from
            // `Attention` by shape, so the three stay separable without relying on tint alone —
            // the same colour-blind argument the two above are drawn apart by.
            Some(BadgeEmphasis::Stopped) => icon(Icon::ActivityEnded, badge.size, r.error),
            // No emphasis — `Unknown` from a signal, or a running process from feature 026's
            // mark — draws nothing (H2). The slot is still reserved below.
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

    /// The stopped mark (feature 026 FR-012c) needs this badge for a state that has no
    /// [`ActivitySignal`] at all: a process lifecycle is not daemon activity, and reaching the dot
    /// through a contrived signal would put that lie on the gallery page, where
    /// `showcase_completeness.rs` poses variants by name. So the emphasis becomes the input and the
    /// signal becomes sugar over it — the two must agree for every signal, or the sidebar has
    /// quietly changed while a second caller was being added.
    #[test]
    fn a_badge_is_built_from_an_emphasis_and_the_signal_form_is_sugar_over_it() {
        let r = micold_core::tokens::roles(micold_core::theme::ColorScheme::Dark);
        for signal in [
            ActivitySignal::Working,
            ActivitySignal::AwaitingInput,
            ActivitySignal::Ended {
                reason: "exit 0".into(),
            },
            ActivitySignal::Unknown,
        ] {
            let from_signal: ActivityBadge<'_, ()> = ActivityBadge::new(signal.clone(), r);
            let from_emphasis: ActivityBadge<'_, ()> =
                ActivityBadge::for_emphasis(emphasis(&signal), r);
            assert_eq!(
                from_signal.emphasis, from_emphasis.emphasis,
                "the signal form and the emphasis form disagree for {signal:?}"
            );
        }
    }

    /// The reserved-empty case, asked of the new constructor directly. The stopped mark is drawn in
    /// a tab's leading spacer, which every tab reserves whether or not a mark goes in it (feature
    /// 026 FR-012c, research R4): a slot that collapsed when empty would make a tab's children vary
    /// with its process's lifecycle, which is the positional-`diff_children` trap feature 023
    /// FR-008a exists for.
    #[test]
    fn an_emphasis_less_badge_reserves_its_slot_and_draws_nothing() {
        let r = micold_core::tokens::roles(micold_core::theme::ColorScheme::Dark);
        let element: Element<'_, ()> = ActivityBadge::for_emphasis(None, r).into();
        assert_eq!(
            element.as_widget().size().width,
            Length::Fixed(dot_size()),
            "an empty badge must still reserve the slot it would have drawn in"
        );
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
