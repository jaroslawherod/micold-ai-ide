//! `ReviewCommentCard` — one review comment under its anchor row (feature 482,
//! contracts/changes-view.md C3).
//!
//! The comment's text, its state (Pending / Sent), an Outdated tag when the lines it quotes changed,
//! and Edit / Delete for a pending comment that is not in a send.
//!
//! Builder form: `ReviewCommentCard::new(&text, CardState::Pending, roles).outdated(false)
//! .on_edit(Msg::Edit(id)).on_delete(Msg::Delete(id)).into()`.

use iced::widget::Space;
use iced::Element;
use micold_core::tokens::Roles;

/// Where a comment is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    /// Not sent, editable.
    Pending,
    /// Pending but in a send under way: not editable.
    InSend,
    /// Delivered to a session.
    Sent,
}

/// The state label.
pub const PENDING: &str = "Pending";
/// The state label once delivered.
pub const SENT: &str = "Sent";
/// The tag of a comment whose quoted lines changed.
pub const OUTDATED: &str = "Outdated";

/// What a card shows besides its text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Parts {
    /// The state label.
    pub label: &'static str,
    /// The Outdated tag.
    pub outdated: bool,
    /// Edit and Delete.
    pub actions: bool,
}

/// One comment's card.
pub struct ReviewCommentCard<'a, M> {
    text: &'a str,
    state: CardState,
    roles: Roles,
    outdated: bool,
    on_edit: Option<M>,
    on_delete: Option<M>,
}

impl<'a, M: Clone + 'a> ReviewCommentCard<'a, M> {
    /// The card of a comment reading `text` in `state`, themed by `roles`.
    pub fn new(text: &'a str, state: CardState, roles: Roles) -> Self {
        Self {
            text,
            state,
            roles,
            outdated: false,
            on_edit: None,
            on_delete: None,
        }
    }

    /// Show the Outdated tag.
    pub fn outdated(mut self, outdated: bool) -> Self {
        self.outdated = outdated;
        self
    }

    /// The message Edit sends; shown only on a pending comment not in a send.
    pub fn on_edit(mut self, message: M) -> Self {
        self.on_edit = Some(message);
        self
    }

    /// The message Delete sends; shown only on a pending comment not in a send.
    pub fn on_delete(mut self, message: M) -> Self {
        self.on_delete = Some(message);
        self
    }

    /// What the card shows besides its text.
    pub fn parts(&self) -> Parts {
        Parts::default()
    }
}

impl<'a, M: Clone + 'a> From<ReviewCommentCard<'a, M>> for Element<'a, M> {
    fn from(card: ReviewCommentCard<'a, M>) -> Self {
        let _ = (card.text, card.roles);
        Space::new().into()
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
        let mut element: Element<'_, ()> = ReviewCommentCard::new(text, CardState::Pending, LIGHT)
            .on_edit(())
            .on_delete(())
            .into();
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
    fn a_pending_card_has_edit_and_delete() {
        let parts = card(CardState::Pending).parts();
        assert_eq!(parts.label, PENDING);
        assert!(parts.actions);
        assert!(!parts.outdated);
    }

    #[test]
    fn a_sent_card_and_one_in_a_send_have_no_actions() {
        let sent = card(CardState::Sent).parts();
        assert_eq!((sent.label, sent.actions), (SENT, false));
        let in_send = card(CardState::InSend).parts();
        assert_eq!((in_send.label, in_send.actions), (PENDING, false));
    }

    #[test]
    fn an_outdated_card_shows_its_tag() {
        assert!(card(CardState::Sent).outdated(true).parts().outdated);
    }

    #[test]
    fn the_text_wraps_inside_the_card() {
        let long = "word ".repeat(60);
        let wide = height(&long, 900.0);
        let narrow = height(&long, 240.0);
        assert!(wide > 0.0, "the card has a body");
        assert!(narrow > wide, "narrow {narrow} against wide {wide}");
    }
}
