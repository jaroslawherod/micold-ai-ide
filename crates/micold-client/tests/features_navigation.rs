//! Keyboard navigation in isolation (feature 803, SC-004 of feature 021).
//!
//! The file builds a default `State` (the resolver reads one) and names no other feature's types.
//! What a press lands on in a populated project is `tests/navigation.rs`; how the root applies it
//! is `tests/navigation_apply.rs`.

use micold_client::app::State;
use micold_client::features::navigation::{self, Step};
use micold_client::keymap::{NavAction, NavLevel};
use micold_core::navigation::Dir;

#[test]
fn with_no_project_open_there_is_no_location_and_a_press_changes_nothing() {
    let state = State::default();
    assert!(navigation::location_order(&state).is_empty());
    for dir in [Dir::Next, Dir::Previous] {
        let action = NavAction {
            level: NavLevel::Location,
            dir,
        };
        assert_eq!(navigation::update(&state, action), Step::None);
    }
}
