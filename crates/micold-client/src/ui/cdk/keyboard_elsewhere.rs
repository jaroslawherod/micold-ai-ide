//! `KeyboardElsewhere` — the application's keyboard is somewhere no widget focus can see
//! (018 BUG-016, FR-022a).
//!
//! # Why this exists
//!
//! Two things say where the keyboard is, and they are held in different places. The focused
//! terminal is an application fact (`State::terminal_focused`): the pane is told, and routes keys by
//! it. A button's focus is a widget fact, held in `material::keyboard_focus`'s wrapper, and a pointer
//! press sets it. Handing the keyboard to the terminal changes the first and leaves the second
//! alone, so a button clicked on the way there — the terminal bar's `claude` tab, the sidebar's start
//! action — still holds focus while every key goes to the terminal. It then answers Enter and Space
//! as well: it re-sends its press and draws its focus ring beside the terminal's.
//!
//! A capture check in the wrapper covers only a button laid out *after* the pane. Rows and columns
//! deliver every event to every child in order, so a button laid out before the pane — the whole
//! sidebar is — sees the key first. This primitive is the order-independent half: wrapped around
//! the window's content and told that the terminal holds the keyboard, it clears every widget's
//! focus **before** any child sees the key, so nothing anywhere in the tree can answer it.
//!
//! # When it clears
//!
//! On every key press while the terminal holds the keyboard, which is the case that matters; and on
//! the first event of any kind after the terminal takes it, so a button that was showing its ring
//! (a traversal, then Enter on a start action) stops showing it without waiting for a keystroke.
//! Not on every event: an operation walks the whole tree, and the terminal redraws continually.
//!
//! # What it assumes
//!
//! No text field or select lives in the wrapped content while the terminal can hold the keyboard:
//! every one is in a dialog (an overlay, which `operate` does not reach) or in Settings (which turns
//! `terminal_focused` off). The flag is read from the last `view`, so a field placed in the window
//! itself — a sidebar filter, an inline rename — could lose a focus a click gave it to a key arriving
//! in the same event batch. Such a field needs its own answer here first.
//!
//! It holds no appearance and adds no layout node, which is why it lives in the `cdk`.

use iced::advanced::widget::operation::focusable;
use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell};
use iced::{keyboard, Element, Event, Length, Rectangle, Size, Vector};

/// Content that loses every widget focus while the keyboard is the terminal's.
pub struct KeyboardElsewhere<'a, M> {
    content: Element<'a, M>,
    elsewhere: bool,
}

impl<'a, M> KeyboardElsewhere<'a, M> {
    /// Wrap `content`. `elsewhere` is whether the application has given the keyboard to something
    /// that holds no widget focus — the focused terminal.
    pub fn new(content: impl Into<Element<'a, M>>, elsewhere: bool) -> Self {
        Self {
            content: content.into(),
            elsewhere,
        }
    }
}

/// Whether the keyboard was elsewhere as of the last event, so the first event after it moves there
/// can be told from the rest.
#[derive(Debug, Default)]
struct Seen {
    elsewhere: bool,
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for KeyboardElsewhere<'a, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Seen>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Seen::default())
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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
        let seen = tree.state.downcast_mut::<Seen>();
        let arrived = self.elsewhere && !seen.elsewhere;
        seen.elsewhere = self.elsewhere;
        let key = matches!(event, Event::Keyboard(keyboard::Event::KeyPressed { .. }));
        if self.elsewhere && (arrived || key) {
            let mut unfocus = focusable::unfocus::<()>();
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                &mut unfocus as &mut dyn Operation,
            );
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: 'a> From<KeyboardElsewhere<'a, M>> for Element<'a, M> {
    fn from(w: KeyboardElsewhere<'a, M>) -> Self {
        Element::new(w)
    }
}
