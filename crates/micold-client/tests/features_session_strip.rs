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
use micold_client::features::structured_transcript::TranscriptMsg;
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
        if matches!(
            press,
            Message::Session(SessionMsg::ShellInstanceOpenRequested)
        ) {
            // The binary opens the instance; the core sees the active session gain a shell.
            let active = state.session.active.expect("an active session");
            for session in state.workspace.sessions.values_mut().flatten() {
                if session.id == active {
                    session.open_shell_instance();
                }
            }
        }
        state.update(press);
        assert_eq!(state.header_bar.menu, HeaderMenu::None, "the menu closed");
    }
}

fn with_lifecycle(
    mut state: State,
    lifecycle: micold_core::session::SessionLifecycle,
) -> State {
    for session in state.workspace.sessions.values_mut().flatten() {
        session.lifecycle = lifecycle.clone();
    }
    state
}

fn working_of(state: &State) -> Vec<bool> {
    StripModel::of(state)
        .expect("a strip")
        .members
        .iter()
        .filter_map(|m| match m {
            Member::Session { working, .. } => Some(*working),
            _ => None,
        })
        .collect()
}

/// T022 (FR-006): a working session's tab carries the activity dot; failed, exited or idle ones
/// carry none, and no member carries a `running` word.
#[test]
fn only_a_working_session_tab_shows_the_activity_dot() {
    use micold_core::session::SessionLifecycle;
    let running = with_lifecycle(transcript_pane_insights(), SessionLifecycle::Running);
    assert_eq!(working_of(&running), [true], "a running session is working");
    for lifecycle in [
        SessionLifecycle::Idle,
        SessionLifecycle::Starting,
        SessionLifecycle::Failed {
            reason: "boom".into(),
            attempts: 3,
        },
    ] {
        let state = with_lifecycle(transcript_pane_insights(), lifecycle.clone());
        assert_eq!(working_of(&state), [false], "{lifecycle:?} shows no dot");
    }
    for member in StripModel::of(&running).expect("a strip").members {
        if let Member::Session { label, .. } = member {
            assert!(!label.contains("running"), "no running text: {label:?}");
        }
    }
}

/// T022 (FR-001, FR-010, FR-013): the `N hidden` chip exists only while events are hidden, is named
/// "N hidden events, show" and shows them.
#[test]
fn the_hidden_chip_shows_the_count_only_while_events_are_hidden() {
    let mut state = transcript_pane_insights();
    let session = state.session.active.expect("a session");
    state.settings.show_raw_events = false;
    let hidden = state
        .structured_transcript
        .transcript(session)
        .expect("a transcript")
        .hidden_count(false);
    assert!(hidden > 0, "the premise: the recording hides events");
    let chip = StripModel::of(&state)
        .expect("a strip")
        .hidden
        .expect("a chip while events are hidden");
    assert_eq!(chip.text, format!("{hidden} hidden"));
    assert_eq!(chip.name, format!("{hidden} hidden events, show"));
    assert_eq!(
        chip.press,
        Message::Session(SessionMsg::Transcript(TranscriptMsg::ShowRawEvents(true)))
    );
    state.settings.show_raw_events = true;
    assert_eq!(
        StripModel::of(&state).expect("a strip").hidden,
        None,
        "nothing is hidden with raw events shown"
    );
}
