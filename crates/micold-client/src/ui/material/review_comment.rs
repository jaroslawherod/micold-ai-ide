//! `ReviewCommentCard` — one review comment under its anchor row (feature 482,
//! contracts/changes-view.md C3).
//!
//! The comment's text, its state (Draft / Sent, or the Applied / Answered tag of a reported one,
//! feature 946), an Outdated tag when the lines it quotes changed, the agent's reply under the
//! text with "Show all" when it was shortened (pressable when `.on_show_reply` is set), Edit /
//! Delete for a draft comment that is not in a send, and Mark applied / Answer… for a Sent one.
//!
//! Builder form: `ReviewCommentCard::new(&text, CardState::Draft, roles).outdated(false)
//! .reply(&reply, shortened).on_edit(Msg::Edit(id)).on_delete(Msg::Delete(id)).into()`.

use std::borrow::Cow;

use iced::widget::{column, container, row, Space};
use iced::{Alignment, Border, Element, Length};
use micold_core::tokens::{spacing, Roles};

use super::{style, Button, Tag, Text, TypeRole};

/// Where a comment is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    /// Not sent, editable.
    Draft,
    /// A draft, but in a send under way: not editable.
    InSend,
    /// Delivered to a session.
    Sent,
    /// Reported as applied (feature 946).
    Applied,
    /// Reported as answered: not applied, the reply says why (feature 946).
    Answered,
}

/// The state label.
pub const DRAFT: &str = "Draft";
/// The state label once delivered.
pub const SENT: &str = "Sent";
/// The tag of a comment reported applied.
pub const APPLIED: &str = "Applied";
/// The tag of a comment reported answered.
pub const ANSWERED: &str = "Answered";
/// The affordance under a shortened reply.
pub const SHOW_ALL: &str = "Show all";
/// The Sent card's action that marks the comment applied by hand (feature 946).
pub const MARK_APPLIED: &str = "Mark applied";
/// The Sent card's action that opens the composer for an answer by hand (feature 946).
pub const ANSWER: &str = "Answer…";
/// The tag of a comment whose quoted lines changed.
pub const OUTDATED: &str = "Outdated";

/// What a card shows besides its text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Parts {
    /// The state label.
    pub label: &'static str,
    /// The label is drawn as a status tag (Applied, Answered), not as muted text.
    pub tag: bool,
    /// A reply is drawn under the text.
    pub reply: bool,
    /// "Show all" is drawn under the reply: it was shortened.
    pub show_all: bool,
    /// "Show all" is a button: the card was given what it sends.
    pub show_all_pressable: bool,
    /// The Outdated tag.
    pub outdated: bool,
    /// Edit and Delete.
    pub actions: bool,
    /// Mark applied and Answer…, on a Sent card.
    pub mark_actions: bool,
    /// The card is the one the review panel jumped to: drawn with the highlighted outline.
    pub highlighted: bool,
}

/// One comment's card.
pub struct ReviewCommentCard<'a, M> {
    text: &'a str,
    state: CardState,
    roles: Roles,
    outdated: bool,
    highlighted: bool,
    reply: Option<(Cow<'a, str>, bool)>,
    on_edit: Option<M>,
    on_delete: Option<M>,
    on_mark_applied: Option<M>,
    on_answer: Option<M>,
    on_show_reply: Option<M>,
}

impl<'a, M: Clone + 'a> ReviewCommentCard<'a, M> {
    /// The card of a comment reading `text` in `state`, themed by `roles`.
    pub fn new(text: &'a str, state: CardState, roles: Roles) -> Self {
        Self {
            text,
            state,
            roles,
            outdated: false,
            highlighted: false,
            reply: None,
            on_edit: None,
            on_delete: None,
            on_mark_applied: None,
            on_answer: None,
            on_show_reply: None,
        }
    }

    /// Show the Outdated tag.
    pub fn outdated(mut self, outdated: bool) -> Self {
        self.outdated = outdated;
        self
    }

    /// Draw the highlighted outline: the review panel jumped to this card.
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// Draw `reply` (already made displayable) under the text; `shortened` adds "Show all". An
    /// empty reply draws nothing.
    pub fn reply(mut self, reply: impl Into<Cow<'a, str>>, shortened: bool) -> Self {
        let reply = reply.into();
        self.reply = (!reply.is_empty()).then_some((reply, shortened));
        self
    }

    /// The message Edit sends; shown only on a draft comment not in a send.
    pub fn on_edit(mut self, message: M) -> Self {
        self.on_edit = Some(message);
        self
    }

    /// The message Delete sends; shown only on a draft comment not in a send.
    pub fn on_delete(mut self, message: M) -> Self {
        self.on_delete = Some(message);
        self
    }

    /// The message **Mark applied** sends; shown only on a Sent comment, with `on_answer`.
    pub fn on_mark_applied(mut self, message: M) -> Self {
        self.on_mark_applied = Some(message);
        self
    }

    /// The message **Answer…** sends; shown only on a Sent comment.
    pub fn on_answer(mut self, message: M) -> Self {
        self.on_answer = Some(message);
        self
    }

    /// The message "Show all" sends; it is a button only under a shortened reply.
    pub fn on_show_reply(mut self, message: M) -> Self {
        self.on_show_reply = Some(message);
        self
    }

    /// What the card shows besides its text.
    fn parts(&self) -> Parts {
        Parts {
            label: match self.state {
                CardState::Draft | CardState::InSend => DRAFT,
                CardState::Sent => SENT,
                CardState::Applied => APPLIED,
                CardState::Answered => ANSWERED,
            },
            tag: matches!(self.state, CardState::Applied | CardState::Answered),
            reply: self.reply.is_some(),
            show_all: self.reply.as_ref().is_some_and(|(_, shortened)| *shortened),
            show_all_pressable: self.on_show_reply.is_some()
                && self.reply.as_ref().is_some_and(|(_, shortened)| *shortened),
            outdated: self.outdated,
            actions: self.state == CardState::Draft
                && (self.on_edit.is_some() || self.on_delete.is_some()),
            mark_actions: self.state == CardState::Sent
                && (self.on_mark_applied.is_some() || self.on_answer.is_some()),
            highlighted: self.highlighted,
        }
    }
}

impl<'a, M: Clone + 'a> From<ReviewCommentCard<'a, M>> for Element<'a, M> {
    fn from(card: ReviewCommentCard<'a, M>) -> Self {
        let parts = card.parts();
        let r = card.roles;
        let label: Element<'a, M> = match card.state {
            CardState::Applied => Tag::new(APPLIED, r.primary).into(),
            CardState::Answered => Tag::new(ANSWERED, r.secondary).into(),
            _ => Text::new(parts.label, TypeRole::Caption, r).muted().into(),
        };
        let mut head = row![label].spacing(spacing::SM).align_y(Alignment::Center);
        if parts.outdated {
            head = head.push(Tag::new(OUTDATED, r.tertiary));
        }
        head = head.push(Space::new().width(Length::Fill));
        if parts.actions {
            if let Some(message) = card.on_edit {
                head = head.push(Button::text("Edit", r).on_press(message));
            }
            if let Some(message) = card.on_delete {
                head = head.push(Button::text("Delete", r).on_press(message));
            }
        }
        if parts.mark_actions {
            if let Some(message) = card.on_mark_applied {
                head = head.push(Button::text(MARK_APPLIED, r).on_press(message));
            }
            if let Some(message) = card.on_answer {
                head = head.push(Button::text(ANSWER, r).on_press(message));
            }
        }
        let mut body = column![head, Text::new(card.text, TypeRole::Body, r)].spacing(spacing::XS);
        if let Some((reply, shortened)) = card.reply {
            body = body.push(Text::new(reply, TypeRole::Body, r).muted());
            if shortened {
                body = body.push(match card.on_show_reply {
                    Some(message) => Element::from(Button::text(SHOW_ALL, r).on_press(message)),
                    None => Text::new(SHOW_ALL, TypeRole::Label, r).muted().into(),
                });
            }
        }
        container(body)
            .padding(spacing::SM)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(style::color(
                    r.surface_container_low,
                ))),
                border: Border {
                    color: style::color(if parts.highlighted {
                        r.primary
                    } else {
                        r.outline_variant
                    }),
                    width: if parts.highlighted { 2.0 } else { 1.0 },
                    radius: 8.0.into(),
                },
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;
    use iced::Size;
    use micold_core::tokens::LIGHT;

    fn card(state: CardState) -> ReviewCommentCard<'static, ()> {
        ReviewCommentCard::new("rename this", state, LIGHT)
            .on_edit(())
            .on_delete(())
    }

    /// The card's height laid out `width` wide.
    fn height(text: &str, width: f32) -> f32 {
        let card = ReviewCommentCard::new(text, CardState::Draft, LIGHT)
            .on_edit(())
            .on_delete(());
        height_of(card, width)
    }

    /// `card`'s height laid out 400 wide.
    fn laid_out(card: ReviewCommentCard<'_, ()>) -> f32 {
        height_of(card, 400.0)
    }

    fn height_of(card: ReviewCommentCard<'_, ()>, width: f32) -> f32 {
        let mut element: Element<'_, ()> = card.into();
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        element
            .as_widget_mut()
            .layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, Size::new(width, 2_000.0)),
            )
            .bounds()
            .height
    }

    #[test]
    fn a_draft_card_has_edit_and_delete() {
        let parts = card(CardState::Draft).parts();
        assert_eq!(parts.label, DRAFT);
        assert!(parts.actions);
        assert!(!parts.outdated);
    }

    #[test]
    fn a_sent_card_and_one_in_a_send_have_no_actions() {
        let sent = card(CardState::Sent).parts();
        assert_eq!((sent.label, sent.actions), (SENT, false));
        let in_send = card(CardState::InSend).parts();
        assert_eq!((in_send.label, in_send.actions), (DRAFT, false));
    }

    #[test]
    fn an_outdated_card_shows_its_tag() {
        assert!(card(CardState::Sent).outdated(true).parts().outdated);
    }

    #[test]
    fn applied_and_answered_draw_their_status_tag_and_no_actions() {
        let applied = card(CardState::Applied).parts();
        assert_eq!(
            (applied.label, applied.tag, applied.actions),
            (APPLIED, true, false)
        );
        let answered = card(CardState::Answered).parts();
        assert_eq!(
            (answered.label, answered.tag, answered.actions),
            (ANSWERED, true, false)
        );
        assert!(
            !card(CardState::Sent).parts().tag,
            "Sent stays a muted label"
        );
    }

    #[test]
    fn a_reply_is_drawn_under_the_text_with_show_all_only_when_shortened() {
        let whole = card(CardState::Answered).reply("not needed", false).parts();
        assert!(whole.reply && !whole.show_all);
        let cut = card(CardState::Answered).reply("not needed", true).parts();
        assert!(cut.reply && cut.show_all);
        assert!(!card(CardState::Applied).reply("", false).parts().reply);

        let plain = laid_out(ReviewCommentCard::new("rename", CardState::Answered, LIGHT));
        let replied = laid_out(
            ReviewCommentCard::new("rename", CardState::Answered, LIGHT).reply("not needed", false),
        );
        let shortened = laid_out(
            ReviewCommentCard::new("rename", CardState::Answered, LIGHT).reply("not needed", true),
        );
        assert!(
            replied > plain,
            "the reply adds a line: {replied} > {plain}"
        );
        assert!(
            shortened > replied,
            "Show all adds a line: {shortened} > {replied}"
        );
    }

    #[test]
    fn the_text_wraps_inside_the_card() {
        let long = "word ".repeat(60);
        let wide = height(&long, 900.0);
        let narrow = height(&long, 240.0);
        assert!(wide > 0.0, "the card has a body");
        assert!(narrow > wide, "narrow {narrow} against wide {wide}");
    }

    #[test]
    fn a_sent_card_draws_two_mark_actions_and_no_other_state_does() {
        let sent = card(CardState::Sent)
            .on_mark_applied(())
            .on_answer(())
            .parts();
        assert!(sent.mark_actions, "Mark applied and Answer… on a Sent card");
        assert!(!sent.actions, "Edit and Delete stay a draft's");
        for state in [
            CardState::Draft,
            CardState::InSend,
            CardState::Applied,
            CardState::Answered,
        ] {
            let parts = card(state).on_mark_applied(()).on_answer(()).parts();
            assert!(!parts.mark_actions, "{state:?} offers no mark");
        }
        assert!(
            !card(CardState::Sent).parts().mark_actions,
            "without handlers"
        );
        let plain = laid_out(ReviewCommentCard::new("rename", CardState::Sent, LIGHT));
        let marked = laid_out(
            ReviewCommentCard::new("rename", CardState::Sent, LIGHT)
                .on_mark_applied(())
                .on_answer(()),
        );
        assert!(marked >= plain, "the actions sit in the head row");
    }

    #[test]
    fn a_highlighted_card_draws_the_outline_and_keeps_the_plain_card_size() {
        assert!(!card(CardState::Draft).parts().highlighted);
        assert!(card(CardState::Draft).highlighted(true).parts().highlighted);
        assert!(
            !card(CardState::Draft)
                .highlighted(false)
                .parts()
                .highlighted
        );
        let plain = laid_out(ReviewCommentCard::new("rename", CardState::Sent, LIGHT));
        let marked =
            laid_out(ReviewCommentCard::new("rename", CardState::Sent, LIGHT).highlighted(true));
        let off =
            laid_out(ReviewCommentCard::new("rename", CardState::Sent, LIGHT).highlighted(false));
        assert_eq!(off, plain, "false matches the plain card");
        assert_eq!(marked, plain, "the outline takes no room");
    }

    #[test]
    fn show_all_is_pressable_only_with_a_handler() {
        let cut = card(CardState::Answered).reply("not needed", true);
        assert!(!cut.parts().show_all_pressable);
        let pressable = card(CardState::Answered)
            .reply("not needed", true)
            .on_show_reply(());
        let parts = pressable.parts();
        assert!(parts.show_all && parts.show_all_pressable);
        let whole = card(CardState::Answered)
            .reply("not needed", false)
            .on_show_reply(());
        assert!(
            !whole.parts().show_all_pressable,
            "nothing to show when whole"
        );
    }
}
