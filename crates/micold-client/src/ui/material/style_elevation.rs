//! Elevation reads as depth, in both schemes (feature 018, T001 — FR-015, FR-016, SC-002).
//!
//! US1's goal is *real* depth: surfaces separated by graded tone and a drop shadow rather than by a
//! 1px outline pretending to be an edge. Two things have to hold for that, and only one of them is
//! visible in a light-mode screenshot.
//!
//! **Each level carries its own tone.** If two levels resolve to the same fill, the hierarchy is
//! invisible no matter how correct the shadows are — and a shadow alone is nearly invisible against
//! a dark background, which is why FR-016 makes the tonal shift the primary cue rather than a
//! garnish. So this asserts the tones are distinct *and* that each is the role its level names.
//!
//! **Level 0 casts nothing.** Without that, "elevation 0" quietly acquires a shadow and every flat
//! surface in the application grows one.
//!
//! Lives inside the crate for the same reason `style_snapshot` does: the layer it asserts is
//! `pub(crate)` by design (feature 017 FR-002), and reaching it from `tests/` would mean widening
//! the very boundary `material_boundary.rs` calls "belt to the structure's braces". The task named
//! `tests/style_elevation.rs`; that path cannot exist without forfeiting a 017 guarantee.

use super::style;
use iced::widget::container;
use iced::{Background, Theme};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, elevation, Rgb, Roles};

/// The surface role a level names, resolved against a scheme.
///
/// Deliberately a second statement of contract §4's level→role mapping rather than a call into the
/// style layer: a test that asked the implementation what it does would agree with itself no matter
/// what it did.
fn tone_for(r: Roles, level: u8) -> Rgb {
    use elevation::SurfaceRole::*;
    match elevation::LEVELS[level as usize].surface {
        Surface => r.surface,
        SurfaceContainerLow => r.surface_container_low,
        SurfaceContainer => r.surface_container,
        SurfaceContainerHigh => r.surface_container_high,
        SurfaceContainerHighest => r.surface_container_highest,
    }
}

/// A boxed container style function — each `impl Fn` from the style layer is its own opaque type,
/// so they are boxed behind one signature to be listed together.
type StyleFn = Box<dyn Fn(&Theme) -> container::Style>;

fn background_of(s: &container::Style) -> Option<iced::Color> {
    match s.background {
        Some(Background::Color(c)) => Some(c),
        _ => None,
    }
}

/// The elevated surfaces and the level each is assigned by contract §4.
fn elevated_surfaces(r: Roles) -> Vec<(&'static str, StyleFn, u8)> {
    vec![
        (
            "surface",
            Box::new(style::surface(r)) as StyleFn,
            elevation::CARD,
        ),
        (
            "menu_surface",
            Box::new(style::menu_surface(r)),
            elevation::MENU,
        ),
        ("dialog", Box::new(style::dialog(r)), elevation::DIALOG),
    ]
}

#[test]
fn every_elevated_surface_draws_its_levels_tone_and_a_shadow() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        for (name, style_fn, level) in elevated_surfaces(r) {
            let s = style_fn(&theme);
            assert_eq!(
                background_of(&s),
                Some(style::color(tone_for(r, level))),
                "{scheme:?} {name}: background is not the tone its elevation level names \
                 (level {level}, contract §4)"
            );
            assert!(
                s.shadow.blur_radius > 0.0,
                "{scheme:?} {name}: no shadow. An elevated surface separates from what is behind it \
                 by tone AND shadow (FR-015)"
            );
            assert!(
                s.shadow.color.a > 0.0,
                "{scheme:?} {name}: the shadow is fully transparent, which is the same as no shadow"
            );
        }
    }
}

/// The levels must be *distinguishable*, not merely assigned. Three surfaces that all resolve to
/// the same fill are a hierarchy nobody can see — which is exactly the failure SC-002 describes,
/// and it would survive every other assertion here.
#[test]
fn the_elevated_levels_are_visibly_distinct_from_one_another() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let backgrounds: Vec<_> = elevated_surfaces(r)
            .into_iter()
            .map(|(name, f, _)| (name, background_of(&f(&theme))))
            .collect();
        for (i, (a_name, a)) in backgrounds.iter().enumerate() {
            for (b_name, b) in backgrounds.iter().skip(i + 1) {
                assert_ne!(
                    a, b,
                    "{scheme:?}: `{a_name}` and `{b_name}` resolve to the same fill, so their \
                     elevation difference is invisible"
                );
            }
        }
    }
}

/// A resting surface casts nothing (contract §4: the app bar at rest and page content are level 0).
#[test]
fn an_elevation_zero_surface_casts_no_shadow() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        for (name, style_fn) in [
            (
                "toolbar_surface",
                Box::new(style::toolbar_surface(r)) as StyleFn,
            ),
            ("window_bg", Box::new(style::window_bg(r))),
        ] {
            let s = style_fn(&theme);
            assert_eq!(
                s.shadow.blur_radius, 0.0,
                "{scheme:?} {name} is at elevation 0 and must not cast a shadow"
            );
        }
    }
}

/// A context menu opened over a dialog keeps a shadow of its own (T011, FR-017).
///
/// The two must not flatten into one plane. Stacking *order* is already deterministic — feature
/// 017's overlay primitive owns it by `Layer`, pinned in `tests/overlay_stacking.rs` — so what is
/// left to check is that each surface carries its own depth rather than the upper one inheriting or
/// cancelling the lower's.
///
/// Their shadows differ because their levels do (menu 2, dialog 3). The menu still reads as *above*
/// the dialog despite the lower level: elevation grades the resting hierarchy, the overlay layer
/// decides what is in front.
#[test]
fn a_menu_over_a_dialog_keeps_its_own_shadow() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        let dialog = style::dialog(r)(&theme);
        let menu = style::menu_surface(r)(&theme);

        assert!(
            dialog.shadow.blur_radius > 0.0 && menu.shadow.blur_radius > 0.0,
            "{scheme:?}: a menu over a dialog needs both to cast — one without is the flattening \
             FR-017 forbids"
        );
        assert_ne!(
            dialog.shadow.blur_radius, menu.shadow.blur_radius,
            "{scheme:?}: the menu and the dialog cast identical shadows, so they read as one plane"
        );
        assert_ne!(
            background_of(&dialog),
            background_of(&menu),
            "{scheme:?}: the menu and the dialog share a fill, so the menu has no edge against it"
        );
    }
}

/// The dark scheme's shadow is the stronger of the two, or it is lost entirely against a dark
/// background — the reason §4 states two alphas rather than one.
#[test]
fn the_dark_scheme_draws_the_stronger_shadow() {
    let light = tokens::roles(ColorScheme::Light);
    let dark = tokens::roles(ColorScheme::Dark);
    let light_dialog = style::dialog(light)(&style::theme(ColorScheme::Light));
    let dark_dialog = style::dialog(dark)(&style::theme(ColorScheme::Dark));
    assert!(
        dark_dialog.shadow.color.a > light_dialog.shadow.color.a,
        "the dark-scheme shadow ({}) is not stronger than the light one ({}) — against a dark \
         background the weaker alpha is invisible (FR-016)",
        dark_dialog.shadow.color.a,
        light_dialog.shadow.color.a
    );
}
