//! `QueueRow` — one message waiting in a session's queue (feature 941, T052, FR-018, FR-019).
//!
//! A message sent while Claude answers waits above the composer until its turn. Its row says where
//! it stands (its position, from 1), what it says (its text on one line, cut with an ellipsis, its
//! line breaks shown as spaces), whether it carries pictures ("2 pictures"), and offers what can
//! still be done to it: **Edit**, **Remove** and **Move to composer**. Each action is drawn only
//! when the caller gives its message, so the composer withholds Move to composer while its draft
//! holds anything (spec US4 AS3). The actions are text buttons: each names itself, and each is a
//! keyboard stop.
//!
//! Builder form: `QueueRow::new(1, text, roles).pictures(2).on_edit(…).on_remove(…)
//! .on_move_to_composer(…).into()`.

use iced::widget::{container, row};
use iced::{Alignment, Element, Length, Padding};
use micold_core::tokens::{spacing, Roles};

use super::{style, Button, Ellipsized, Text, TypeRole};

/// The Edit action's words.
pub const EDIT: &str = "Edit";
/// The Remove action's words.
pub const REMOVE: &str = "Remove";
/// The Move to composer action's words.
pub const MOVE_TO_COMPOSER: &str = "Move to composer";

/// `text` on one line: every run of whitespace, line breaks included, as one space.
pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What a row says of its pictures: nothing for none, "1 picture", "3 pictures".
pub fn pictures_label(count: usize) -> Option<String> {
    match count {
        0 => None,
        1 => Some("1 picture".to_string()),
        n => Some(format!("{n} pictures")),
    }
}

/// One queued message.
pub struct QueueRow<'a, M> {
    position: usize,
    text: String,
    pictures: usize,
    roles: Roles,
    on_edit: Option<M>,
    on_remove: Option<M>,
    on_move: Option<M>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> QueueRow<'a, M> {
    /// The row of the message at `position` (from 1) whose text is `text`, themed by `roles`; it
    /// offers no action until one is given.
    pub fn new(position: usize, text: impl Into<String>, roles: Roles) -> Self {
        Self {
            position,
            text: text.into(),
            pictures: 0,
            roles,
            on_edit: None,
            on_remove: None,
            on_move: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// How many pictures the message carries.
    pub fn pictures(mut self, count: usize) -> Self {
        self.pictures = count;
        self
    }

    /// Offer **Edit**, sending `message`.
    pub fn on_edit(mut self, message: M) -> Self {
        self.on_edit = Some(message);
        self
    }

    /// Offer **Remove**, sending `message`.
    pub fn on_remove(mut self, message: M) -> Self {
        self.on_remove = Some(message);
        self
    }

    /// Offer **Move to composer** when `message` is given; `None` withholds it (the draft holds
    /// something it would displace).
    pub fn on_move_to_composer(mut self, message: Option<M>) -> Self {
        self.on_move = message;
        self
    }
}

impl<'a, M: Clone + 'a> From<QueueRow<'a, M>> for Element<'a, M> {
    fn from(q: QueueRow<'a, M>) -> Self {
        let r = q.roles;
        let mut parts = row![
            Text::new(q.position.to_string(), TypeRole::Label, r).muted(),
            Ellipsized::<M>::at_role(one_line(&q.text), TypeRole::Body, r.on_surface),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
        if let Some(label) = pictures_label(q.pictures) {
            parts = parts.push(
                Text::new(label, TypeRole::Label, r)
                    .muted()
                    .wrapping(iced::widget::text::Wrapping::None),
            );
        }
        for (words, message) in [
            (EDIT, q.on_edit),
            (REMOVE, q.on_remove),
            (MOVE_TO_COMPOSER, q.on_move),
        ] {
            if let Some(message) = message {
                parts = parts.push(Button::text(words, r).on_press(message));
            }
        }
        container(parts)
            .padding(Padding {
                top: 0.0,
                bottom: 0.0,
                left: spacing::MD,
                right: spacing::XS,
            })
            .width(Length::Fill)
            .style(style::queue_row(r))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, named_key, roles, Focusables, Mounted};
    use super::*;
    use iced::advanced::mouse;
    use iced::keyboard::key::Named;
    use iced::Size;

    fn full(r: Roles) -> QueueRow<'static, String> {
        QueueRow::new(2, "Then run the tests\nand fix what fails", r)
            .pictures(3)
            .on_edit("edit".to_string())
            .on_remove("remove".to_string())
            .on_move_to_composer(Some("move".to_string()))
    }

    fn words(m: &mut Mounted<'_>) -> Vec<String> {
        m.painted_text().into_iter().map(|p| p.content).collect()
    }

    /// FR-018: the position, the text on one line, the pictures' count and the three actions.
    #[test]
    fn the_row_shows_its_position_text_count_and_actions() {
        for r in [roles(), dark_roles()] {
            let drawn = words(&mut Mounted::new_in(full(r), Size::new(800.0, 100.0)));
            for part in [
                "2",
                "Then run the tests and fix what fails",
                "3 pictures",
                EDIT,
                REMOVE,
                MOVE_TO_COMPOSER,
            ] {
                assert!(drawn.iter().any(|d| d == part), "`{part}`: {drawn:?}");
            }
        }
    }

    /// Each action is a keyboard stop, in order, and Enter on one sends its message.
    #[test]
    fn each_action_is_a_keyboard_stop() {
        let mut m = Mounted::new(full(roles()));
        assert_eq!(m.drive(Focusables(0)), Some(3));
        let mut sent = Vec::new();
        for _ in 0..3 {
            m.focus_next();
            sent.extend(m.send(named_key(Named::Enter), mouse::Cursor::Unavailable));
        }
        assert_eq!(sent, vec!["edit", "remove", "move"]);
    }

    /// Withheld actions are not drawn, and no count is drawn for no pictures.
    #[test]
    fn withheld_actions_and_no_pictures_draw_nothing() {
        let row = QueueRow::new(1, "Hello", roles())
            .on_edit("edit".to_string())
            .on_move_to_composer(None);
        let mut m = Mounted::new(row);
        let drawn = words(&mut m);
        assert!(drawn.iter().any(|d| d == EDIT));
        assert!(!drawn.iter().any(|d| d == MOVE_TO_COMPOSER || d == REMOVE));
        assert!(!drawn.iter().any(|d| d.contains("picture")), "{drawn:?}");
        assert_eq!(m.drive(Focusables(0)), Some(1));
    }

    #[test]
    fn one_line_joins_lines_and_pictures_are_counted() {
        assert_eq!(one_line("a\n\n  b\tc "), "a b c");
        assert_eq!(pictures_label(0), None);
        assert_eq!(pictures_label(1).as_deref(), Some("1 picture"));
        assert_eq!(pictures_label(4).as_deref(), Some("4 pictures"));
    }
}
