//! The attach dialog's reducer, in isolation (feature 582, T009; FR-004, scenario 3).
//!
//! Builds an app `State` (the reducers take one) and names only the attach feature and the core's
//! attach types. The dialog lists stored provider sessions only: no worktree rows (feature 552).

use micold_client::app::State;
use micold_client::features::attach::{self, Listing, Msg};
use micold_core::attach::{
    AttachItem, AttachOutcome, AttachResult, AttachTarget, DiscoveryNote, DiscoveryReport,
    RefuseReason, ResumableSession, ResumableStatus, SkipReason, UnresumableReason,
};
use micold_core::notify::Level;
use micold_core::session::AiCli;
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

fn session(n: u128, status: ResumableStatus) -> ResumableSession {
    ResumableSession {
        id: Uuid::from_u128(n),
        provider: AiCli::ClaudeCode,
        title: Some(format!("session {n}")),
        last_activity: SystemTime::UNIX_EPOCH,
        target: AttachTarget::Default,
        status,
    }
}

fn report() -> DiscoveryReport {
    DiscoveryReport {
        sessions: vec![
            session(1, ResumableStatus::Resumable),
            session(2, ResumableStatus::Resumable),
            session(
                3,
                ResumableStatus::Unresumable(UnresumableReason::WorktreeMissing),
            ),
        ],
        ..Default::default()
    }
}

fn open_state() -> State {
    let mut state = State::default();
    let path = PathBuf::from("/p");
    state.workspace.active = Some(path.clone());
    attach::update(&mut state, Msg::Opened);
    attach::update(
        &mut state,
        Msg::Listed {
            project: path,
            report: report(),
        },
    );
    state
}

fn target(n: u128) -> AttachItem {
    AttachItem::Session {
        id: Uuid::from_u128(n),
        replace: false,
        allow_additional_session: false,
    }
}

#[test]
fn opening_without_a_project_does_nothing() {
    let mut state = State::default();
    attach::update(&mut state, Msg::Opened);
    assert!(state.attach.dialog.is_none());
}

#[test]
fn opening_waits_for_the_report_then_lists_sessions_and_no_worktree_rows() {
    let mut state = State::default();
    state.workspace.active = Some(PathBuf::from("/p"));
    attach::update(&mut state, Msg::Opened);
    assert_eq!(
        state.attach.dialog.as_ref().map(|d| &d.listing),
        Some(&Listing::Loading)
    );
    attach::update(
        &mut state,
        Msg::Listed {
            project: PathBuf::from("/p"),
            report: report(),
        },
    );
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert_eq!(dialog.listing, Listing::Listed);
    assert_eq!(dialog.sessions.len(), 3, "every stored session is a row");
    assert_eq!(dialog.resumable().len(), 2, "two of them offer Resume");
}

#[test]
fn a_report_for_another_project_is_dropped() {
    let mut state = State::default();
    state.workspace.active = Some(PathBuf::from("/p"));
    attach::update(&mut state, Msg::Opened);
    attach::update(
        &mut state,
        Msg::Listed {
            project: PathBuf::from("/other"),
            report: report(),
        },
    );
    assert_eq!(
        state.attach.dialog.as_ref().map(|d| &d.listing),
        Some(&Listing::Loading)
    );
}

#[test]
fn resume_all_sends_every_resumable_session_and_not_the_unresumable_one() {
    let mut state = open_state();
    attach::update(&mut state, Msg::ResumeAll);
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert_eq!(dialog.in_flight, vec![target(1), target(2)]);
    assert!(dialog.applying());
}

#[test]
fn resume_all_with_nothing_resumable_sends_nothing() {
    let mut state = State::default();
    state.workspace.active = Some(PathBuf::from("/p"));
    attach::update(&mut state, Msg::Opened);
    attach::update(
        &mut state,
        Msg::Listed {
            project: PathBuf::from("/p"),
            report: DiscoveryReport::default(),
        },
    );
    attach::update(&mut state, Msg::ResumeAll);
    assert!(!state.attach.dialog.as_ref().unwrap().applying());
}

#[test]
fn a_second_press_while_applying_changes_nothing() {
    let mut state = open_state();
    attach::update(&mut state, Msg::ResumeAll);
    attach::update(
        &mut state,
        Msg::Resume {
            id: Uuid::from_u128(1),
        },
    );
    assert_eq!(
        state.attach.dialog.as_ref().unwrap().in_flight,
        vec![target(1), target(2)]
    );
}

fn result(n: u128, outcome: AttachOutcome) -> AttachResult {
    AttachResult {
        item: target(n),
        outcome,
        existing: None,
    }
}

#[test]
fn an_applied_batch_closes_the_dialog_and_says_what_happened() {
    let mut state = open_state();
    attach::update(&mut state, Msg::ResumeAll);
    let outcomes = attach::update(
        &mut state,
        Msg::Applied(vec![
            result(1, AttachOutcome::Attached),
            result(2, AttachOutcome::Attached),
        ]),
    );
    assert!(state.attach.dialog.is_none());
    let [micold_client::features::Outcome::NotificationRaised(n)] = outcomes.as_slice() else {
        panic!("expected one notification, got {outcomes:?}");
    };
    assert_eq!(n.message, "Attached 2 sessions.");
    assert_eq!(n.level, Level::Info);
}

#[test]
fn already_attached_is_said_not_silent() {
    let mut state = open_state();
    attach::update(&mut state, Msg::ResumeAll);
    let outcomes = attach::update(
        &mut state,
        Msg::Applied(vec![result(1, AttachOutcome::AlreadyAttached)]),
    );
    let [micold_client::features::Outcome::NotificationRaised(n)] = outcomes.as_slice() else {
        panic!("expected one notification");
    };
    assert_eq!(n.message, "1 already attached.");
    assert_eq!(n.level, Level::Info);
}

#[test]
fn a_failed_apply_keeps_the_dialog_and_shows_why() {
    let mut state = open_state();
    attach::update(&mut state, Msg::ResumeAll);
    attach::update(&mut state, Msg::ApplyFailed("disk full".into()));
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert!(!dialog.applying());
    assert_eq!(dialog.error.as_deref(), Some("disk full"));
    assert_eq!(dialog.listing, Listing::Listed, "the list is kept");
    assert!(!dialog.resumable().is_empty(), "so Resume all stays usable");
}

#[test]
fn cancel_is_ignored_while_an_apply_is_outstanding() {
    let mut state = open_state();
    attach::update(&mut state, Msg::ResumeAll);
    attach::update(&mut state, Msg::Cancelled);
    assert!(state.attach.dialog.is_some());
}

#[test]
fn a_stale_answer_with_no_apply_outstanding_is_dropped() {
    let mut state = open_state();
    let before = state.attach.dialog.clone();
    let out = attach::update(&mut state, Msg::Applied(vec![]));
    assert!(out.is_empty());
    attach::update(&mut state, Msg::ApplyFailed("late".into()));
    attach::update(&mut state, Msg::ListFailed("late".into()));
    assert_eq!(state.attach.dialog, before);
}

#[test]
fn cancel_closes_the_dialog() {
    let mut state = open_state();
    attach::update(&mut state, Msg::Cancelled);
    assert!(state.attach.dialog.is_none());
}

// --- Sessions (T020; Story 2) ---------------------------------------------------------------

fn state_with(sessions: Vec<ResumableSession>, notes: Vec<DiscoveryNote>) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/p");
    state.workspace.active = Some(path.clone());
    attach::update(&mut state, Msg::Opened);
    attach::update(
        &mut state,
        Msg::Listed {
            project: path,
            report: DiscoveryReport { sessions, notes },
        },
    );
    state
}

fn note(provider: AiCli, reason: SkipReason) -> DiscoveryNote {
    DiscoveryNote {
        provider: Some(provider),
        path: None,
        reason,
    }
}

#[test]
fn the_report_s_sessions_are_kept_for_the_dialog_to_draw() {
    let state = state_with(vec![session(1, ResumableStatus::Resumable)], vec![]);
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert_eq!(dialog.sessions.len(), 1);
    assert_eq!(dialog.sessions[0].id, Uuid::from_u128(1));
}

#[test]
fn resume_sends_one_session_target() {
    let mut state = state_with(
        vec![
            session(1, ResumableStatus::Resumable),
            session(2, ResumableStatus::Resumable),
        ],
        vec![],
    );
    attach::update(
        &mut state,
        Msg::Resume {
            id: Uuid::from_u128(2),
        },
    );
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert_eq!(
        dialog.in_flight,
        vec![AttachItem::Session {
            id: Uuid::from_u128(2),
            replace: false,
            allow_additional_session: false,
        }]
    );
    assert!(dialog.applying());
}

#[test]
fn an_unresumable_session_offers_no_resume_and_says_why() {
    let reasons = [
        (UnresumableReason::WorktreeMissing, "folder is missing"),
        (UnresumableReason::WorktreeInvalid, "not a usable worktree"),
        (UnresumableReason::NoLocation, "no worktree"),
    ];
    for (n, (reason, text)) in reasons.into_iter().enumerate() {
        let s = session(n as u128 + 1, ResumableStatus::Unresumable(reason));
        assert!(!attach::can_resume(&s));
        assert!(
            attach::status_text(&s).contains(text),
            "{reason:?}: {}",
            attach::status_text(&s)
        );
        let mut state = state_with(vec![s.clone()], vec![]);
        attach::update(&mut state, Msg::Resume { id: s.id });
        assert!(!state.attach.dialog.as_ref().unwrap().applying());
    }
    assert!(attach::can_resume(&session(9, ResumableStatus::Resumable)));
    assert_eq!(
        attach::status_text(&session(9, ResumableStatus::Resumable)),
        "Ready to resume"
    );
}

#[test]
fn a_second_resume_while_one_is_applying_changes_nothing() {
    let mut state = state_with(
        vec![
            session(1, ResumableStatus::Resumable),
            session(2, ResumableStatus::Resumable),
        ],
        vec![],
    );
    attach::update(
        &mut state,
        Msg::Resume {
            id: Uuid::from_u128(1),
        },
    );
    attach::update(
        &mut state,
        Msg::Resume {
            id: Uuid::from_u128(2),
        },
    );
    assert_eq!(
        state.attach.dialog.as_ref().unwrap().in_flight,
        vec![AttachItem::Session {
            id: Uuid::from_u128(1),
            replace: false,
            allow_additional_session: false,
        }]
    );
}

#[test]
fn a_resumed_session_closes_the_dialog_and_a_refusal_names_its_reason() {
    let mut state = state_with(vec![session(1, ResumableStatus::Resumable)], vec![]);
    attach::update(
        &mut state,
        Msg::Resume {
            id: Uuid::from_u128(1),
        },
    );
    let item = AttachItem::Session {
        id: Uuid::from_u128(1),
        replace: false,
        allow_additional_session: false,
    };
    let outcomes = attach::update(
        &mut state,
        Msg::Applied(vec![AttachResult {
            item: item.clone(),
            outcome: AttachOutcome::Refused(RefuseReason::AlreadyRunning),
            existing: None,
        }]),
    );
    let [micold_client::features::Outcome::NotificationRaised(n)] = outcomes.as_slice() else {
        panic!("expected one notification, got {outcomes:?}");
    };
    assert!(n.message.contains("already running"), "{}", n.message);
    assert_eq!(n.level, Level::Error);
    assert_eq!(
        attach::summary(&[AttachResult {
            item,
            outcome: AttachOutcome::Attached,
            existing: None,
        }]),
        "Attached 1 session."
    );
}

#[test]
fn notes_are_a_footer_when_nothing_was_found_and_for_real_problems() {
    // An absent store is the ordinary case: silent while sessions are listed...
    let listed = state_with(
        vec![session(1, ResumableStatus::Resumable)],
        vec![note(AiCli::Pi, SkipReason::StoreMissing)],
    );
    assert!(listed
        .attach
        .dialog
        .as_ref()
        .unwrap()
        .footer_notes()
        .is_empty());
    // ...but explains an empty list,
    let empty = state_with(vec![], vec![note(AiCli::Pi, SkipReason::StoreMissing)]);
    assert_eq!(
        empty.attach.dialog.as_ref().unwrap().footer_notes().len(),
        1
    );
    // ...and an unreadable store or a sandbox is always said.
    let problem = state_with(
        vec![session(1, ResumableStatus::Resumable)],
        vec![
            note(AiCli::Copilot, SkipReason::StoreUnreadable),
            DiscoveryNote {
                provider: None,
                path: None,
                reason: SkipReason::SandboxStoreNotReadable,
            },
        ],
    );
    let notes = problem.attach.dialog.as_ref().unwrap().footer_notes();
    assert_eq!(notes.len(), 2);
    assert!(notes[0].contains("Copilot"), "{}", notes[0]);
    assert!(notes[1].contains("sandbox"), "{}", notes[1]);
}
