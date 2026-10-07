//! `TextArea` — a multi-line filled field (feature 482, research R15, contracts/changes-view.md C2).
//!
//! iced's `text_editor` is the editing engine (cursor, selection, IME, clipboard); this wraps it in
//! `FilledField`'s chrome — the `surface_container_highest` box rounded on top, the hover layer and
//! the bottom indicator that thickens while the editor has focus — and grows with its lines from one
//! field's height. Ctrl/Cmd+Enter sends `.on_submit`'s message.
//!
//! Builder form: `TextArea::new(&content, roles).placeholder("Comment").on_action(Msg::Edit)
//! .on_submit(Msg::Save).into()`.

use iced::widget::text_editor::{Action, Content};
use iced::widget::TextEditor;
use iced::{Color, Element};
use micold_core::tokens::Roles;

/// The height of an empty text area: one filled field's (§7.7).
pub const MIN_HEIGHT: f32 = 56.0;

/// The indicator's colour and thickness: thicker, in `primary`, while focused.
pub fn indicator(r: Roles, focused: bool) -> (Color, f32) {
    let _ = (r, focused);
    (Color::TRANSPARENT, 0.0)
}

/// A multi-line filled field over `content`.
pub struct TextArea<'a, M> {
    content: &'a Content,
    roles: Roles,
    placeholder: String,
    on_action: Option<Box<dyn Fn(Action) -> M + 'a>>,
    on_submit: Option<M>,
}

impl<'a, M: Clone + 'a> TextArea<'a, M> {
    /// An area over `content`, themed by `roles`; read-only until [`Self::on_action`].
    pub fn new(content: &'a Content, roles: Roles) -> Self {
        Self {
            content,
            roles,
            placeholder: String::new(),
            on_action: None,
            on_submit: None,
        }
    }

    /// What it shows while empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// The message an edit sends; the caller performs the action on its `Content`.
    pub fn on_action(mut self, f: impl Fn(Action) -> M + 'a) -> Self {
        self.on_action = Some(Box::new(f));
        self
    }

    /// The message Ctrl/Cmd+Enter sends.
    pub fn on_submit(mut self, message: M) -> Self {
        self.on_submit = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<TextArea<'a, M>> for Element<'a, M> {
    fn from(area: TextArea<'a, M>) -> Self {
        let _ = (area.roles, &area.placeholder, &area.on_action, &area.on_submit);
        TextEditor::<_, M, iced::Theme, iced::Renderer>::new(area.content)
            .height(0.0)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;
    use iced::Size;
    use micold_core::tokens::{anatomy, LIGHT};

    /// The height `content`'s area takes laid out `width` wide.
    fn height(content: &Content, width: f32) -> f32 {
        let mut element: Element<'_, ()> = TextArea::new(content, LIGHT)
            .placeholder("Comment")
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
    fn an_empty_area_with_its_placeholder_is_one_field_tall() {
        let empty = Content::new();
        assert_eq!(height(&empty, 400.0), MIN_HEIGHT);
    }

    #[test]
    fn the_area_grows_with_its_lines() {
        let one = Content::with_text("one line");
        let five = Content::with_text("one\ntwo\nthree\nfour\nfive");
        assert_eq!(height(&one, 400.0), MIN_HEIGHT, "one line fits the field");
        assert!(
            height(&five, 400.0) >= MIN_HEIGHT + 4.0 * 16.0,
            "five lines: {}",
            height(&five, 400.0)
        );
    }

    #[test]
    fn focus_thickens_the_indicator_in_primary() {
        let (idle_colour, idle) = indicator(LIGHT, false);
        let (focused_colour, focused) = indicator(LIGHT, true);
        assert_eq!(idle, anatomy::text_field::INDICATOR);
        assert_eq!(focused, anatomy::text_field::INDICATOR_ACTIVE);
        assert!(focused > idle);
        assert_eq!(focused_colour, super::super::style::color(LIGHT.primary));
        assert_ne!(idle_colour, focused_colour);
    }
}
