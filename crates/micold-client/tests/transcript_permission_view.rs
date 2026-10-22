//! #942 M7 (T060–T063, FR-006–FR-008): the transcript draws a structured session's permission
//! requests — a tool request under the card of its call (or after the last entry when the card is
//! not held), the plan view and the question view — and what their buttons send.
//!
//! The transcript is the recorded `prompt-tool-request` turn; the requests are the window's
//! `features::permission_prompts` state, drawn by `ui::transcript::view_with_prompts`.

mod support;

use iced::Point;
use micold_client::features::permission_prompts::{Action, Msg as PromptsMsg, State as Prompts};
use micold_client::features::structured_transcript::{State, TranscriptMsg};
use micold_client::ui::transcript::permission::{
    always_allow_note, resolution_label, ALLOW, ALWAYS_ALLOW, APPROVE, APPROVE_ACCEPT_EDITS,
    DECLINE, DENY, DENY_PLACEHOLDER, KEEP_PLANNING, NOTE_PLACEHOLDER, OTHER, SHOW_ALL, SUBMIT,
};
use micold_client::ui::transcript::{view_with_prompts, BlockCache, BlockRenderers, PromptsView};
use micold_core::protocol::messages::{PermissionRequest, PromptId, Resolution, WithdrawReason};
use micold_core::session::{PermissionMode, SessionId};
use micold_core::structured::permission::request::{Question, QuestionOption, RequestKind};
use support::layout::{self as lay, Overflow, WINDOW};
use support::transcript::{ctx, state_of};

/// The Bash call of `prompt-tool-request.jsonl`.
const TOOL_USE: &str = "toolu_01Ny2tmpWZg9pk6d6NqZR4q9";
const ID: PromptId = PromptId(3);
const RULE: &str = "Bash commands that start with \"echo hi\"";

/// What the mounted view publishes: the transcript's messages or the prompts'.
#[derive(Debug, Clone, PartialEq)]
enum Out {
    Transcript(TranscriptMsg),
    Prompt(PromptsMsg),
}

fn tool_request(session: SessionId, tool_use_id: &str) -> PermissionRequest {
    PermissionRequest {
        id: ID,
        session,
        tool_use_id: Some(tool_use_id.into()),
        tool_name: "Bash".into(),
        input_preview: r#"{"command":"echo hi > out.txt","description":"Write hi"}"#.into(),
        input_bytes: 55,
        kind: RequestKind::Tool,
        always_allow: Some(RULE.into()),
    }
}

fn plan_request(session: SessionId) -> PermissionRequest {
    PermissionRequest {
        tool_use_id: None,
        tool_name: "ExitPlanMode".into(),
        kind: RequestKind::PlanApproval {
            plan: "Step 1: read the parser.\nStep 2: change the parser.".into(),
        },
        always_allow: None,
        ..tool_request(session, "")
    }
}

fn question(header: &str, multi_select: bool) -> Question {
    Question {
        header: header.into(),
        text: format!("Which {header}?"),
        options: [("Postgres", "The main store"), ("SQLite", "")]
            .into_iter()
            .map(|(label, description)| QuestionOption {
                label: format!("{label} {header}"),
                description: description.into(),
            })
            .collect(),
        multi_select,
    }
}

fn question_request(session: SessionId) -> PermissionRequest {
    PermissionRequest {
        tool_use_id: None,
        tool_name: "AskUserQuestion".into(),
        kind: RequestKind::Question {
            questions: vec![question("store", false), question("cache", true)],
        },
        always_allow: None,
        ..tool_request(session, "")
    }
}

fn transcript(session: SessionId) -> State {
    state_of(session, "prompt-tool-request")
}

fn holding(request: PermissionRequest) -> Prompts {
    let mut prompts = Prompts::default();
    prompts.requested(request);
    prompts
}

fn element<'a>(
    state: &'a State,
    session: SessionId,
    prompts: &'a Prompts,
    cache: &'a BlockCache,
) -> iced::Element<'a, Out> {
    view_with_prompts(
        state,
        session,
        &BlockRenderers::PLAIN,
        ctx(cache),
        Out::Transcript,
        PromptsView {
            state: prompts,
            on_prompt: Out::Prompt,
        },
    )
}

fn painted(state: &State, session: SessionId, prompts: &Prompts) -> Vec<Overflow> {
    let cache = BlockCache::default();
    let mut renderer = lay::renderer();
    lay::painted_text_settled_at(
        element(state, session, prompts, &cache),
        &mut renderer,
        WINDOW,
    )
    .0
}

fn find<'p>(painted: &'p [Overflow], content: &str) -> &'p Overflow {
    painted
        .iter()
        .find(|p| p.content == content)
        .unwrap_or_else(|| {
            panic!(
                "`{content}` is drawn: {:?}",
                painted.iter().map(|p| &p.content).collect::<Vec<_>>()
            )
        })
}

fn centre(p: &Overflow) -> Point {
    Point::new(
        p.origin.x + p.natural_width.min(p.allowed_width) / 2.0,
        p.origin.y + p.natural_height / 2.0,
    )
}

/// What a click on the text `label` sends.
fn click(state: &State, session: SessionId, prompts: &Prompts, label: &str) -> Vec<Out> {
    let at = centre(find(&painted(state, session, prompts), label));
    let cache = BlockCache::default();
    lay::clicks_at(
        element(state, session, prompts, &cache),
        &lay::renderer(),
        [at],
    )
    .into_iter()
    .filter(|m| matches!(m, Out::Prompt(_)))
    .collect()
}

fn submitted(session: SessionId, action: Action) -> Out {
    Out::Prompt(PromptsMsg::Submitted {
        session,
        id: ID,
        action,
    })
}

#[test]
fn a_tool_request_is_drawn_under_its_card_with_its_input_and_three_answers() {
    let session = SessionId::new();
    let state = transcript(session);
    let prompts = holding(tool_request(session, TOOL_USE));
    let shown = painted(&state, session, &prompts);
    let card = find(&shown, "Bash");
    let allow = find(&shown, ALLOW);
    for label in [DENY, ALWAYS_ALLOW, DENY_PLACEHOLDER, "echo hi > out.txt"] {
        find(&shown, label);
    }
    find(&shown, &always_allow_note(RULE));
    assert!(
        allow.origin.y > card.origin.y,
        "the request is under its card: {allow:?} {card:?}"
    );
    let bare = painted(&state, session, &Prompts::default());
    let last = bare.iter().map(|p| p.origin.y).fold(f32::MIN, f32::max);
    let shifted = shown.iter().map(|p| p.origin.y).fold(f32::MIN, f32::max);
    assert!(
        shifted > last && allow.origin.y < shifted,
        "entries after the card are pushed down below the request, which is not at the end"
    );
}

#[test]
fn a_request_whose_card_is_not_held_is_drawn_after_the_last_entry() {
    let session = SessionId::new();
    let state = transcript(session);
    let prompts = holding(tool_request(session, "toolu_not_in_the_transcript"));
    let shown = painted(&state, session, &prompts);
    let allow = find(&shown, ALLOW);
    let bare = painted(&state, session, &Prompts::default());
    for entry in &bare {
        assert!(
            allow.origin.y > entry.origin.y,
            "`{}` comes before the request: {allow:?}",
            entry.content
        );
    }
}

#[test]
fn the_tool_requests_buttons_send_their_answers() {
    let session = SessionId::new();
    let state = transcript(session);
    let prompts = holding(tool_request(session, TOOL_USE));
    for (label, action) in [
        (ALLOW, Action::Allow),
        (DENY, Action::Deny),
        (ALWAYS_ALLOW, Action::AlwaysAllow),
    ] {
        assert_eq!(
            click(&state, session, &prompts, label),
            vec![submitted(session, action)],
            "{label}"
        );
    }
}

#[test]
fn a_cut_preview_offers_show_all_which_asks_for_the_whole_input() {
    let session = SessionId::new();
    let state = transcript(session);
    let mut request = tool_request(session, TOOL_USE);
    let whole = request.input_preview.len() as u64;
    let prompts = holding(request.clone());
    assert!(
        painted(&state, session, &prompts)
            .iter()
            .all(|p| p.content != SHOW_ALL),
        "a whole input has no Show all"
    );
    request.input_bytes = whole * 100;
    let prompts = holding(request);
    assert_eq!(
        click(&state, session, &prompts, SHOW_ALL),
        vec![Out::Prompt(PromptsMsg::ShowAllRequested {
            session,
            id: ID
        })]
    );
}

#[test]
fn an_answer_on_its_way_disables_the_buttons() {
    let session = SessionId::new();
    let state = transcript(session);
    let mut prompts = holding(tool_request(session, TOOL_USE));
    prompts.sending(session, ID);
    assert_eq!(click(&state, session, &prompts, ALLOW), vec![]);
}

#[test]
fn a_refused_answer_shows_why() {
    let session = SessionId::new();
    let state = transcript(session);
    let mut prompts = holding(tool_request(session, TOOL_USE));
    prompts.sending(session, ID);
    prompts.decision_failed(session, ID, "The request is no longer pending".into());
    find(
        &painted(&state, session, &prompts),
        "The request is no longer pending",
    );
}

#[test]
fn a_resolved_request_leaves_its_label_under_its_card() {
    let session = SessionId::new();
    let state = transcript(session);
    for resolution in [
        Resolution::Allowed,
        Resolution::Denied,
        Resolution::Withdrawn {
            reason: WithdrawReason::TurnEnded,
        },
    ] {
        let mut prompts = holding(tool_request(session, TOOL_USE));
        prompts.resolved(session, ID, Some(TOOL_USE.into()), resolution);
        let shown = painted(&state, session, &prompts);
        let label = find(&shown, resolution_label(resolution));
        assert!(label.origin.y > find(&shown, "Bash").origin.y);
        assert!(shown.iter().all(|p| p.content != ALLOW), "no longer asks");
    }
}

#[test]
fn every_resolution_has_its_label() {
    assert_eq!(resolution_label(Resolution::Allowed), "Allowed");
    assert_eq!(
        resolution_label(Resolution::AllowedAlways),
        "Always allowed"
    );
    assert_eq!(resolution_label(Resolution::Denied), "Denied");
    assert_eq!(
        resolution_label(Resolution::PlanApproved {
            mode: PermissionMode::AcceptEdits
        }),
        "Plan approved"
    );
    assert_eq!(resolution_label(Resolution::KeptPlanning), "Kept planning");
    assert_eq!(resolution_label(Resolution::Answered), "Answered");
    assert_eq!(resolution_label(Resolution::Declined), "Declined");
    assert_eq!(
        resolution_label(Resolution::Withdrawn {
            reason: WithdrawReason::SessionStopped
        }),
        "Withdrawn"
    );
}

#[test]
fn the_plan_view_shows_the_whole_plan_and_its_three_answers() {
    let session = SessionId::new();
    // No entries: the request, after the last one, is in view.
    let state = State::default();
    let prompts = holding(plan_request(session));
    let shown = painted(&state, session, &prompts);
    // The painted-text probe joins a paragraph's lines with a space.
    find(
        &shown,
        "Step 1: read the parser. Step 2: change the parser.",
    );
    find(&shown, NOTE_PLACEHOLDER);
    for (label, action) in [
        (
            APPROVE_ACCEPT_EDITS,
            Action::ApprovePlan { accept_edits: true },
        ),
        (
            APPROVE,
            Action::ApprovePlan {
                accept_edits: false,
            },
        ),
        (KEEP_PLANNING, Action::KeepPlanning),
    ] {
        assert_eq!(
            click(&state, session, &prompts, label),
            vec![submitted(session, action)],
            "{label}"
        );
    }
}

#[test]
fn the_question_view_shows_every_question_option_and_other() {
    let session = SessionId::new();
    // No entries: the request, after the last one, is in view.
    let state = State::default();
    let prompts = holding(question_request(session));
    let shown = painted(&state, session, &prompts);
    for text in [
        "store",
        "Which store?",
        "Postgres store",
        "SQLite store",
        "cache",
        "Which cache?",
        "Postgres cache",
        "The main store",
    ] {
        find(&shown, text);
    }
    assert_eq!(
        shown.iter().filter(|p| p.content == OTHER).count(),
        2,
        "each question has its Other row"
    );
}

#[test]
fn a_question_is_submitted_only_once_every_question_is_answered() {
    let session = SessionId::new();
    // No entries: the request, after the last one, is in view.
    let state = State::default();
    let mut prompts = holding(question_request(session));
    assert_eq!(
        click(&state, session, &prompts, SUBMIT),
        vec![],
        "unanswered"
    );
    prompts.toggle_option(session, ID, 0, 1);
    assert_eq!(
        click(&state, session, &prompts, SUBMIT),
        vec![],
        "half answered"
    );
    prompts.toggle_option(session, ID, 1, 0);
    assert_eq!(
        click(&state, session, &prompts, SUBMIT),
        vec![submitted(session, Action::Answer)]
    );
    assert_eq!(
        click(&state, session, &prompts, DECLINE),
        vec![submitted(session, Action::Decline)]
    );
}

#[test]
fn pressing_an_option_or_other_sends_it() {
    let session = SessionId::new();
    // No entries: the request, after the last one, is in view.
    let state = State::default();
    let prompts = holding(question_request(session));
    assert_eq!(
        click(&state, session, &prompts, "SQLite cache"),
        vec![Out::Prompt(PromptsMsg::OptionToggled {
            session,
            id: ID,
            question: 1,
            option: 1,
        })]
    );
    let at = {
        let shown = painted(&state, session, &prompts);
        let first_other = shown
            .iter()
            .find(|p| p.content == OTHER)
            .expect("an Other row");
        centre(first_other)
    };
    let cache = BlockCache::default();
    let sent: Vec<Out> = lay::clicks_at(
        element(&state, session, &prompts, &cache),
        &lay::renderer(),
        [at],
    )
    .into_iter()
    .filter(|m| matches!(m, Out::Prompt(_)))
    .collect();
    assert_eq!(
        sent,
        vec![Out::Prompt(PromptsMsg::OtherChosen {
            session,
            id: ID,
            question: 0,
        })]
    );
}
