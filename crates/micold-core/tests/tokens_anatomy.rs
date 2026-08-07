//! Component anatomy matches contract §7 (feature 018, T042 — FR-025 – FR-032, SC-008).
//!
//! A component "matching Material" is mostly a list of numbers: how tall an app bar is, how much
//! padding a dialog has, how wide a chip's ends are. Each is individually unremarkable and each is
//! individually wrong today, so the only way to know the set is right is to write it down against
//! the contract and compare.
//!
//! # Why these live in the render-free core
//!
//! The same reason the type scale and the palette do (feature 018's Phase 0): a number that only
//! exists inside a widget is a number no test can see without a renderer. Held here, the whole
//! anatomy is checkable in a crate that has never heard of iced, and a component that disagrees
//! with the contract fails the build in milliseconds rather than by eye.
//!
//! # Heights are derived, not restated
//!
//! `density.rs` already owns the base heights and the one piece of arithmetic that applies a
//! density step. Anything with a height therefore asks *it* rather than declaring its own copy —
//! two constants for one row height is precisely how the sidebar ended up at 28dp while the
//! contract said 36dp. What [`anatomy`] adds is everything density does not have an opinion about:
//! paddings, gaps, thicknesses and widths.

use micold_core::tokens::{anatomy, density};

// -------------------------------------------------------------------------------------------
// §7.1 Top app bar
// -------------------------------------------------------------------------------------------

/// The small variant, and only the small variant — the contract adopts neither medium nor large.
#[test]
fn the_app_bar_is_the_small_variant() {
    assert_eq!(anatomy::app_bar::HEIGHT, 64.0);
    assert_eq!(anatomy::app_bar::PADDING, 16.0);
    assert_eq!(
        anatomy::app_bar::LEADING_ICON_PADDING,
        4.0,
        "a leading icon button sits closer to the edge than text does, because its own padding \
         supplies the rest of the gap"
    );
    assert_eq!(anatomy::app_bar::ICON_TARGET, 48.0);
    assert_eq!(anatomy::app_bar::DIVIDER, 1.0);
    assert_eq!(
        anatomy::app_bar::BOTTOM_EDGE,
        65.0,
        "the bar plus its divider — what a panel anchored below the bar has to clear (FR-029a). \
         Stated as a constant so no component has to add the two up by eye, which is what BUG-003 \
         was: two components that each guessed 52"
    );
}

// -------------------------------------------------------------------------------------------
// §7.2 List and tree rows — two named densities, no third
// -------------------------------------------------------------------------------------------

/// Both densities resolve through the density scale rather than being written down twice.
#[test]
fn the_two_row_densities_are_the_contract_heights() {
    assert_eq!(
        density::height(density::LIST_ROW_BASE, density::STANDARD),
        48.0,
        "the standard row is 48dp (§7.2)"
    );
    assert_eq!(
        density::height(density::LIST_ROW_BASE, density::DENSE),
        36.0,
        "the dense row is 36dp (§7.2) — the sidebar's compactness as a named density step, not an \
         ad-hoc shrink"
    );
}

/// Each density carries its own horizontal padding and leading-icon gap.
#[test]
fn each_row_density_has_its_own_padding_and_icon_gap() {
    assert_eq!(anatomy::list_row::STANDARD_PADDING, 16.0);
    assert_eq!(anatomy::list_row::DENSE_PADDING, 8.0);
    assert_eq!(anatomy::list_row::STANDARD_ICON_GAP, 16.0);
    assert_eq!(anatomy::list_row::DENSE_ICON_GAP, 8.0);
}

/// The dense row really is denser. Stated as a property rather than as two numbers, because the
/// point of the dense density is the *relationship* — it exists to keep the sidebar compact.
#[test]
#[allow(clippy::assertions_on_constants)] // guarding the values is the point; clippy can see the answer
fn the_dense_row_is_shorter_and_tighter_than_the_standard_one() {
    assert!(
        density::height(density::LIST_ROW_BASE, density::DENSE)
            < density::height(density::LIST_ROW_BASE, density::STANDARD)
    );
    assert!(anatomy::list_row::DENSE_PADDING < anatomy::list_row::STANDARD_PADDING);
    assert!(anatomy::list_row::DENSE_ICON_GAP < anatomy::list_row::STANDARD_ICON_GAP);
}

// -------------------------------------------------------------------------------------------
// §7.3 Buttons — the minimum touch target is the load-bearing one
// -------------------------------------------------------------------------------------------

/// 48×48 is honoured even where the visible container is smaller (§7.3).
///
/// The container is 40dp and the icon button's glyph is 24dp, so every button in the application
/// has a visible box smaller than the target it must present. A target that merely matched the
/// container would look identical and be wrong on every one of them.
#[test]
fn the_minimum_touch_target_exceeds_every_visible_container() {
    assert_eq!(anatomy::button::MIN_TOUCH_TARGET, 48.0);
    assert!(
        anatomy::button::MIN_TOUCH_TARGET
            > density::height(density::BUTTON_BASE, density::STANDARD),
        "the touch target must exceed the 40dp container, or honouring it is a no-op"
    );
}

#[test]
fn buttons_carry_the_contract_padding_and_icon_sizes() {
    assert_eq!(
        density::height(density::BUTTON_BASE, density::STANDARD),
        40.0
    );
    assert_eq!(anatomy::button::PADDING_FILLED, 24.0);
    assert_eq!(anatomy::button::PADDING_OUTLINED, 24.0);
    assert_eq!(anatomy::button::PADDING_TEXT, 12.0);
    assert_eq!(anatomy::button::PADDING_ICON, 8.0);
    assert_eq!(anatomy::button::LEADING_ICON, 18.0);
    assert_eq!(anatomy::button::ICON_BUTTON_GLYPH, 24.0);
}

// -------------------------------------------------------------------------------------------
// §7.4 Dialogs
// -------------------------------------------------------------------------------------------

#[test]
fn the_dialog_anatomy_matches_the_contract() {
    assert_eq!(anatomy::dialog::PADDING, 24.0);
    assert_eq!(anatomy::dialog::TITLE_TO_BODY, 16.0);
    assert_eq!(anatomy::dialog::BODY_TO_ACTIONS, 24.0);
    assert_eq!(anatomy::dialog::ACTION_GAP, 8.0);
    assert_eq!(anatomy::dialog::MIN_WIDTH, 280.0);
    assert_eq!(anatomy::dialog::MAX_WIDTH, 560.0);
    assert_eq!(anatomy::dialog::ICON, 24.0);
}

/// A dialog cannot be narrower than it is wide. Trivially true of the written numbers, and the
/// kind of thing that stops being true when one of them is edited alone.
#[test]
#[allow(clippy::assertions_on_constants)] // guarding the values is the point; clippy can see the answer
fn the_dialog_width_bounds_are_the_right_way_round() {
    assert!(anatomy::dialog::MIN_WIDTH < anatomy::dialog::MAX_WIDTH);
}

// -------------------------------------------------------------------------------------------
// §7.5 Menus
// -------------------------------------------------------------------------------------------

#[test]
fn the_menu_anatomy_matches_the_contract() {
    assert_eq!(
        density::height(density::MENU_ITEM_BASE, density::STANDARD),
        48.0,
        "a menu item is a 48dp row (§7.5)"
    );
    assert_eq!(anatomy::menu::VERTICAL_PADDING, 8.0);
    assert_eq!(anatomy::menu::ITEM_PADDING, 12.0);
    assert_eq!(anatomy::menu::ITEM_ICON, 24.0);
    assert_eq!(anatomy::menu::DIVIDER, 1.0);
}

// -------------------------------------------------------------------------------------------
// §7.6 Chips
// -------------------------------------------------------------------------------------------

#[test]
fn the_chip_anatomy_matches_the_contract() {
    assert_eq!(anatomy::chip::HEIGHT, 32.0);
    assert_eq!(anatomy::chip::PADDING, 12.0);
    assert_eq!(
        anatomy::chip::PADDING_WITH_ICON,
        8.0,
        "a leading icon takes some of the padding's job (§7.6)"
    );
    assert_eq!(anatomy::chip::ICON, 18.0);
}

// -------------------------------------------------------------------------------------------
// §7.7 Text fields — the largest departure from what ships today
// -------------------------------------------------------------------------------------------

#[test]
fn the_text_field_anatomy_matches_the_contract() {
    assert_eq!(
        density::height(density::TEXT_FIELD_BASE, density::STANDARD),
        56.0,
        "a filled text field is 56dp — today's is roughly 30dp (§7.7)"
    );
    assert_eq!(anatomy::text_field::PADDING, 16.0);
    assert_eq!(anatomy::text_field::INDICATOR, 1.0);
    assert_eq!(anatomy::text_field::INDICATOR_ACTIVE, 2.0);
    assert_eq!(anatomy::text_field::TRAILING_ICON, 24.0);
}

/// The active indicator thickens on focus. That difference *is* the focus affordance for a filled
/// field — there is no border to recolour — so an equal pair would leave focus invisible.
#[test]
#[allow(clippy::assertions_on_constants)] // guarding the values is the point; clippy can see the answer
fn the_active_indicator_thickens_when_the_field_is_focused() {
    assert!(
        anatomy::text_field::INDICATOR_ACTIVE > anatomy::text_field::INDICATOR,
        "the focused indicator must be thicker than the resting one, or focus is unreadable"
    );
}

// -------------------------------------------------------------------------------------------
// §7.8 Snackbar
// -------------------------------------------------------------------------------------------

#[test]
fn the_snackbar_anatomy_matches_the_contract() {
    assert_eq!(anatomy::snackbar::MIN_HEIGHT, 48.0);
    assert_eq!(anatomy::snackbar::PADDING_H, 16.0);
    assert_eq!(anatomy::snackbar::PADDING_V, 14.0);
    assert_eq!(anatomy::snackbar::MAX_WIDTH, 600.0);
}

// -------------------------------------------------------------------------------------------
// §7.9 Progress indicator
// -------------------------------------------------------------------------------------------

#[test]
fn the_progress_anatomy_matches_the_contract() {
    assert_eq!(anatomy::progress::THICKNESS, 4.0);
    assert_eq!(anatomy::progress::LABEL_GAP, 4.0);
}

// -------------------------------------------------------------------------------------------
// Properties that hold across the whole anatomy
// -------------------------------------------------------------------------------------------

/// Every anatomy value is a positive, finite number of dp.
///
/// Cheap, and it catches the two ways a constant goes wrong without looking wrong: a sign flip, and
/// a `0.0` left behind by a placeholder.
#[test]
fn every_anatomy_value_is_a_positive_finite_length() {
    for (name, value) in anatomy::ALL {
        assert!(
            value.is_finite() && value > 0.0,
            "{name} is {value}dp, which is not a length"
        );
    }
}

/// The inventory covers the anatomy rather than a sample of it.
///
/// Without this the property above is only as good as whoever last remembered to extend `ALL`, and
/// a check that silently stops covering new constants is the failure mode this whole file exists
/// to avoid.
#[test]
fn the_inventory_is_not_a_token_sample() {
    assert!(
        anatomy::ALL.len() >= 30,
        "only {} anatomy values are listed — §7 specifies far more, so `ALL` has fallen behind and \
         the property check above is covering a fraction of what it claims",
        anatomy::ALL.len()
    );
}

/// Every height in the anatomy lands on a whole dp at every density step.
///
/// A fractional row height renders blurred, and the density scale is what makes it possible for one
/// to appear: a base that is not a multiple of the step size produces one at some step.
#[test]
fn no_component_height_becomes_fractional_at_any_density() {
    for (name, base) in [
        ("list row", density::LIST_ROW_BASE),
        ("menu item", density::MENU_ITEM_BASE),
        ("text field", density::TEXT_FIELD_BASE),
        ("button", density::BUTTON_BASE),
    ] {
        for step in density::STEPS {
            let h = density::height(base, step);
            assert_eq!(
                h.fract(),
                0.0,
                "the {name} is {h}dp at density step {step}, which cannot land on a pixel boundary"
            );
            assert!(h > 0.0, "the {name} collapses to {h}dp at step {step}");
        }
    }
}
