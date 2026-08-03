//! Token relocation guard (feature 017, T004 — FR-021), as amended by feature 018's Phase 0.
//!
//! Feature 017 moved the design tokens into the render-free core, and this file pinned every value
//! so the move stayed **mechanical**: the same values, a new home. It said in as many words that
//! re-valuing them was feature 018's work.
//!
//! **That has now happened, and the colour half of this guard has been retired.** 018's Phase 0
//! re-authored every role as a palette-and-tone pair on the Material 3 baseline ramps (FR-005a,
//! FR-005b), which is precisely the change this file existed to prevent happening *by accident*.
//! Keeping the old assertions would not protect anything — it would only assert that the deliberate
//! change had not been made. `tests/tokens_contrast.rs` is the guard that replaces them, and it is a
//! stronger one: it checks the property the values are supposed to have (AA in both schemes,
//! monotonic ramps, the seed at tone 40) rather than the digits themselves.
//!
//! What survives is the half 018 Phase 0 does **not** touch. T000e carries the spacing, type and
//! shape scales across the module split without re-valuing them, so those assertions still guard a
//! live promise, and they now guard the split as well as the move.

use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, shape, spacing, type_scale, Rgb};

/// The colour values did change, and that is the point.
///
/// Replaces the two retired guards (`every_role_value_survives_the_move_unchanged` and
/// `every_tag_value_survives_the_move_unchanged`). Rather than assert the old digits — which would
/// now be asserting that a deliberate change had not been made — this asserts the change actually
/// landed, so a botched merge that restored feature 003's palette fails here instead of shipping.
#[test]
fn the_palette_is_no_longer_feature_003s() {
    let light = tokens::roles(ColorScheme::Light);
    assert_ne!(
        light.primary,
        Rgb::hex(0x005DB8),
        "the light accent is still feature 003's brand blue — Phase 0 re-authors it as the \
         Material 3 baseline purple (FR-005b)"
    );
    let dark = tokens::roles(ColorScheme::Dark);
    assert_ne!(
        dark.primary,
        Rgb::hex(0xA6C8FF),
        "the dark accent is still feature 003's blue"
    );
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
    // Plain data: no conversion, no theme, no widget. The value is the Material 3 baseline seed
    // since Phase 0 (it was feature 003's #005DB8 before); what this test is really about is that
    // the channels are reachable as plain integers from a crate that cannot see a renderer.
    assert_eq!(r.primary, Rgb::hex(0x6750A4));
    assert_eq!(
        (r.primary.r, r.primary.g, r.primary.b),
        (0x67, 0x50, 0xA4),
        "Rgb must stay a plain 8-bit-per-channel value"
    );
}
