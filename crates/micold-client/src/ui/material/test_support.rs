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
use iced::{Element, Size};

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
fn laid_out<M>(mut element: Element<'_, M>, room: Size) -> layout::Node {
    let renderer = renderer();
    let mut tree = Tree::new(element.as_widget());
    element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, room))
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
