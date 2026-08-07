//! US2: icon tint correctness (FR-004, FR-007, SC-004). Pure — no iced, runs under
//! `cargo test --no-default-features`.
//!
//! `icon_role` selects the foreground tint for each surface. Asserting the chosen color has
//! sufficient contrast against the *actual background the icon sits on* guarantees an icon is
//! never tinted invisibly, in BOTH schemes — proving the SC-004 legibility outcome directly
//! (including the `error`-tinted unavailable marker, which the text-only pairs don't cover).

use micold_client::icons::{icon_role, IconSurface};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{roles, Rgb};

/// Linearize a single 0..=255 sRGB channel per the WCAG definition.
fn linearize(channel: u8) -> f64 {
    let c = channel as f64 / 255.0;
    if c <= 0.039_28 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(c: Rgb) -> f64 {
    0.2126 * linearize(c.r) + 0.7152 * linearize(c.g) + 0.0722 * linearize(c.b)
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// WCAG 1.4.11 minimum contrast for non-text graphics (icons are >= 18px graphical marks).
const ICON_MIN: f64 = 3.0;

#[test]
fn every_icon_tint_is_legible_on_its_surface() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        // The background each surface's icon is actually drawn on.
        let cases = [
            (IconSurface::AppBarAction, r.surface),
            (IconSurface::PrimaryButton, r.primary),
            (IconSurface::Badge, r.surface_variant),
            (IconSurface::Unavailable, r.surface),
            // A menu item's glyph sits on the menu panel, which is `surface_container` (§7.5).
            (IconSurface::MenuItem, r.surface_container),
        ];
        for (surface, background) in cases {
            let ratio = contrast(icon_role(surface, r), background);
            assert!(
                ratio >= ICON_MIN,
                "{scheme:?}: {surface:?} icon contrast {ratio:.2} < {ICON_MIN} on its background"
            );
        }
    }
}

#[test]
fn icon_role_maps_each_surface_to_its_expected_foreground() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        assert_eq!(icon_role(IconSurface::AppBarAction, r), r.on_surface);
        assert_eq!(icon_role(IconSurface::PrimaryButton, r), r.on_primary);
        assert_eq!(icon_role(IconSurface::Badge, r), r.on_surface_variant);
        assert_eq!(icon_role(IconSurface::Unavailable, r), r.error);
        assert_eq!(icon_role(IconSurface::MenuItem, r), r.on_surface_variant);
    }
}

#[test]
fn all_surfaces_are_covered() {
    assert_eq!(IconSurface::ALL.len(), 5, "surface contexts");
}
