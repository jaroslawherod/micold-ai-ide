//! The start-up offer's rules, in isolation (feature 582, T032; FR-012, Story 4).
//!
//! `offer_visible = no_records && !report.is_empty() && !dismissed`. Nothing is attached without a
//! user action; dismissal is per project and per run.

use micold_client::app::State;
use micold_client::features::attach::{self, offer_visible, Msg};
use micold_core::attach::{
    AttachItem, AttachOutcome, AttachResult, AttachableWorktree, Availability, DiscoveryReport,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn project() -> PathBuf {
    PathBuf::from("/p")
}

fn worktree(name: &str, availability: Availability) -> AttachableWorktree {
    AttachableWorktree {
        dir_name: name.into(),
        path: project().join(".claude/worktrees").join(name),
        branch: None,
        provider: None,
        session_count: 0,
        availability,
    }
}

fn found() -> DiscoveryReport {
    DiscoveryReport {
        worktrees: vec![
            worktree("a", Availability::Attachable),
            worktree("b", Availability::Attachable),
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

fn wt(name: &str) -> AttachItem {
    AttachItem::Worktree {
        dir_name: name.into(),
    }
}

#[test]
fn a_project_with_no_records_and_something_found_is_offered() {
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
fn a_project_with_provenance_records_never_gets_the_offer() {
    let mut state = state_with(found());
    state
        .workspace
        .worktree_provenance
        .insert(project(), BTreeSet::from(["made".to_string()]));
    assert!(!offer_visible(&state));
}

#[test]
fn an_unreadable_project_is_not_known_to_have_no_records() {
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
    assert_eq!(state.attach.dialog.as_ref().unwrap().attachable().len(), 2);
}

#[test]
fn attach_all_sends_every_attachable_worktree_once() {
    let mut report = found();
    report.worktrees.push(worktree(
        "gone",
        Availability::Unavailable(micold_core::attach::Unavailable::Missing),
    ));
    let mut state = state_with(report);
    attach::update(&mut state, Msg::OfferAttachAll);
    assert_eq!(state.attach.offer.in_flight, vec![wt("a"), wt("b")]);

    // A second press while the first is outstanding sends nothing new.
    attach::update(&mut state, Msg::OfferAttachAll);
    assert_eq!(state.attach.offer.in_flight, vec![wt("a"), wt("b")]);
}

#[test]
fn the_answer_ends_the_offer_and_says_what_happened() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferAttachAll);
    let outcomes = attach::update(
        &mut state,
        Msg::OfferApplied(vec![
            AttachResult {
                item: wt("a"),
                outcome: AttachOutcome::Attached,
            },
            AttachResult {
                item: wt("b"),
                outcome: AttachOutcome::Attached,
            },
        ]),
    );
    assert!(!offer_visible(&state));
    assert!(state.attach.offer.in_flight.is_empty());
    assert_eq!(outcomes.len(), 1);
}

#[test]
fn a_failed_attach_keeps_the_offer_so_it_can_be_retried() {
    let mut state = state_with(found());
    attach::update(&mut state, Msg::OfferAttachAll);
    let outcomes = attach::update(&mut state, Msg::OfferApplyFailed("boom".into()));
    assert!(offer_visible(&state));
    assert!(state.attach.offer.in_flight.is_empty());
    assert_eq!(outcomes.len(), 1);
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
            item: wt("a"),
            outcome: AttachOutcome::Refused(
                micold_core::attach::RefuseReason::NotAWorktreeOfProject,
            ),
        }]),
    );
    assert!(offer_visible(&state));
    assert!(state.attach.offer.in_flight.is_empty());
}
