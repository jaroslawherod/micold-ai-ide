//! The overflow menu and the About dialog it opens, in isolation (feature 021, SC-004).
//!
//! One feature, not two: "About" is the single action the Help menu offers (feature 001, FR-003),
//! so the menu and the dialog live in one module and are tested in one file.
//!
//! Missing until T071 for the reason given in `features_window.rs` — SC-004's "eight feature
//! modules" was the count on the day it was written, and `features/help.rs` was created after it
//! by T031.
//!
//! The file builds a `State` (these reducers take one) and names no other feature's types. That
//! boundary decides something real here: `menu_toggled` returns outcomes, and what they close is
//! three *other* features' surfaces. Asserting on those would be this file's own violation, so it
//! asserts on the outcome's shape and leaves the relation to
//! `tests/popover_displacement.rs`, which is where it is stated in full.
//!
//! The one place it reaches past its own fields is `opening_about_closes_the_menu_it_was_chosen
//! _from`, and that is about help's own two surfaces rather than a neighbour's.

use micold_client::app::State;
use micold_client::features::help::{self, HELP_ACTIONS};

#[test]
fn the_menu_offers_exactly_one_action() {
    // A one-entry menu is a design decision, not an accident of the current feature set: "Help"
    // exists so About has somewhere to live (feature 001, FR-003/FR-004).
    assert_eq!(HELP_ACTIONS, ["About"]);
    assert_eq!(help::help_actions(), &["About"]);
}

#[test]
fn the_menu_toggles_and_reports_only_the_opening() {
    let mut st = State::default();

    let opened = help::menu_toggled(&mut st);
    assert!(st.help_menu_open);
    assert_eq!(
        opened.len(),
        1,
        "opening reports one outcome — that this surface opened"
    );

    let closed = help::menu_toggled(&mut st);
    assert!(!st.help_menu_open);
    assert!(
        closed.is_empty(),
        "shutting it reports nothing: it did not open, and no other surface should move"
    );
}

#[test]
fn opening_about_is_idempotent() {
    // FR-015: opening while already open keeps a single instance rather than stacking a second.
    let mut st = State::default();

    help::about_opened(&mut st);
    assert!(st.about_open);

    help::about_opened(&mut st);
    assert!(st.about_open, "still exactly one, not a second instance");
}

#[test]
fn dismissing_about_when_nothing_is_open_changes_nothing() {
    // FR-012's edge case. A close that has already happened is not an error.
    let mut st = State::default();
    assert!(!st.about_open);

    help::about_closed(&mut st);

    assert!(!st.about_open);
}

#[test]
fn opening_about_closes_the_menu_it_was_chosen_from() {
    // I wrote this test the other way round first, asserting the two fields were independent, and
    // it failed. `about_opened` calls `State::clear_for_dialog` before setting its own flag, which
    // closes whatever dialog and popovers were open (FR-012) — so choosing "About" from the menu
    // puts the dialog up and takes the menu down, which is what a user would expect and what the
    // code has always done.
    //
    // It is worth pinning *here* because the mechanism is invisible from this module: help writes
    // one field, and the closing is a rule over the registry that no feature states. The guard
    // against this becoming a per-surface special case is `tests/overlay_registry.rs`; what this
    // asserts is only that the help feature goes through it.
    let mut st = State::default();
    let _ = help::menu_toggled(&mut st);
    assert!(st.help_menu_open);

    help::about_opened(&mut st);

    assert!(st.about_open);
    assert!(
        !st.help_menu_open,
        "the menu the action was chosen from does not stay open behind the dialog it opened"
    );
}
