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
