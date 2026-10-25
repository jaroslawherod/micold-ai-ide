//! `CountBadge` — a count on a filled pill (Constitution Principle VIII; feature 1269, R11).
//!
//! A number of things waiting — the drafts of a review — drawn as a small opaque pill in the
//! `primary` role with its text in `on_primary`, capped at [`CAP`]: `1`…`99`, then `99+`. A count is
//! never zero: the constructor takes a [`NonZeroUsize`], so a host with nothing to count draws no
//! badge rather than a `0`.
//!
//! It is not the [`UnreadMark`](super::UnreadMark), which marks "not looked at since" with a dot,
//! nor the client's `ActivityBadge` of a session's activity. The section list's badge chip is the same
//! pill around a word, so both are drawn by [`chip`] and cannot drift apart.
//!
//! Exposed as a chainable builder terminating in `.into()`:
//! `CountBadge::new(count, roles).tooltip("2 drafts").into()`.

use crate::tag::Tag;
use crate::{Tooltip, TooltipPosition, TypeRole};
use iced::Element;
use micold_core::tokens::{Rgb, Roles};
use std::num::NonZeroUsize;

/// The largest count spelled out; above it the badge reads `99+`.
pub const CAP: usize = 99;

/// The type role the pill's text is drawn in: the section list's badge role, so the two read alike.
pub const ROLE: TypeRole = TypeRole::Caption;

/// The filled pill both the count badge and the section list's badge are drawn as: `label` in
/// `on_fill` on an opaque `fill`.
pub fn chip<'a, M: 'a>(label: impl Into<String>, fill: Rgb, on_fill: Rgb) -> Tag<'a, M> {
    Tag::new(label, fill).solid(on_fill).role(ROLE)
}

/// The text a count is drawn as: `1`…[`CAP`], then `99+`.
pub fn text(count: NonZeroUsize) -> String {
    if count.get() > CAP {
        format!("{CAP}+")
    } else {
        count.to_string()
    }
}

/// A count on a filled pill. See the module documentation.
pub struct CountBadge {
    count: NonZeroUsize,
    roles: Roles,
    tooltip: Option<String>,
}

impl CountBadge {
    /// A badge showing `count`, themed by `roles`.
    pub fn new(count: NonZeroUsize, roles: Roles) -> Self {
        Self {
            count,
            roles,
            tooltip: None,
        }
    }

    /// Say on hover what is counted (`2 drafts`).
    pub fn tooltip(mut self, label: impl Into<String>) -> Self {
        self.tooltip = Some(label.into());
        self
    }

    /// The text drawn on the pill.
    pub fn label(&self) -> String {
        text(self.count)
    }
}

impl<'a, M: 'a> From<CountBadge> for Element<'a, M> {
    fn from(badge: CountBadge) -> Self {
        let r = badge.roles;
        let pill: Element<'a, M> = chip::<M>(badge.label(), r.primary, r.on_primary).into();
        match badge.tooltip {
            Some(label) => Tooltip::new(pill, label, r)
                .position(TooltipPosition::Bottom)
                .into(),
            None => pill,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, roles};
    use super::*;
    use iced::Size;

    fn count(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).expect("a count of one or more")
    }

    fn size_of(element: Element<'_, ()>) -> Size {
        super::super::test_support::size_of(element, Size::new(400.0, 100.0))
    }

    /// Feature 1269 T032 (R11, FR-009): `1`…`99` spelled out, then `99+`.
    #[test]
    fn the_badge_spells_a_count_up_to_99_then_99_plus() {
        let label = |n| CountBadge::new(count(n), roles()).label();
        assert_eq!(label(1), "1");
        assert_eq!(label(99), "99");
        assert_eq!(label(100), "99+");
        assert_eq!(label(12_345), "99+", "capped, however many");
    }

    /// T032: the pill grows with its text, never past `99+`'s width, and a tooltip changes no
    /// size.
    #[test]
    fn the_pill_is_widest_at_the_cap_and_a_tooltip_changes_no_size() {
        for r in [roles(), dark_roles()] {
            let one = size_of(CountBadge::new(count(1), r).into());
            let cap = size_of(CountBadge::new(count(100), r).into());
            let more = size_of(CountBadge::new(count(5_000), r).into());
            assert!(
                one.width > 0.0 && one.height > 0.0,
                "the pill is drawn: {one:?}"
            );
            assert!(one.width < cap.width, "{one:?} against {cap:?}");
            assert_eq!(cap, more, "every count above the cap draws `99+`");
            assert_eq!(
                size_of(CountBadge::new(count(1), r).tooltip("1 draft").into()),
                one
            );
        }
    }

    /// T032 (R11): the count badge and the section list's badge chip are one pill — the same
    /// component around a number or a word, at the same height.
    #[test]
    fn the_badge_is_the_section_lists_chip_around_a_number() {
        let r = roles();
        let badge = size_of(CountBadge::new(count(3), r).into());
        let chip = size_of(chip::<()>("3", r.primary, r.on_primary).into());
        assert_eq!(badge, chip);
    }
}
