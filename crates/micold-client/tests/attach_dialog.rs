//! The attach-worktrees dialog's reducer, in isolation (feature 582, T009; FR-004, scenario 3).
//!
//! Builds an app `State` (the reducers take one) and names only the attach feature and the core's
//! attach types.

use micold_client::app::State;
use micold_client::features::attach::{self, Listing, Msg};
use micold_core::attach::{
    AttachItem, AttachOutcome, AttachResult, AttachableWorktree, Availability, DiscoveryReport,
    RefuseReason, Unavailable,
};
use micold_core::notify::Level;
use std::path::PathBuf;

fn worktree(name: &str, availability: Availability) -> AttachableWorktree {
    AttachableWorktree {
        dir_name: name.into(),
        path: PathBuf::from("/p/.claude/worktrees").join(name),
        branch: Some(format!("b-{name}")),
        provider: None,
        session_count: 0,
        availability,
    }
}

fn report() -> DiscoveryReport {
    DiscoveryReport {
        worktrees: vec![
            worktree("a", Availability::Attachable),
            worktree("b", Availability::Attachable),
            worktree("gone", Availability::Unavailable(Unavailable::Missing)),
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

fn toggle(state: &mut State, name: &str, checked: bool) {
    attach::update(
        state,
        Msg::Toggled {
            dir_name: name.into(),
            checked,
        },
    );
}

fn wt(name: &str) -> AttachItem {
    AttachItem::Worktree {
        dir_name: name.into(),
    }
}

#[test]
fn opening_without_a_project_does_nothing() {
    let mut state = State::default();
    attach::update(&mut state, Msg::Opened);
    assert!(state.attach.dialog.is_none());
}

#[test]
fn opening_waits_for_the_report_then_lists_it() {
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
    assert!(matches!(&dialog.listing, Listing::Listed(rows) if rows.len() == 3));
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
fn checkbox_selection_toggles_and_ignores_unavailable_rows() {
    let mut state = open_state();
    toggle(&mut state, "a", true);
    toggle(&mut state, "gone", true);
    toggle(&mut state, "nope", true);
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert_eq!(dialog.selected.iter().collect::<Vec<_>>(), vec!["a"]);

    toggle(&mut state, "a", false);
    assert!(state.attach.dialog.as_ref().unwrap().selected.is_empty());
}

#[test]
fn attach_selected_sends_only_the_ticked_rows() {
    let mut state = open_state();
    toggle(&mut state, "b", true);
    attach::update(&mut state, Msg::AttachSelected);
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert_eq!(dialog.in_flight, vec![wt("b")]);
    assert!(dialog.applying());
}

#[test]
fn attach_selected_with_nothing_ticked_sends_nothing() {
    let mut state = open_state();
    attach::update(&mut state, Msg::AttachSelected);
    assert!(!state.attach.dialog.as_ref().unwrap().applying());
}

#[test]
fn attach_all_sends_every_attachable_row_and_not_the_missing_one() {
    let mut state = open_state();
    attach::update(&mut state, Msg::AttachAll);
    assert_eq!(
        state.attach.dialog.as_ref().unwrap().in_flight,
        vec![wt("a"), wt("b")]
    );
}

#[test]
fn a_second_press_while_applying_changes_nothing() {
    let mut state = open_state();
    attach::update(&mut state, Msg::AttachAll);
    toggle(&mut state, "a", true);
    attach::update(&mut state, Msg::AttachSelected);
    assert_eq!(
        state.attach.dialog.as_ref().unwrap().in_flight,
        vec![wt("a"), wt("b")]
    );
}

fn result(name: &str, outcome: AttachOutcome) -> AttachResult {
    AttachResult {
        item: wt(name),
        outcome,
    }
}

#[test]
fn an_applied_batch_closes_the_dialog_and_says_what_happened() {
    let mut state = open_state();
    attach::update(&mut state, Msg::AttachAll);
    let outcomes = attach::update(
        &mut state,
        Msg::Applied(vec![
            result("a", AttachOutcome::Attached),
            result("b", AttachOutcome::Attached),
        ]),
    );
    assert!(state.attach.dialog.is_none());
    let [micold_client::features::Outcome::NotificationRaised(n)] = outcomes.as_slice() else {
        panic!("expected one notification, got {outcomes:?}");
    };
    assert_eq!(n.message, "Attached 2 worktrees.");
    assert_eq!(n.level, Level::Info);
}

#[test]
fn already_attached_is_said_not_silent() {
    let mut state = open_state();
    attach::update(&mut state, Msg::AttachAll);
    let outcomes = attach::update(
        &mut state,
        Msg::Applied(vec![result("a", AttachOutcome::AlreadyAttached)]),
    );
    let [micold_client::features::Outcome::NotificationRaised(n)] = outcomes.as_slice() else {
        panic!("expected one notification");
    };
    assert_eq!(n.message, "1 already attached.");
    assert_eq!(n.level, Level::Info);
}

#[test]
fn a_refusal_is_reported_as_an_error() {
    let results = vec![
        result("a", AttachOutcome::Attached),
        result(
            "b",
            AttachOutcome::Refused(RefuseReason::NotAWorktreeOfProject),
        ),
    ];
    assert_eq!(
        attach::summary(&results),
        "Attached 1 worktree. 1 could not be attached."
    );
}

#[test]
fn a_failed_apply_keeps_the_dialog_and_shows_why() {
    let mut state = open_state();
    attach::update(&mut state, Msg::AttachAll);
    attach::update(&mut state, Msg::ApplyFailed("disk full".into()));
    let dialog = state.attach.dialog.as_ref().unwrap();
    assert!(!dialog.applying());
    assert_eq!(dialog.error.as_deref(), Some("disk full"));
    assert!(
        matches!(dialog.listing, Listing::Listed(_)),
        "the list is kept"
    );
    assert!(
        !dialog.attachable().is_empty(),
        "so Attach all stays usable"
    );
}

#[test]
fn cancel_is_ignored_while_an_apply_is_outstanding() {
    let mut state = open_state();
    attach::update(&mut state, Msg::AttachAll);
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
