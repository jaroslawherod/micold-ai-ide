//! Design-token invariants (SC-005): every `on_*` foreground role must meet WCAG AA
//! contrast (>= 4.5:1 for normal text) against its paired surface, in BOTH the light and
//! dark schemes. Pure — no iced, runs under `cargo test --no-default-features`.
//!
//! Contrast is computed with the WCAG 2.x relative-luminance formula so a future palette
//! tweak that breaks legibility fails CI (contracts/design-tokens.md).

use micold_client::tokens::{roles, sidebar, type_scale, Rgb, Roles};
use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;

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

/// The foreground/surface pairs that carry text and must meet AA. Includes the worktree tag
/// chips (feature 008): every per-type fill and the issue fill, paired with `on_tag`.
fn pairs(r: &Roles) -> Vec<(&'static str, Rgb, Rgb)> {
    let mut out = vec![
        ("on_background/background", r.on_background, r.background),
        ("on_surface/surface", r.on_surface, r.surface),
        (
            "on_surface_variant/surface_variant",
            r.on_surface_variant,
            r.surface_variant,
        ),
        ("on_primary/primary", r.on_primary, r.primary),
        ("on_error/error", r.on_error, r.error),
    ];
    for &t in ConventionalType::ALL {
        let (fill, on) = r.type_tag(t);
        out.push((t.as_str(), on, fill));
    }
    let (issue_fill, issue_on) = r.issue_tag();
    out.push(("tag_issue", issue_on, issue_fill));
    out
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

/// Alpha-composite `fg` at opacity `a` over opaque `bg` (straight-alpha "over").
fn composite(fg: Rgb, bg: Rgb, a: f64) -> Rgb {
    let mix = |f: u8, b: u8| ((f as f64) * a + (b as f64) * (1.0 - a)).round() as u8;
    Rgb {
        r: mix(fg.r, bg.r),
        g: mix(fg.g, bg.g),
        b: mix(fg.b, bg.b),
    }
}

/// The tag chip's tint opacity — MUST match `alpha(accent, _)` in `ui::style::chip`.
const CHIP_TINT_ALPHA: f64 = 0.20;

/// The sidebar tags render as TONAL chips: accent-colored text on a faint accent tint over the
/// sidebar surface (see `ui::style::chip`). Verify that rendered text meets AA in both schemes,
/// for every type accent and the issue accent (SC-007). This is the combination actually drawn;
/// the solid pairs above cover the filter chips' active (filled) state.
#[test]
fn tonal_tag_chips_meet_aa_contrast() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        let mut accents: Vec<(&str, Rgb)> = ConventionalType::ALL
            .iter()
            .map(|&t| (t.as_str(), r.tag_fill(t)))
            .collect();
        accents.push(("issue", r.tag_issue));
        for (name, accent) in accents {
            let bg = composite(accent, r.surface, CHIP_TINT_ALPHA);
            let ratio = contrast(accent, bg);
            assert!(
                ratio >= AA_NORMAL,
                "{scheme:?} tonal tag {name}: contrast {ratio:.2} < {AA_NORMAL} (accent {accent:?} on tint {bg:?})"
            );
        }
    }
}

/// Every worktree type has a distinct fill in both schemes (FR-005).
#[test]
fn type_tag_fills_are_distinct() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        let fills: Vec<Rgb> = ConventionalType::ALL
            .iter()
            .map(|&t| r.tag_fill(t))
            .collect();
        for i in 0..fills.len() {
            for j in (i + 1)..fills.len() {
                assert_ne!(
                    fills[i], fills[j],
                    "{scheme:?}: type tag fills {i} and {j} collide"
                );
            }
        }
    }
}

/// Sidebar sizes are exactly 80% of the app-wide scale, rounded (FR-012).
#[test]
fn sidebar_sizes_are_eighty_percent() {
    let round80 = |base: u16| ((base as f64) * 0.8).round() as u16;
    assert_eq!(sidebar::NAME, round80(type_scale::BODY));
    assert_eq!(sidebar::TAG, round80(type_scale::LABEL));
    assert_eq!(sidebar::SESSION, round80(type_scale::BODY));
}
