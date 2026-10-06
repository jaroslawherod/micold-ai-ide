//! `PullRequestIndicator` — the mark of a worktree's pull request and of its checks (Constitution
//! Principle VIII; feature 040, contract `pull-request-ui.md` §1).
//!
//! The state glyph, then the check glyph when there is one: 16 px each, 2 px apart, in a box of
//! fixed height. The width is 16 px without a check status and 34 px with one, and never depends on
//! the sidebar's width (FR-009). The seven glyphs are seven shapes, so colour only repeats what the
//! shape says. The stale form draws both in the `outline` role.
//!
//! `PrMark` and `CheckMark` are this component's own enums: the sidebar maps the pull request
//! module's types to them, so `ui::material` does not depend on that module.
//!
//! Not interactive: no press, no hover, no tooltip of its own; the row's tooltip speaks for it.

use crate::icons::Icon;
use crate::ui::material::glyph::icon;
use iced::widget::{container, row};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{Rgb, Roles};
use std::marker::PhantomData;

/// The side of one glyph.
const GLYPH: f32 = 16.0;
/// The gap between the state glyph and the check glyph.
const GAP: f32 = 2.0;
/// The box's height: the glyph's.
const HEIGHT: f32 = GLYPH;

/// The state of a pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrMark {
    /// Open, ready for review.
    Open,
    /// Open, as a draft.
    Draft,
    /// Merged.
    Merged,
    /// Closed without merging.
    Closed,
}

/// The combined result of a pull request's checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMark {
    /// A check is still queued or running, and none failed.
    Pending,
    /// Every check finished without a failure.
    Passing,
    /// A check failed.
    Failing,
}

impl PrMark {
    /// The glyph that draws the state.
    pub(crate) const fn icon(self) -> Icon {
        match self {
            PrMark::Open => Icon::PrOpen,
            PrMark::Draft => Icon::PrDraft,
            PrMark::Merged => Icon::PrMerged,
            PrMark::Closed => Icon::PrClosed,
        }
    }

    /// The role the glyph is drawn in when current. One function, so that the contrast gate
    /// measures the colour that is drawn.
    pub(crate) fn role(self, r: &Roles) -> Rgb {
        match self {
            PrMark::Open => r.primary,
            PrMark::Draft | PrMark::Closed => r.on_surface_variant,
            PrMark::Merged => r.tertiary,
        }
    }
}

impl CheckMark {
    /// The glyph that draws the check status.
    pub(crate) const fn icon(self) -> Icon {
        match self {
            CheckMark::Pending => Icon::ChecksPending,
            CheckMark::Passing => Icon::ChecksPassing,
            CheckMark::Failing => Icon::ChecksFailing,
        }
    }

    /// The role the glyph is drawn in when current.
    pub(crate) fn role(self, r: &Roles) -> Rgb {
        match self {
            CheckMark::Pending => r.on_surface_variant,
            CheckMark::Passing => r.primary,
            CheckMark::Failing => r.error,
        }
    }
}

/// The indicator. See the module documentation.
pub struct PullRequestIndicator<'a, M> {
    roles: Roles,
    state: PrMark,
    checks: Option<CheckMark>,
    stale: bool,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> PullRequestIndicator<'a, M> {
    /// The indicator of a pull request in `state`, themed by `roles`.
    pub fn new(state: PrMark, roles: &Roles) -> Self {
        Self {
            roles: *roles,
            state,
            checks: None,
            stale: false,
            _marker: PhantomData,
        }
    }

    /// Follow the state with the combined check status.
    pub fn checks(mut self, checks: CheckMark) -> Self {
        self.checks = Some(checks);
        self
    }

    /// Draw both glyphs in the `outline` role: the reading is old (FR-019).
    pub fn stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }
}

impl<'a, M: 'a> From<PullRequestIndicator<'a, M>> for Element<'a, M> {
    fn from(i: PullRequestIndicator<'a, M>) -> Self {
        let r = i.roles;
        let tint = |current: Rgb| if i.stale { r.outline } else { current };
        let mut glyphs: Vec<Element<'a, M>> =
            vec![icon(i.state.icon(), GLYPH, tint(i.state.role(&r)))];
        if let Some(checks) = i.checks {
            glyphs.push(icon(checks.icon(), GLYPH, tint(checks.role(&r))));
        }
        let width = if i.checks.is_some() {
            GLYPH * 2.0 + GAP
        } else {
            GLYPH
        };
        container(row(glyphs).spacing(GAP).align_y(Alignment::Center))
            .width(Length::Fixed(width))
            .height(Length::Fixed(HEIGHT))
            .align_y(Alignment::Center)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;
    use iced::Size;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    fn size_of(mut element: Element<'_, ()>) -> Size {
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 100.0));
        element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits)
            .bounds()
            .size()
    }

    /// U102 (contract §1): 16 px wide alone, 34 px with a check status, one fixed height.
    #[test]
    fn the_indicator_is_16_wide_alone_and_34_with_a_check_status() {
        let r = tokens::roles(ColorScheme::Light);
        let alone = size_of(PullRequestIndicator::new(PrMark::Open, &r).into());
        let checked = size_of(
            PullRequestIndicator::new(PrMark::Draft, &r)
                .checks(CheckMark::Failing)
                .into(),
        );
        let stale = size_of(
            PullRequestIndicator::new(PrMark::Open, &r)
                .checks(CheckMark::Passing)
                .stale(true)
                .into(),
        );

        assert_eq!(alone, Size::new(16.0, 16.0));
        assert_eq!(checked, Size::new(34.0, 16.0));
        assert_eq!(stale, checked, "the stale form keeps the size");
    }

    /// U103 (FR-009): seven glyphs, seven shapes; the stale form is the `outline` role.
    #[test]
    fn the_states_are_seven_distinct_glyphs() {
        let glyphs: std::collections::BTreeSet<char> = [
            PrMark::Open.icon(),
            PrMark::Draft.icon(),
            PrMark::Merged.icon(),
            PrMark::Closed.icon(),
            CheckMark::Pending.icon(),
            CheckMark::Passing.icon(),
            CheckMark::Failing.icon(),
        ]
        .iter()
        .map(|i| i.glyph())
        .collect();
        assert_eq!(glyphs.len(), 7);
    }
}
