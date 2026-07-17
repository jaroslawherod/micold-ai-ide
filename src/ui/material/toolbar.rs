//! `Toolbar` — a reusable Material toolbar primitive (Constitution Principle VIII).
//!
//! A flat `surface` bar with a thin bottom border separating it from the content below: a
//! title on the leading edge and a set of action elements pushed to the trailing edge. Reused
//! by the app shell; any future top bar should reuse it rather than fork a bespoke bar.

use crate::ui::style;
use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Background, Element, Length};
use micold_ai_ide::tokens::{spacing, type_scale, Roles};

/// A toolbar with a `title` on the leading edge and trailing action elements (Principle VIII
/// builder-API rule): construct with the required title + roles, add actions, then `.into()`.
pub struct Toolbar<'a, M> {
    title: String,
    roles: Roles,
    actions: Vec<Element<'a, M>>,
}

impl<'a, M: 'a> Toolbar<'a, M> {
    /// A toolbar titled `title`, themed by `roles`, with no actions yet.
    pub fn new(title: impl Into<String>, roles: Roles) -> Self {
        Self {
            title: title.into(),
            roles,
            actions: Vec::new(),
        }
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
        let mut bar = row![
            text(t.title).size(type_scale::BODY),
            Space::with_width(Length::Fill),
        ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        for action in t.actions {
            bar = bar.push(action);
        }

        // Compact bar: tight vertical padding (`XS`) with comfortable horizontal padding (`SM`).
        let bar = container(bar)
            .width(Length::Fill)
            .padding(iced::Padding::from([
                spacing::XS as f32,
                spacing::SM as f32,
            ]))
            .style(style::toolbar_surface(t.roles));

        // A thin bottom border separating the toolbar from the content below (a `Container`
        // border applies to all four sides, so this is a dedicated 1px line rather than the
        // surface style's own border).
        let separator = container(Space::new(Length::Fill, Length::Fixed(1.0))).style(
            move |_theme: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(style::separator(t.roles))),
                ..Default::default()
            },
        );

        column![bar, separator].into()
    }
}
