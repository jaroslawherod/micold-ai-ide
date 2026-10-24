//! The session strip's pure rules, in isolation (feature 1126, T003, T032, SC-004).
//!
//! `features::session_strip` holds what the strip above the terminal decides without drawing: its
//! members, which is marked, and the strip's arithmetic. The module's own unit tests cover each rule
//! against fixture sessions; this file pins the public surface from outside the crate, building only
//! a `State` and naming no other feature's types.

use micold_client::app::State;
use micold_client::features::session_strip::{
    marked_tab, marked_tab_index, member_offsets, scroll_into_view, Member, StripModel, StripTab,
};
use micold_core::session::{SessionId, ShellInstanceId};

#[test]
fn with_no_session_shown_the_ai_tab_is_marked_and_there_is_no_strip() {
    let state = State::default();
    assert_eq!(
        marked_tab(&state, SessionId::new()),
        StripTab::Ai,
        "the AI tab is the marked one"
    );
    assert_eq!(marked_tab_index(&state), None, "but no member is marked");
    assert_eq!(StripModel::of(&state), None, "and there is no strip");
}

fn shells(n: u32) -> Vec<Member> {
    let session = SessionId::new();
    (1..=n)
        .map(|i| Member::Shell {
            session,
            instance: ShellInstanceId(i),
            label: i.to_string(),
            marked: false,
            stopped: false,
        })
        .collect()
}

#[test]
fn the_strip_measures_its_members_from_the_leading_edge() {
    assert!(member_offsets(&[]).is_empty());
    let two = member_offsets(&shells(2));
    assert_eq!(two[0].0, 0.0, "the first member starts at the leading edge");
    assert!(two[1].0 > two[0].1, "a gap between two members");
}

#[test]
fn a_member_scrolls_just_into_view_and_not_when_it_is_already_there() {
    let members = shells(6);
    let one = member_offsets(&members)[0].1;
    assert_eq!(
        scroll_into_view(0, 0.0, one * 3.0, &members),
        None,
        "in view"
    );
    assert_eq!(
        scroll_into_view(0, one, one * 3.0, &members),
        Some(0.0),
        "behind the leading edge"
    );
    let past = scroll_into_view(5, 0.0, one, &members).expect("past the trailing edge");
    assert!(past > 0.0);
    assert_eq!(
        scroll_into_view(5, past, one, &members),
        None,
        "just in view after the scroll"
    );
}
