//! `material` animation wrappers (Constitution Principle VIII) mimicking Angular Material
//! motion.
//!
//! iced 0.13 exposes no element opacity, so [`Fade`] approximates a fade by compositing a
//! scrim of the surrounding surface color over its child via `fill_quad` (alpha = `1 -
//! progress`). [`Slide`] performs a real horizontal reveal using the renderer's
//! transformation/clip: it animates its own laid-out width and slides the child in from the
//! left, clipped to the visible area. Both are passthrough widgets — layout, events, and the
//! overlay are delegated to the child.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{Color, Element, Event, Length, Rectangle, Size, Vector};

// ---------------------------------------------------------------------------------------
// Fade
// ---------------------------------------------------------------------------------------

/// Wrap `content` in a fade: `progress` 1.0 is fully visible, 0.0 fully faded to `backdrop`.
/// The backdrop should match the surface the content sits on (there is no true opacity in
/// iced 0.13, so this composites a scrim over the child).
pub fn fade<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    progress: f32,
    backdrop: Color,
) -> Element<'a, Message> {
    Fade {
        content: content.into(),
        progress: progress.clamp(0.0, 1.0),
        backdrop,
    }
    .into()
}

struct Fade<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    progress: f32,
    backdrop: Color,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Fade<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
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
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> iced::event::Status {
        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
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
        renderer: &mut Renderer,
        theme: &Theme,
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
        let alpha = 1.0 - self.progress;
        if alpha > 0.001 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    ..Default::default()
                },
                iced::Background::Color(Color {
                    a: alpha,
                    ..self.backdrop
                }),
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Fade<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(fade: Fade<'a, Message, Theme, Renderer>) -> Self {
        Element::new(fade)
    }
}

// ---------------------------------------------------------------------------------------
// Slide (horizontal reveal from the left)
// ---------------------------------------------------------------------------------------

/// Wrap `content` in a horizontal slide reveal: `progress` 1.0 is fully expanded, 0.0 fully
/// collapsed (zero width). The child slides in from the left, clipped to the visible width.
pub fn slide<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    progress: f32,
) -> Element<'a, Message> {
    Slide {
        content: content.into(),
        progress: progress.clamp(0.0, 1.0),
    }
    .into()
}

struct Slide<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    progress: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Slide<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        let inner = self.content.as_widget().size();
        Size::new(Length::Shrink, inner.height)
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits);
        let full = child.size();
        let width = (full.width * self.progress).max(0.0);
        // Reserve the animated width; slide the child left so it reveals from the edge.
        let child = child.translate(Vector::new(-(full.width - width), 0.0));
        layout::Node::with_children(Size::new(width, full.height), vec![child])
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget()
                .operate(&mut tree.children[0], child, renderer, operation);
        }
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> iced::event::Status {
        match layout.children().next() {
            Some(child) => self.content.as_widget_mut().on_event(
                &mut tree.children[0],
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            ),
            None => iced::event::Status::Ignored,
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        match layout.children().next() {
            Some(child) => self.content.as_widget().mouse_interaction(
                &tree.children[0],
                child,
                cursor,
                viewport,
                renderer,
            ),
            None => mouse::Interaction::None,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        // Clip to the (animated) visible width so the sliding child never overflows.
        renderer.with_layer(layout.bounds(), |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child = layout.children().next()?;
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], child, renderer, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Slide<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(slide: Slide<'a, Message, Theme, Renderer>) -> Self {
        Element::new(slide)
    }
}
