//! A press draws one state layer, ripple included — checked in rasterised pixels (BUG-014, FR-022a).
//!
//! The ripple is drawn by a wrapper over a button whose own style already draws a hover or pressed
//! layer, and neither can see the other. So the only honest place to ask "how many layers is this?"
//! is the frame: render a real `Button` through a press, frame by frame at stated instants, and read
//! the fill back.
//!
//! Every comparison is against another pixel from the same renderer rather than against a computed
//! opacity, so the answer does not depend on which colour space the rasteriser blends in.

use std::time::{Duration, Instant};

use iced::advanced::renderer::{Headless as _, Style};
use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Renderer as _};
use iced::{Color, Element, Event, Point, Rectangle, Size};
use micold_core::tokens::{self, Roles};

use crate::ui::cdk::ripple::Ripple as RippleState;

const WINDOW: Size = Size::new(240.0, 60.0);

/// One frame's spacing: the period the motion primitive itself assumes.
const FRAME: Duration = crate::ui::cdk::motion::FRAME;

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Dark)
}

fn button<'a>(r: Roles) -> Element<'a, String> {
    super::Button::filled("Save", r)
        .on_press("saved".to_string())
        .into()
}

/// A `Button` mounted with its tree, driven at explicit instants so a ripple's progress is the
/// test's to choose rather than the wall clock's.
struct Pressed<'a> {
    element: Element<'a, String>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
    start: Instant,
    now: Instant,
    pointer: mouse::Cursor,
}

impl Pressed<'_> {
    fn new() -> Self {
        let mut element = button(roles());
        let renderer = super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
        let start = Instant::now();
        let mut mounted = Self {
            element,
            tree,
            node,
            renderer,
            start,
            now: start,
            pointer: mouse::Cursor::Unavailable,
        };
        // iced's button has no status until its first redraw event, and draws a missing status as
        // `Disabled` — so without this, "at rest" would be the disabled fill and every distance
        // below would be measured from a colour the button never shows once on screen.
        mounted.frame();
        mounted
    }

    fn bounds(&self) -> Rectangle {
        self.node.bounds()
    }

    /// Inside the pill, in the leading padding — clear of the label's glyphs.
    fn leading(&self) -> Point {
        let b = self.bounds();
        Point::new(b.x + 10.0, b.center_y())
    }

    /// The same, at the other end.
    fn trailing(&self) -> Point {
        let b = self.bounds();
        Point::new(b.x + b.width - 10.0, b.center_y())
    }

    fn send(&mut self, event: Event) {
        let mut messages = Vec::new();
        let mut shell = iced::advanced::Shell::new(&mut messages);
        self.element.as_widget_mut().update(
            &mut self.tree,
            &event,
            Layout::new(&self.node),
            self.pointer,
            &self.renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
    }

    fn move_to(&mut self, at: Point) {
        self.pointer = mouse::Cursor::Available(at);
        self.send(Event::Mouse(mouse::Event::CursorMoved { position: at }));
        self.frame();
    }

    fn press(&mut self) {
        self.send(Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        )));
        self.frame();
    }

    fn release(&mut self) {
        self.send(Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        )));
        self.frame();
    }

    /// One frame tick: the event that advances the ripple and settles the button's status.
    fn frame(&mut self) {
        self.now += FRAME;
        self.send(Event::Window(iced::window::Event::RedrawRequested(
            self.now,
        )));
    }

    fn ripple(&self) -> &RippleState {
        // `Button` is the keyboard wrapper around the ripple around iced's button.
        self.tree.children[0].state.downcast_ref::<RippleState>()
    }

    /// Frames until the circle has grown to cover the element — the last frame at full strength.
    fn until_expanded(&mut self) {
        while self.ripple().expansion() < 1.0 {
            self.frame();
            assert!(
                self.now - self.start < Duration::from_secs(2),
                "the ripple never finished expanding"
            );
        }
        assert!(
            (self.ripple().strength() - 1.0).abs() < f32::EPSILON,
            "the ripple had begun to fade on the frame it arrived, so this would measure less than \
             its full layer"
        );
    }

    /// Frames until the ripple has released its state.
    fn until_settled(&mut self) {
        while !self.ripple().is_idle() {
            self.frame();
            assert!(
                self.now - self.start < Duration::from_secs(2),
                "the ripple never settled"
            );
        }
        // One more, so the button's status reflects the frame after the handoff.
        self.frame();
    }

    /// Render the current frame and read the pixel at `at`.
    fn pixel(&mut self, at: Point) -> [u8; 3] {
        let viewport = Rectangle::with_size(WINDOW);
        self.renderer.reset(viewport);
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            &iced::Theme::Dark,
            &Style::default(),
            Layout::new(&self.node),
            self.pointer,
            &viewport,
        );
        let pixels = self.renderer.screenshot(
            Size::new(WINDOW.width as u32, WINDOW.height as u32),
            1.0,
            Color::BLACK,
        );
        let i = ((at.y as u32 * WINDOW.width as u32 + at.x as u32) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    }
}

/// How far `pixel` has moved from `rest`, on the channel that moves most.
fn distance(pixel: [u8; 3], rest: [u8; 3]) -> i32 {
    (0..3)
        .map(|c| (pixel[c] as i32 - rest[c] as i32).abs())
        .max()
        .unwrap_or(0)
}

/// The filled button's fill at rest, and with its own pressed layer and nothing else.
fn references() -> ([u8; 3], [u8; 3]) {
    let mut held = Pressed::new();
    let at = held.leading();
    let rest = held.pixel(at);
    held.move_to(held.bounds().center());
    held.press();
    held.until_settled();
    let pressed = held.pixel(at);
    assert!(
        distance(pressed, rest) >= 8,
        "the pressed layer is not measurable here ({rest:?} at rest, {pressed:?} pressed), so every \
         comparison below would pass on nothing"
    );
    (rest, pressed)
}

/// Channel units of slack: rounding in the rasteriser, not a layer.
const SLACK: i32 = 2;

#[test]
fn a_held_press_draws_one_layer_under_its_ripple() {
    let (rest, pressed) = references();

    let mut held = Pressed::new();
    let at = held.leading();
    held.move_to(held.bounds().center());
    held.press();
    held.until_expanded();
    let under_ripple = held.pixel(at);

    assert!(
        distance(under_ripple, rest) <= distance(pressed, rest) + SLACK,
        "held pressed with the ripple covering it, the fill is {under_ripple:?} against \
         {pressed:?} for the pressed layer alone (rest {rest:?}) — the ripple's layer is drawn over \
         the button's own instead of replacing it (§5, FR-022a, BUG-014)"
    );
}

#[test]
fn a_click_draws_one_layer_under_its_ripple() {
    let (rest, pressed) = references();

    let mut clicked = Pressed::new();
    let at = clicked.leading();
    clicked.move_to(clicked.bounds().center());
    clicked.press();
    clicked.release();
    clicked.until_expanded();
    let under_ripple = clicked.pixel(at);

    assert!(
        distance(under_ripple, rest) <= distance(pressed, rest) + SLACK,
        "clicked and still hovered with the ripple covering it, the fill is {under_ripple:?}, \
         heavier than the pressed layer's {pressed:?} (rest {rest:?}) — the ripple is drawn over \
         the hover layer instead of replacing it (§5, FR-022a, BUG-014)"
    );
}

/// The remedy must not buy conformance by making the ripple invisible: it is only seen as a
/// difference against what is under it (FR-024a).
#[test]
fn the_ripple_still_stands_out_from_the_surface_it_crosses() {
    let (rest, pressed) = references();

    let mut early = Pressed::new();
    let origin = early.leading();
    let far = early.trailing();
    early.move_to(origin);
    early.press();
    early.frame();
    early.frame();
    assert!(
        early.ripple().expansion() < 0.5,
        "the circle has already grown most of the way, so the far end would be inside it"
    );
    let inside = early.pixel(origin);
    let outside = early.pixel(far);

    assert!(
        distance(inside, rest) - distance(outside, rest) >= distance(pressed, rest) / 2,
        "early in a press the circle ({inside:?}) is not distinguishable from the surface beyond \
         it ({outside:?}; rest {rest:?}, pressed {pressed:?}) — a ripple nobody can see is FR-024a \
         unmet"
    );
}

/// The ripple hands the surface back to its own layer without a visible step: whatever it fades to
/// is what the button draws on the next frame.
#[test]
fn the_ripple_hands_back_to_the_hover_layer_without_a_jump() {
    let (rest, pressed) = references();
    let step = (distance(pressed, rest) / 3).max(SLACK + 1);

    let mut clicked = Pressed::new();
    let at = clicked.leading();
    clicked.move_to(clicked.bounds().center());
    clicked.press();
    clicked.release();
    clicked.until_expanded();

    let mut previous = clicked.pixel(at);
    while !clicked.ripple().is_idle() {
        clicked.frame();
        let current = clicked.pixel(at);
        assert!(
            distance(current, previous) <= step,
            "the fill stepped from {previous:?} to {current:?} in one frame at {:?} — the ripple \
             fades to something other than the layer the button resumes",
            clicked.now - clicked.start
        );
        previous = current;
    }
    clicked.frame();
    let settled = clicked.pixel(at);
    assert!(
        distance(settled, previous) <= step,
        "the handoff frame stepped from {previous:?} to {settled:?}"
    );
}
