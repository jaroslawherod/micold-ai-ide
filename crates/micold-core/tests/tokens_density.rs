//! The density scale (feature 018, T000d — FR-026b, FR-026c; contract §7.2).
//!
//! Density is a theming axis applied uniformly, not a per-component enumeration. FR-026b states the
//! arithmetic and then requires it be *asserted* rather than trusted, which is what this file is.
//!
//! The requirement worth restating: the moment one list defines its own "compact" variant, density
//! stops being something anyone can reason about globally and becomes a scattering of independent
//! decisions that drift. So the assertions here are as much about the scale having no exceptions as
//! about its numbers.

use micold_core::tokens::density;

/// Exactly four steps. A fifth would mean a component had invented its own, which FR-026b forbids
/// precisely because density stops being a uniform theming axis the moment one component opts out.
#[test]
fn the_density_scale_has_exactly_four_steps() {
    assert_eq!(density::STEPS, [0, -1, -2, -3]);
}

/// Each step below 0 subtracts 4dp — the arithmetic FR-026b requires be asserted rather than
/// trusted.
#[test]
fn each_step_below_zero_subtracts_four_dp() {
    let base = density::LIST_ROW_BASE;
    for step in density::STEPS {
        let expected = base + (step as f32) * 4.0;
        assert_eq!(
            density::height(base, step),
            expected,
            "density {step} on a {base}dp base"
        );
    }
    assert_eq!(density::height(base, 0), 48.0);
    assert_eq!(density::height(base, -3), 36.0);
}

/// No component resolves to a fractional height (FR-026b). A half-pixel row is a blurred row.
#[test]
fn no_density_step_produces_a_fractional_height() {
    for base in [
        density::LIST_ROW_BASE,
        density::MENU_ITEM_BASE,
        density::TEXT_FIELD_BASE,
        density::BUTTON_BASE,
    ] {
        for step in density::STEPS {
            let h = density::height(base, step);
            assert_eq!(
                h.fract(),
                0.0,
                "density {step} on a {base}dp base gives {h}dp, which is fractional"
            );
            assert!(
                h > 0.0,
                "density {step} on {base}dp collapses the component"
            );
        }
    }
}

/// The sidebar's dense row is an *application* of the scale, not a bespoke variant (FR-026c), and it
/// lands on §7.2's stated 36dp.
#[test]
fn the_sidebar_dense_row_is_density_minus_three_on_the_standard_row() {
    assert_eq!(density::DENSE, -3);
    assert_eq!(density::STANDARD, 0);
    assert_eq!(
        density::height(density::LIST_ROW_BASE, density::DENSE),
        36.0,
        "the dense sidebar row must be the standard row at density -3 (contract §7.2)"
    );
}
