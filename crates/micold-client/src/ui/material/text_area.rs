//! `TextArea` — a multi-line filled field (feature 482, research R15, contracts/changes-view.md C2).
//!
//! iced's `text_editor` is the editing engine (cursor, selection, IME, clipboard); this wraps it in
//! `FilledField`'s chrome — the `surface_container_highest` box rounded on top, the hover layer and
//! the bottom indicator that thickens while the editor has focus — and grows with its lines from one
//! field's height. Ctrl/Cmd+Enter sends `.on_submit`'s message.
//!
//! Builder form: `TextArea::new(&content, roles).placeholder("Comment").on_action(Msg::Edit)
//! .on_submit(Msg::Save).into()`.

use iced::advanced::text::highlighter::PlainText;
use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Layout, Renderer as _, Shell};
use iced::keyboard::{key, Key};
use iced::widget::text_editor::{self, Action, Binding, Content, KeyPress};
use iced::widget::TextEditor;
use iced::{Background, Color, Element, Event, Length, Padding, Rectangle, Size};

use micold_core::tokens::{spacing, Roles};

use super::style;

/// The height of an empty text area: one filled field's (§7.7).
pub const MIN_HEIGHT: f32 = 56.0;

/// The space between the field's edge and its text: a filled field's 16dp sides, and the top and
/// bottom that centre one 24dp line in [`MIN_HEIGHT`].
const PADDING: Padding = Padding {
    top: 16.0,
    right: spacing::MD,
    bottom: 16.0,
    left: spacing::MD,
};

/// The indicator's colour and thickness: thicker, in `primary`, while focused.
pub fn indicator(r: Roles, focused: bool) -> (Color, f32) {
    style::field_indicator(r, focused, false)
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
        let r = area.roles;
        let line = super::TypeRole::Body;
        let on_submit = area.on_submit;
        let mut editor = TextEditor::<PlainText, M, iced::Theme, iced::Renderer>::new(area.content)
            .placeholder(area.placeholder)
            .size(line.size())
            .line_height(line.line_height())
            .font(line.font())
            .padding(PADDING)
            .min_height(MIN_HEIGHT - PADDING.top - PADDING.bottom)
            .style(move |_theme, status| {
                let field = style::field_input(r)(
                    _theme,
                    if matches!(status, text_editor::Status::Disabled) {
                        iced::widget::text_input::Status::Disabled
                    } else {
                        iced::widget::text_input::Status::Active
                    },
                );
                text_editor::Style {
                    background: field.background,
                    border: field.border,
                    placeholder: field.placeholder,
                    value: field.value,
                    selection: field.selection,
                }
            })
            .key_binding(move |press: KeyPress| {
                let submit = matches!(press.key.as_ref(), Key::Named(key::Named::Enter))
                    && press.modifiers.command()
                    && matches!(press.status, text_editor::Status::Focused { .. });
                match (&on_submit, submit) {
                    (Some(message), true) => Some(Binding::Custom(message.clone())),
                    _ => Binding::from_key_press(press),
                }
            });
        if let Some(f) = area.on_action {
            editor = editor.on_action(f);
        }
        Element::new(Chrome {
            editor: editor.into(),
            roles: r,
        })
    }
}

/// `FilledField`'s container, hover layer and indicator around the editor, focus read off the
/// editor's own state.
struct Chrome<'a, M> {
    editor: Element<'a, M>,
    roles: Roles,
}

impl<M> Chrome<'_, M> {
    fn focused(tree: &Tree) -> bool {
        tree.children
            .first()
            .map(|editor| {
                editor
                    .state
                    .downcast_ref::<text_editor::State<PlainText>>()
                    .is_focused()
            })
            .unwrap_or(false)
    }
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for Chrome<'a, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.editor)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.editor));
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .editor
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(child.size(), vec![child])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style_: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let r = self.roles;
        let container = style::field_container(r)(theme);
        let quad = renderer::Quad {
            bounds,
            border: container.border,
            ..Default::default()
        };
        renderer.fill_quad(
            quad,
            container
                .background
                .unwrap_or(Background::Color(Color::TRANSPARENT)),
        );
        if cursor.is_over(bounds) {
            renderer.fill_quad(
                quad,
                Background::Color(style::state_fill(
                    style::color(r.on_surface),
                    micold_core::tokens::state::HOVER,
                )),
            );
        }
        let (colour, thickness) = indicator(r, Self::focused(tree));
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    y: bounds.y + bounds.height - thickness,
                    height: thickness,
                    ..bounds
                },
                ..Default::default()
            },
            Background::Color(colour),
        );
        if let Some(child) = layout.children().next() {
            self.editor.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style_,
                child,
                cursor,
                viewport,
            );
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if let Some(child) = layout.children().next() {
            self.editor.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.editor
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, operation);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map(|child| {
                self.editor.as_widget().mouse_interaction(
                    &tree.children[0],
                    child,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .unwrap_or_default()
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
        let mut element: Element<'_, ()> =
            TextArea::new(content, LIGHT).placeholder("Comment").into();
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
