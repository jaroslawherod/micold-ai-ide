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
use micold_core::tokens::{self, spacing, typography, Rgb};

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

/// What feature 018 does **not** touch: the spacing scale.
///
/// The type and shape scales that this test also used to guard are gone from it. Both are
/// *replaced* by 018 rather than moved — the type scale onto Material's fifteen roles (§2.5, which
/// changes `TITLE` 18 → 22 and the sidebar sizes 11 → 12 and 10 → 11), the shape scale onto seven
/// sizes — and `tokens_scales.rs` pins each of them properly, by role, rather than as loose
/// integers. Asserting the old numbers here would only assert that a deliberate change had not
/// happened.
///
/// Spacing is untouched by 018, so it still guards a live promise, and it now guards the module
/// split as well as the original move.
#[test]
fn the_spacing_scale_survives_unchanged() {
    assert_eq!(spacing::XS, 4.0);
    assert_eq!(spacing::SM, 8.0);
    assert_eq!(spacing::MD, 16.0);
    assert_eq!(spacing::LG, 24.0);
    assert_eq!(spacing::XL, 32.0);
}

/// The sidebar's deliberate density reduction survives the move onto the Material scale.
///
/// It is no longer "80% of body, rounded" — §2.4 maps each sidebar role to the *nearest smaller
/// role* rather than to an invented size, which is what keeps it inside the scale instead of
/// beside it. What must not happen is the reduction being lost: the sidebar's text stays smaller
/// than the body text it nests under, and that is the property worth asserting.
#[test]
#[allow(clippy::assertions_on_constants)] // the point is to guard the values; clippy can see the answer
fn the_sidebar_stays_denser_than_the_body_text() {
    assert!(
        typography::SIDEBAR_NAME.size < typography::BODY_MEDIUM.size,
        "the sidebar name ({}) is no longer smaller than body text ({}) — feature 009's density \
         decision has been lost in the move onto the Material scale (FR-011)",
        typography::SIDEBAR_NAME.size,
        typography::BODY_MEDIUM.size
    );
    assert!(
        typography::SIDEBAR_TAG.size < typography::LABEL_MEDIUM.size,
        "the sidebar tag is no longer smaller than the label role"
    );
    assert_eq!(
        typography::SIDEBAR_SESSION,
        typography::SIDEBAR_NAME,
        "a session line and the worktree name it nests under share a role (§2.4)"
    );
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
