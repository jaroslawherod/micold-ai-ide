//! `PullRequestRow`: one pull request of the Changes pane's list (feature 775, FR-003).
//!
//! The state mark and the check mark (the sidebar's [`PullRequestIndicator`]), `#number`, and the
//! title on one fixed-height line; `selected` outlines it, `stale` draws the mark and the text in
//! their quieter form; a title cut with "…" reads whole in its tooltip. The row is informational
//! until a press is wired (M2).

use crate::ui::material::{
    CheckMark, Ellipsized, PrMark, PullRequestIndicator, Text, Tooltip, TypeRole,
};
use iced::widget::{container, row};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{spacing, Roles};
use std::borrow::Cow;
use std::marker::PhantomData;

/// The row's one height, the same as the Changes view's file rows.
pub const HEIGHT: f32 = 36.0;

/// The row. See the module documentation.
pub struct PullRequestRow<'a, M> {
    roles: Roles,
    number: u64,
    title: Cow<'a, str>,
    state: PrMark,
    checks: Option<CheckMark>,
    selected: bool,
    stale: bool,
    _marker: PhantomData<M>,
}

impl<'a, M: 'a> PullRequestRow<'a, M> {
    /// The row of pull request `number` titled `title`, in `state`, themed by `roles`.
    pub fn new(number: u64, title: impl Into<Cow<'a, str>>, state: PrMark, roles: &Roles) -> Self {
        Self {
            roles: *roles,
            number,
            title: title.into(),
            state,
            checks: None,
            selected: false,
            stale: false,
            _marker: PhantomData,
        }
    }

    /// Follow the state mark with the check mark (feature 1003, FR-003). No call, no check mark.
    pub fn checks(mut self, checks: CheckMark) -> Self {
        self.checks = Some(checks);
        self
    }

    /// Draw the row as the one shown in the diff.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Draw the row in its lower-emphasis form: the reading is old (FR-012).
    pub fn stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }
}

impl<'a, M: 'a> From<PullRequestRow<'a, M>> for Element<'a, M> {
    fn from(r: PullRequestRow<'a, M>) -> Self {
        let roles = r.roles;
        let text = |content: Cow<'a, str>| {
            let text = Text::new(content, TypeRole::Body, roles);
            if r.stale {
                text.muted()
            } else {
                text
            }
        };
        let mut mark = PullRequestIndicator::<M>::new(r.state, &roles).stale(r.stale);
        if let Some(checks) = r.checks {
            mark = mark.checks(checks);
        }
        let title = Ellipsized::<M>::at_role_spaced(
            r.title.into_owned(),
            TypeRole::Body,
            roles.on_surface_variant,
        );
        let title = if r.stale {
            title
        } else {
            title.inherit_color()
        };
        let line = row![
            Element::from(mark),
            text(Cow::Owned(format!("#{}", r.number))),
            // One line ending in "…": wrapped, a long title's second line was cut by the fixed
            // height (#1276 visual pass, check 02). Cut, it reads whole on hover.
            container(Tooltip::whole_label(title, roles)).width(Length::Fill),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
        let mut body = container(line)
            .width(Length::Fill)
            .height(Length::Fixed(HEIGHT))
            .padding([0.0, spacing::SM])
            .align_y(Alignment::Center);
        if r.selected {
            body = body.style(move |_| container::Style {
                border: iced::Border {
                    color: super::style::color(roles.outline),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..container::Style::default()
            });
        }
        body.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    fn size_of(element: Element<'_, ()>) -> Size {
        super::super::test_support::size_of(element, Size::new(400.0, 100.0))
    }

    /// FR-003: one fixed height whatever the state, selection, staleness or title length.
    #[test]
    fn the_row_has_one_height_in_every_state() {
        let r = tokens::roles(ColorScheme::Light);
        let long = "a very long pull request title ".repeat(8);
        let heights: Vec<f32> = [
            PullRequestRow::<()>::new(1, "short", PrMark::Open, &r).into(),
            PullRequestRow::<()>::new(2, "short", PrMark::Draft, &r)
                .selected(true)
                .into(),
            PullRequestRow::<()>::new(3, long.as_str(), PrMark::Merged, &r)
                .stale(true)
                .into(),
            PullRequestRow::<()>::new(4, "short", PrMark::Closed, &r).into(),
        ]
        .into_iter()
        .map(|e: Element<'_, ()>| size_of(e).height)
        .collect();
        assert!(heights.iter().all(|h| *h == HEIGHT), "{heights:?}");
    }

    /// #1276 visual pass (check 02): a title longer than the row is one line ending in "…",
    /// inside the row, never a second line cut off by the row's fixed height.
    #[test]
    fn a_long_title_is_one_ellipsized_line_inside_the_row() {
        let r = tokens::roles(ColorScheme::Light);
        let long = "Scheduler priorities: let heavy jobs jump the queue, keep FIFO among equal \
                    weights, and document the cutoff for operators running mixed pools";
        for stale in [false, true] {
            let row = PullRequestRow::<String>::new(57, long, PrMark::Open, &r).stale(stale);
            let painted = super::super::test_support::Mounted::new_in(row, Size::new(360.0, 100.0))
                .painted_text();
            let title = painted
                .iter()
                .find(|p| p.content.starts_with("Scheduler"))
                .unwrap_or_else(|| panic!("the title is painted: {painted:?}"));
            assert!(title.content.ends_with('\u{2026}'), "{title:?}");
            assert!(
                title.bounds.height < 2.0 * TypeRole::Body.size()
                    && title.bounds.y + title.bounds.height <= HEIGHT
                    && title.bounds.x + title.bounds.width <= 360.0,
                "one line inside the row: {title:?}"
            );
        }
    }

    /// #1276 review A F1: a cut title reads whole in its tooltip; one that fits has none.
    #[test]
    fn a_cut_title_reads_whole_in_its_tooltip() {
        let r = tokens::roles(ColorScheme::Light);
        let long = "Scheduler priorities: let heavy jobs jump the queue, keep FIFO among equal \
                    weights, and document the cutoff for operators running mixed pools";
        let short = "Scheduler fix";
        for (title, tooltip) in [(long, vec![long.to_owned()]), (short, vec![])] {
            let row = PullRequestRow::<String>::new(57, title, PrMark::Open, &r);
            let mut mounted =
                super::super::test_support::Mounted::new_in(row, Size::new(360.0, 100.0));
            let at = mounted
                .painted_text()
                .into_iter()
                .find(|p| p.content.starts_with("Scheduler"))
                .unwrap_or_else(|| panic!("the title is painted: {title:?}"));
            assert_eq!(
                mounted.floated_text(at.bounds.center()),
                tooltip,
                "{title:?}"
            );
        }
    }

    /// FR-003: a check mark widens the mark but never the row's height, and it is drawn.
    #[test]
    fn a_check_mark_keeps_the_row_height_and_is_drawn() {
        let r = tokens::roles(ColorScheme::Light);
        let row = |checks: Option<CheckMark>| {
            let row = PullRequestRow::<()>::new(1, "short", PrMark::Merged, &r);
            Element::from(match checks {
                Some(c) => row.checks(c),
                None => row,
            })
        };
        let room = Size::new(400.0, 100.0);
        let laid = |checks| super::super::test_support::laid_out(row(checks), room);
        let bare = laid(None);
        for mark in [CheckMark::Pending, CheckMark::Passing, CheckMark::Failing] {
            let with = laid(Some(mark));
            assert_eq!(with.size().height, HEIGHT, "{mark:?}");
            assert_eq!(
                with.size().width,
                bare.size().width,
                "the row fills its width"
            );
            // The mark widens the indicator, so the pieces after it sit further right.
            assert_ne!(
                super::super::test_support::leaves(&with),
                super::super::test_support::leaves(&bare),
                "{mark:?} is drawn"
            );
        }
    }
}
