//! The window's permission prompts, in isolation (#942 M6, data-model *Client state*, T049–T054).
//!
//! Render-free: what a structured session's pending requests are, what the user typed and chose
//! for each, what an answer sends, and how the window learns that a request ended. The draw code
//! (M7, M8) reads this state; the shell (`shell/permission_prompts.rs`, `shell/daemon_sync.rs`)
//! sends what it decides and routes the replies to the calls tested here.

use micold_client::features::permission_prompts::{Action, Placement, State};
use micold_core::protocol::messages::{
    PermissionRequest, PromptChoice, PromptId, QuestionAnswer, Resolution, WithdrawReason,
};
use micold_core::session::{PermissionMode, SessionId};
use micold_core::structured::permission::request::{Question, QuestionOption, RequestKind};
use micold_core::structured::transcript::{EntryKey, Transcript};
use micold_core::structured::{parse_line, SequencedEvent};

const ID: PromptId = PromptId(7);

fn request(session: SessionId, id: PromptId, kind: RequestKind) -> PermissionRequest {
    PermissionRequest {
        id,
        session,
        tool_use_id: Some(format!("toolu_{}", id.0)),
        tool_name: "Bash".into(),
        input_preview: r#"{"command":"npm test"}"#.into(),
        input_bytes: 22,
        kind,
        always_allow: Some(r#"Bash: "npm test""#.into()),
    }
}

fn tool(session: SessionId) -> PermissionRequest {
    request(session, ID, RequestKind::Tool)
}

fn plan(session: SessionId) -> PermissionRequest {
    PermissionRequest {
        tool_name: "ExitPlanMode".into(),
        always_allow: None,
        ..request(
            session,
            ID,
            RequestKind::PlanApproval {
                plan: "# Plan\n\n1. Read.\n2. Change.\n".repeat(2_000),
            },
        )
    }
}

fn question(header: &str, multi_select: bool) -> Question {
    Question {
        header: header.into(),
        text: format!("{header}?"),
        options: ["One", "Two", "Three"]
            .into_iter()
            .map(|label| QuestionOption {
                label: label.into(),
                description: String::new(),
            })
            .collect(),
        multi_select,
    }
}

/// Two questions: the first single choice, the second multiple choice.
fn questions(session: SessionId) -> PermissionRequest {
    PermissionRequest {
        tool_name: "AskUserQuestion".into(),
        always_allow: None,
        ..request(
            session,
            ID,
            RequestKind::Question {
                questions: vec![question("Database", false), question("Features", true)],
            },
        )
    }
}

fn holding(request: PermissionRequest) -> (State, SessionId) {
    let session = request.session;
    let mut state = State::default();
    state.requested(request);
    (state, session)
}

// ---- T049: tool requests, resolutions, sending, sessions, placement ---------------------------

#[test]
fn a_request_is_shown_under_its_session_and_a_second_one_for_a_held_id_is_ignored() {
    let (mut state, session) = holding(tool(SessionId::new()));
    let shown = state.shown(session, ID).expect("shown");
    assert_eq!(shown.request.tool_name, "Bash");
    assert!(!shown.sending);

    state.edit_deny_message(session, ID, "not now".into());
    let mut again = tool(session);
    again.tool_name = "Write".into();
    state.requested(again);
    let shown = state.shown(session, ID).expect("still shown");
    assert_eq!(shown.request.tool_name, "Bash", "the held request stays");
    assert_eq!(shown.draft.deny_message, "not now", "and so does its draft");
    assert_eq!(state.pending(session).count(), 1);
}

#[test]
fn a_resolution_removes_the_request_and_labels_its_card() {
    for resolution in [
        Resolution::Allowed,
        Resolution::Denied,
        Resolution::Withdrawn {
            reason: WithdrawReason::TurnEnded,
        },
    ] {
        let (mut state, session) = holding(tool(SessionId::new()));
        state.resolved(session, ID, Some("toolu_7".into()), resolution);
        assert!(
            state.shown(session, ID).is_none(),
            "{resolution:?} closes it"
        );
        assert_eq!(state.resolution(session, "toolu_7"), Some(resolution));
        assert_eq!(state.resolution(session, "toolu_8"), None);
    }
}

#[test]
fn a_tool_request_builds_allow_always_allow_and_deny_with_the_typed_message() {
    let (mut state, session) = holding(tool(SessionId::new()));
    assert_eq!(
        state.choice(session, ID, Action::Allow),
        Some(PromptChoice::Allow)
    );
    assert_eq!(
        state.choice(session, ID, Action::AlwaysAllow),
        Some(PromptChoice::AlwaysAllow)
    );
    assert_eq!(
        state.choice(session, ID, Action::Deny),
        Some(PromptChoice::Deny { message: None }),
        "nothing typed: no message"
    );
    state.edit_deny_message(session, ID, "  \n ".into());
    assert_eq!(
        state.choice(session, ID, Action::Deny),
        Some(PromptChoice::Deny { message: None }),
        "a blank message is none"
    );
    state.edit_deny_message(session, ID, "Use the staging database.".into());
    assert_eq!(
        state.choice(session, ID, Action::Deny),
        Some(PromptChoice::Deny {
            message: Some("Use the staging database.".into())
        })
    );
    assert_eq!(
        state.choice(session, ID, Action::Answer),
        None,
        "not a question"
    );
}

#[test]
fn always_allow_is_not_built_when_the_request_offers_no_rule() {
    let mut request = tool(SessionId::new());
    request.always_allow = None;
    let (state, session) = holding(request);
    assert_eq!(state.choice(session, ID, Action::AlwaysAllow), None);
    assert_eq!(
        state.choice(session, ID, Action::Allow),
        Some(PromptChoice::Allow)
    );
}

#[test]
fn sending_is_set_on_submit_and_cleared_on_an_operation_error() {
    let (mut state, session) = holding(tool(SessionId::new()));
    state.sending(session, ID);
    assert!(state.shown(session, ID).expect("shown").sending);
    assert!(!state.can_submit(session, ID), "one answer at a time");
    assert_eq!(state.choice(session, ID, Action::Allow), None);

    state.decision_failed(session, ID, "This request was already answered.".into());
    let shown = state.shown(session, ID).expect("still shown");
    assert!(!shown.sending);
    assert_eq!(
        shown.error.as_deref(),
        Some("This request was already answered.")
    );
    assert_eq!(
        state.choice(session, ID, Action::Allow),
        Some(PromptChoice::Allow)
    );

    state.sending(session, ID);
    assert_eq!(
        state.shown(session, ID).expect("shown").error,
        None,
        "a new answer clears the old refusal"
    );
}

#[test]
fn a_sessions_state_is_dropped_with_the_session() {
    let (mut state, session) = holding(tool(SessionId::new()));
    state.resolved(
        session,
        PromptId(1),
        Some("toolu_1".into()),
        Resolution::Allowed,
    );
    state.forget(session);
    assert!(state.shown(session, ID).is_none());
    assert!(!state.waiting(session));
    assert_eq!(state.resolution(session, "toolu_1"), None);
}

#[test]
fn two_sessions_are_isolated() {
    let (a, b) = (SessionId::new(), SessionId::new());
    let mut state = State::default();
    state.requested(tool(a));
    state.requested(tool(b));
    state.edit_deny_message(a, ID, "a's reason".into());
    state.resolved(b, ID, Some("toolu_7".into()), Resolution::Denied);

    assert_eq!(
        state.shown(a, ID).expect("a's request").draft.deny_message,
        "a's reason"
    );
    assert!(state.shown(b, ID).is_none());
    assert_eq!(state.resolution(a, "toolu_7"), None);
    assert_eq!(state.resolution(b, "toolu_7"), Some(Resolution::Denied));
    assert!(state.waiting(a));
    assert!(!state.waiting(b));
}

/// A transcript holding one tool card, `toolu_7`.
fn transcript_with_card() -> Transcript {
    let line = r#"{"type":"assistant","message":{"id":"msg_1","type":"message","role":"assistant","model":"m","content":[{"type":"tool_use","id":"toolu_7","name":"Bash","input":{"command":"npm test"}}]},"session_id":"s"}"#;
    let mut transcript = Transcript::new();
    transcript.apply(
        0,
        &[SequencedEvent {
            seq: 0,
            event: parse_line(line),
        }],
        true,
    );
    assert!(
        transcript
            .entry(&EntryKey::Tool("toolu_7".into()))
            .is_some(),
        "the fixture holds the card"
    );
    transcript
}

#[test]
fn a_request_is_placed_under_its_card_when_the_transcript_holds_it_else_at_the_end() {
    let session = SessionId::new();
    let mut state = State::default();
    state.requested(tool(session));
    let mut orphan = request(session, PromptId(9), RequestKind::Tool);
    orphan.tool_use_id = Some("toolu_gone".into());
    state.requested(orphan);
    let mut unnamed = request(session, PromptId(11), RequestKind::Tool);
    unnamed.tool_use_id = None;
    state.requested(unnamed);

    let transcript = transcript_with_card();
    assert_eq!(
        state.placement(session, Some(&transcript)),
        vec![
            (Placement::UnderCard(EntryKey::Tool("toolu_7".into())), ID),
            (Placement::AtEnd, PromptId(9)),
            (Placement::AtEnd, PromptId(11)),
        ]
    );
    assert_eq!(
        state.placement(session, None),
        vec![
            (Placement::AtEnd, ID),
            (Placement::AtEnd, PromptId(9)),
            (Placement::AtEnd, PromptId(11)),
        ],
        "no transcript yet: every request at the end"
    );
}

#[test]
fn the_input_preview_is_cut_when_the_input_is_longer_than_it() {
    let mut request = tool(SessionId::new());
    let (state, session) = holding(request.clone());
    assert!(!state.shown(session, ID).expect("shown").input_cut());

    request.input_bytes = request.input_preview.len() as u64 + 1;
    let (state, session) = holding(request);
    assert!(state.shown(session, ID).expect("shown").input_cut());
}

// ---- T050: plan approvals and questions ----------------------------------------------------------

#[test]
fn a_plan_is_held_whole_and_builds_its_three_choices() {
    let request = plan(SessionId::new());
    let RequestKind::PlanApproval { plan: text } = &request.kind else {
        unreachable!("a plan approval")
    };
    let text = text.clone();
    let (mut state, session) = holding(request);
    match &state.shown(session, ID).expect("shown").request.kind {
        RequestKind::PlanApproval { plan } => {
            assert_eq!(plan.as_str(), text.as_str(), "the whole plan")
        }
        other => panic!("a plan approval, got {other:?}"),
    }

    assert_eq!(
        state.choice(session, ID, Action::ApprovePlan { accept_edits: true }),
        Some(PromptChoice::ApprovePlan { accept_edits: true })
    );
    assert_eq!(
        state.choice(
            session,
            ID,
            Action::ApprovePlan {
                accept_edits: false
            }
        ),
        Some(PromptChoice::ApprovePlan {
            accept_edits: false
        })
    );
    assert_eq!(
        state.choice(session, ID, Action::KeepPlanning),
        Some(PromptChoice::KeepPlanning { note: None })
    );
    state.edit_note(session, ID, "Check the migration first.".into());
    assert_eq!(
        state.choice(session, ID, Action::KeepPlanning),
        Some(PromptChoice::KeepPlanning {
            note: Some("Check the migration first.".into())
        })
    );
    assert_eq!(state.choice(session, ID, Action::Allow), None, "not a tool");
}

#[test]
fn a_question_request_is_submittable_only_when_every_question_is_answered() {
    let (mut state, session) = holding(questions(SessionId::new()));
    assert_eq!(
        state.shown(session, ID).expect("shown").draft.answers.len(),
        2
    );
    assert!(!state.can_submit(session, ID));
    assert_eq!(state.choice(session, ID, Action::Answer), None);

    state.toggle_option(session, ID, 0, 1);
    assert!(!state.can_submit(session, ID), "the second is unanswered");

    state.choose_other(session, ID, 1);
    assert!(!state.can_submit(session, ID), "a blank Other is no answer");
    state.edit_other(session, ID, 1, "   ".into());
    assert!(!state.can_submit(session, ID));
    state.edit_other(session, ID, 1, "Search".into());
    assert!(state.can_submit(session, ID));
    assert_eq!(
        state.choice(session, ID, Action::Answer),
        Some(PromptChoice::Answer {
            answers: vec![
                QuestionAnswer {
                    labels: vec!["Two".into()],
                    other: None
                },
                QuestionAnswer {
                    labels: vec![],
                    other: Some("Search".into())
                },
            ]
        })
    );
}

#[test]
fn typed_other_text_counts_only_while_other_is_chosen() {
    let (mut state, session) = holding(questions(SessionId::new()));
    state.toggle_option(session, ID, 0, 0);
    state.edit_other(session, ID, 1, "Search".into());
    assert!(
        !state.can_submit(session, ID),
        "text in an Other not chosen is no answer"
    );
}

#[test]
fn a_single_choice_question_holds_one_option_and_a_multi_select_several() {
    let (mut state, session) = holding(questions(SessionId::new()));
    state.toggle_option(session, ID, 0, 0);
    state.toggle_option(session, ID, 0, 2);
    state.toggle_option(session, ID, 1, 0);
    state.toggle_option(session, ID, 1, 2);
    let answers = &state.shown(session, ID).expect("shown").draft.answers;
    assert_eq!(answers[0].chosen.iter().copied().collect::<Vec<_>>(), [2]);
    assert_eq!(
        answers[1].chosen.iter().copied().collect::<Vec<_>>(),
        [0, 2]
    );

    // Other is one more choice: on a single-choice question it replaces the option, and an
    // option chosen afterwards replaces Other.
    state.choose_other(session, ID, 0);
    let first = &state.shown(session, ID).expect("shown").draft.answers[0];
    assert!(first.other_chosen && first.chosen.is_empty());
    state.toggle_option(session, ID, 0, 1);
    let first = &state.shown(session, ID).expect("shown").draft.answers[0];
    assert!(!first.other_chosen);
    assert_eq!(first.chosen.iter().copied().collect::<Vec<_>>(), [1]);

    // A second toggle of a multi-select option unchooses it; an option out of range is ignored.
    state.toggle_option(session, ID, 1, 0);
    state.toggle_option(session, ID, 1, 9);
    state.toggle_option(session, ID, 5, 0);
    let answers = &state.shown(session, ID).expect("shown").draft.answers;
    assert_eq!(answers[1].chosen.iter().copied().collect::<Vec<_>>(), [2]);
    assert_eq!(answers.len(), 2);

    assert_eq!(
        state.choice(session, ID, Action::Answer),
        Some(PromptChoice::Answer {
            answers: vec![
                QuestionAnswer {
                    labels: vec!["Two".into()],
                    other: None
                },
                QuestionAnswer {
                    labels: vec!["Three".into()],
                    other: None
                },
            ]
        })
    );
}

#[test]
fn declining_a_question_needs_no_answer() {
    let (state, session) = holding(questions(SessionId::new()));
    assert_eq!(
        state.choice(session, ID, Action::Decline),
        Some(PromptChoice::DeclineQuestion)
    );
    assert_eq!(state.choice(session, ID, Action::Allow), None);
}

// ---- T051: waiting -------------------------------------------------------------------------------

#[test]
fn a_session_waits_while_any_of_its_requests_is_pending() {
    let session = SessionId::new();
    let mut state = State::default();
    assert!(!state.waiting(session));
    state.requested(tool(session));
    state.requested(request(session, PromptId(8), RequestKind::Tool));
    assert!(state.waiting(session));
    state.resolved(session, ID, None, Resolution::Allowed);
    assert!(state.waiting(session), "one still pending");
    state.resolved(session, PromptId(8), None, Resolution::Denied);
    assert!(!state.waiting(session), "the last resolved");
}

// ---- T053: what the shell routes back ------------------------------------------------------------

#[test]
fn the_whole_input_is_kept_when_it_arrives_and_a_refusal_shows_on_the_request() {
    let mut request = tool(SessionId::new());
    request.input_bytes = 100_000;
    let (mut state, session) = holding(request);
    state.input_text(session, ID, r#"{"command":"the whole of it"}"#.into());
    assert_eq!(
        state
            .shown(session, ID)
            .expect("shown")
            .full_input
            .as_deref(),
        Some(r#"{"command":"the whole of it"}"#)
    );

    state.decision_failed(session, ID, "The request is no longer pending.".into());
    assert_eq!(
        state.shown(session, ID).expect("shown").error.as_deref(),
        Some("The request is no longer pending.")
    );
}

#[test]
fn a_late_reply_for_a_resolved_request_changes_nothing() {
    let (mut state, session) = holding(tool(SessionId::new()));
    state.resolved(session, ID, Some("toolu_7".into()), Resolution::Allowed);
    state.decision_failed(session, ID, "late".into());
    state.input_text(session, ID, "{}".into());
    state.sending(session, ID);
    assert!(state.shown(session, ID).is_none());
    assert!(!state.waiting(session));
}

#[test]
fn a_lost_connection_drops_the_pending_requests_and_keeps_the_labels() {
    let (mut state, session) = holding(tool(SessionId::new()));
    state.resolved(
        session,
        PromptId(1),
        Some("toolu_1".into()),
        Resolution::Allowed,
    );
    state.connection_lost(session);
    assert!(
        !state.waiting(session),
        "the reconnect replays what is still pending"
    );
    assert_eq!(
        state.resolution(session, "toolu_1"),
        Some(Resolution::Allowed)
    );
}

#[test]
fn a_mode_change_is_sending_until_its_reply_and_a_refusal_is_the_sessions_notice() {
    let session = SessionId::new();
    let mut state = State::default();
    assert_eq!(
        state.request_mode(session, PermissionMode::AcceptEdits),
        Some(PermissionMode::AcceptEdits)
    );
    state.mode_sending(session, PermissionMode::AcceptEdits);
    assert_eq!(
        state.mode_change(session),
        Some(PermissionMode::AcceptEdits)
    );
    state.mode_applied(session);
    assert_eq!(state.mode_change(session), None);
    assert_eq!(state.notice(session), None);

    state.mode_sending(session, PermissionMode::Plan);
    state.mode_failed(session, "Claude did not answer the mode change.".into());
    assert_eq!(state.mode_change(session), None);
    assert_eq!(
        state.notice(session),
        Some("Claude did not answer the mode change.")
    );
    state.mode_sending(session, PermissionMode::Default);
    assert_eq!(state.notice(session), None, "a new change clears it");
}

// ---- T054: a mode change to Bypass is confirmed first (FR-017) -----------------------------------

#[test]
fn a_mode_change_to_bypass_waits_for_its_confirmation() {
    let session = SessionId::new();
    let mut state = State::default();
    assert_eq!(
        state.request_mode(session, PermissionMode::Bypass),
        None,
        "nothing is sent yet"
    );
    assert_eq!(state.mode_confirmation(), Some(session));
    assert_eq!(
        state.confirm_mode(),
        Some((session, PermissionMode::Bypass)),
        "the confirm sends it"
    );
    assert_eq!(state.mode_confirmation(), None);
    assert_eq!(state.confirm_mode(), None, "and only once");
}

#[test]
fn a_cancelled_bypass_confirmation_sends_nothing() {
    let session = SessionId::new();
    let mut state = State::default();
    assert_eq!(state.request_mode(session, PermissionMode::Bypass), None);
    state.cancel_mode();
    assert_eq!(state.mode_confirmation(), None);
    assert_eq!(state.confirm_mode(), None);
}

#[test]
fn a_forgotten_session_takes_its_bypass_confirmation_with_it() {
    let session = SessionId::new();
    let mut state = State::default();
    assert_eq!(state.request_mode(session, PermissionMode::Bypass), None);
    state.forget(SessionId::new());
    assert_eq!(
        state.mode_confirmation(),
        Some(session),
        "another session's"
    );
    state.forget(session);
    assert_eq!(state.confirm_mode(), None);
}
