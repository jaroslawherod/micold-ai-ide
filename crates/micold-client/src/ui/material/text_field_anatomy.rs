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

use iced::widget::text_input;
use iced::{Background, Color, Theme};
use micold_core::tokens::{self, anatomy, shape, Roles};

use super::style;

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
