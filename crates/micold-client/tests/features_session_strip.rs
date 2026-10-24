//! The session strip's pure rules, in isolation (feature 1126, T003, SC-004).
//!
//! `features::session_strip` holds what the strip under (and, from M2, above) the terminal decides
//! without drawing: which tab is marked, and the strip's arithmetic. The module's own unit tests
//! cover each rule against fixture sessions; this file pins the public surface from outside the
//! crate, building only a `State` and naming no other feature's types.

use micold_client::app::State;
use micold_client::features::session_strip::{
    leading_slack, marked_tab, marked_tab_index, natural_strip_width, scroll_into_view, StripTab,
};
use micold_core::session::SessionId;

#[test]
fn with_no_session_shown_the_ai_tab_is_marked_and_nothing_scrolls() {
    let state = State::default();
    assert_eq!(marked_tab(&state, SessionId::new()), StripTab::Ai);
    assert_eq!(marked_tab_index(&state), None);
}

#[test]
fn the_strip_measures_its_tabs_and_leaves_the_rest_at_its_leading_edge() {
    assert_eq!(natural_strip_width(0), 0.0);
    let one = natural_strip_width(1);
    let two = natural_strip_width(2);
    assert!(one > 0.0 && two > 2.0 * one, "a gap between two tabs");
    assert_eq!(leading_slack(two + 10.0, 2), 10.0);
    assert_eq!(leading_slack(one, 2), 0.0, "no slack once the tabs overflow");
}

#[test]
fn a_tab_scrolls_just_into_view_and_not_when_it_is_already_there() {
    let one = natural_strip_width(1);
    assert_eq!(scroll_into_view(0, 0.0, one * 3.0), None, "in view");
    assert_eq!(scroll_into_view(0, one, one * 3.0), Some(0.0), "behind the leading edge");
    let past = scroll_into_view(5, 0.0, one).expect("past the trailing edge");
    assert!(past > 0.0);
    assert_eq!(scroll_into_view(5, past, one), None, "just in view after the scroll");
}
