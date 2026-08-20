//! The filled text field's anatomy (feature 018, T044 — FR-031; contract §7.7).
//!
//! In-crate for the same reason as `form_field_anatomy.rs`: `material` is `pub(crate)`, so the
//! `tests/text_field_anatomy.rs` path tasks.md names is not reachable.
//!
//! §7.7 is the largest single departure in this feature, and it records the current state next to
//! the target for exactly that reason. What ships today is `surface` — the same tone as the dialog
//! behind it — inside a uniform 1dp box with 8dp of padding, roughly 30dp tall, focus shown by
//! recolouring the border. The target is a `surface_container_highest` container with **no border
//! at all**, rounded at the top and square at the bottom, 56dp tall, focus shown by a bottom
//! indicator that thickens.
//!
//! Each of those is checkable as a value rather than by eye, so each is checked here.

use std::collections::HashSet;

use iced::advanced::renderer::Headless as _;
use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, renderer, Layout, Renderer as _};
use iced::widget::text_input;
use iced::{Background, Color, Element, Rectangle, Size, Theme};
use micold_core::tokens::{self, anatomy, shape, Roles};

use super::style;
use crate::icons::Icon;

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

fn theme() -> Theme {
    super::theme(micold_core::theme::ColorScheme::Light)
}

// ---------------------------------------------------------------------------------------------
// The container
// ---------------------------------------------------------------------------------------------

/// The container is `surface_container_highest`, not `surface`.
///
/// The defect this replaces: a field the same tone as the dialog behind it, which is why today's
/// field needs a border to be visible at all. A filled field is read by its *fill*.
#[test]
fn the_container_is_distinguishable_from_the_surface_behind_it() {
    let r = roles();
    let s = style::field_container(r)(&theme());

    assert_eq!(
        s.background,
        Some(Background::Color(style::color(r.surface_container_highest)))
    );
    assert_ne!(
        s.background,
        Some(Background::Color(style::color(r.surface))),
        "the field is the same tone as the surface behind it, which is the defect §7.7 records"
    );
}

/// Rounded at the top, square at the bottom (§7.7).
///
/// Not decoration: the squared bottom is what makes the active indicator read as part of the field
/// rather than as a line underneath it. A uniform radius leaves the indicator's ends floating past
/// the container's curve.
#[test]
fn the_container_is_rounded_on_top_and_square_beneath() {
    let s = style::field_container(roles())(&theme());
    let radius = s.border.radius;

    assert_eq!(radius.top_left, shape::EXTRA_SMALL);
    assert_eq!(radius.top_right, shape::EXTRA_SMALL);
    assert_eq!(
        radius.bottom_right, 0.0,
        "a rounded bottom leaves the active indicator's ends floating past the curve"
    );
    assert_eq!(radius.bottom_left, 0.0);
}

/// No border. The fill and the indicator carry the field; an outline on top of them is the
/// duplication §7.7 removes, and it is what makes today's field read as a box rather than a field.
#[test]
fn the_container_has_no_border() {
    let s = style::field_container(roles())(&theme());
    assert_eq!(
        s.border.width, 0.0,
        "the filled container still draws a border, so it reads as a box"
    );
}

// ---------------------------------------------------------------------------------------------
// The input inside it
// ---------------------------------------------------------------------------------------------

/// The input draws no chrome of its own, in any status (FR-031c).
///
/// The container and the indicator belong to `FormField`. An input that kept its own box would put
/// a 1dp outline *inside* the filled container — which is precisely what composing the two would
/// have produced if the input's style had been left alone.
#[test]
fn the_input_draws_no_container_of_its_own() {
    let r = roles();
    let style_fn = style::field_input(r);

    for status in [
        text_input::Status::Active,
        text_input::Status::Hovered,
        text_input::Status::Focused { is_hovered: false },
        text_input::Status::Focused { is_hovered: true },
        text_input::Status::Disabled,
    ] {
        let s = style_fn(&theme(), status);
        assert_eq!(
            s.background,
            Background::Color(Color::TRANSPARENT),
            "the input paints a background in {status:?}, so it covers the field's own fill"
        );
        assert_eq!(
            s.border.width, 0.0,
            "the input draws a border in {status:?}, inside the container that already has one job"
        );
    }
}

/// The input still colours its own *text* — that part is its job, and dropping it would leave the
/// value unreadable against the container.
#[test]
fn the_input_still_colours_its_text() {
    let r = roles();
    let s = style::field_input(r)(&theme(), text_input::Status::Active);

    assert_eq!(s.value, style::color(r.on_surface));
    assert_eq!(s.placeholder, style::color(r.on_surface_variant));
    assert_ne!(
        s.value, s.placeholder,
        "the value and the placeholder are the same colour, so a filled field is indistinguishable \
         from an empty one"
    );
}

// ---------------------------------------------------------------------------------------------
// The active indicator — the whole of the focus affordance
// ---------------------------------------------------------------------------------------------

/// Focus thickens the indicator to 2dp and takes the accent (§7.7).
///
/// There is no border here to recolour, so this *is* how a filled field shows focus. The old
/// behaviour — recolouring a box outline — has nothing left to act on.
#[test]
fn focus_thickens_the_indicator_to_the_accent() {
    let r = roles();
    let (resting, resting_w) = style::field_indicator(r, false, false);
    let (focused, focused_w) = style::field_indicator(r, true, false);

    assert_eq!(resting_w, anatomy::text_field::INDICATOR);
    assert_eq!(focused_w, anatomy::text_field::INDICATOR_ACTIVE);
    assert_eq!(resting, style::color(r.on_surface_variant));
    assert_eq!(focused, style::color(r.primary));
}

/// The padding is the contract's, and wider than what ships today.
#[test]
#[allow(clippy::assertions_on_constants)] // guarding the values is the point; clippy can see the answer
fn the_field_pads_to_the_contract() {
    assert_eq!(anatomy::text_field::PADDING, 16.0);
    assert!(
        anatomy::text_field::PADDING > tokens::spacing::SM,
        "the field still pads at the old `spacing::SM`, which is half what §7.7 asks for"
    );
}

// ---------------------------------------------------------------------------------------------
// Drawn pixels — the gate a layout tree cannot be (021 BUG-002)
// ---------------------------------------------------------------------------------------------
//
// 021 BUG-002 was a leading search icon drawn on top of the first letter of its own field's label,
// and every geometry gate was green on it. That is not an oversight in any of them: in a layout
// tree each node is exactly where its own layout says it is, so a collision between two siblings is
// not a containment failure (neither escapes its parent), not a size failure (both are their stated
// size) and not a placement failure (each is where its own rule puts it). The suite is
// *structurally* blind to overlap, and another geometry gate would not help.
//
// What found it was reading rendered pixels — the `visual-pass` skill, by eye. This is that check
// made automatic, in the style of `select_anatomy`'s indicator test, which already rasterises.
//
// The method is a difference rather than a threshold, which is what makes it exact. A bare field
// draws its container, its indicator and nothing else; adding one part and diffing against that
// baseline isolates the pixels *that part* draws. Two parts collide exactly when their two
// difference sets share a pixel — no tolerance to tune, no "how close is too close" to argue about,
// and the container's own chrome cancels out of both sides.

/// The canvas each field is drawn on. Wide enough that nothing wraps or elides.
const PIXEL_CANVAS: Size = Size::new(400.0, 200.0);

/// A bare field: no label, no icon, and an **empty placeholder** so the input contributes nothing.
///
/// The baseline every mask below is measured against. Its placeholder is empty rather than absent
/// because a field with no label never suppresses one (`label_floats` has nothing to float), so a
/// prompt here would be ink in every image and would cancel the very difference being measured.
fn bare(r: Roles) -> super::TextField<'static, String> {
    super::TextField::new("", "", r)
}

/// Rasterise `element` and return its pixels with the width they were drawn at.
fn rasterise(element: impl Into<Element<'static, String>>) -> (Vec<u8>, u32) {
    let mut element = element.into();
    let mut renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, PIXEL_CANVAS),
    );

    let size = node.bounds().size();
    let viewport = Rectangle::with_size(size);
    renderer.reset(viewport);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &theme(),
        &renderer::Style {
            text_color: style::color(roles().on_surface),
        },
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );

    let (w, h) = (size.width.ceil() as u32, size.height.ceil() as u32);
    (renderer.screenshot(Size::new(w, h), 1.0, Color::WHITE), w)
}

/// Which pixels `element` draws that `baseline` does not — the ink of whatever was added.
///
/// A tolerance of 8 in the largest channel, the same figure `select_anatomy` uses: above the
/// rasteriser's own noise on an antialiased edge, far below the difference a glyph makes.
fn added_ink(
    element: impl Into<Element<'static, String>>,
    baseline: impl Into<Element<'static, String>>,
) -> HashSet<(u32, u32)> {
    let (drawn, width) = rasterise(element);
    let (plain, plain_width) = rasterise(baseline);
    assert_eq!(
        (drawn.len(), width),
        (plain.len(), plain_width),
        "the two frames are different sizes, so a pixel comparison would be meaningless"
    );

    (0..drawn.len() / 4)
        .filter(|i| {
            (0..3)
                .map(|c| drawn[i * 4 + c].abs_diff(plain[i * 4 + c]))
                .max()
                .unwrap_or(0)
                > 8
        })
        .map(|i| (i as u32 % width, i as u32 / width))
        .collect()
}

/// Where two rasterisations of the same field differ, restricted to `within`.
///
/// The heart of the check. Adding a label must change nothing where the icon is drawn — so the two
/// frames are compared *inside the icon's own pixels*, and any difference there is the label having
/// been painted over it.
///
/// Stated this way round, and not as "do the two ink masks intersect", because the two parts cannot
/// be posed apart: a field without a leading icon does not indent its label, so a label rendered
/// alone is at a position it never occupies in the field under test. Isolating it by subtraction
/// would make the answer true by construction. Comparing a field against the same field with one
/// part removed keeps the geometry fixed and asks the question the eye asks.
fn differences_within(a: (Vec<u8>, u32), b: (Vec<u8>, u32), within: &HashSet<(u32, u32)>) -> usize {
    let ((drawn, width), (plain, plain_width)) = (a, b);
    assert_eq!(
        (drawn.len(), width),
        (plain.len(), plain_width),
        "the two frames are different sizes, so a pixel comparison would be meaningless"
    );
    within
        .iter()
        .filter(|(x, y)| {
            let i = ((y * width + x) * 4) as usize;
            (0..3)
                .map(|c| drawn[i + c].abs_diff(plain[i + c]))
                .max()
                .unwrap_or(0)
                > 8
        })
        .count()
}

/// The bug itself: nothing is painted over the leading icon (021 BUG-002).
///
/// Both quantities are asserted non-vacuous first. Without that the check passes when nothing is
/// drawn at all — a broken rasteriser, a missing font, a field that stopped rendering — which is
/// the one way a difference test can be green for the worst possible reason.
#[test]
fn adding_a_label_changes_nothing_where_the_leading_icon_is_drawn() {
    let r = roles();
    let icon = added_ink(bare(r).leading_icon(Icon::Search), bare(r));
    assert!(
        !icon.is_empty(),
        "the leading icon drew nothing, so this test would pass without checking anything"
    );

    let with_label = rasterise(bare(r).leading_icon(Icon::Search).label("Branch"));
    let without = rasterise(bare(r).leading_icon(Icon::Search));

    let everywhere: HashSet<(u32, u32)> = (0..with_label.1)
        .flat_map(|x| (0..(with_label.0.len() as u32 / 4 / with_label.1)).map(move |y| (x, y)))
        .collect();
    assert!(
        differences_within(with_label.clone(), without.clone(), &everywhere) > 0,
        "the label drew nothing, so this test would pass without checking anything"
    );

    let over = differences_within(with_label, without, &icon);
    assert_eq!(
        over, 0,
        "the resting label is painted over {over} of the leading icon's own pixels — the magnifier \
         under the first letter of \"Branch\", which is 021 BUG-002 and which every geometry gate \
         is structurally unable to see"
    );
}

/// The half BUG-002 recorded as *probable, to be confirmed while fixing*: once the field is focused
/// the label floats and the input draws its placeholder instead, and in the reported capture the
/// magnifier appeared to touch the "S" of "Search branches…".
///
/// It does not. That inset is the rendering stack's own rather than this application's, so it is a
/// different mechanism and is checked separately rather than assumed to have been carried along by
/// the label's fix.
#[test]
fn adding_a_placeholder_changes_nothing_where_the_leading_icon_is_drawn() {
    let r = roles();
    let focused = || bare(r).label("Branch").active(true);
    let icon = added_ink(focused().leading_icon(Icon::Search), focused());
    assert!(!icon.is_empty(), "the leading icon drew nothing");

    let with_prompt = rasterise(
        super::TextField::new("Search branches…", "", r)
            .label("Branch")
            .active(true)
            .leading_icon(Icon::Search),
    );
    let without = rasterise(focused().leading_icon(Icon::Search));

    let over = differences_within(with_prompt, without, &icon);
    assert_eq!(
        over, 0,
        "the placeholder is painted over {over} of the leading icon's own pixels"
    );
}
