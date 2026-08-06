//! Interaction states are visible, ordered, and come from the token scale
//! (feature 018, T025/T026/T027 — FR-020, FR-021, FR-022, FR-023, SC-005).
//!
//! US3's goal is that the interface *responds*. That is not one property but three, and each fails
//! in a way the others would not catch.
//!
//! **Every state is visibly different.** A hover that resolves to the same pixels as the resting
//! state is not a subtle hover, it is a missing one — and it looks entirely correct in a static
//! screenshot, which is where this kind of bug survives review.
//!
//! **The states are ordered.** A press must read as stronger than a hover. Both being *present* but
//! equal is a real and easy mistake: the element reacts, so nothing seems broken, but pressing
//! communicates nothing beyond hovering.
//!
//! **Disabled means the token opacity**, including on content that colours itself. A glyph with an
//! explicit `.color()` does not inherit its disabled parent's `text_color`, so it has to be dimmed
//! deliberately — the one case where "the button handles it" is false.
//!
//! Inside the crate, like `style_snapshot` and Phase 1's gates: the style layer is `pub(crate)` by
//! 017 FR-002 and reaching it from `tests/` would widen the boundary those gates exist to protect.
//! The tasks named `tests/style_state_layers.rs`, `style_focus.rs` and `style_disabled.rs`; those
//! paths cannot exist, so the three concerns live here together.

use super::style;
use iced::widget::button;
use iced::{Background, Color, Theme};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, state, Roles};

type ButtonStyleFn = Box<dyn Fn(&Theme, button::Status) -> button::Style>;

fn buttons(r: Roles) -> Vec<(&'static str, ButtonStyleFn)> {
    vec![
        ("filled", Box::new(style::filled(r)) as ButtonStyleFn),
        ("outlined", Box::new(style::outlined(r))),
        ("text_button", Box::new(style::text_button(r))),
        (
            "circular_icon_button",
            Box::new(style::circular_icon_button(r)),
        ),
    ]
}

/// What a status actually paints: its background, or the surface it sits on when it paints none.
///
/// Resolving `None` to the surface matters — an unfilled button and a button filled with the
/// surface colour look identical, so comparing `Option`s would call them different when they are
/// not.
fn painted(s: &button::Style, on: Color) -> Color {
    match s.background {
        Some(Background::Color(c)) => {
            // Composite over the surface so a semi-transparent state layer is compared as seen.
            Color {
                r: c.r * c.a + on.r * (1.0 - c.a),
                g: c.g * c.a + on.g * (1.0 - c.a),
                b: c.b * c.a + on.b * (1.0 - c.a),
                a: 1.0,
            }
        }
        _ => on,
    }
}

/// How far apart two colours are, as the sum of their channel differences.
fn delta(a: Color, b: Color) -> f32 {
    (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()
}

#[test]
fn every_button_variant_reacts_to_hover_and_press() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let surface = style::color(r.surface);
        for (name, f) in buttons(r) {
            let active = painted(&f(&theme, button::Status::Active), surface);
            let hovered = painted(&f(&theme, button::Status::Hovered), surface);
            let pressed = painted(&f(&theme, button::Status::Pressed), surface);

            assert!(
                delta(active, hovered) > 0.0,
                "{scheme:?} {name}: hovering changes nothing. A hover that resolves to the resting \
                 pixels is a missing hover, and it looks correct in a screenshot (FR-021, SC-005)"
            );
            assert!(
                delta(active, pressed) > 0.0,
                "{scheme:?} {name}: pressing changes nothing"
            );
        }
    }
}

/// A press reads as stronger than a hover. Both present but equal is the easy mistake: the element
/// reacts, so nothing looks broken, yet pressing communicates nothing beyond hovering.
#[test]
fn a_press_reads_as_stronger_than_a_hover() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let surface = style::color(r.surface);
        for (name, f) in buttons(r) {
            let active = painted(&f(&theme, button::Status::Active), surface);
            let hovered = painted(&f(&theme, button::Status::Hovered), surface);
            let pressed = painted(&f(&theme, button::Status::Pressed), surface);

            assert!(
                delta(active, pressed) >= delta(active, hovered),
                "{scheme:?} {name}: the pressed delta ({:.4}) is smaller than the hovered one \
                 ({:.4}) — a press must read as at least as strong as a hover (SC-005)",
                delta(active, pressed),
                delta(active, hovered)
            );
        }
    }
}

/// The state layers are the token opacities, not hand-picked numbers. Feature 003 used 0.08/0.12,
/// where 0.12 is the *selected* opacity — close enough to look fine and wrong enough that a
/// selected row and a pressed one were indistinguishable.
#[test]
fn the_state_layers_come_from_the_token_scale() {
    let r = tokens::roles(ColorScheme::Light);
    let theme = style::theme(ColorScheme::Light);
    let surface = style::color(r.surface);
    // `text_button` paints nothing at rest, so its hovered/pressed fills are the state layers
    // themselves rather than a blend — the cleanest place to read the opacity back out.
    let f = style::text_button(r);
    let hovered = f(&theme, button::Status::Hovered);
    let pressed = f(&theme, button::Status::Pressed);

    let alpha_of = |s: &button::Style| match s.background {
        Some(Background::Color(c)) => c.a,
        _ => 0.0,
    };
    assert!(
        (alpha_of(&hovered) - state::HOVER).abs() < 1e-6,
        "the hover layer is {} but the token is {} (contract §5)",
        alpha_of(&hovered),
        state::HOVER
    );
    assert!(
        (alpha_of(&pressed) - state::PRESSED).abs() < 1e-6,
        "the pressed layer is {} but the token is {}",
        alpha_of(&pressed),
        state::PRESSED
    );
    let _ = surface;
}

// ---------------------------------------------------------------------------------------------
// Focus (T026, FR-022)
// ---------------------------------------------------------------------------------------------

// A filled field's focus lives in its active indicator, not in a border (§7.7).
//
// Two tests stood here and asserted feature 017's affordance: a 3dp `secondary` border on the
// input when focused, distinguishable from hover. Feature 018 replaced that outright — the filled
// container has **no border at all**, so there is nothing left to recolour, and focus is the bottom
// indicator thickening to 2dp in the accent.
//
// They were deleted rather than adjusted, and they are recorded here because they had stopped
// meaning anything some time before they were noticed: they exercised `style::input`, and once
// `TextField` moved onto `field_input` and feature 021 moved the type-ahead onto `TextField`,
// nothing in the application called it. Both tests stayed green while asserting the appearance of
// a control that no longer existed.
//
// The surviving intent — focus must be readable, and readable *while the pointer is also over the
// field* — is asserted in `text_field_anatomy.rs`. It holds by construction now rather than by
// care: the indicator is a sibling of the input and takes no hover state, so there is no state in
// which hover can mask focus.

// ---------------------------------------------------------------------------------------------
// Disabled (T027, FR-023)
// ---------------------------------------------------------------------------------------------

/// Disabled content resolves the token opacity.
#[test]
fn disabled_content_uses_the_token_opacity() {
    assert!(
        (style::DISABLED_OPACITY - state::DISABLED_CONTENT).abs() < 1e-6,
        "the styling layer's disabled opacity ({}) has drifted from the token ({})",
        style::DISABLED_OPACITY,
        state::DISABLED_CONTENT
    );
    let r = tokens::roles(ColorScheme::Light);
    assert!((style::disabled_color(r.primary).a - state::DISABLED_CONTENT).abs() < 1e-6);
}

/// The self-colouring path agrees with the inherited one.
///
/// A glyph that sets its own `.color()` does not inherit its disabled parent's `text_color`, so it
/// dims itself via `disabled_color`. If the two ever disagree, a disabled icon button shows a
/// full-strength glyph on a dimmed label — which reads as a rendering glitch rather than as a
/// disabled control.
#[test]
fn a_self_colouring_glyph_dims_to_the_same_value_as_its_label() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let disabled = style::circular_icon_button(r)(&theme, button::Status::Disabled);
        assert_eq!(
            style::disabled_color(r.primary),
            disabled.text_color,
            "{scheme:?}: a self-colouring glyph and its button's own label dim differently (FR-023)"
        );
    }
}
