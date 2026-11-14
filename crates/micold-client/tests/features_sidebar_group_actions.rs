//! The user-group actions in isolation (feature 1786; SC-004 of feature 021): this feature's own
//! state and messages only. Creating a group's effect on the layout is in
//! `tests/sidebar_group_actions.rs`, which reads the sidebar's layout through the root.

use micold_client::app::State;
use micold_client::features::sidebar_group_actions::{refusal, update, Msg};
use micold_core::sidebar_layout::LayoutError;

#[test]
fn with_no_project_new_opens_nothing_and_a_confirm_does_nothing() {
    let mut state = State::default();
    assert!(update(&mut state, Msg::New).is_empty());
    assert!(state.sidebar_groups.naming.is_none());
    assert!(update(&mut state, Msg::Confirmed).is_empty());
    assert!(update(&mut state, Msg::Cancelled).is_empty());
}

#[test]
fn the_refusals_name_the_reason() {
    assert_eq!(
        refusal(LayoutError::Blank, "  "),
        "Enter a name for the group."
    );
    assert_eq!(
        refusal(LayoutError::Duplicate, " Work "),
        "A group named \u{201c}Work\u{201d} already exists."
    );
}
