//! A dialog whose body scrolls and whose action row stays put (feature 1092, FR-011).
//!
//! [`dialog::confirm`](super::dialog::confirm) lays a short question out and lets the dialog grow
//! with it. A form dialog cannot: a long form would push the action row off a small window. This
//! keeps §7.4's anatomy (title, body, actions on a [`SurfaceKind::Dialog`] surface) but puts the
//! body in a [`Scrollable`] capped so the whole dialog fits `max_height`, and leaves the actions
//! outside it, always visible.
//!
//! The scroll container is deliberately not `Length::Fill`: a fill child makes a shrink column take
//! the full height, so a short body would stretch the dialog to the window.

use super::{dialog, Scrollable, Surface, SurfaceKind, Text, TypeRole};
use iced::widget::{column, container};
use iced::{Element, Length};
use micold_core::tokens::{anatomy, Roles};

/// The height of the title line (a headline).
const TITLE_HEIGHT: f32 = 32.0;
/// The room the modal leaves around a dialog, top and bottom together.
const MODAL_MARGIN: f32 = 32.0;
/// The default height of a single row of action buttons.
const ACTIONS_HEIGHT: f32 = 40.0;
/// The default width.
const WIDTH: f32 = 560.0;

/// A titled dialog with a scrolling body and pinned actions. Builder form (Principle VIII):
/// `ScrollingDialog::new("Edit", body, actions, roles).max_height(window_height).into()`.
pub struct ScrollingDialog<'a, M> {
    title: String,
    body: Element<'a, M>,
    actions: Element<'a, M>,
    roles: Roles,
    width: f32,
    max_height: Option<f32>,
    actions_height: f32,
}

impl<'a, M: Clone + 'a> ScrollingDialog<'a, M> {
    /// A dialog titled `title` with `body` above `actions`.
    pub fn new(
        title: impl Into<String>,
        body: impl Into<Element<'a, M>>,
        actions: impl Into<Element<'a, M>>,
        roles: Roles,
    ) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            actions: actions.into(),
            roles,
            width: WIDTH,
            max_height: None,
            actions_height: ACTIONS_HEIGHT,
        }
    }

    /// Lay the dialog out at `width` dp; the dialog kind caps it at §7.4's maximum.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The height of the window the dialog sits in; the body scrolls to keep the dialog inside it.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// The height of the actions region, when it is taller than one row of buttons.
    pub fn actions_height(mut self, height: f32) -> Self {
        self.actions_height = height;
        self
    }
}

impl<'a, M: Clone + 'a> From<ScrollingDialog<'a, M>> for Element<'a, M> {
    fn from(d: ScrollingDialog<'a, M>) -> Self {
        let title = Text::new(d.title, TypeRole::Headline, d.roles);
        let scroll = container(Scrollable::new(d.body, d.roles));
        let scroll = match d.max_height {
            Some(window) => {
                let chrome = 2.0 * anatomy::dialog::PADDING
                    + anatomy::dialog::TITLE_TO_BODY
                    + TITLE_HEIGHT
                    + anatomy::dialog::BODY_TO_ACTIONS
                    + d.actions_height
                    + MODAL_MARGIN;
                scroll.max_height((window - chrome).max(0.0))
            }
            None => scroll,
        };
        let fields = dialog::fields(column![title, scroll]);
        Surface::new(
            dialog::body(fields, d.actions),
            SurfaceKind::Dialog,
            d.roles,
        )
        .width(Length::Fixed(d.width))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::showcase::state::Message;
    use crate::ui::material::test_support::{bounds_at, roles, size_of};
    use crate::ui::material::{dialog, Button};
    use iced::widget::{row, Space};
    use iced::{Length, Size};

    fn tall_body() -> Element<'static, Message> {
        Space::new().height(Length::Fixed(2000.0)).into()
    }

    fn actions() -> Element<'static, Message> {
        let r = roles();
        dialog::actions(row![
            Button::filled("Save daemon", r).on_press(Message::NoOp),
            Button::outlined("Cancel", r).on_press(Message::NoOp),
        ])
        .into()
    }

    fn tall_dialog(room_height: f32) -> ScrollingDialog<'static, Message> {
        ScrollingDialog::new("Edit daemon", tall_body(), actions(), roles()).max_height(room_height)
    }

    /// FR-011: a body taller than the window scrolls instead of growing the dialog past it.
    #[test]
    fn a_body_taller_than_the_window_scrolls_inside_it() {
        let room = Size::new(800.0, 480.0);
        let size = size_of(tall_dialog(room.height).into(), room);
        assert!(
            size.height <= room.height,
            "the dialog was {}dp tall in a {}dp window",
            size.height,
            room.height
        );
    }

    /// FR-011, SC-005: the action row stays inside the window however tall the body is.
    #[test]
    fn the_actions_stay_pinned_inside_the_window() {
        let room = Size::new(800.0, 480.0);
        // Surface > column > [fields, actions]; the actions are the column's second child.
        let actions = bounds_at(tall_dialog(room.height).into(), room, &[0, 1]);
        assert!(
            actions.y + actions.height <= room.height,
            "the action row ended at {}dp in a {}dp window",
            actions.y + actions.height,
            room.height
        );
        assert!(actions.height > 0.0);
    }

    /// A short body is not stretched to the window: the dialog only takes what it needs.
    #[test]
    fn a_short_body_does_not_fill_the_window() {
        let room = Size::new(800.0, 900.0);
        let short = ScrollingDialog::new(
            "Add daemon",
            Space::new().height(Length::Fixed(40.0)),
            actions(),
            roles(),
        )
        .max_height(room.height);
        let size = size_of(short.into(), room);
        assert!(
            size.height < 300.0,
            "a dialog with a 40dp body was {}dp tall",
            size.height
        );
    }

    /// `SurfaceKind::Dialog` caps the width; a wider request is clamped by the kind.
    #[test]
    fn the_width_honours_the_dialog_kind() {
        let room = Size::new(1400.0, 900.0);
        let wide = tall_dialog(room.height).width(1200.0);
        let size = size_of(wide.into(), room);
        assert!(
            size.width <= micold_core::tokens::anatomy::dialog::MAX_WIDTH + 0.5,
            "the dialog was {}dp wide",
            size.width
        );
        let narrow = tall_dialog(room.height).width(400.0);
        let size = size_of(narrow.into(), room);
        assert!((size.width - 400.0).abs() < 0.5, "{}", size.width);
    }
}
