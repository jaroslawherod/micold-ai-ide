//! `ReviewPanelRow` — one comment of the Changes view's review panel (feature 946, FR-016–FR-018).
//!
//! Its line range and side, a status tag, the start of the comment's text and, once the agent
//! answered, the start of its reply, each on one clamped line, so every row has the same height
//! and `VirtualRows` can window 200 of them. Edit and Delete show on a draft row only; pressing the
//! row selects it, and `.selected(true)` draws it as the comment the diff is showing.
//!
//! Builder form: `ReviewPanelRow::new("Lines 3–5", &text, Status::Draft, roles).reply(&reply)
//! .selected(false).on_press(Msg::Jump(id)).on_edit(Msg::Edit(id)).on_delete(Msg::Delete(id))`.
//!
//! **Whole-reply mode** (feature 1270, contract D7) is the comments drawer's Replies row:
//! `.whole_reply(&reply, shortened)` draws the reply whole and wrapped, so the row is as tall as it
//! needs, with **Show all** under a reply shortened past 4,000 characters (`.on_show_reply`), and a
//! Sent row offers **Mark applied** and **Answer…** (`.on_mark_applied`, `.on_answer`).

use std::borrow::Cow;

use iced::widget::{column, container, mouse_area, row, Space};
use iced::{Alignment, Border, Element, Length};
use micold_core::review::comment::Status;
use micold_core::tokens::{spacing, Rgb, Roles};

use super::line_clamp::LineClamped;
use super::review_comment::{show_all, ANSWER, MARK_APPLIED};
use super::{style, Button, Tag, Text, TypeRole};

/// Every row's height, whatever it shows, outside whole-reply mode.
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
    /// The reply is drawn whole and wrapped, and the row is as tall as it needs (feature 1270).
    pub whole_reply: bool,
    /// "Show all" is drawn under the whole reply: it was shortened past 4,000 characters.
    pub show_all: bool,
    /// Mark applied and Answer…, on a Sent row.
    pub mark_actions: bool,
}

/// One comment's row.
pub struct ReviewPanelRow<'a, M> {
    lines_label: Cow<'a, str>,
    text: Cow<'a, str>,
    status: Status,
    roles: Roles,
    reply: Option<Cow<'a, str>>,
    /// `Some(shortened)` in whole-reply mode.
    whole: Option<bool>,
    selected: bool,
    not_in_diff: bool,
    on_press: Option<M>,
    on_edit: Option<M>,
    on_delete: Option<M>,
    on_mark_applied: Option<M>,
    on_answer: Option<M>,
    on_show_reply: Option<M>,
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
            whole: None,
            selected: false,
            not_in_diff: false,
            on_press: None,
            on_edit: None,
            on_delete: None,
            on_mark_applied: None,
            on_answer: None,
            on_show_reply: None,
        }
    }

    /// Draw the start of the agent's `reply` under the text; an empty one draws nothing.
    pub fn reply(mut self, reply: impl Into<Cow<'a, str>>) -> Self {
        let reply = reply.into();
        self.reply = (!reply.is_empty()).then_some(reply);
        self
    }

    /// Whole-reply mode: draw `reply` (already made displayable) whole and wrapped, the row as
    /// tall as it needs; `shortened` (cut past 4,000 characters) adds "Show all" (contract D7).
    pub fn whole_reply(mut self, reply: impl Into<Cow<'a, str>>, shortened: bool) -> Self {
        self = self.reply(reply);
        self.whole = Some(shortened);
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

    /// The message **Mark applied** sends; shown only on a Sent row.
    pub fn on_mark_applied(mut self, message: M) -> Self {
        self.on_mark_applied = Some(message);
        self
    }

    /// The message **Answer…** sends; shown only on a Sent row.
    pub fn on_answer(mut self, message: M) -> Self {
        self.on_answer = Some(message);
        self
    }

    /// The message "Show all" sends; it is drawn only under a shortened whole reply.
    pub fn on_show_reply(mut self, message: M) -> Self {
        self.on_show_reply = Some(message);
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
            whole_reply: self.whole.is_some(),
            show_all: self.reply.is_some() && self.whole == Some(true),
            mark_actions: self.status == Status::Sent
                && (self.on_mark_applied.is_some() || self.on_answer.is_some()),
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
        match (row_.reply, parts.whole_reply) {
            (Some(reply), true) => {
                body = body.push(Text::new(reply, TypeRole::Body, r).muted());
                if parts.show_all {
                    body = body.push(show_all(row_.on_show_reply, r));
                }
            }
            (Some(reply), false) => {
                body = body.push(Element::from(
                    row![
                        Text::new("Reply:", TypeRole::Caption, r).muted(),
                        LineClamped::<M>::new(reply.into_owned(), TypeRole::Caption, 1),
                    ]
                    .spacing(spacing::XS),
                ));
            }
            (None, _) => {}
        }
        // On a row of their own, under the reply: beside the lines and the tag they would not fit
        // the drawer's 360 px (half of a narrow window's diff area).
        if parts.mark_actions {
            let mut actions = row![Space::new().width(Length::Fill)]
                .spacing(spacing::SM)
                .align_y(Alignment::Center);
            if let Some(message) = row_.on_mark_applied {
                actions = actions.push(Button::text(MARK_APPLIED, r).on_press(message));
            }
            if let Some(message) = row_.on_answer {
                actions = actions.push(Button::text(ANSWER, r).on_press(message));
            }
            body = body.push(actions);
        }
        let selected = parts.selected;
        let height = if parts.whole_reply {
            Length::Shrink
        } else {
            Length::Fixed(HEIGHT)
        };
        let card = container(body)
            .padding(spacing::SM)
            .width(Length::Fill)
            .height(height)
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

    fn height<M>(element: Element<'_, M>, width: f32) -> f32 {
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

    fn painted(row: ReviewPanelRow<'_, String>, width: f32) -> Vec<String> {
        super::super::test_support::Mounted::new_in(row, Size::new(width, 2_000.0))
            .painted_text()
            .into_iter()
            .map(|p| p.content)
            .collect()
    }

    fn replied(status: Status, reply: &str, shortened: bool) -> ReviewPanelRow<'_, String> {
        ReviewPanelRow::new("Lines 3\u{2013}5", "rename this", status, LIGHT)
            .whole_reply(reply, shortened)
            .on_show_reply("show".to_owned())
    }

    /// Feature 1270 T029 (contract D7): in whole-reply mode the reply is laid out whole and
    /// wrapped, so the row grows with it.
    #[test]
    fn a_whole_reply_is_laid_out_whole_and_wrapped() {
        let long = "The height is passed in now, so the layout is read once. ".repeat(12);
        let shown = painted(replied(Status::Answered, &long, false), 360.0);
        assert!(shown.contains(&long), "the whole reply: {shown:?}");
        let wide = height(replied(Status::Answered, &long, false).into(), 900.0);
        let narrow = height(replied(Status::Answered, &long, false).into(), 360.0);
        assert!(narrow > HEIGHT, "grows past the fixed height: {narrow}");
        assert!(narrow > wide, "wraps: {narrow} > {wide}");
        assert!(replied(Status::Answered, "x", false).parts().whole_reply);
        assert!(!row_of(Status::Answered).reply("x").parts().whole_reply);
    }

    /// Feature 1270 T029: Show all only under a reply shortened past 4,000 characters.
    #[test]
    fn show_all_only_under_a_shortened_reply() {
        let short = painted(replied(Status::Answered, "not needed", false), 360.0);
        assert!(!short.iter().any(|t| t == "Show all"), "{short:?}");
        let cut = painted(replied(Status::Answered, "not needed", true), 360.0);
        assert!(cut.iter().any(|t| t == "Show all"), "{cut:?}");
        assert!(replied(Status::Answered, "x", true).parts().show_all);
        assert!(!replied(Status::Answered, "x", false).parts().show_all);
    }

    /// Feature 1270 T029: Mark applied and Answer… only on a Sent row given their handlers.
    #[test]
    fn mark_applied_and_answer_only_on_a_sent_row_with_handlers() {
        let marked = |status| {
            ReviewPanelRow::new("Lines 1", "rename this", status, LIGHT)
                .on_mark_applied("mark".to_owned())
                .on_answer("answer".to_owned())
        };
        let sent = painted(marked(Status::Sent), 480.0);
        assert!(sent.iter().any(|t| t == "Mark applied"), "{sent:?}");
        assert!(sent.iter().any(|t| t == "Answer\u{2026}"), "{sent:?}");
        assert!(marked(Status::Sent).parts().mark_actions);
        for status in [Status::Draft, Status::Applied, Status::Answered] {
            let shown = painted(marked(status), 480.0);
            assert!(!shown.iter().any(|t| t == "Mark applied"), "{status:?}");
            assert!(!marked(status).parts().mark_actions, "{status:?}");
        }
        let narrow = super::super::test_support::Mounted::new_in(
            marked(Status::Sent).whole_reply("done", false),
            Size::new(240.0, 2_000.0),
        )
        .painted_text();
        for label in ["Mark applied", "Answer\u{2026}"] {
            let at = narrow
                .iter()
                .find(|p| p.content == label)
                .unwrap_or_else(|| panic!("{label} painted"));
            assert!(
                at.bounds.x + at.bounds.width <= 240.0,
                "{label} inside a 240 px row: {:?}",
                at.bounds
            );
        }
        let bare = painted(
            ReviewPanelRow::<String>::new("Lines 1", "rename this", Status::Sent, LIGHT),
            480.0,
        );
        assert!(
            !bare.iter().any(|t| t == "Mark applied"),
            "without handlers"
        );
    }

    #[test]
    fn each_status_has_its_own_tag() {
        let labels: Vec<&str> = ALL.iter().map(|s| row_of(*s).parts().label).collect();
        assert_eq!(labels, ["Draft", "Sent", "Applied", "Answered"]);
    }
}
