//! `Toolbar` — a reusable Material toolbar primitive (Constitution Principle VIII).
//!
//! A flat `surface` bar with a thin bottom border separating it from the content below: a
//! title on the leading edge and a set of action elements pushed to the trailing edge. Reused
//! by the app shell; any future top bar should reuse it rather than fork a bespoke bar.

use crate::ui::material::style;
use crate::ui::material::{Text, TypeRole};
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Background, Element, Length};
use micold_core::tokens::{anatomy, Roles};

/// A toolbar with a `title` on the leading edge and trailing action elements (Principle VIII
/// builder-API rule): construct with the required title + roles, add actions, then `.into()`.
pub struct Toolbar<'a, M> {
    title: String,
    roles: Roles,
    actions: Vec<Element<'a, M>>,
    elevated: bool,
}

impl<'a, M: 'a> Toolbar<'a, M> {
    /// A toolbar titled `title`, themed by `roles`, with no actions yet.
    pub fn new(title: impl Into<String>, roles: Roles) -> Self {
        Self {
            title: title.into(),
            roles,
            actions: Vec::new(),
            elevated: false,
        }
    }

    /// Whether content is scrolled beneath the bar, which raises it to elevation 2 (FR-025a).
    ///
    /// Supplied rather than observed: the bar has no way to see the sidebar's scroll position, and
    /// the sidebar is the only scroll region under it (contract §7.1). `State::app_bar_elevated`
    /// derives the flag from that one offset.
    pub fn elevated(mut self, elevated: bool) -> Self {
        self.elevated = elevated;
        self
    }

    /// Append a trailing action element.
    pub fn action(mut self, action: impl Into<Element<'a, M>>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Append several trailing action elements.
    pub fn actions(mut self, actions: Vec<Element<'a, M>>) -> Self {
        self.actions.extend(actions);
        self
    }
}

impl<'a, M: 'a> From<Toolbar<'a, M>> for Element<'a, M> {
    fn from(t: Toolbar<'a, M>) -> Self {
        // `title_large`, which is what §7.1 gives the small top app bar. It was `Section`
        // (`title_medium`) as a hedge against growing the bar — but the bar is now a fixed 64dp,
        // so the hedge costs nothing to drop and the contract's role applies as written.
        let mut bar = row![
            Text::new(t.title, TypeRole::Title, t.roles),
            Space::new().width(Length::Fill),
        ]
        .spacing(anatomy::app_bar::PADDING)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        for action in t.actions {
            bar = bar.push(action);
        }

        // §7.1's small app bar: a fixed 64dp with 16dp of horizontal padding. Vertical padding is
        // the height's business now — the bar is a fixed box and the row centres inside it.
        let elevated = t.elevated;
        let bar = container(bar)
            .width(Length::Fill)
            .height(Length::Fixed(anatomy::app_bar::HEIGHT))
            .padding(iced::Padding {
                top: 0.0,
                bottom: 0.0,
                left: anatomy::app_bar::PADDING,
                right: anatomy::app_bar::PADDING,
            })
            .style(style::app_bar_surface(t.roles, elevated));

        // The separator belongs to the bar *at rest* only. Once the bar is raised its own shadow
        // and tonal shift do that job (§7.1), and keeping a hairline under a shadow reads as two
        // edges — the thing FR-015 replaces lines with depth to avoid.
        if elevated {
            return bar.into();
        }
        let separator = container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
            .style(move |_theme: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(style::separator(t.roles))),
                ..Default::default()
            });

        column![bar, separator].into()
    }
}
