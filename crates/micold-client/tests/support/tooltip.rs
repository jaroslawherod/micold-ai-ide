//! A `cdk::tooltip::Tooltip` driven without a window (feature 038).
//!
//! The tooltip's rest mode is a rule about events in time: a cursor, a clock, a press. This hands a
//! real tooltip those events one at a time and reports what it asked the runtime for, so
//! `tooltip_rest_glue.rs` and `idle_requests_no_frames.rs` hold the same widget the same way.
//!
//! **The clock.** A redraw event carries its instant, and that is the one a test controls. A mouse
//! event carries none, so the widget reads the wall clock for it. Cases that need an exact instant
//! therefore move the cursor *with a redraw* — the widget observes the cursor on every redraw, which
//! is also how it sees a row that moved under a still cursor.

use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::Tree;
use iced::advanced::{mouse, renderer, Clipboard, Shell, Widget};
use iced::widget::{container, text};
use iced::{keyboard, window, Element, Event, Length, Point, Rectangle, Size, Vector};

use micold_client::ui::cdk::tooltip::{Position, Tooltip};

use super::layout::renderer;

/// The delay the issue list uses (FR-015).
pub const DELAY: Duration = Duration::from_secs(3);

/// The trigger's size: wide enough to move a cursor about inside it.
pub const TRIGGER: Size = Size::new(200.0, 40.0);

/// The window the tooltip is laid out in.
pub const WINDOW: Size = Size::new(400.0, 400.0);

/// What the trigger says it received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// A left press over the trigger.
    Pressed,
    /// A key press.
    Key,
}

/// The trigger: a fixed-size box that reports the presses and keys that reach it.
struct Probe;

impl<Theme, Renderer> Widget<Msg, Theme, Renderer> for Probe
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(TRIGGER.width), Length::Fixed(TRIGGER.height))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(TRIGGER)
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        _viewport: &Rectangle,
    ) {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(layout.bounds()) =>
            {
                shell.publish(Msg::Pressed);
            }
            Event::Keyboard(keyboard::Event::KeyPressed { .. }) => shell.publish(Msg::Key),
            _ => {}
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }
}

/// A tooltip over the probe: in rest mode when `rest` is given, describing `subject` when given.
pub fn tooltip(rest: Option<Duration>, subject: Option<u64>) -> Tooltip<'static, Msg> {
    let mut tip = Tooltip::new(
        Element::new(Probe),
        container(text("panel")),
        Position::Bottom,
    );
    if let Some(delay) = rest {
        tip = tip.after_rest(delay);
    }
    if let Some(key) = subject {
        tip = tip.subject(key);
    }
    tip
}

/// What one event made the tooltip ask for.
#[derive(Debug)]
pub struct Seen {
    /// What the trigger published.
    pub messages: Vec<Msg>,
    /// The frame request left on the shell: `Wait` is none at all.
    pub redraw: window::RedrawRequest,
}

/// A tooltip and the widget tree it keeps its state in.
///
/// The tree is built once and kept across rebuilds on purpose: what is under test is state that
/// outlives the view that made it.
pub struct Driven {
    tip: Tooltip<'static, Msg>,
    tree: Tree,
    renderer: iced::Renderer,
    /// Where the trigger is laid out. Moving it is a list that scrolled.
    pub at: Point,
}

impl Driven {
    /// `tip` in a fresh tree, its trigger at `(50, 50)`.
    pub fn new(tip: Tooltip<'static, Msg>) -> Self {
        let tree = Tree {
            tag: tip.tag(),
            state: tip.state(),
            children: tip.children(),
        };
        Self {
            tip,
            tree,
            renderer: renderer(),
            at: Point::new(50.0, 50.0),
        }
    }

    /// The view was rebuilt: `tip` takes the place of the old one, on the same tree.
    pub fn rebuild(&mut self, tip: Tooltip<'static, Msg>) {
        self.tip = tip;
        self.tip.diff(&mut self.tree);
    }

    fn node(&mut self) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, WINDOW);
        self.tip
            .layout(&mut self.tree, &self.renderer, &limits)
            .move_to(self.at)
    }

    /// A cursor over the trigger, `dx` right of its left edge.
    pub fn over(&self, dx: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(self.at.x + dx, self.at.y + TRIGGER.height / 2.0))
    }

    /// Hand the tooltip one event with the cursor at `cursor`.
    pub fn send(&mut self, event: Event, cursor: mouse::Cursor) -> Seen {
        let node = self.node();
        let mut messages = Vec::new();
        let redraw = {
            let mut shell = Shell::new(&mut messages);
            self.tip.update(
                &mut self.tree,
                &event,
                Layout::new(&node),
                cursor,
                &self.renderer,
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &Rectangle::with_size(WINDOW),
            );
            shell.redraw_request()
        };
        Seen { messages, redraw }
    }

    /// A redraw at `now` with the cursor at `cursor`.
    pub fn frame(&mut self, now: Instant, cursor: mouse::Cursor) -> Seen {
        self.send(Event::Window(window::Event::RedrawRequested(now)), cursor)
    }

    /// The cursor moved to `cursor`, now by the wall clock.
    pub fn moved(&mut self, cursor: mouse::Cursor) -> Seen {
        let position = cursor.position().unwrap_or(Point::ORIGIN);
        self.send(Event::Mouse(mouse::Event::CursorMoved { position }), cursor)
    }

    /// A left press with the cursor at `cursor`.
    pub fn pressed(&mut self, cursor: mouse::Cursor) -> Seen {
        self.send(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            cursor,
        )
    }

    /// Whether the panel is showing.
    pub fn is_open(&mut self) -> bool {
        let node = self.node();
        let viewport = Rectangle::with_size(WINDOW);
        let present = {
            let layout = Layout::new(&node);
            self.tip
                .overlay(
                    &mut self.tree,
                    layout,
                    &self.renderer,
                    &viewport,
                    Vector::ZERO,
                )
                .is_some()
        };
        present
    }

    /// Rest the cursor on the trigger from `start` until the panel opens, by redraws alone.
    pub fn rest_until_open(&mut self, start: Instant) -> Instant {
        let cursor = self.over(20.0);
        self.frame(start, cursor);
        let opened = start + DELAY;
        self.frame(opened, cursor);
        assert!(self.is_open(), "the fixture opens after the delay at rest");
        opened
    }
}
