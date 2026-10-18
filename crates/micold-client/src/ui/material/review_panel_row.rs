//! `ReviewPanelRow` — one comment of the Changes view's review panel (feature 946, FR-016–FR-018).
//!
//! Its line range and side, a status tag, the start of the comment's text and, once the agent
//! answered, the start of its reply, each on one clamped line, so every row has the same height
//! and `VirtualRows` can window 200 of them. Edit and Delete show on a draft row only; pressing the
//! row selects it, and `.selected(true)` draws it as the comment the diff is showing.
//!
//! Builder form: `ReviewPanelRow::new("Lines 3–5", &text, Status::Draft, roles).reply(&reply)
//! .selected(false).on_press(Msg::Jump(id)).on_edit(Msg::Edit(id)).on_delete(Msg::Delete(id))`.

use std::borrow::Cow;

use iced::widget::{column, container, mouse_area, row, Space};
use iced::{Alignment, Border, Element, Length};
use micold_core::review::comment::Status;
use micold_core::tokens::{spacing, Rgb, Roles};

use super::line_clamp::LineClamped;
use super::{style, Button, Tag, Text, TypeRole};

/// Every row's height, whatever it shows.
pub const HEIGHT: f32 = 104.0;
/// The tag of a row whose file the current diff does not list.
pub const NOT_IN_DIFF: &str = "Not in the current diff";

/// What a row shows besides its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parts {
    /// The status tag's label.
    pub label: &'static str,
    /// A reply start is drawn under the text.
    pub reply: bool,
    /// Edit and Delete are drawn.
    pub actions: bool,
    /// The row is drawn as the selected one.
    pub selected: bool,
    /// The "Not in the current diff" tag.
    pub not_in_diff: bool,
}

/// One comment's row.
pub struct ReviewPanelRow<'a, M> {
    lines_label: Cow<'a, str>,
    text: Cow<'a, str>,
    status: Status,
    roles: Roles,
    reply: Option<Cow<'a, str>>,
    selected: bool,
    not_in_diff: bool,
    on_press: Option<M>,
    on_edit: Option<M>,
    on_delete: Option<M>,
}

/// The label of a status tag.
pub fn status_label(status: Status) -> &'static str {
    match status {
        Status::Draft => "Draft",
        Status::Sent => "Sent",
        Status::Applied => "Applied",
        Status::Answered => "Answered",
    }
}

/// The accent of a status tag, matching the card's.
fn accent(status: Status, r: Roles) -> Rgb {
    match status {
        Status::Draft => r.tertiary,
        Status::Sent => r.outline,
        Status::Applied => r.primary,
        Status::Answered => r.secondary,
    }
}

impl<'a, M: Clone + 'a> ReviewPanelRow<'a, M> {
    /// The row of a comment on `lines_label` reading `text` in `status`, themed by `roles`.
    pub fn new(
        lines_label: impl Into<Cow<'a, str>>,
        text: impl Into<Cow<'a, str>>,
        status: Status,
        roles: Roles,
    ) -> Self {
        Self {
            lines_label: lines_label.into(),
            text: text.into(),
            status,
            roles,
            reply: None,
            selected: false,
            not_in_diff: false,
            on_press: None,
            on_edit: None,
            on_delete: None,
        }
    }

    /// Draw the start of the agent's `reply` under the text; an empty one draws nothing.
    pub fn reply(mut self, reply: impl Into<Cow<'a, str>>) -> Self {
        let reply = reply.into();
        self.reply = (!reply.is_empty()).then_some(reply);
        self
    }

    /// Draw the row as the comment the diff is showing.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Mark the row's file as one the current diff does not list.
    pub fn not_in_diff(mut self, not_in_diff: bool) -> Self {
        self.not_in_diff = not_in_diff;
        self
    }

    /// The message pressing the row sends.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The message Edit sends; shown only on a draft row.
    pub fn on_edit(mut self, message: M) -> Self {
        self.on_edit = Some(message);
        self
    }

    /// The message Delete sends; shown only on a draft row.
    pub fn on_delete(mut self, message: M) -> Self {
        self.on_delete = Some(message);
        self
    }

    /// What the row shows besides its text.
    fn parts(&self) -> Parts {
        Parts {
            label: status_label(self.status),
            reply: self.reply.is_some(),
            actions: self.status == Status::Draft
                && (self.on_edit.is_some() || self.on_delete.is_some()),
            selected: self.selected,
            not_in_diff: self.not_in_diff,
        }
    }
}

impl<'a, M: Clone + 'a> From<ReviewPanelRow<'a, M>> for Element<'a, M> {
    fn from(row_: ReviewPanelRow<'a, M>) -> Self {
        let parts = row_.parts();
        let r = row_.roles;
        let mut head = row![
            Text::new(row_.lines_label, TypeRole::Caption, r).muted(),
            Tag::new(parts.label, accent(row_.status, r)),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
        if parts.not_in_diff {
            head = head.push(Tag::new(NOT_IN_DIFF, r.error));
        }
        head = head.push(Space::new().width(Length::Fill));
        if parts.actions {
            if let Some(message) = row_.on_edit {
                head = head.push(Button::text("Edit", r).on_press(message));
            }
            if let Some(message) = row_.on_delete {
                head = head.push(Button::text("Delete", r).on_press(message));
            }
        }
        let mut body = column![
            head,
            LineClamped::<M>::new(row_.text.into_owned(), TypeRole::Body, 1)
        ]
        .spacing(spacing::XS);
        if let Some(reply) = row_.reply {
            body = body.push(Element::from(
                row![
                    Text::new("Reply:", TypeRole::Caption, r).muted(),
                    LineClamped::<M>::new(reply.into_owned(), TypeRole::Caption, 1),
                ]
                .spacing(spacing::XS),
            ));
        }
        let selected = parts.selected;
        let card = container(body)
            .padding(spacing::SM)
            .width(Length::Fill)
            .height(Length::Fixed(HEIGHT))
            .style(move |_| container::Style {
                background: selected
                    .then(|| iced::Background::Color(style::color(r.surface_container_highest))),
                border: Border {
                    color: style::color(if selected {
                        r.outline
                    } else {
                        r.outline_variant
                    }),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..container::Style::default()
            });
        match row_.on_press {
            Some(message) => mouse_area(card).on_press(message).into(),
            None => card.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;
    use micold_core::tokens::LIGHT;

    const ALL: [Status; 4] = [
        Status::Draft,
        Status::Sent,
        Status::Applied,
        Status::Answered,
    ];

    fn row_of(status: Status) -> ReviewPanelRow<'static, ()> {
        ReviewPanelRow::new("Lines 3\u{2013}5", "rename this", status, LIGHT)
    }

    fn height(element: Element<'_, ()>, width: f32) -> f32 {
        super::super::test_support::size_of(element, Size::new(width, 600.0)).height
    }

    #[test]
    fn every_state_has_the_one_fixed_height() {
        let long = "word ".repeat(200);
        let mut heights = Vec::new();
        for status in ALL {
            heights.push(height(row_of(status).into(), 360.0));
            heights.push(height(
                row_of(status).reply("done").selected(true).into(),
                360.0,
            ));
            heights.push(height(
                row_of(status)
                    .reply(long.clone())
                    .not_in_diff(true)
                    .on_edit(())
                    .on_delete(())
                    .into(),
                360.0,
            ));
            heights.push(height(
                ReviewPanelRow::<()>::new("Lines 1", long.clone(), status, LIGHT)
                    .reply(long.clone())
                    .into(),
                200.0,
            ));
        }
        assert!(heights.iter().all(|h| *h == HEIGHT), "{heights:?}");
    }

    #[test]
    fn a_reply_is_drawn_on_one_clamped_line() {
        assert!(!row_of(Status::Answered).parts().reply);
        assert!(!row_of(Status::Answered).reply("").parts().reply);
        assert!(row_of(Status::Answered).reply("not needed").parts().reply);
    }

    #[test]
    fn a_draft_row_draws_two_actions_and_no_other_status_does() {
        let draft = row_of(Status::Draft).on_edit(()).on_delete(()).parts();
        assert!(draft.actions);
        for status in [Status::Sent, Status::Applied, Status::Answered] {
            let parts = row_of(status).on_edit(()).on_delete(()).parts();
            assert!(!parts.actions, "{status:?} offers neither");
        }
        assert!(!row_of(Status::Draft).parts().actions, "without handlers");
    }

    #[test]
    fn the_selected_state_layer_and_the_mark_are_drawn_on_request() {
        assert!(!row_of(Status::Sent).parts().selected);
        assert!(row_of(Status::Sent).selected(true).parts().selected);
        assert!(!row_of(Status::Sent).parts().not_in_diff);
        assert!(row_of(Status::Sent).not_in_diff(true).parts().not_in_diff);
    }

    #[test]
    fn each_status_has_its_own_tag() {
        let labels: Vec<&str> = ALL.iter().map(|s| row_of(*s).parts().label).collect();
        assert_eq!(labels, ["Draft", "Sent", "Applied", "Answered"]);
    }
}
