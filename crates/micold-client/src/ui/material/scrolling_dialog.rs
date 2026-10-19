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
