//! Content that is at least as wide as the pane it scrolls in, and wider when it must be (#1466).
//!
//! A [`Scrollable`](crate::Scrollable) laid out along both axes gives its content unbounded width,
//! so a `Length::Fill` row shrinks to its text and a tint stops short of the pane. The pane's width
//! is known only when the scrollable itself is laid out, so [`Pane`] reads it there and hands it
//! to the [`Ruler`] inside: a zero-height leaf that is `max(pane, least)` wide. Rows that fill their
//! column stretch to the widest sibling, which is the ruler, so every row is as wide as the pane
//! when its text fits and as wide as its text when it does not.

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// The width of the [`Pane`] that was laid out last, shared with the [`Ruler`] inside it.
pub(crate) type PaneWidth = Rc<Cell<f32>>;

/// A leaf `height` tall and `max(pane width, least)` wide, drawing nothing.
pub(crate) struct Ruler {
    pane: PaneWidth,
    least: f32,
    height: f32,
}

impl Ruler {
    pub(crate) fn new(pane: &PaneWidth, least: f32, height: f32) -> Self {
        Self {
            pane: Rc::clone(pane),
            least,
            height,
        }
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Ruler {
    fn size(&self) -> Size<Length> {
        // Shrink, not `Fixed(height)`: a column drops a child whose size hint is a fixed zero, and
        // this one is zero tall at the top of a list and must still set the content's width.
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(self.pane.get().max(self.least), self.height))
    }

    fn draw(
        &self,
        _tree: &Tree,
        _renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }
}

impl<'a, M: 'a> From<Ruler> for Element<'a, M> {
    fn from(ruler: Ruler) -> Self {
        Element::new(ruler)
    }
}

/// Wraps a scrollable and tells its [`Ruler`] how wide the pane is before laying the scrollable out.
pub(crate) struct Pane<'a, M> {
    pane: PaneWidth,
    content: Element<'a, M>,
}

impl<'a, M> Pane<'a, M> {
    pub(crate) fn new(pane: PaneWidth, content: Element<'a, M>) -> Self {
        Self { pane, content }
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Pane<'_, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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
        let width = limits.max().width;
        self.pane.set(if width.is_finite() { width } else { 0.0 });
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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

impl<'a, M: 'a> From<Pane<'a, M>> for Element<'a, M> {
    fn from(pane: Pane<'a, M>) -> Self {
        Element::new(pane)
    }
}
