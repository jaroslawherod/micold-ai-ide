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

use iced::advanced::renderer::Headless;

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

/// The CPU rasteriser, with both shipped Roboto faces loaded.
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
    });

    block_on(<iced::Renderer as Headless>::new(
        super::ROBOTO,
        iced::Pixels(14.0),
        Some("tiny-skia"),
    ))
    .expect("the tiny-skia headless renderer must construct without a GPU")
}
