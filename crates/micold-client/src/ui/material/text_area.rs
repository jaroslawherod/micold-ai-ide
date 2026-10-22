//! `TextArea` — a multi-line filled field (feature 482, research R15, contracts/changes-view.md C2).
//!
//! iced's `text_editor` is the editing engine (cursor, selection, IME, clipboard); this wraps it in
//! `FilledField`'s chrome — the `surface_container_highest` box rounded on top, the hover layer and
//! the bottom indicator that thickens while the editor has focus — and grows with its lines from one
//! field's height. Ctrl/Cmd+Enter sends `.on_submit`'s message.
//!
//! Builder form: `TextArea::new(&content, roles).placeholder("Comment").on_action(Msg::Edit)
//! .on_submit(Msg::Save).into()`.
//!
//! Feature 941 (FR-008): `.max_height(…)` stops the growth at a height, past which the lines scroll
//! inside the area; `.key_binding(…)` replaces the default keys, for a composer whose Enter sends
//! and whose Up and Down step through its history.

use crate::ui::cdk::passthrough::forward_to_child;
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
    key_binding: Option<KeyBindingFn<'a, M>>,
    max_height: Option<f32>,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

/// What a key press does, as `.key_binding(…)` decides it.
type KeyBindingFn<'a, M> = Box<dyn Fn(KeyPress) -> Option<Binding<M>> + 'a>;

impl<'a, M: Clone + 'a> TextArea<'a, M> {
    /// An area over `content`, themed by `roles`; read-only until [`Self::on_action`].
    pub fn new(content: &'a Content, roles: Roles) -> Self {
        Self {
            content,
            roles,
            placeholder: String::new(),
            on_action: None,
            on_submit: None,
            key_binding: None,
            max_height: None,
            on_focus_change: None,
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

    /// Decide every key press with `f` instead of the editor's default bindings (and
    /// [`Self::on_submit`]'s Ctrl/Cmd+Enter): `None` ignores the key, and
    /// `Binding::from_key_press(press)` keeps the default for it.
    pub fn key_binding(mut self, f: impl Fn(KeyPress) -> Option<Binding<M>> + 'a) -> Self {
        self.key_binding = Some(Box::new(f));
        self
    }

    /// Report the editor gaining (`true`) and losing (`false`) the keyboard, so the application
    /// knows a field holds it: without the report a window that gives keys to its terminal takes
    /// the focus back on the first key (feature 941, `KeyboardElsewhere`).
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }

    /// Grow with the lines up to `height`, the whole field's, then hold it and scroll the lines
    /// inside (FR-008). Never below [`MIN_HEIGHT`].
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height.max(MIN_HEIGHT));
        self
    }
}

impl<'a, M: Clone + 'a> From<TextArea<'a, M>> for Element<'a, M> {
    fn from(area: TextArea<'a, M>) -> Self {
        let r = area.roles;
        let line = super::TypeRole::Body;
        let on_submit = area.on_submit;
        let key_binding = area.key_binding;
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
                if let Some(f) = &key_binding {
                    return f(press);
                }
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
            max_height: area.max_height,
            on_focus_change: area.on_focus_change,
        })
    }
}

/// `FilledField`'s container, hover layer and indicator around the editor, focus read off the
/// editor's own state.
struct Chrome<'a, M> {
    editor: Element<'a, M>,
    roles: Roles,
    /// The height growth stops at, the field's whole height.
    max_height: Option<f32>,
    /// What the editor's focus changing sends.
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

/// The editor's focus as last reported.
#[derive(Default)]
struct Reported(bool);

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
    forward_to_child!(editor, M, iced::Theme, iced::Renderer;
        children, diff);

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Reported>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Reported::default())
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
        // Held at the limit, the editor is laid out in the limit's room — its own viewport, which
        // scrolls — and its node cut to the limit: the editor's layout adds its padding on top of
        // the room it was given, so the room alone would overshoot by the padding.
        let limits = match self.max_height {
            Some(max) => limits.max_height(max),
            None => *limits,
        };
        let child = self
            .editor
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        let child = match self.max_height {
            Some(max) if child.size().height > max => {
                layout::Node::new(Size::new(child.size().width, max))
            }
            _ => child,
        };
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
        // After the editor: the event that moves its focus is the one it has just handled.
        if let Some(on_focus_change) = &self.on_focus_change {
            let focused = Self::focused(tree);
            let reported = tree.state.downcast_mut::<Reported>();
            if reported.0 != focused {
                reported.0 = focused;
                shell.publish(on_focus_change(focused));
            }
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
        laid_out_height(TextArea::new(content, LIGHT).placeholder("Comment"), width)
    }

    /// The height `area` takes laid out `width` wide.
    fn laid_out_height(area: TextArea<'_, ()>, width: f32) -> f32 {
        let mut element: Element<'_, ()> = area.into();
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

    fn lines(n: usize) -> Content {
        let text = (1..=n).map(|i| format!("line {i}")).collect::<Vec<_>>();
        Content::with_text(&text.join("\n"))
    }

    /// FR-008: below its limit the area grows as before; past it, it holds the limit.
    #[test]
    fn past_its_max_height_the_area_holds_it() {
        let limited = |content: &Content| {
            laid_out_height(TextArea::new(content, LIGHT).max_height(200.0), 400.0)
        };
        let (three, forty) = (lines(3), lines(40));
        assert_eq!(limited(&three), height(&three, 400.0), "under the limit");
        assert!(limited(&three) < 200.0);
        assert_eq!(limited(&forty), 200.0, "the limit holds");
        assert!(height(&forty, 400.0) > 200.0, "unlimited it would grow");
        assert_eq!(
            laid_out_height(TextArea::new(&three, LIGHT).max_height(10.0), 400.0),
            MIN_HEIGHT,
            "never below one field"
        );
    }

    /// FR-008: the wheel over a held area scrolls its lines inside it.
    #[test]
    fn the_wheel_scrolls_a_held_area_inside() {
        use super::super::test_support::Mounted;
        use iced::advanced::mouse;
        let forty = lines(40);
        let area =
            TextArea::new(&forty, LIGHT)
                .max_height(200.0)
                .on_action(|action| match action {
                    Action::Scroll { .. } => "scroll".to_string(),
                    _ => "other".to_string(),
                });
        let mut m = Mounted::new_in(area, Size::new(400.0, 600.0));
        let inside = iced::Point::new(100.0, 100.0);
        let out = m.send(
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -3.0 },
            }),
            mouse::Cursor::Available(inside),
        );
        assert_eq!(out, vec!["scroll".to_string()]);
    }

    /// FR-008: a scroll performed on a held area's content moves its lines inside it — what is
    /// drawn changes — and the area keeps its height.
    #[test]
    fn a_scrolled_held_area_draws_other_lines_at_the_same_height() {
        use super::super::test_support::Mounted;
        let drawn = |content: &Content| {
            let area = TextArea::<String>::new(content, LIGHT).max_height(200.0);
            let mut m = Mounted::new_in(area, Size::new(400.0, 600.0));
            (m.drawn(Color::WHITE).0, m.container().height)
        };
        let mut forty = lines(40);
        let (before, height) = drawn(&forty);
        let (again, _) = drawn(&forty);
        assert_eq!(before, again, "drawing is repeatable");
        forty.perform(Action::Scroll { lines: 10 });
        let (after, scrolled_height) = drawn(&forty);
        assert_ne!(before, after, "the lines moved");
        assert_eq!(height, scrolled_height);
    }

    /// A key binding decides the keys: Enter sends instead of breaking the line, and a key it
    /// hands back to the default still edits.
    #[test]
    fn a_key_binding_replaces_the_default_keys() {
        use super::super::test_support::{named_key, Mounted};
        use iced::advanced::mouse;
        let content = Content::with_text("draft");
        let area = || {
            TextArea::new(&content, LIGHT)
                .on_action(|_| "edit".to_string())
                .key_binding(|press| match press.key.as_ref() {
                    Key::Named(key::Named::Enter) => Some(Binding::Custom("send".to_string())),
                    Key::Named(key::Named::Escape) => None,
                    _ => Binding::from_key_press(press),
                })
        };
        let mut m = Mounted::new(area());
        m.focus_next();
        let send = |m: &mut Mounted<'_>, k| m.send(named_key(k), mouse::Cursor::Unavailable);
        assert_eq!(send(&mut m, key::Named::Enter), vec!["send".to_string()]);
        assert!(send(&mut m, key::Named::Escape).is_empty(), "ignored");
        assert_eq!(
            send(&mut m, key::Named::Backspace),
            vec!["edit".to_string()],
            "the default still edits"
        );
    }

    /// Feature 941: a press in the area reports its focus, and a press outside it the loss, so the
    /// application knows a field holds the keyboard (without it, the first key typed in a
    /// structured session's composer went back to the window and was lost).
    #[test]
    fn a_press_reports_the_editors_focus_and_a_press_elsewhere_its_loss() {
        use super::super::test_support::Mounted;
        use iced::Point;
        let content = Content::with_text("draft");
        let mut m = Mounted::new(
            TextArea::new(&content, LIGHT)
                .on_action(|_| "edit".to_string())
                .on_focus_change(|focused| format!("focus={focused}")),
        );
        let inside = m.container().center();
        assert!(m.press(inside).contains(&"focus=true".to_string()));
        assert!(
            !m.press(inside).iter().any(|m| m.starts_with("focus=")),
            "reported once, not on every press"
        );
        let outside = Point::new(inside.x, m.container().y + m.container().height + 100.0);
        assert!(m.press(outside).contains(&"focus=false".to_string()));
    }
}
