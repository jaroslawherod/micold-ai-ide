//! `StatusChip` — a Jira ticket's current status as a tinted chip (feature 1101,
//! contracts/status-chip-ui.md).
//!
//! Built on [`Tag`]: the tint says which group of statuses the ticket is in, the text always says
//! the status itself, so the group is never carried by colour alone. A stale status is dimmed and
//! its tooltip says so.

use super::{Tag, Tooltip};
use iced::Element;
use micold_core::jira_status::{StatusCategory, TicketStatus};
use micold_core::tokens::{over, Rgb, Roles};

/// The widest a chip is drawn, in dp: a longer name ends in an ellipsis.
pub const MAX_WIDTH: f32 = 120.0;

/// Names up to this many characters always fit [`MAX_WIDTH`], so they size themselves and the chip
/// is only as wide as its text; a longer one is fixed at [`MAX_WIDTH`] and truncated.
const SHORT_NAME: usize = 12;

/// How much of the tint a stale chip keeps; the rest is the surface behind it.
const STALE_STRENGTH: f64 = 0.55;

/// A status chip. Construct with the status and the roles, optionally `.stale(true)`, then `.into()`.
pub struct StatusChip {
    status: TicketStatus,
    stale: bool,
    roles: Roles,
}

impl StatusChip {
    /// A chip for `status`.
    pub fn new(status: &TicketStatus, roles: Roles) -> Self {
        Self {
            status: status.clone(),
            stale: false,
            roles,
        }
    }

    /// Mark the status as possibly out of date: the chip is dimmed and says so.
    pub fn stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }

    /// The chip for a status that may be absent: `None` draws nothing and adds no spacing (FR-009).
    pub fn maybe<'a, M: 'a>(
        status: Option<&TicketStatus>,
        stale: bool,
        roles: Roles,
    ) -> Option<Element<'a, M>> {
        status.map(|s| Self::new(s, roles).stale(stale).into())
    }

    /// The colour a status in `category` is tinted with: secondary for To Do, primary for In
    /// Progress, tertiary for Done, and a neutral for no category (FR-004).
    pub fn accent(category: Option<StatusCategory>, roles: Roles) -> Rgb {
        match category {
            Some(StatusCategory::ToDo) => roles.secondary,
            Some(StatusCategory::InProgress) => roles.primary,
            Some(StatusCategory::Done) => roles.tertiary,
            None => roles.on_surface_variant,
        }
    }

    /// The text a screen reader and the tooltip give (FR-013).
    pub fn accessible_text(status: &TicketStatus, stale: bool) -> String {
        status.accessible_text(stale)
    }
}

impl StatusChip {
    /// The colour the chip is drawn in: the category's tint, dimmed toward the surface when stale.
    fn tint(&self) -> Rgb {
        let accent = Self::accent(self.status.category, self.roles);
        if self.stale {
            over(accent, STALE_STRENGTH, self.roles.surface)
        } else {
            accent
        }
    }

    /// The tooltip (and screen-reader) text of the chip as built.
    fn label(&self) -> String {
        Self::accessible_text(&self.status, self.stale)
    }
}

impl<'a, M: 'a> From<StatusChip> for Element<'a, M> {
    fn from(chip: StatusChip) -> Self {
        let mut tag = Tag::new(chip.status.name.clone(), chip.tint());
        if chip.status.name.chars().count() > SHORT_NAME {
            tag = tag.truncate_to(MAX_WIDTH);
        }
        let label = chip.label();
        Tooltip::new(Element::from(tag), label, chip.roles).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::roles;

    fn status(category: Option<StatusCategory>) -> TicketStatus {
        TicketStatus {
            name: "In Review".into(),
            category,
        }
    }

    #[test]
    fn the_tint_differs_by_category() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = roles(scheme);
            let tints = [
                Some(StatusCategory::ToDo),
                Some(StatusCategory::InProgress),
                Some(StatusCategory::Done),
                None,
            ]
            .map(|c| StatusChip::accent(c, r));
            for (i, a) in tints.iter().enumerate() {
                for b in &tints[i + 1..] {
                    assert_ne!(a, b, "{scheme:?}: two categories share a tint");
                }
            }
        }
    }

    /// What the chip is built from, not the static helper: a stale chip is dimmed and says so,
    /// a fresh one is neither (FR-008, FR-013).
    #[test]
    fn a_stale_chip_is_dimmed_and_its_tooltip_says_so() {
        let r = roles(ColorScheme::Dark);
        let s = status(Some(StatusCategory::InProgress));
        let fresh = StatusChip::new(&s, r);
        let stale = StatusChip::new(&s, r).stale(true);
        assert_eq!(fresh.tint(), StatusChip::accent(s.category, r));
        assert_ne!(stale.tint(), fresh.tint(), "a stale chip is dimmed");
        assert_eq!(fresh.label(), "In Review");
        assert_eq!(stale.label(), "In Review, may be out of date");
        // And the element is built from them without panicking, stale or not.
        let _: Element<'_, ()> = stale.into();
    }

    #[test]
    fn no_status_draws_nothing() {
        let r = roles(ColorScheme::Dark);
        assert!(StatusChip::maybe::<()>(None, false, r).is_none());
        let s = status(None);
        assert!(StatusChip::maybe::<()>(Some(&s), false, r).is_some());
    }

    #[test]
    fn the_tooltip_is_the_accessible_text() {
        let s = status(Some(StatusCategory::InProgress));
        assert_eq!(StatusChip::accessible_text(&s, false), "In Review");
        assert_eq!(
            StatusChip::accessible_text(&s, true),
            "In Review, may be out of date"
        );
    }
}
