//! The attach-worktrees feature in isolation (feature 582, SC-004). The reducer's behaviours are
//! in `attach_dialog.rs`; this file holds the isolation obligation: it builds a `State` and names
//! only this feature's types.

use micold_client::app::State;
use micold_client::features::attach::{self, Listing, Msg};
use std::path::PathBuf;

#[test]
fn opening_then_cancelling_leaves_nothing_behind() {
    let mut state = State::default();
    state.workspace.active = Some(PathBuf::from("/p"));

    attach::update(&mut state, Msg::Opened);
    assert_eq!(
        state.attach.dialog.as_ref().map(|d| d.listing.clone()),
        Some(Listing::Loading)
    );

    let outcomes = attach::update(&mut state, Msg::Cancelled);
    assert!(state.attach.dialog.is_none());
    assert!(outcomes.is_empty());
}
