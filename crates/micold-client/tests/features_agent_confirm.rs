//! An agent's destructive request, in isolation (feature 034, FR-014; U207-U213).
//!
//! Builds a `State` and names no other feature's types. What reaches the wire when the user answers
//! is the shell's, and is held in `shell::daemon_sync`'s own tests.

use micold_client::app::{Message, State};
use micold_client::features::agent_confirm::{self, Msg, Prompt};
use micold_client::overlay::registry;
use micold_core::protocol::messages::ConfirmOperation;
use std::path::PathBuf;

fn prompt(id: u64, target: &str) -> Prompt {
    Prompt {
        id,
        project: PathBuf::from("/fixture/project"),
        caller_label: "planner".to_string(),
        operation: ConfirmOperation::DeleteWorktree {
            stop_sessions: false,
            delete_branch: false,
        },
        target_label: target.to_string(),
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

#[test]
fn u207_a_request_adds_a_pending_prompt_and_opens_the_dialog() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(7, "feat-x")));

    assert_eq!(st.agent_confirm.pending, vec![prompt(7, "feat-x")]);
    assert_eq!(open_dialog(&st), Some("confirm_agent_request"));
}

#[test]
fn u208_a_withdrawal_removes_the_prompt_and_closes_the_dialog() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(7, "feat-x")));
    send(&mut st, Msg::Withdrawn(7));

    assert!(st.agent_confirm.pending.is_empty());
    assert_eq!(open_dialog(&st), None);
}

#[test]
fn u209_a_withdrawal_for_an_unknown_id_changes_nothing() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(7, "feat-x")));
    let before = st.clone();

    send(&mut st, Msg::Withdrawn(99));

    assert_eq!(st, before);
}

#[test]
fn u210_allowing_closes_that_prompt() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(7, "feat-x")));
    send(&mut st, Msg::Answered { id: 7, allow: true });

    assert!(st.agent_confirm.pending.is_empty());
    assert_eq!(open_dialog(&st), None);
}

#[test]
fn u211_denying_closes_that_prompt() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(7, "feat-x")));
    send(
        &mut st,
        Msg::Answered {
            id: 7,
            allow: false,
        },
    );

    assert!(st.agent_confirm.pending.is_empty());
    assert_eq!(open_dialog(&st), None);
}

#[test]
fn u212_several_prompts_show_in_arrival_order() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(3, "first")));
    send(&mut st, Msg::Requested(prompt(1, "second")));

    assert_eq!(shown_id(&st), Some(3), "the oldest shows first");
    assert_eq!(open_dialog(&st), Some("confirm_agent_request"));

    send(&mut st, Msg::Answered { id: 3, allow: true });
    assert_eq!(shown_id(&st), Some(1), "then the next one");
    assert_eq!(open_dialog(&st), Some("confirm_agent_request"));
}

#[test]
fn a_duplicate_id_is_kept_once() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(7, "feat-x")));
    send(&mut st, Msg::Requested(prompt(7, "feat-x")));

    assert_eq!(st.agent_confirm.pending.len(), 1);
}

#[test]
fn a_second_request_does_not_dismiss_the_one_showing() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(1, "a")));
    send(&mut st, Msg::Requested(prompt(2, "b")));

    let ids: Vec<u64> = st.agent_confirm.pending.iter().map(|p| p.id).collect();
    assert_eq!(
        ids,
        vec![1, 2],
        "the shown prompt stays; the new one queues behind it"
    );
}

#[test]
fn dismissing_drops_only_the_shown_prompt() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(1, "a")));
    send(&mut st, Msg::Requested(prompt(2, "b")));

    send(&mut st, Msg::Dismissed);

    assert_eq!(shown_id(&st), Some(2));
    assert_eq!(st.agent_confirm.pending.len(), 1);
}

#[test]
fn escape_dismisses_the_shown_prompt() {
    let mut st = State::default();
    send(&mut st, Msg::Requested(prompt(1, "a")));

    assert_eq!(
        registry::escape(&st),
        Some(Message::AgentConfirm(Msg::Dismissed)),
        "Escape dismisses, it does not deny"
    );
}

#[test]
fn the_headline_names_the_caller_the_operation_and_the_target() {
    let mut p = prompt(1, "feat-x");
    assert_eq!(
        agent_confirm::headline(&p),
        "“planner” asks to delete worktree “feat-x”"
    );

    p.operation = ConfirmOperation::DeleteWorktree {
        stop_sessions: true,
        delete_branch: true,
    };
    assert_eq!(
        agent_confirm::headline(&p),
        "“planner” asks to delete worktree “feat-x” and its branch, stopping its sessions"
    );

    p.operation = ConfirmOperation::SendInput;
    p.target_label = "reviewer".to_string();
    assert_eq!(
        agent_confirm::headline(&p),
        "“planner” asks to type into session “reviewer”"
    );
}

#[test]
fn every_operation_has_its_own_phrase() {
    let ops = [
        ConfirmOperation::DeleteWorktree {
            stop_sessions: false,
            delete_branch: false,
        },
        ConfirmOperation::DeleteWorktree {
            stop_sessions: false,
            delete_branch: true,
        },
        ConfirmOperation::DeleteWorktree {
            stop_sessions: true,
            delete_branch: false,
        },
        ConfirmOperation::DeleteWorktree {
            stop_sessions: true,
            delete_branch: true,
        },
        ConfirmOperation::DeleteSession,
        ConfirmOperation::StopSession,
        ConfirmOperation::InterruptSession,
        ConfirmOperation::SendInput,
    ];
    let phrases: Vec<String> = ops.iter().map(agent_confirm::operation_phrase).collect();
    assert_eq!(
        phrases,
        vec![
            "delete worktree",
            "delete worktree and its branch",
            "delete worktree, stopping its sessions",
            "delete worktree and its branch, stopping its sessions",
            "delete session",
            "stop session",
            "interrupt session",
            "type into session",
        ]
    );
}
