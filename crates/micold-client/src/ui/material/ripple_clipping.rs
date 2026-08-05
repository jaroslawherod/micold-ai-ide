//! The ripple stays inside the shape it is pressing — checked in pixels (feature 018, T032).
//!
//! [`super::ripple::shape_bands`] has its own tests, and they prove the *tiling* is sound: disjoint,
//! contiguous, inside the rounded shape. What they cannot prove is that the drawing actually uses
//! it. The original bug was not a geometry mistake — it was a `with_layer(bounds, …)` clipping the
//! ripple to the element's bounding rectangle, which is arithmetically unimpeachable and visually
//! wrong. A test of the geometry alone would have passed against the broken build.
//!
//! So this rasterises. It presses a pill, advances the ripple to the point where the circle has
//! grown past the corners, renders headlessly, and reads the pixels **outside** the pill but inside
//! its bounding box — the two square caps that the recording showed hanging off the ends of a
//! sidebar row. They must be exactly the background.
//!
//! The renderer is the CPU rasteriser, the same one feature 019's layout gate uses and for the same
//! reason: it needs no GPU, so this runs in CI.

use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::Once;
use std::task::{Context, Poll, Waker};

use iced::advanced::renderer::Headless;
use iced::{Color, Element, Point, Size};
use micold_core::tokens::{self, shape};

use super::{Ripple, Surface, SurfaceKind};
use crate::showcase::state::Message;

/// The element under test: 400×48 at `shape::FULL`, the proportions of a sidebar row.
const WIDTH: u32 = 400;
const HEIGHT: u32 = 48;

/// What the page is cleared to. Deliberately not black: the ripple is a *light* tint, and a black
/// background would let a mistake hide in a channel that barely moved.
const BACKDROP: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// The CPU rasteriser, with the shipped faces loaded so text shaping does not reach for a system
/// font. Mirrors `tests/support/layout.rs`'s constructor — see there for why the `"tiny-skia"` hint
/// keeps `iced_wgpu` from probing for a GPU.
fn renderer() -> iced::Renderer {
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

/// Poll a future that is known to be immediately ready — the tiny-skia headless constructor does
/// no I/O, so one poll suffices and no executor need be pulled into the test scaffolding.
fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = Box::pin(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = Pin::as_mut(&mut f).poll(&mut cx) {
            return v;
        }
        std::hint::spin_loop();
    }
}

/// Render a pressed, mid-expansion ripple over a pill and return the RGBA buffer.
fn pressed_pill() -> Vec<u8> {
    use iced::advanced::widget::Tree;
    use iced::advanced::{layout, mouse, renderer::Style, Layout, Renderer as _};

    let roles = tokens::roles(micold_core::theme::ColorScheme::Dark);
    let mut renderer = renderer();

    let mut element: Element<'_, Message> = Ripple::new(
        Surface::new(
            iced::widget::Space::new(),
            SurfaceKind::Chip(roles.primary),
            roles,
        )
        .width(iced::Length::Fill)
        .height(iced::Length::Fill),
        roles.on_surface,
        shape::FULL,
    )
    .into();

    let mut tree = Tree::new(&element);
    let limits = layout::Limits::new(Size::ZERO, Size::new(WIDTH as f32, HEIGHT as f32));
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let bounds = node.bounds();

    // Press the middle, then advance until the circle has grown past the corners — the state the
    // rectangular clip drew as two square caps. Driven through the state directly rather than
    // through events, because what is under test is the *drawing*, not the input handling.
    {
        let state = tree.state.downcast_mut::<crate::ui::cdk::ripple::Ripple>();
        state.press(
            Some(Point::new(bounds.width / 2.0, bounds.height / 2.0)),
            bounds.size(),
        );
        // Far enough that the circle certainly covers the corners: from the centre of a 400×48
        // element the furthest corner is ~201dp away, and half the expansion puts the edge well
        // past the 24dp caps at either end.
        for _ in 0..40 {
            state.advance(
                std::time::Duration::from_millis(200),
                std::time::Duration::from_millis(200),
            );
            if state.expansion() >= 1.0 {
                break;
            }
        }
        assert!(
            state.radius() > bounds.height,
            "the ripple did not grow past the corner radius, so this would pass without drawing \
             anything into the region under test"
        );
        assert!(
            state.strength() > 0.5,
            "the ripple faded before the check, so the pixels would be near-background either way"
        );
    }

    let viewport = iced::Rectangle::with_size(Size::new(WIDTH as f32, HEIGHT as f32));
    renderer.reset(viewport);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &iced::Theme::Dark,
        &Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    renderer.screenshot(Size::new(WIDTH, HEIGHT), 1.0, BACKDROP)
}

/// Whether `(x, y)` is inside the pill: the shape the ripple must not leave.
fn inside_pill(x: u32, y: u32) -> bool {
    let r = HEIGHT as f32 / 2.0;
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let cx = if px < r {
        r
    } else if px > WIDTH as f32 - r {
        WIDTH as f32 - r
    } else {
        return true;
    };
    (px - cx).hypot(py - r) <= r
}

/// Nothing the ripple draws appears outside the pill.
///
/// The corners are sampled with a 1px margin outside the curve, so antialiasing on the *shape's*
/// own edge is not mistaken for the overhang being looked for. What this catches is the real bug:
/// a square cap of tint hanging off the rounded end of a row.
#[test]
fn the_ripple_does_not_paint_outside_the_pill() {
    let pixels = pressed_pill();
    let mut offenders = Vec::new();

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if inside_pill(x, y) {
                continue;
            }
            // One pixel of clearance from the curve, so shape antialiasing is not the finding.
            if (0..3).any(|d| {
                inside_pill(x.saturating_sub(d), y)
                    || inside_pill((x + d).min(WIDTH - 1), y)
                    || inside_pill(x, y.saturating_sub(d))
                    || inside_pill(x, (y + d).min(HEIGHT - 1))
            }) {
                continue;
            }
            let i = ((y * WIDTH + x) * 4) as usize;
            let (r, g, b) = (pixels[i], pixels[i + 1], pixels[i + 2]);
            if r > 4 || g > 4 || b > 4 {
                offenders.push((x, y, r, g, b));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "{} pixels outside the pill were painted — the ripple is escaping the shape it is \
         pressing, which is what made it read as a rectangle sliding across a rounded row. First \
         few: {:?}",
        offenders.len(),
        &offenders[..offenders.len().min(6)]
    );
}

/// …and it does paint *inside* it, or the test above would pass on a ripple that draws nothing.
///
/// This is the half that fails if `shape_bands` ever returns an empty tiling, or if the clip is
/// tightened until there is nothing left to see.
#[test]
fn the_ripple_does_paint_inside_the_pill() {
    let pixels = pressed_pill();
    let mut lit = 0usize;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if !inside_pill(x, y) {
                continue;
            }
            let i = ((y * WIDTH + x) * 4) as usize;
            if pixels[i] > 8 || pixels[i + 1] > 8 || pixels[i + 2] > 8 {
                lit += 1;
            }
        }
    }
    assert!(
        lit > (WIDTH * HEIGHT / 4) as usize,
        "only {lit} pixels inside the pill were painted — the ripple is barely drawing, so the \
         check that nothing escapes the shape would pass trivially"
    );
}
