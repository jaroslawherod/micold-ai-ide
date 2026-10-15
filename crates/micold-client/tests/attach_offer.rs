//! The start-up offer's rules, in isolation (feature 582, T032; FR-012, Story 4; feature 552 T023).
//!
//! `offer_visible = !report.is_empty() && !dismissed && readable`: the offer fires on stored
//! provider sessions the catalog lacks. Nothing is attached without a user action; a dismissal is
//! per project, held in memory only, and does not survive a restart.

use micold_client::app::State;
use micold_client::features::attach::{self, offer_visible, Msg};
use micold_client::features::Outcome;
use micold_core::attach::{
    AttachItem, AttachOutcome, AttachResult, AttachTarget, DiscoveryReport, ResumableSession,
    ResumableStatus, UnresumableReason,
};
use micold_core::notify::Level;
use micold_core::session::AiCli;
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

fn project() -> PathBuf {
    PathBuf::from("/p")
}

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

fn found() -> DiscoveryReport {
    DiscoveryReport {
        sessions: vec![
            session(1, ResumableStatus::Resumable),
            session(2, ResumableStatus::Resumable),
        ],
        ..Default::default()
    }
}

fn state_with(report: DiscoveryReport) -> State {
    let mut state = State::default();
    state.workspace.active = Some(project());
    attach::update(
        &mut state,
        Msg::OfferListed {
            project: project(),
            report,
        },
    );
    state
}

fn item(n: u128) -> AttachItem {
    AttachItem::Session {
        id: Uuid::from_u128(n),
        replace: false,
        allow_additional_session: false,
    }
}

#[test]
fn stored_provider_sessions_the_catalog_lacks_fire_the_offer() {
    assert!(offer_visible(&state_with(found())));
}

#[test]
fn nothing_found_means_no_offer() {
    assert!(!offer_visible(&state_with(DiscoveryReport::default())));
}

#[test]
fn no_report_yet_means_no_offer() {
    let mut state = State::default();
    state.workspace.active = Some(project());
    assert!(!offer_visible(&state));
}

#[test]
fn an_unreadable_project_is_not_offered() {
    let mut state = state_with(found());
    state.workspace.unreadable_projects.insert(project());
    assert!(!offer_visible(&state));
}

#[test]
fn a_report_for_another_project_is_dropped() {
    let mut state = State::default();
    state.workspace.active = Some(project());
    attach::update(
        &mut state,
        Msg::OfferListed {
            project: PathBuf::from("/other"),
            report: found(),
        },
    );
    assert!(!offer_visible(&state));
}

#[test]
fn listing_attaches_nothing() {
    let state = state_with(found());
    assert!(state.attach.offer.in_flight.is_empty());
    assert!(state.attach.dialog.is_none());
}

#[test]
fn dismissing_hides_the_offer_for_this_project_only() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferDismissed);
    assert!(!offer_visible(&state));
    assert!(state.attach.offer.in_flight.is_empty());

    // Another project still gets its offer.
    state.workspace.active = Some(PathBuf::from("/q"));
    attach::update(
        &mut state,
        Msg::OfferListed {
            project: PathBuf::from("/q"),
            report: found(),
        },
    );
    assert!(offer_visible(&state));
}

#[test]
fn a_dismissal_lives_in_memory_and_is_not_persisted() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferDismissed);
    assert!(state.attach.offer.dismissed.contains(&project()));
    // The workspace is what is persisted: a dismissal leaves it as it was, so a restart (a fresh
    // `Offer`) offers again.
    let persisted_before = state_with(found()).workspace;
    assert_eq!(state.workspace, persisted_before);
    state.attach.offer = Default::default();
    attach::update(
        &mut state,
        Msg::OfferListed {
            project: project(),
            report: found(),
        },
    );
    assert!(offer_visible(&state));
}

#[test]
fn a_dismissed_offer_stays_dismissed_when_the_report_is_refreshed() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferDismissed);
    attach::update(
        &mut state,
        Msg::OfferListed {
            project: project(),
            report: found(),
        },
    );
    assert!(!offer_visible(&state));
}

#[test]
fn dismissing_leaves_the_attach_dialog_listing_everything() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferDismissed);
    attach::update(&mut state, Msg::Opened);
    attach::update(
        &mut state,
        Msg::Listed {
            project: project(),
            report: found(),
        },
    );
    assert_eq!(state.attach.dialog.as_ref().unwrap().resumable().len(), 2);
}

#[test]
fn attach_all_sends_every_resumable_session_once() {
    let mut report = found();
    report.sessions.push(session(
        3,
        ResumableStatus::Unresumable(UnresumableReason::WorktreeMissing),
    ));
    let mut state = state_with(report);
    attach::update(&mut state, Msg::OfferAttachAll);
    assert_eq!(state.attach.offer.in_flight, vec![item(1), item(2)]);

    // A second press while the first is outstanding sends nothing new.
    attach::update(&mut state, Msg::OfferAttachAll);
    assert_eq!(state.attach.offer.in_flight, vec![item(1), item(2)]);
}

#[test]
fn the_answer_ends_the_offer_and_says_what_happened() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferAttachAll);
    let outcomes = attach::update(
        &mut state,
        Msg::OfferApplied(vec![
            AttachResult {
                item: item(1),
                outcome: AttachOutcome::Attached,
                existing: None,
            },
            AttachResult {
                item: item(2),
                outcome: AttachOutcome::Attached,
                existing: None,
            },
        ]),
    );
    assert!(!offer_visible(&state));
    assert!(state.attach.offer.in_flight.is_empty());
    let [Outcome::NotificationRaised(n)] = outcomes.as_slice() else {
        panic!("expected one notification, got {outcomes:?}");
    };
    assert_eq!(n.message, "Attached 2 sessions.");
    assert_eq!(n.level, Level::Info);
}

#[test]
fn a_failed_attach_keeps_the_offer_so_it_can_be_retried() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferAttachAll);
    let outcomes = attach::update(&mut state, Msg::OfferApplyFailed("boom".into()));
    assert!(offer_visible(&state));
    assert!(state.attach.offer.in_flight.is_empty());
    let [Outcome::NotificationRaised(n)] = outcomes.as_slice() else {
        panic!("expected one notification, got {outcomes:?}");
    };
    assert_eq!(n.message, "Couldn't resume the found sessions: boom");
    assert_eq!(n.level, Level::Error);
}

#[test]
fn an_answer_with_nothing_in_flight_is_ignored() {
    let mut state = state_with(found());
    let outcomes = attach::update(&mut state, Msg::OfferApplied(Vec::new()));
    assert!(offer_visible(&state));
    assert!(outcomes.is_empty());
}

#[test]
fn a_batch_that_all_failed_keeps_the_offer() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferAttachAll);
    attach::update(
        &mut state,
        Msg::OfferApplied(vec![AttachResult {
            item: item(1),
            outcome: AttachOutcome::Refused(micold_core::attach::RefuseReason::AlreadyRunning),
            existing: None,
        }]),
    );
    assert!(offer_visible(&state));
    assert!(state.attach.offer.in_flight.is_empty());
}
