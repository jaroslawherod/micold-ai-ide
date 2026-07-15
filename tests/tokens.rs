//! Design-token invariants (SC-005): every `on_*` foreground role must meet WCAG AA
//! contrast (>= 4.5:1 for normal text) against its paired surface, in BOTH the light and
//! dark schemes. Pure — no iced, runs under `cargo test --no-default-features`.
//!
//! Contrast is computed with the WCAG 2.x relative-luminance formula so a future palette
//! tweak that breaks legibility fails CI (contracts/design-tokens.md).

use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{roles, Rgb, Roles};

/// Linearize a single 0..=255 sRGB channel per the WCAG definition.
fn linearize(channel: u8) -> f64 {
    let c = channel as f64 / 255.0;
    if c <= 0.039_28 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG relative luminance of an sRGB color.
fn luminance(color: Rgb) -> f64 {
    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

/// WCAG contrast ratio between two colors (>= 1.0; higher is more legible).
fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// The foreground/surface pairs that carry text and must meet AA.
fn pairs(r: &Roles) -> [(&'static str, Rgb, Rgb); 5] {
    [
        ("on_background/background", r.on_background, r.background),
        ("on_surface/surface", r.on_surface, r.surface),
        (
            "on_surface_variant/surface_variant",
            r.on_surface_variant,
            r.surface_variant,
        ),
        ("on_primary/primary", r.on_primary, r.primary),
        ("on_error/error", r.on_error, r.error),
    ]
}

const AA_NORMAL: f64 = 4.5;

#[test]
fn light_scheme_meets_aa_contrast() {
    let r = roles(ColorScheme::Light);
    for (name, fg, bg) in pairs(&r) {
        let ratio = contrast(fg, bg);
        assert!(
            ratio >= AA_NORMAL,
            "light {name}: contrast {ratio:.2} < {AA_NORMAL} (fg {fg:?} on bg {bg:?})"
        );
    }
}

#[test]
fn dark_scheme_meets_aa_contrast() {
    let r = roles(ColorScheme::Dark);
    for (name, fg, bg) in pairs(&r) {
        let ratio = contrast(fg, bg);
        assert!(
            ratio >= AA_NORMAL,
            "dark {name}: contrast {ratio:.2} < {AA_NORMAL} (fg {fg:?} on bg {bg:?})"
        );
    }
}
