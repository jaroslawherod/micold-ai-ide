//! `PullRequestRow`: one pull request of the Changes pane's list (feature 775, FR-003).
//!
//! The state mark (the sidebar's [`PullRequestIndicator`], without check status), `#number`, and the
//! title on one fixed-height line; `selected` outlines it, `stale` draws the mark and the text in
//! their quieter form. The row is informational until a press is wired (M2).

use crate::ui::material::{PrMark, PullRequestIndicator, Text, TypeRole};
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
    selected: bool,
    stale: bool,
    _marker: PhantomData<M>,
}

impl<'a, M: 'a> PullRequestRow<'a, M> {
    /// The row of pull request `number` titled `title`, in `state`, themed by `roles`.
    pub fn new(
        number: u64,
        title: impl Into<Cow<'a, str>>,
        state: PrMark,
        roles: &Roles,
    ) -> Self {
        Self {
            roles: *roles,
            number,
            title: title.into(),
            state,
            selected: false,
            stale: false,
            _marker: PhantomData,
        }
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
        let mark = PullRequestIndicator::<M>::new(r.state, &roles).stale(r.stale);
        let line = row![
            Element::from(mark),
            text(Cow::Owned(format!("#{}", r.number))),
            container(text(r.title)).width(Length::Fill),
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
}
