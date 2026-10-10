//! The Daemons feature in isolation (feature 491, SC-004): it constructs only its own types and
//! the core's, and names no other feature.

use micold_client::features::daemons::{self, State};

#[test]
fn a_fresh_state_holds_nothing() {
    let state = State::default();
    assert!(state.draft.is_none());
    assert!(state.prompt().is_none());
    assert!(state.ended_by_removal().is_empty());
    assert!(daemons::details(&state, micold_core::daemons::DaemonId(1))
        .errors
        .is_empty());
}
