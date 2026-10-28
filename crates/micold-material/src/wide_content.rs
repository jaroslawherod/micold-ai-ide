//! Content that is at least as wide as the pane it scrolls in, and wider when it must be (#1466).
//!
//! A [`Scrollable`](crate::Scrollable) laid out along both axes gives its content unbounded width,
//! so a `Length::Fill` row shrinks to its text and a tint stops short of the pane. The pane's width
//! is known only when the scrollable itself is laid out, so [`Pane`] reads it there and hands it
//! to the [`Ruler`] inside: a zero-height leaf that is `max(pane, least)` wide. Rows that fill their
//! column stretch to the widest sibling, which is the ruler, so every row is as wide as the pane
//! when its text fits and as wide as its text when it does not.
//!
//! Content that must stay in view however far the rows scroll sideways — a comment card hung under
//! a row, with its actions at its right end — goes through [`within_pane`], which lays it out no
//! wider than the same pane, clear of the vertical scrollbar, and draws it (and takes its input)
//! shifted right by as far as the rows have scrolled, so it stays at the pane's left edge (#1519).
//! The shift is the viewport's offset from the wrapper, which assumes no ancestor between the
//! scrollable and the wrapper clips the viewport on the left.

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::InputMethod;
use iced::advanced::{mouse, overlay, renderer, Clipboard, Renderer as _, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::scrollable::BAR_FOOTPRINT;

/// The width of the [`Pane`] that was laid out last, shared with the [`Ruler`] and the
/// [`within_pane`] content inside it; 0 until the pane is laid out.
pub type PaneWidth = Rc<Cell<f32>>;

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

/// `content` laid out no wider than `pane`, the [`PaneWidth`] of the scrolling list it is inside,
/// less the vertical scrollbar that floats over the pane's right edge, and kept in view as the list
/// scrolls sideways (#1519). Before that pane is laid out (or when it was laid out at an unbounded
/// width, which records 0) it is laid out as it would be without this.
pub fn within_pane<'a, M: 'a>(
    pane: &PaneWidth,
    content: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    Pane {
        pane: Rc::clone(pane),
        content: content.into(),
        role: Role::Fit,
    }
    .into()
}

/// What a [`Pane`] does with its width as it is laid out.
enum Role {
    /// Record the width it is given, for the [`Ruler`] inside (the scrollable's wrapper).
    Measure,
    /// Hold the content to the width recorded and keep it at the viewport's left edge (content
    /// inside the scrollable).
    Fit,
}

/// Wraps a scrollable and tells its [`Ruler`] how wide the pane is before laying the scrollable out;
/// or, as [`within_pane`], holds content inside the scrollable to that width.
pub(crate) struct Pane<'a, M> {
    pane: PaneWidth,
    content: Element<'a, M>,
    role: Role,
}

impl<'a, M> Pane<'a, M> {
    pub(crate) fn new(pane: PaneWidth, content: Element<'a, M>) -> Self {
        Self {
            pane,
            content,
            role: Role::Measure,
        }
    }

    /// How far to shift the content right so it stays in view: as a [`Role::Fit`], by however far
    /// the `viewport` (where the content shows once `translation` is added to it) starts right of
    /// this wrapper's left edge, within the wrapper: never left of where the content was laid out,
    /// never past the wrapper's right edge.
    fn shift(&self, layout: Layout<'_>, viewport: &Rectangle, translation: Vector) -> Vector {
        match self.role {
            Role::Measure => Vector::ZERO,
            Role::Fit => {
                let (outer, inner) = (layout.bounds(), self.content_layout(layout).bounds());
                Vector::new(
                    (viewport.x - (outer.x + translation.x))
                        .min(outer.width - inner.width)
                        .max(0.0),
                    0.0,
                )
            }
        }
    }

    /// The layout of the content: a [`Role::Fit`] wraps it in a node of its own.
    fn content_layout<'l>(&self, layout: Layout<'l>) -> Layout<'l> {
        match self.role {
            Role::Measure => layout,
            Role::Fit => layout
                .children()
                .next()
                .expect("a fitted pane lays out its content as its one child"),
        }
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
        let fitted = match self.role {
            Role::Measure => {
                let width = limits.max().width;
                self.pane.set(if width.is_finite() { width } else { 0.0 });
                *limits
            }
            Role::Fit => match self.pane.get() - BAR_FOOTPRINT {
                pane if pane > 0.0 => {
                    let (min, max) = (limits.min(), limits.max());
                    layout::Limits::with_compression(
                        Size::new(min.width.min(pane), min.height),
                        Size::new(max.width.min(pane), max.height),
                        limits.compression(),
                    )
                }
                _ => *limits,
            },
        };
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &fitted);
        match self.role {
            Role::Measure => content,
            // A fitted content's wrapper spans the rows, so the content shifted along it is still
            // inside its parent's bounds, which a column checks against the viewport before drawing.
            Role::Fit => {
                let width = Some(limits.max().width)
                    .filter(|w| w.is_finite())
                    .unwrap_or(0.0);
                let size = Size::new(width.max(content.size().width), content.size().height);
                layout::Node::with_children(size, vec![content])
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
        let layout = self.content_layout(layout);
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
        let shift = self.shift(layout, viewport, Vector::ZERO);
        let layout = self.content_layout(layout);
        let had_input_method = shell.input_method().is_enabled();
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor - shift,
            renderer,
            clipboard,
            shell,
            &(*viewport - shift),
        );
        // A focused text field asks for the input method at its caret in layout coordinates; it is
        // drawn `shift` further right, so the IME window goes there too (as a scrollable does).
        if !had_input_method {
            if let InputMethod::Enabled { cursor, .. } = shell.input_method_mut() {
                *cursor = *cursor + shift;
            }
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
        let shift = self.shift(layout, viewport, Vector::ZERO);
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            self.content_layout(layout),
            cursor - shift,
            &(*viewport - shift),
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
        let shift = self.shift(layout, viewport, Vector::ZERO);
        renderer.with_translation(shift, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                self.content_layout(layout),
                cursor - shift,
                &(*viewport - shift),
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        let shift = self.shift(layout, viewport, translation);
        let layout = self.content_layout(layout);
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation + shift,
        )
    }
}

impl<'a, M: 'a> From<Pane<'a, M>> for Element<'a, M> {
    fn from(pane: Pane<'a, M>) -> Self {
        Element::new(pane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use iced::widget::{mouse_area, Space};
    use iced::Point;

    use crate::test_support;

    /// The rows' width, wider than the pane, as the scrollable lays its content out.
    const ROWS: f32 = 2_000.0;
    const PANE: f32 = 800.0;

    /// A click at `x` (in the rows' coordinates) on a pane-wide content [`within_pane`], with the
    /// rows scrolled `scrolled` px sideways: whether the content takes it.
    fn clicked(scrolled: f32, x: f32) -> bool {
        let pane: PaneWidth = Rc::new(Cell::new(PANE));
        let target = Space::new().width(Length::Fill).height(20.0);
        let mut element: Element<'_, ()> = within_pane(&pane, mouse_area(target).on_press(()));
        let renderer = test_support::renderer();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(ROWS, 600.0)),
        );
        let fitted = node.children()[0].size().width;
        assert_eq!(
            fitted,
            PANE - BAR_FOOTPRINT,
            "laid out clear of the scrollbar"
        );
        let at = Point::new(x, 10.0);
        let mut messages = Vec::new();
        element.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(at),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
            // What a scrollable scrolled `scrolled` px right shows of its content.
            &Rectangle::new(Point::new(scrolled, 0.0), Size::new(PANE, 600.0)),
        );
        !messages.is_empty()
    }

    /// #1519: content held to the pane stays at the pane's left edge however far the rows scroll
    /// sideways, so its right end (where a card's actions are) is where the pane's is.
    #[test]
    fn content_within_the_pane_stays_in_view_as_the_rows_scroll_sideways() {
        let right_end = PANE - BAR_FOOTPRINT - 4.0;
        assert!(clicked(0.0, right_end), "unscrolled");
        assert!(clicked(700.0, 700.0 + right_end), "scrolled 700 px");
        // Scrolled to the end, the content ends where the rows do, not past them.
        let end = ROWS - PANE;
        assert!(clicked(end, end + right_end), "scrolled to the end");
        assert!(!clicked(ROWS, ROWS + 1.0), "past the rows' right edge");
    }

    /// A pane-wide text field stand-in: asks for the input method at its left edge, in layout
    /// coordinates, as iced's text input does at its caret.
    struct Caret;

    impl<M> Widget<M, iced::Theme, iced::Renderer> for Caret {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fill, Length::Fixed(20.0))
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &iced::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(limits.resolve(Length::Fill, 20.0, Size::ZERO))
        }

        fn update(
            &mut self,
            _tree: &mut Tree,
            _event: &Event,
            layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _renderer: &iced::Renderer,
            _clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, M>,
            _viewport: &Rectangle,
        ) {
            let at = layout.bounds();
            shell.request_input_method(&InputMethod::<&str>::Enabled {
                cursor: Rectangle::new(at.position(), Size::new(1.0, at.height)),
                purpose: iced::advanced::input_method::Purpose::Normal,
                preedit: None,
            });
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

    /// #1519 review: a focused field in content held to the pane, with the rows scrolled 700 px
    /// sideways, puts the IME window where the field is drawn (700 px right of where it was laid
    /// out), not 700 px left of it.
    #[test]
    fn the_input_method_follows_content_within_the_pane_as_the_rows_scroll() {
        let pane: PaneWidth = Rc::new(Cell::new(PANE));
        let mut element: Element<'_, ()> = within_pane(&pane, Element::new(Caret));
        let renderer = test_support::renderer();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(ROWS, 600.0)),
        );
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::new(Point::new(700.0, 0.0), Size::new(PANE, 600.0)),
        );
        match shell.input_method() {
            InputMethod::Enabled { cursor, .. } => assert_eq!(cursor.x, 700.0),
            InputMethod::Disabled => panic!("the field asked for the input method"),
        }
    }
}
