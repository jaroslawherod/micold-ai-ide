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
use iced::advanced::widget::operation::{self, focusable, Outcome};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{clipboard, mouse, Renderer as _};
use iced::{Element, Event, Point, Rectangle, Size, Vector};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};
use std::sync::{Arc, Mutex};

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

/// Every childless node of `node` (the pieces actually drawn), as bounds in the root's coordinates.
pub fn leaves(node: &layout::Node) -> Vec<Rectangle> {
    fn walk(n: &layout::Node, at: Vector, out: &mut Vec<Rectangle>) {
        let b = n.bounds() + at;
        if n.children().is_empty() {
            out.push(b);
        }
        for c in n.children() {
            walk(c, Vector::new(b.x, b.y), out);
        }
    }
    let mut out = Vec::new();
    walk(node, Vector::ZERO, &mut out);
    out
}

/// Whether `a` and `b` share more than a hairline of area.
pub fn overlap(a: &Rectangle, b: &Rectangle) -> bool {
    a.x < b.x + b.width - 0.01
        && b.x < a.x + a.width - 0.01
        && a.y < b.y + b.height - 0.01
        && b.y < a.y + a.height - 0.01
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
    /// The window the element lays out, takes events and floats overlays in: [`WINDOW`] unless
    /// [`Self::new_in`] named another.
    pub room: Size,
}

impl<'a> Mounted<'a> {
    pub fn new(element: impl Into<Element<'a, String>>) -> Self {
        Self::new_in(element, WINDOW)
    }

    /// [`Self::new`] with `room` in place of [`WINDOW`]: an element wider than the window, such as
    /// a 480 px card (feature 1120).
    pub fn new_in(element: impl Into<Element<'a, String>>, room: Size) -> Self {
        let mut element = element.into();
        let renderer = renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, room),
        );
        Self {
            element,
            tree,
            node,
            renderer,
            origin: std::time::Instant::now(),
            frame: 0,
            room,
        }
    }

    /// Lay the element out again over the same tree.
    pub fn relayout(&mut self) {
        self.node = self.element.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, self.room),
        );
    }

    /// The filled container alone — the first of `FormField`'s two bands.
    pub fn container(&self) -> Rectangle {
        self.node.children()[0].bounds()
    }

    /// The frame tick every widget sees, with the pointer away. iced delivers `RedrawRequested`
    /// through `update` like any other event, which is what gives a widget somewhere to publish
    /// from after an *operation* (a traversal, a `focus(id)`) changed something: an operation
    /// carries no shell and can say nothing itself.
    pub fn redraw(&mut self) -> Vec<String> {
        self.send(
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            mouse::Cursor::Unavailable,
        )
    }

    /// One event, with no settling, returning whatever was published.
    pub fn send(&mut self, event: Event, cursor: mouse::Cursor) -> Vec<String> {
        self.send_captured(event, cursor).0
    }

    /// [`Self::send`], and whether the element captured the event, so no later handler sees it.
    pub fn send_captured(&mut self, event: Event, cursor: mouse::Cursor) -> (Vec<String>, bool) {
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
            &Rectangle::with_size(self.room),
        );
        let captured = shell.is_event_captured();
        let invalid = shell.is_layout_invalid();
        // As the runtime does: a widget that asks for a new layout gets it before the next event.
        if invalid {
            self.relayout();
        }
        (messages, captured)
    }

    /// Run `operation` against the element to the end of its chain, the way the runtime does,
    /// returning what it produced.
    ///
    /// The loop is not ceremony. `focus_next` is two passes chained — count the focusables, then
    /// move to the one after the focused — and a single `operate` call runs only the first, so a
    /// test that made one call would prove nothing and pass either way.
    pub fn drive<T: Send + 'static>(&mut self, operation: impl Operation<T>) -> Option<T> {
        let found = Arc::new(Mutex::new(None));
        let sink = Arc::clone(&found);
        let mut current: Box<dyn Operation> = Box::new(operation::map(operation, move |value| {
            *sink.lock().expect("the sink is not poisoned") = Some(value);
        }));
        loop {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                current.as_mut(),
            );
            match current.finish() {
                Outcome::Chain(next) => current = next,
                Outcome::Some(()) | Outcome::None => break,
            }
        }
        let value = found.lock().expect("the sink is not poisoned").take();
        value
    }

    /// Every scrollable under the element, as (viewport, content) bounds, asked as the runtime
    /// would.
    pub fn scrollables(&mut self) -> Vec<(Rectangle, Rectangle)> {
        self.drive(FindScrollables(Vec::new()))
            .expect("the operation finishes")
    }

    /// Move the keyboard forward through this element, the way `Message::FocusMoved` does.
    pub fn focus_next(&mut self) {
        self.drive(focusable::focus_next::<()>());
    }

    /// The element drawn over `backdrop`: the buffer and its width.
    pub fn drawn(&mut self, backdrop: iced::Color) -> (Vec<u8>, u32) {
        let size = self.paint();
        let (w, h) = (size.width.ceil() as u32, size.height.ceil() as u32);
        (self.renderer.screenshot(Size::new(w, h), 1.0, backdrop), w)
    }

    /// Every paragraph the element paints, in draw order: what it says, at which size, in which
    /// face and colour, and where — what `tests/support/layout.rs::painted_text` reads, for the
    /// in-crate tests that cannot reach it (feature 890, T001).
    pub fn painted_text(&mut self) -> Vec<Painted> {
        self.paint();
        self.paragraphs()
    }

    /// The words of whatever floats over the element with the pointer resting at `at`, a tooltip
    /// say: empty when nothing floats (feature 1120, T032).
    pub fn floated_text(&mut self, at: Point) -> Vec<String> {
        self.floated(at).into_iter().map(|p| p.content).collect()
    }

    /// [`Self::floated_text`] with where each paragraph floats.
    pub fn floated(&mut self, at: Point) -> Vec<Painted> {
        self.floated_in(at, Rectangle::with_size(self.room))
    }

    /// [`Self::floated`] with `viewport` handed to `overlay`: the window a scrollable would hand
    /// it, which need not be the view the element was updated and drawn in.
    pub fn floated_in(&mut self, at: Point, viewport: Rectangle) -> Vec<Painted> {
        self.hover(at);
        self.renderer.reset(Rectangle::with_size(self.room));
        let Some(mut floated) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &viewport,
            iced::Vector::ZERO,
        ) else {
            return Vec::new();
        };
        let node = floated.as_overlay_mut().layout(&self.renderer, self.room);
        floated.as_overlay().draw(
            &mut self.renderer,
            &super::theme(micold_core::theme::ColorScheme::Light),
            &iced::advanced::renderer::Style {
                text_color: iced::Color::BLACK,
            },
            Layout::new(&node),
            mouse::Cursor::Available(at),
        );
        drop(floated);
        self.paragraphs()
    }

    /// Every paragraph in the renderer's layers, in draw order.
    fn paragraphs(&mut self) -> Vec<Painted> {
        use iced::advanced::text::Paragraph as _;
        let iced_renderer::fallback::Renderer::Secondary(cpu) = &mut self.renderer else {
            panic!("reading painted text needs the CPU rasteriser; see `renderer`")
        };
        let mut found = Vec::new();
        for layer in cpu.layers() {
            for item in &layer.text {
                for text in item.as_slice() {
                    if let iced::advanced::graphics::text::Text::Paragraph {
                        paragraph,
                        position,
                        color,
                        ..
                    } = text
                    {
                        let Some(p) = paragraph.upgrade() else {
                            continue;
                        };
                        found.push(Painted {
                            content: p
                                .buffer()
                                .lines
                                .iter()
                                .map(|line| line.text())
                                .collect::<Vec<_>>()
                                .join(" "),
                            size: p.size().0,
                            font: p.font(),
                            color: *color,
                            bounds: Rectangle::new(*position, paragraph.min_bounds),
                        });
                    }
                }
            }
        }
        found
    }

    /// Draw the element into the renderer at its own size, returning that size.
    fn paint(&mut self) -> Size {
        let size = self.node.bounds().size();
        let viewport = Rectangle::with_size(size);
        self.renderer.reset(viewport);
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            &super::theme(micold_core::theme::ColorScheme::Light),
            &iced::advanced::renderer::Style {
                text_color: iced::Color::BLACK,
            },
            Layout::new(&self.node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        size
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

/// One paragraph a mounted element painted — [`Mounted::painted_text`].
#[derive(Debug, Clone)]
pub struct Painted {
    pub content: String,
    pub size: f32,
    pub font: iced::Font,
    pub color: iced::Color,
    /// Where it was drawn and the room its shaped text takes.
    pub bounds: Rectangle,
}

/// Every scrollable under the element, as (viewport, content) bounds.
struct FindScrollables(Vec<(Rectangle, Rectangle)>);

impl Operation<Vec<(Rectangle, Rectangle)>> for FindScrollables {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn Operation<Vec<(Rectangle, Rectangle)>>),
    ) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: iced::Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.0.push((bounds, content_bounds));
    }

    fn finish(&self) -> Outcome<Vec<(Rectangle, Rectangle)>> {
        Outcome::Some(self.0.clone())
    }
}

/// A press of the named key, with no modifiers and no text.
pub fn named_key(named: iced::keyboard::key::Named) -> Event {
    key_with(named, iced::keyboard::Modifiers::empty())
}

/// A press of the named key with `modifiers` held, and no text.
pub fn key_with(named: iced::keyboard::key::Named, modifiers: iced::keyboard::Modifiers) -> Event {
    Event::Keyboard(iced::keyboard::Event::KeyPressed {
        key: iced::keyboard::Key::Named(named),
        modified_key: iced::keyboard::Key::Named(named),
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        location: iced::keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

/// Counts the focusable widgets an element offers its keyboard traversal.
pub struct Focusables(pub usize);

impl iced::advanced::widget::Operation<usize> for Focusables {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<usize>),
    ) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        _state: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        self.0 += 1;
    }

    fn finish(&self) -> iced::advanced::widget::operation::Outcome<usize> {
        iced::advanced::widget::operation::Outcome::Some(self.0)
    }
}
