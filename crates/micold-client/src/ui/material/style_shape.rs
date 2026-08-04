//! Corner sizes come from the shape scale (feature 018, T003 — FR-018, FR-019).
//!
//! Two assignments change with this feature, and both are the kind of thing that reads as "not
//! quite Material" without anyone being able to say why: buttons become fully pill-shaped rather
//! than slightly rounded, and dialogs take the 28dp extra-large corner rather than 16.
//!
//! Pinned here because a radius is a number at a call site, and a number drifts silently. Inside the
//! crate for the reason `style_snapshot` states — the style layer is `pub(crate)` by design.

use super::style;
use iced::widget::{button, container};
use iced::Theme;
use micold_core::theme::ColorScheme;
use micold_core::tokens::shape;

/// Boxed style functions, so the variants can be listed together.
type StyleFn = Box<dyn Fn(&Theme) -> container::Style>;
type ButtonStyleFn = Box<dyn Fn(&Theme, button::Status) -> button::Style>;

/// A `Radius`'s four corners, which must agree before comparing against a scale value.
fn uniform(radius: iced::border::Radius, what: &str) -> f32 {
    assert_eq!(
        radius.top_left, radius.top_right,
        "{what}: corners differ; the shape scale is one value per surface"
    );
    assert_eq!(
        radius.top_left, radius.bottom_left,
        "{what}: corners differ"
    );
    assert_eq!(
        radius.top_left, radius.bottom_right,
        "{what}: corners differ"
    );
    radius.top_left
}

#[test]
fn every_button_variant_is_a_pill() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = micold_core::tokens::roles(scheme);
        let theme = style::theme(scheme);
        let variants: Vec<(&str, ButtonStyleFn)> = vec![
            ("filled", Box::new(style::filled(r)) as ButtonStyleFn),
            ("outlined", Box::new(style::outlined(r))),
            ("text_button", Box::new(style::text_button(r))),
            (
                "circular_icon_button",
                Box::new(style::circular_icon_button(r)),
            ),
        ];
        for (name, style_fn) in variants {
            let s = style_fn(&theme, button::Status::Active);
            assert_eq!(
                uniform(s.border.radius, name),
                shape::FULL,
                "{scheme:?} {name} is not a pill — every button container is fully rounded (FR-019)"
            );
        }
    }
}

#[test]
fn cards_dialogs_and_menus_take_their_scale_sizes() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = micold_core::tokens::roles(scheme);
        let theme = style::theme(scheme);
        let cases: Vec<(&str, StyleFn, f32)> = vec![
            (
                "surface (card)",
                Box::new(style::surface(r)) as StyleFn,
                shape::MEDIUM,
            ),
            ("dialog", Box::new(style::dialog(r)), shape::EXTRA_LARGE),
            (
                "menu_surface",
                Box::new(style::menu_surface(r)),
                shape::EXTRA_SMALL,
            ),
        ];
        for (name, style_fn, expected) in cases {
            let s = style_fn(&theme);
            assert_eq!(
                uniform(s.border.radius, name),
                expected,
                "{scheme:?} {name}: wrong corner size (contract §3)"
            );
        }
    }
}

/// Tag chips stay pills. Unchanged by this feature, and pinned so the shape rework cannot take them
/// with it.
#[test]
fn tag_chips_stay_pills() {
    let theme = style::theme(ColorScheme::Light);
    let r = micold_core::tokens::roles(ColorScheme::Light);
    let s = style::chip(r.tag_fill(micold_core::naming::ConventionalType::Feat))(&theme);
    assert_eq!(uniform(s.border.radius, "chip"), shape::FULL);
}
