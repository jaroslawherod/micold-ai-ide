//! `ResizeHandle` — the draggable edge between a panel and the content beside it.
//!
//! Two things used to live outside this edge that were only ever about the edge itself.
//!
//! Its hover highlight was a track in the application's central animator, which meant the
//! application knew that a boundary line brightens when a pointer nears it. And its *drag* was
//! three messages and a `sidebar_dragging` flag, because a pointer leaves a 6px-wide target almost
//! immediately: the binary mounted a full-window transparent capture layer for the duration, so the
//! drag survived the pointer moving away.
//!
//! Neither is needed. A widget's `update` sees every mouse event, not only the ones over its own
//! bounds — so a handle that remembers it is being dragged can follow a pointer anywhere on screen
//! by itself. The capture layer, the flag, and the start/end messages all go; what remains is the
//! one message that carries a *decision* rather than a mechanism: the new width.

use std::marker::PhantomData;
use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{mouse, renderer, Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Size};
use micold_core::tokens::Roles;

use super::style;
use crate::ui::cdk::motion::Progress;

/// Total width of the handle: an invisible grab zone plus the rule at its right edge.
pub const WIDTH: f32 = 6.0;

/// The visible part — a hairline that reads as the panel's border, not as a control.
const RULE: f32 = 1.0;

/// How long the hover highlight takes to travel its full range — a deliberately gentle ramp, so
/// the edge warms as the pointer approaches rather than snapping on under it.
const HOVER: Duration = Duration::from_millis(800);

/// A draggable vertical edge. Builder form (Principle VIII):
/// `ResizeHandle::new(roles).on_resize(Message::SidebarWidthDragged).into()`.
pub struct ResizeHandle<'a, M> {
    roles: Roles,
    on_resize: Option<Box<dyn Fn(f32) -> M + 'a>>,
    marker: PhantomData<&'a M>,
}

impl<'a, M> ResizeHandle<'a, M> {
    /// An edge that highlights on hover but cannot be dragged until given [`Self::on_resize`].
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            on_resize: None,
            marker: PhantomData,
        }
    }

    /// Report the pointer's x position, in window coordinates, while the edge is being dragged.
    ///
    /// The width is the caller's to decide: this says where the pointer is, not how wide anything
    /// should now be. Clamping to a sensible minimum, snapping, refusing — all of that is the
    /// application's business, and none of it is presentation.
    pub fn on_resize(mut self, message: impl Fn(f32) -> M + 'a) -> Self {
        self.on_resize = Some(Box::new(message));
        self
    }
}

/// What the edge knows about itself: how lit it is, and whether it is currently being dragged.
struct State {
    hover: Progress,
    dragging: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            // Starts dark and unmoving: an edge that has never been approached must not animate
            // itself into existence on the first frame.
            hover: Progress::new(0.0),
            dragging: false,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ResizeHandle<'_, Message>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(WIDTH), Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(WIDTH, limits.max().height))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<State>();

        match step(event, over, state.dragging) {
            Step::Start => {
                state.dragging = true;
                shell.capture_event();
            }
            Step::Report(x) => {
                if let Some(to_message) = &self.on_resize {
                    shell.publish(to_message(x));
                }
                shell.capture_event();
            }
            Step::Stop => {
                state.dragging = false;
                shell.capture_event();
            }
            Step::Ignore => {}
        }

        // A drag holds the highlight lit even once the pointer has left: releasing somewhere else
        // should not make the edge look untouched while it is still being moved.
        let state = tree.state.downcast_mut::<State>();
        let lit = if over || state.dragging { 1.0 } else { 0.0 };
        let _ = state.hover.on_frame(event, lit, HOVER, shell);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.dragging || cursor.is_over(layout.bounds()) {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let lit = tree.state.downcast_ref::<State>().hover.value();

        // The grab zone is painted in the panel's own surface colour and sits on the left, so it
        // reads as part of the panel rather than as a gap beside it.
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    width: (bounds.width - RULE).max(0.0),
                    ..bounds
                },
                ..renderer::Quad::default()
            },
            style::color(self.roles.surface),
        );

        // The rule sits flush against the content on the right, and brightens toward the accent as
        // the pointer nears it. Blended in the renderer's colour space, not in 8-bit token space:
        // an 8-bit blend rounds at every step and would not reproduce the previous appearance.
        let from = style::separator(self.roles);
        let to = style::color(self.roles.primary);
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + bounds.width - RULE,
                    width: RULE,
                    ..bounds
                },
                ..renderer::Quad::default()
            },
            iced::Color {
                r: from.r + (to.r - from.r) * lit,
                g: from.g + (to.g - from.g) * lit,
                b: from.b + (to.b - from.b) * lit,
                a: from.a + (to.a - from.a) * lit,
            },
        );
    }
}

/// What an incoming event means to a handle that is, or is not, currently being dragged.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    /// A press landed on the edge: the drag begins.
    Start,
    /// The pointer moved mid-drag; the value is its x in window coordinates.
    Report(f32),
    /// The button came up: the drag is over.
    Stop,
    /// Nothing this handle cares about.
    Ignore,
}

/// The whole drag protocol, as a decision about one event.
///
/// Pulled out of [`Widget::update`] so it can be checked directly: the interesting cases are the
/// ones involving a pointer that is *not* over the handle, and those are precisely the ones that
/// are awkward to stage through a widget tree.
fn step(event: &Event, over: bool, dragging: bool) -> Step {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over => Step::Start,
        // Deliberately not gated on `over`. A pointer leaves a 6px-wide target almost at once, and
        // following it anyway is the entire reason the full-window capture layer is gone.
        Event::Mouse(mouse::Event::CursorMoved { position }) if dragging => {
            Step::Report(position.x)
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if dragging => Step::Stop,
        _ => Step::Ignore,
    }
}

impl<'a, Message, Theme, Renderer> From<ResizeHandle<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(handle: ResizeHandle<'a, Message>) -> Self {
        Element::new(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    fn press() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    fn release() -> Event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
    }

    fn moved(x: f32) -> Event {
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(x, 0.0),
        })
    }

    #[test]
    fn pressing_the_edge_starts_a_drag() {
        assert_eq!(step(&press(), true, false), Step::Start);
    }

    /// A press anywhere else is somebody else's press.
    #[test]
    fn pressing_elsewhere_does_nothing() {
        assert_eq!(step(&press(), false, false), Step::Ignore);
    }

    /// The property the whole design rests on, and the one the old full-window capture layer
    /// existed to provide: a drag keeps reporting once the pointer has left the handle. A 6px
    /// target is narrower than a single frame's pointer movement, so a drag gated on `over` would
    /// stop almost immediately.
    #[test]
    fn a_drag_follows_the_pointer_away_from_the_handle() {
        assert_eq!(step(&moved(300.0), false, true), Step::Report(300.0));
    }

    /// The other half: movement is only a resize while a drag is actually in progress. Without
    /// this the handle would rewrite the width on every idle mouse move that crossed the window.
    #[test]
    fn movement_without_a_drag_is_ignored() {
        assert_eq!(step(&moved(300.0), true, false), Step::Ignore);
        assert_eq!(step(&moved(300.0), false, false), Step::Ignore);
    }

    /// Releasing ends the drag wherever the pointer happens to be — the release that matters is
    /// usually nowhere near the handle.
    #[test]
    fn releasing_anywhere_ends_the_drag() {
        assert_eq!(step(&release(), false, true), Step::Stop);
        assert_eq!(step(&release(), true, true), Step::Stop);
    }

    /// A release with nothing in flight must not be claimed, or the handle would swallow clicks
    /// meant for whatever is beneath it.
    #[test]
    fn releasing_without_a_drag_is_ignored() {
        assert_eq!(step(&release(), true, false), Step::Ignore);
    }
}
