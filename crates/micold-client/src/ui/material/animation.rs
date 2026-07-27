//! `material` animation wrappers (Constitution Principle VIII) mimicking Angular Material
//! motion.
//!
//! iced exposes no element opacity (still true as of 0.14 — `widget::opaque` gates events, it
//! does not blend), so [`Fade`] approximates a fade by compositing a
//! scrim of the surrounding surface color over its child via `fill_quad` (alpha = `1 -
//! progress`). [`Slide`] performs a real horizontal reveal using the renderer's
//! transformation/clip: it animates its own laid-out width and slides the child in from the
//! left, clipped to the visible area. Both are passthrough widgets — layout, events, and the
//! overlay are delegated to the child.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{Color, Element, Event, Length, Rectangle, Size, Transformation, Vector};

/// Report the cursor as [`mouse::Cursor::Unavailable`] whenever it sits outside `bounds`.
/// Used by widgets (like [`Expand`]) whose child keeps its full, untranslated layout even
/// while visually clipped/collapsed — without this, a child's own hit-test would still see
/// the real cursor position and could respond to clicks landing outside the widget's actual
/// (animated) visible area.
fn clip_cursor_to(cursor: mouse::Cursor, bounds: Rectangle) -> mouse::Cursor {
    if cursor.is_over(bounds) {
        cursor
    } else {
        mouse::Cursor::Unavailable
    }
}

/// Shared `children`/`diff`/`operate` for a single-child widget whose `layout()` wraps its
/// child in `layout::Node::with_children(outer_size, vec![child])` (as [`Slide`] and
/// [`Expand`] both do) rather than returning the child's node directly (as [`Fade`]/[`Scale`]
/// do, so they don't need this — they forward to the child using their own `layout` as-is).
/// `on_event`/`mouse_interaction`/`draw`/`overlay` stay hand-written per widget: `Slide`
/// forwards the raw cursor (its translate-based reveal already moves the hidden child out of
/// the interactive area), while `Expand` clips the cursor to its own bounds first (its
/// top-anchored reveal never translates the child, so it needs that instead) — a real
/// difference, not incidental duplication.
macro_rules! wrapped_child_widget {
    () => {
        fn children(&self) -> Vec<Tree> {
            vec![Tree::new(&self.content)]
        }

        fn diff(&self, tree: &mut Tree) {
            tree.diff_children(std::slice::from_ref(&self.content));
        }

        fn operate(
            &mut self,
            tree: &mut Tree,
            layout: Layout<'_>,
            renderer: &Renderer,
            operation: &mut dyn Operation,
        ) {
            if let Some(child) = layout.children().next() {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    child,
                    renderer,
                    operation,
                );
            }
        }
    };
}

// ---------------------------------------------------------------------------------------
// Fade
// ---------------------------------------------------------------------------------------

/// Wrap `content` in a fade: `progress` 1.0 is fully visible, 0.0 fully faded to `backdrop`.
/// The backdrop should match the surface the content sits on (iced still has no true opacity,
/// so this composites a scrim over the child).
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
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
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
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
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
    wrapped_child_widget!();

    fn size(&self) -> Size<Length> {
        let inner = self.content.as_widget().size();
        Size::new(Length::Shrink, inner.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let full = child.size();
        let width = (full.width * self.progress).max(0.0);
        // Reserve the animated width; slide the child left so it reveals from the edge.
        let child = child.translate(Vector::new(-(full.width - width), 0.0));
        layout::Node::with_children(Size::new(width, full.height), vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().update(
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
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
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

// ---------------------------------------------------------------------------------------
// Expand (vertical accordion reveal, top-anchored)
// ---------------------------------------------------------------------------------------

/// Wrap `content` in a vertical accordion reveal: `progress` 1.0 is fully expanded, 0.0 fully
/// collapsed (zero height). Unlike [`slide`] (which anchors the reveal to the trailing edge,
/// suited to a panel sliding out from behind a fixed handle), `expand` anchors to the *top* —
/// the child is never translated, so it always reveals top-down, growing the space below it
/// (feature 009's sidebar filter accordion).
pub fn expand<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    progress: f32,
) -> Element<'a, Message> {
    Expand {
        content: content.into(),
        progress: progress.clamp(0.0, 1.0),
    }
    .into()
}

struct Expand<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    progress: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Expand<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    wrapped_child_widget!();

    fn size(&self) -> Size<Length> {
        let inner = self.content.as_widget().size();
        Size::new(inner.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let full = child.size();
        let height = (full.height * self.progress).max(0.0);
        // No translation: the child's top edge stays put, so growing height reveals it
        // top-down (an accordion opening below its trigger), unlike `Slide`'s edge anchor.
        layout::Node::with_children(Size::new(full.width, height), vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                clip_cursor_to(cursor, layout.bounds()),
                renderer,
                clipboard,
                shell,
                viewport,
            );
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
                clip_cursor_to(cursor, layout.bounds()),
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
        // Clip to the (animated) visible height so the growing child never overflows.
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
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Expand<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(expand: Expand<'a, Message, Theme, Renderer>) -> Self {
        Element::new(expand)
    }
}

// ---------------------------------------------------------------------------------------
// Scale (scale about center)
// ---------------------------------------------------------------------------------------

/// The scale applied at `progress` 0.0 — a subtle Material dialog "lift" (grows to full size
/// as it enters, shrinks slightly as it leaves). Kept close to 1.0 so it reads as a lift, not
/// a zoom.
const MIN_SCALE: f32 = 0.96;

/// Wrap `content` in a scale-about-center transform: `progress` 1.0 renders at full size, 0.0
/// at [`MIN_SCALE`], linearly in between. A passthrough widget — layout, events, and the
/// overlay are delegated to the child; only drawing is transformed (via the renderer), so it
/// never reflows the layout around it.
pub fn scale<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    progress: f32,
) -> Element<'a, Message> {
    Scale {
        content: content.into(),
        progress: progress.clamp(0.0, 1.0),
    }
    .into()
}

struct Scale<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    progress: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Scale<'_, Message, Theme, Renderer>
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
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
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
        let scaling = MIN_SCALE + (1.0 - MIN_SCALE) * self.progress;
        // At full size, skip the transform layer entirely (identity — nothing to do).
        if (scaling - 1.0).abs() < 0.0001 {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
            return;
        }
        // Scale about the child's center: translate(c) · scale · translate(-c).
        let center = layout.bounds().center();
        let transformation = Transformation::translate(center.x, center.y)
            * Transformation::scale(scaling)
            * Transformation::translate(-center.x, -center.y);
        renderer.with_transformation(transformation, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Scale<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(scale: Scale<'a, Message, Theme, Renderer>) -> Self {
        Element::new(scale)
    }
}
