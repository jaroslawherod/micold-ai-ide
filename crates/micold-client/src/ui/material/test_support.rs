//! The headless renderer the in-crate component tests share.
//!
//! `material` is `pub(crate)`, so the component tests live inside the crate and cannot use
//! `tests/support/layout.rs`. This is the part of it they need: a CPU rasteriser that constructs
//! without a GPU, with the shipped faces loaded so text shaping does not reach for a system font.
//!
//! One copy rather than one per test module — a second renderer constructor would be a second place
//! for "which font is this measured against" to drift, and the answer has to be the same everywhere
//! or two tests measuring the same string disagree.

use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::Once;
use std::task::{Context, Poll, Waker};

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, mouse};
use iced::{Element, Event, Point, Rectangle, Size, Vector};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};

/// The light role set, which most component tests build against.
pub fn roles() -> Roles {
    tokens::roles(ColorScheme::Light)
}

/// The dark role set, for the tests that need the scheme their light twin cannot show.
pub fn dark_roles() -> Roles {
    tokens::roles(ColorScheme::Dark)
}

/// Poll a future known to be immediately ready.
///
/// The tiny-skia headless constructor does no I/O, so one poll suffices and no executor need be
/// pulled into the test scaffolding.
pub fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = Box::pin(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = Pin::as_mut(&mut f).poll(&mut cx) {
            return v;
        }
        std::hint::spin_loop();
    }
}

/// The CPU rasteriser, with every shipped face loaded — both Roboto weights **and the icon face**.
///
/// The icon face is every bit as load-bearing as the text ones, and it was missing: an icon is a
/// glyph from `Material Symbols Outlined`, shaped and measured like any other text. Without it a
/// 14dp icon resolved through the host's fallback and measured 8.4dp, so any in-crate test reading
/// an icon's box was reading whatever font the machine happened to offer. `tests/support/layout.rs`
/// had the identical gap and closed it; this is the same fix on the in-crate side, found by
/// BUG-003's §7.5 checks, which measure a menu item's leading glyph.
///
/// `Some("tiny-skia")` is load-bearing rather than cosmetic: `iced_wgpu`'s `Headless::new` returns
/// `None` on its first line when the hint is not `"wgpu"`, before it constructs an instance or
/// requests an adapter — so the fallback renderer picks the CPU rasteriser without a GPU ever being
/// probed, and these tests run in CI.
pub fn renderer() -> iced::Renderer {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system()
            .write()
            .expect("the global font system lock was poisoned");
        fonts.load_font(Cow::Borrowed(super::ROBOTO_REGULAR_BYTES));
        fonts.load_font(Cow::Borrowed(super::ROBOTO_MEDIUM_BYTES));
        fonts.load_font(Cow::Borrowed(super::glyph::MATERIAL_SYMBOLS_BYTES));
    });

    block_on(<iced::Renderer as Headless>::new(
        super::ROBOTO,
        iced::Pixels(14.0),
        Some("tiny-skia"),
    ))
    .expect("the tiny-skia headless renderer must construct without a GPU")
}

/// `element` laid out in `room`, as the component tests lay out what they measure.
pub fn laid_out<M>(mut element: Element<'_, M>, room: Size) -> layout::Node {
    let renderer = renderer();
    let mut tree = Tree::new(element.as_widget());
    element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, room))
}

/// The size `element` takes laid out in `room`.
pub fn size_of<M>(element: Element<'_, M>, room: Size) -> Size {
    laid_out(element, room).size()
}

/// The absolute bounds of the node at `path` (child indices, empty for the root) once `element` is
/// laid out in `room`.
///
/// Absolute rather than parent-relative: an inset or a gap is the distance between two boxes, and
/// `Layout` resolves the offsets so the walk cannot get them wrong.
pub fn bounds_at<M>(element: Element<'_, M>, room: Size, path: &[usize]) -> Rectangle {
    let node = laid_out(element, room);
    let mut layout = Layout::new(&node);
    for (depth, &index) in path.iter().enumerate() {
        layout = layout.children().nth(index).unwrap_or_else(|| {
            panic!(
                "no child {index} at depth {depth} of {path:?} — the component's tree changed \
                 shape, so this test is measuring something other than what it names"
            )
        });
    }
    layout.bounds()
}

/// Every node of `node`, depth first, as bounds in the root's own coordinates.
pub fn all_bounds(node: &layout::Node) -> Vec<Rectangle> {
    fn walk(node: &layout::Node, offset: Vector, out: &mut Vec<Rectangle>) {
        let bounds = node.bounds() + offset;
        out.push(bounds);
        for child in node.children() {
            walk(child, Vector::new(bounds.x, bounds.y), out);
        }
    }
    let mut all = Vec::new();
    walk(node, Vector::ZERO, &mut all);
    all
}

/// Whether `host`, laid out in `room`, holds a box of `part`'s own size (`part` laid out alone in
/// the same room) wholly inside the host's own box.
///
/// A host of fixed height keeps that height whatever it holds, so measuring the host alone cannot
/// show a part that no longer fits it: iced clamps the part instead, shrinking it or pushing it
/// past the host's edge. Both leave no box of the part's own size inside the host, which is what
/// this looks for. The host's own box is not a candidate.
pub fn holds_whole<M>(host: Element<'_, M>, part: Element<'_, M>, room: Size) -> bool {
    const TOLERANCE: f32 = 0.5;
    let want = laid_out(part, room).size();
    let host = laid_out(host, room);
    let outer = Layout::new(&host).bounds();
    let mut boxes: Vec<Layout<'_>> = Layout::new(&host).children().collect();
    while let Some(candidate) = boxes.pop() {
        let b = candidate.bounds();
        let same_size =
            (b.width - want.width).abs() < TOLERANCE && (b.height - want.height).abs() < TOLERANCE;
        let inside = b.x >= outer.x - TOLERANCE
            && b.y >= outer.y - TOLERANCE
            && b.x + b.width <= outer.x + outer.width + TOLERANCE
            && b.y + b.height <= outer.y + outer.height + TOLERANCE;
        if same_size && inside {
            return true;
        }
        boxes.extend(candidate.children());
    }
    false
}

/// The window the mounted-element tests lay their element out in.
pub const WINDOW: Size = Size::new(400.0, 800.0);

/// A mounted element, its widget tree and its layout — the three things that have to stay in step
/// for an event to land where the test thinks it does.
///
/// `origin` and `frame` stamp the frame ticks a test hands over: a track ignores a frame it has
/// already seen, so two ticks at the same instant advance it once and the counter keeps them apart.
pub struct Mounted<'a> {
    pub element: Element<'a, String>,
    pub tree: Tree,
    pub node: layout::Node,
    pub renderer: iced::Renderer,
    pub origin: std::time::Instant,
    pub frame: u32,
}

impl<'a> Mounted<'a> {
    pub fn new(element: impl Into<Element<'a, String>>) -> Self {
        let mut element = element.into();
        let renderer = renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
        Self {
            element,
            tree,
            node,
            renderer,
            origin: std::time::Instant::now(),
            frame: 0,
        }
    }

    /// Lay the element out again over the same tree.
    pub fn relayout(&mut self) {
        self.node = self.element.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
    }

    /// The filled container alone — the first of `FormField`'s two bands.
    pub fn container(&self) -> Rectangle {
        self.node.children()[0].bounds()
    }

    /// One event, with no settling, returning whatever was published.
    pub fn send(&mut self, event: Event, cursor: mouse::Cursor) -> Vec<String> {
        let mut messages = Vec::new();
        let mut shell = iced::advanced::Shell::new(&mut messages);
        self.element.as_widget_mut().update(
            &mut self.tree,
            &event,
            Layout::new(&self.node),
            cursor,
            &self.renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
        messages
    }

    /// Press the left button at `at`.
    pub fn press(&mut self, at: Point) -> Vec<String> {
        self.send(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    /// Move the pointer to `at` without pressing.
    pub fn hover(&mut self, at: Point) -> Vec<String> {
        self.send(
            Event::Mouse(mouse::Event::CursorMoved { position: at }),
            mouse::Cursor::Available(at),
        )
    }
}

/// A press of the named key, with no modifiers and no text.
pub fn named_key(named: iced::keyboard::key::Named) -> Event {
    Event::Keyboard(iced::keyboard::Event::KeyPressed {
        key: iced::keyboard::Key::Named(named),
        modified_key: iced::keyboard::Key::Named(named),
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        location: iced::keyboard::Location::Standard,
        modifiers: iced::keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}
