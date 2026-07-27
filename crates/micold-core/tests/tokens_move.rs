//! Token relocation guard (feature 017, T004 — FR-021).
//!
//! This feature moves the design tokens into the render-free core. The move must be **mechanical**:
//! the same values, a new home. Re-valuing them is feature 018's work, and doing it here would
//! forfeit this feature's zero-visual-change property — the thing that makes it reviewable.
//!
//! Every value below is transcribed from the pre-move token module. If a value changes, this test
//! fails and names it. It is deliberately dumb: it asserts digits, not derivations, because the
//! whole point is that nothing was recomputed on the way across.

use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, shape, spacing, type_scale, Rgb};

/// Every semantic role, both schemes, exactly as it was before the move.
#[test]
fn every_role_value_survives_the_move_unchanged() {
    let light = tokens::roles(ColorScheme::Light);
    assert_eq!(light.background, Rgb::hex(0xFDFCFF), "light.background");
    assert_eq!(light.on_background, Rgb::hex(0x1A1C1E), "light.on_background");
    assert_eq!(light.surface, Rgb::hex(0xFFFFFF), "light.surface");
    assert_eq!(light.on_surface, Rgb::hex(0x1A1C1E), "light.on_surface");
    assert_eq!(light.surface_variant, Rgb::hex(0xEEF0F4), "light.surface_variant");
    assert_eq!(light.on_surface_variant, Rgb::hex(0x43474E), "light.on_surface_variant");
    assert_eq!(light.primary, Rgb::hex(0x005DB8), "light.primary");
    assert_eq!(light.on_primary, Rgb::hex(0xFFFFFF), "light.on_primary");
    assert_eq!(light.outline, Rgb::hex(0x73777F), "light.outline");
    assert_eq!(light.error, Rgb::hex(0xBA1A1A), "light.error");
    assert_eq!(light.on_error, Rgb::hex(0xFFFFFF), "light.on_error");

    let dark = tokens::roles(ColorScheme::Dark);
    assert_eq!(dark.background, Rgb::hex(0x1A1C1E), "dark.background");
    assert_eq!(dark.on_background, Rgb::hex(0xE2E2E6), "dark.on_background");
    assert_eq!(dark.surface, Rgb::hex(0x212426), "dark.surface");
    assert_eq!(dark.on_surface, Rgb::hex(0xE2E2E6), "dark.on_surface");
    assert_eq!(dark.surface_variant, Rgb::hex(0x2B2F31), "dark.surface_variant");
    assert_eq!(dark.on_surface_variant, Rgb::hex(0xC3C7CF), "dark.on_surface_variant");
    assert_eq!(dark.primary, Rgb::hex(0xA6C8FF), "dark.primary");
    assert_eq!(dark.on_primary, Rgb::hex(0x00325B), "dark.on_primary");
    assert_eq!(dark.outline, Rgb::hex(0x8D9199), "dark.outline");
    assert_eq!(dark.error, Rgb::hex(0xFFB4AB), "dark.error");
    assert_eq!(dark.on_error, Rgb::hex(0x690005), "dark.on_error");
}

/// The per-type worktree tags and the issue tag. Feature 018 re-derives these from tonal ramps;
/// until then they must not move.
#[test]
fn every_tag_value_survives_the_move_unchanged() {
    let light = tokens::roles(ColorScheme::Light);
    for (t, expected) in [
        (ConventionalType::Feat, 0x1B5E20),
        (ConventionalType::Fix, 0xB71C1C),
        (ConventionalType::Chore, 0x4E342E),
        (ConventionalType::Docs, 0x004D40),
        (ConventionalType::Refactor, 0x4A148C),
        (ConventionalType::Test, 0x0D47A1),
        (ConventionalType::Build, 0x7A2600),
        (ConventionalType::Ci, 0x1A237E),
        (ConventionalType::Perf, 0x880E4F),
        (ConventionalType::Style, 0x33691E),
    ] {
        assert_eq!(light.tag_fill(t), Rgb::hex(expected), "light tag {t:?}");
    }
    assert_eq!(light.tag_issue, Rgb::hex(0x37474F), "light.tag_issue");
    assert_eq!(light.on_tag, Rgb::hex(0xFFFFFF), "light.on_tag");

    let dark = tokens::roles(ColorScheme::Dark);
    for (t, expected) in [
        (ConventionalType::Feat, 0xA5D6A7),
        (ConventionalType::Fix, 0xEF9A9A),
        (ConventionalType::Chore, 0xBCAAA4),
        (ConventionalType::Docs, 0x80CBC4),
        (ConventionalType::Refactor, 0xCE93D8),
        (ConventionalType::Test, 0x90CAF9),
        (ConventionalType::Build, 0xFFAB91),
        (ConventionalType::Ci, 0x9FA8DA),
        (ConventionalType::Perf, 0xF48FB1),
        (ConventionalType::Style, 0xE6EE9C),
    ] {
        assert_eq!(dark.tag_fill(t), Rgb::hex(expected), "dark tag {t:?}");
    }
    assert_eq!(dark.tag_issue, Rgb::hex(0xB0BEC5), "dark.tag_issue");
    assert_eq!(dark.on_tag, Rgb::hex(0x1A1C1E), "dark.on_tag");
}

/// The scales. Feature 018 replaces the type scale with roles carrying weight and line height;
/// this feature carries the raw sizes across untouched — including their retyping to `f32`, which
/// landed on main while this branch was in flight and changed no value.
#[test]
fn every_scale_value_survives_the_move_unchanged() {
    assert_eq!(type_scale::DISPLAY, 32.0);
    assert_eq!(type_scale::HEADLINE, 24.0);
    assert_eq!(type_scale::TITLE, 18.0);
    assert_eq!(type_scale::BODY, 14.0);
    assert_eq!(type_scale::LABEL, 12.0);

    assert_eq!(tokens::sidebar::NAME, 11.0);
    assert_eq!(tokens::sidebar::TAG, 10.0);
    assert_eq!(tokens::sidebar::SESSION, 11.0);

    assert_eq!(spacing::XS, 4.0);
    assert_eq!(spacing::SM, 8.0);
    assert_eq!(spacing::MD, 16.0);
    assert_eq!(spacing::LG, 24.0);
    assert_eq!(spacing::XL, 32.0);

    assert_eq!(shape::SM, 8.0);
    assert_eq!(shape::MD, 12.0);
    assert_eq!(shape::LG, 16.0);
    assert_eq!(shape::FULL, 9999.0);
}

/// The sidebar's 80% density decision is an explicit, auditable mapping, not a re-derivation at
/// call sites. It must survive the move as such.
#[test]
fn the_sidebar_density_decision_stays_auditable() {
    assert_eq!(
        tokens::sidebar::NAME,
        (type_scale::BODY * 0.8).round(),
        "sidebar name should remain 80% of body"
    );
    assert_eq!(
        tokens::sidebar::TAG,
        (type_scale::LABEL * 0.8).round(),
        "sidebar tag should remain 80% of label"
    );
    assert_eq!(tokens::sidebar::SESSION, tokens::sidebar::NAME);
}

/// The move's whole justification: tokens become nameable from a crate that cannot see a renderer.
/// If this file compiles at all, that boundary held — `micold-core` has no rendering dependency, so
/// a rendering type could not be named here even by accident.
#[test]
fn tokens_are_reachable_with_no_renderer_present() {
    let r = tokens::roles(ColorScheme::Light);
    // Plain data: no conversion, no theme, no widget.
    assert_eq!(r.primary, Rgb::hex(0x005DB8));
    assert_eq!(
        (r.primary.r, r.primary.g, r.primary.b),
        (0x00, 0x5D, 0xB8),
        "Rgb must stay a plain 8-bit-per-channel value"
    );
}
