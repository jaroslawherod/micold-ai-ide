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
use iced::widget::{button, text_input};
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

/// A focused text field draws the 3dp `secondary` indicator, and it is distinguishable from hover.
///
/// Both mattering together is the point: an indicator that only appears on focus but looks the same
/// as hover cannot be told apart when an element is both, which is exactly what happens when the
/// pointer is resting over the field you just tabbed into.
#[test]
fn a_focused_text_field_draws_the_focus_indicator() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let f = style::input(r);

        let focused = f(&theme, text_input::Status::Focused { is_hovered: false });
        assert_eq!(
            focused.border.width,
            state::FOCUS_RING_WIDTH,
            "{scheme:?}: the focus indicator must be {}dp (contract §5)",
            state::FOCUS_RING_WIDTH
        );
        assert_eq!(
            focused.border.color,
            style::color(r.secondary),
            "{scheme:?}: the focus indicator is drawn in `secondary` (FR-022)"
        );

        let hovered = f(&theme, text_input::Status::Hovered);
        assert!(
            hovered.border.width != focused.border.width
                || hovered.border.color != focused.border.color,
            "{scheme:?}: focused and hovered look identical, so a field that is both cannot be read"
        );
    }
}

/// Focus survives the pointer being over the field. `Focused { is_hovered: true }` is the ordinary
/// case of tabbing to a field the mouse happens to rest on, and losing the indicator there would
/// make keyboard navigation vanish exactly when the user is also touching the mouse.
#[test]
fn focus_stays_visible_while_also_hovered() {
    let r = tokens::roles(ColorScheme::Light);
    let theme = style::theme(ColorScheme::Light);
    let f = style::input(r);
    let a = f(&theme, text_input::Status::Focused { is_hovered: false });
    let b = f(&theme, text_input::Status::Focused { is_hovered: true });
    assert_eq!(a.border.width, b.border.width);
    assert_eq!(a.border.color, b.border.color);
}

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
