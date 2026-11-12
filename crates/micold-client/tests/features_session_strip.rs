//! The session strip's pure rules, in isolation (feature 1126, T003, T032, SC-004).
//!
//! `features::session_strip` holds what the strip above the terminal decides without drawing: its
//! members, which is marked, and the strip's arithmetic. The module's own unit tests cover each rule
//! against fixture sessions; this file pins the public surface from outside the crate, building only
//! a `State` and naming no other feature's types.

use crate::support::covered_states::transcript_pane_insights;
use micold_client::app::{Message, State};
use micold_client::features::header_bar::{HeaderMenu, Msg as HeaderMsg};
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::session_strip::{
    marked_tab, marked_tab_index, member_offsets, scroll_into_view, Member, PlusMenuModel,
    StripModel, StripTab, ANOTHER_SESSION, PLUS_MENU_NAME, TERMINAL,
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

#[test]
fn the_plus_menu_offers_another_session_and_a_terminal_in_the_shown_location() {
    let state = transcript_pane_insights();
    let menu = PlusMenuModel::of(&state).expect("a session is shown");
    assert_eq!(PLUS_MENU_NAME, "Add session or terminal");
    let labels: Vec<_> = menu.items.iter().map(|item| item.label).collect();
    assert_eq!(labels, [ANOTHER_SESSION, TERMINAL]);
    assert_eq!(ANOTHER_SESSION, "Another session");
    assert_eq!(TERMINAL, "Terminal");
    let location = state.current_session_location().expect("a location");
    assert_eq!(
        menu.items[0].press,
        Message::Session(SessionMsg::AdditionalRequested(location)),
        "the session entry asks for the existing add-session choice"
    );
    assert_eq!(
        menu.items[1].press,
        Message::Session(SessionMsg::ShellInstanceOpenRequested),
        "the terminal entry opens a terminal, every time"
    );
    assert_eq!(PlusMenuModel::of(&State::default()), None, "nothing shown");
}

#[test]
fn there_is_one_plus_and_it_opens_the_menu() {
    let state = transcript_pane_insights();
    let model = StripModel::of(&state).expect("a strip");
    let pluses: Vec<_> = model
        .members
        .iter()
        .filter(|m| matches!(m, Member::Plus))
        .collect();
    assert_eq!(pluses.len(), 1, "exactly one `+`");
    assert_eq!(
        pluses[0].press(),
        Message::HeaderBar(HeaderMsg::Toggled(HeaderMenu::Plus)),
        "it opens the menu rather than adding a terminal"
    );
}

#[test]
fn choosing_an_entry_closes_the_plus_menu() {
    let mut state = transcript_pane_insights();
    for press in [
        Message::Session(SessionMsg::ShellInstanceOpenRequested),
        Message::Session(SessionMsg::AdditionalRequested(
            state.current_session_location().expect("a location"),
        )),
    ] {
        // A selection change closes a menu, and the first update records the selection.
        state.update(Message::HeaderBar(HeaderMsg::Dismissed));
        state.update(Message::HeaderBar(HeaderMsg::Toggled(HeaderMenu::Plus)));
        assert_eq!(state.header_bar.menu, HeaderMenu::Plus);
        state.update(press);
        assert_eq!(state.header_bar.menu, HeaderMenu::None, "the menu closed");
    }
}
