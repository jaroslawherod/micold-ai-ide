//! An agent's prompt and the dialog the user is already in (feature 034, FR-014).
//!
//! A prompt that arrives while another dialog is open waits behind it instead of closing it, so an
//! unsaved form is never lost to an agent's request; it opens when that dialog closes. Uses the
//! About dialog as "another dialog": any registered dialog behaves the same.

use micold_client::app::{Message, State};
use micold_client::features::agent_confirm::{Msg, Prompt};
use micold_client::features::help;
use micold_client::overlay::registry;
use micold_core::protocol::messages::ConfirmOperation;
use std::path::PathBuf;

fn prompt(id: u64) -> Prompt {
    Prompt {
        id,
        project: PathBuf::from("/fixture/project"),
        caller_label: "planner".to_string(),
        operation: ConfirmOperation::DeleteSession,
        target_label: "reviewer".to_string(),
    }
}

fn send(state: &mut State, msg: Msg) {
    state.update(Message::AgentConfirm(msg));
}

fn open_dialog(state: &State) -> Option<&'static str> {
    registry::open_dialog(state).map(|open| open.id().as_str())
}

fn shown_id(state: &State) -> Option<u64> {
    state.agent_confirm.shown().map(|p| p.id)
}

/// Phase A open point 1: a prompt does not close a dialog the user is in; it waits behind it and
/// opens when that dialog closes.
#[test]
fn a_prompt_waits_behind_an_open_dialog_and_opens_when_it_closes() {
    let mut st = State::default();
    st.update(Message::Help(help::Msg::AboutOpened));
    let other = open_dialog(&st).expect("a dialog is open");

    send(&mut st, Msg::Requested(prompt(1)));

    assert_eq!(open_dialog(&st), Some(other), "the open dialog stays");
    assert!(st.agent_confirm.is_pending(1), "the prompt is kept");
    assert_eq!(shown_id(&st), None, "but not shown yet");

    st.update(Message::EscapePressed);

    assert_eq!(open_dialog(&st), Some("confirm_agent_request"));
    assert_eq!(shown_id(&st), Some(1));
    assert!(st.agent_confirm.declined.is_empty(), "nothing was declined");
}

/// A held prompt withdrawn before it is shown leaves nothing behind.
#[test]
fn a_held_prompt_that_is_withdrawn_is_gone() {
    let mut st = State::default();
    st.update(Message::Help(help::Msg::AboutOpened));
    send(&mut st, Msg::Requested(prompt(1)));

    send(&mut st, Msg::Withdrawn(1));
    st.update(Message::EscapePressed);

    assert_eq!(open_dialog(&st), None);
    assert!(!st.agent_confirm.held);
}

/// Review A: another dialog opening over the prompt declines the shown prompt only. The queued
/// ones, which the user never saw, wait behind the new dialog.
#[test]
fn a_dialog_opening_over_the_prompt_declines_only_the_shown_one() {
    let mut st = State::default();
    for id in 1..=3 {
        send(&mut st, Msg::Requested(prompt(id)));
    }
    assert_eq!(shown_id(&st), Some(1));

    st.update(Message::Help(help::Msg::AboutOpened));

    assert_eq!(st.agent_confirm.declined, vec![1]);
    assert!(st.agent_confirm.is_pending(2) && st.agent_confirm.is_pending(3));
    assert_ne!(open_dialog(&st), Some("confirm_agent_request"));

    st.update(Message::EscapePressed);

    assert_eq!(
        shown_id(&st),
        Some(2),
        "the next prompt opens as the dialog closes"
    );
    assert_eq!(st.agent_confirm.declined, vec![1]);
}

/// Escape on the shown prompt declines it, and the next queued prompt opens at once.
#[test]
fn escape_declines_the_shown_prompt_and_the_next_opens() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(1)));
    send(&mut st, Msg::Requested(prompt(2)));

    st.update(Message::EscapePressed);

    assert_eq!(st.agent_confirm.declined, vec![1]);
    assert_eq!(shown_id(&st), Some(2));
    assert_eq!(open_dialog(&st), Some("confirm_agent_request"));
}

/// Review A: losing the connection voids every prompt without declining any; a restarted service
/// reuses ids, so a kept prompt could be answered for a different request.
#[test]
fn losing_the_connection_drops_every_prompt_without_declining() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(1)));
    send(&mut st, Msg::Requested(prompt(2)));

    send(&mut st, Msg::Disconnected);

    assert!(!st.agent_confirm.is_pending(1) && !st.agent_confirm.is_pending(2));
    assert!(st.agent_confirm.declined.is_empty());
    send(&mut st, Msg::Requested(prompt(1)));
    assert_eq!(
        shown_id(&st),
        Some(1),
        "a new request with a reused id is shown"
    );
}
