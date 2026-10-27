//! Back/Forward on the root `State`, with no window (feature 1465, contracts/ui.md U1, U3–U5).
//!
//! Every message goes through `State::update` and then `observe_place`, as the binary's `update`
//! does; a press resolves, dispatches `apply_history`'s follow-ups the same way and finishes the
//! move (U2).

use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::history::{Msg, Source, Step};
use micold_client::features::{navigation, overview, quick_switch, runs, session, sidebar};
use micold_client::keymap::{NavAction, NavLevel};
use micold_core::history::{Dir, Place, View};
use micold_core::quick_switch::EntryKey;
use micold_core::session::{SessionId, SessionLocation};

mod support;
use support::sidebar_fixture::{
    arranged, deliver, listed, run_group, session_at, state_with_active, worktree,
};

/// A message as the binary sends it: the reducer, then the history observes.
fn send(state: &mut State, message: Message) {
    state.update(message);
    state.observe_place();
}

/// A press as the binary's `shell::history::update` runs it (U2).
fn press(state: &mut State, dir: Dir, source: Source) -> Step {
    let step = state.press_history(Msg::Pressed { dir, source });
    for message in state.apply_history(step.clone()) {
        send(state, message);
    }
    if matches!(step, Step::Show(_)) {
        state.finish_history_move();
    }
    state.observe_place();
    step
}

fn back(state: &mut State) -> Step {
    press(state, Dir::Back, Source::Button)
}

fn select(state: &mut State, id: SessionId) {
    send(state, Message::Session(session::Msg::Selected(id)));
}

fn repo() -> PathBuf {
    PathBuf::from("/repo")
}

fn at_session(location: SessionLocation, id: SessionId) -> Place {
    Place {
        project: repo(),
        view: View::Session { location, id },
    }
}

fn at(view: View) -> Place {
    Place {
        project: repo(),
        view,
    }
}

/// `/repo` with sessions D (Default), A (feat-a) and B (feat-b), D shown and recorded.
fn three() -> (State, SessionId, SessionId, SessionId) {
    let (mut state, d, a) = arranged();
    let b = session_at(&mut state, worktree("feat-b"));
    state.observe_place();
    (state, d, a, b)
}

fn entries(state: &State) -> Vec<Place> {
    state.history.list.entries().to_vec()
}

// ---- U1: recording ---------------------------------------------------------------------------

#[test]
fn a_sidebar_selection_records_one_entry() {
    let (mut state, d, a, _) = three();
    select(&mut state, a);
    assert_eq!(
        entries(&state),
        vec![
            at_session(SessionLocation::Default, d),
            at_session(worktree("feat-a"), a)
        ]
    );
}

#[test]
fn re_selecting_the_shown_session_records_nothing() {
    let (mut state, _, a, _) = three();
    select(&mut state, a);
    select(&mut state, a);
    assert_eq!(entries(&state).len(), 2, "FR-003");
}

#[test]
fn a_quick_switcher_choice_records_one_entry() {
    let (mut state, _, a, _) = three();
    let _ = quick_switch::update(
        &mut state,
        quick_switch::Msg::Toggled {
            terminal_had_focus: true,
        },
    );
    let choice = quick_switch::update(
        &mut state,
        quick_switch::Msg::Chosen(EntryKey::Location(worktree("feat-a"))),
    )
    .expect("resolves");
    for message in state.apply_navigation(choice.step.clone(), choice.terminal_had_focus) {
        send(&mut state, message);
    }
    state.finish_navigation(&choice.step, choice.terminal_had_focus);
    state.observe_place();
    assert_eq!(state.session.active, Some(a));
    assert_eq!(entries(&state).len(), 2);
    assert_eq!(
        state.history.list.current(),
        Some(&at_session(worktree("feat-a"), a))
    );
}

#[test]
fn a_navigation_chord_step_records_one_entry() {
    let (mut state, _, a, _) = three();
    let step = navigation::update(
        &state,
        NavAction {
            level: NavLevel::Location,
            dir: micold_core::navigation::Dir::Next,
        },
    );
    for message in state.apply_navigation(step.clone(), true) {
        send(&mut state, message);
    }
    state.finish_navigation(&step, true);
    state.observe_place();
    assert_eq!(state.session.active, Some(a), "Default → feat-a");
    assert_eq!(entries(&state).len(), 2);
}

#[test]
fn the_overview_compare_and_a_review_tab_each_record_one_entry() {
    let (mut state, _, _, _) = three();
    let group = run_group(7, "g", &["feat-a", "feat-b"]);
    let gid = group.id;
    state.runs.groups = vec![group];
    send(
        &mut state,
        Message::Overview(overview::Msg::Toggled {
            settings_open: false,
        }),
    );
    assert_eq!(state.history.list.current(), Some(&at(View::Overview)));
    send(
        &mut state,
        Message::Runs(runs::Msg::CompareOpened {
            group: gid,
            project: repo(),
        }),
    );
    assert_eq!(state.history.list.current(), Some(&at(View::Compare(gid))));
    // Compare covers a Review tab, so it is closed first, as `apply_history` does.
    send(&mut state, Message::Runs(runs::Msg::CompareClosed));
    send(
        &mut state,
        Message::Sidebar(sidebar::Msg::ReviewChangesRequested(worktree("feat-b"))),
    );
    assert_eq!(
        state.history.list.current(),
        Some(&at(View::Review(worktree("feat-b"))))
    );
    let views: Vec<View> = entries(&state).into_iter().map(|p| p.view).collect();
    assert_eq!(views[1..3], [View::Overview, View::Compare(gid)]);
}

#[test]
fn the_shown_sessions_removal_records_where_the_app_moved() {
    let (mut state, _, a, _) = three();
    send(
        &mut state,
        Message::Sidebar(sidebar::Msg::ReviewChangesRequested(worktree("feat-a"))),
    );
    select(&mut state, a);
    send(&mut state, Message::Session(session::Msg::CloseRequested(a)));
    let review = at(View::Review(worktree("feat-a")));
    assert_eq!(
        state.shown_place(),
        Some(review.clone()),
        "the emptied location's Review tab"
    );
    assert_eq!(
        state.history.list.current(),
        Some(&review),
        "the app's own move is recorded like any other (U1)"
    );
}

#[test]
fn background_sessions_and_catalog_updates_record_nothing() {
    let (mut state, _, a, _) = three();
    let before = entries(&state);
    send(
        &mut state,
        Message::Session(session::Msg::TitleUpdated {
            id: a,
            title: "renamed".into(),
        }),
    );
    let _ = session_at(&mut state, worktree("feat-b"));
    state.observe_place();
    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);
    state.observe_place();
    assert_eq!(entries(&state), before, "the shown place did not change");
}

#[test]
fn nothing_is_observed_while_a_move_is_under_way() {
    let (mut state, _, a, _) = three();
    state.history.moving = true;
    select(&mut state, a);
    assert_eq!(entries(&state).len(), 1, "R2: observation is off");
}

#[test]
fn two_windows_keep_independent_histories() {
    let (mut one, _, a, _) = three();
    let (two, _, _, _) = three();
    select(&mut one, a);
    assert_eq!(entries(&one).len(), 2);
    assert_eq!(entries(&two).len(), 1, "FR-001");
}

// ---- U3: the gate ----------------------------------------------------------------------------

/// D → A → B recorded, B shown.
fn walked() -> (State, SessionId, SessionId, SessionId) {
    let (mut state, d, a, b) = three();
    select(&mut state, a);
    select(&mut state, b);
    (state, d, a, b)
}

fn assert_blocked(state: &mut State, why: &str) {
    let before_shown = state.shown_place();
    let before = state.history.list.clone();
    assert_eq!(back(state), Step::None, "{why}");
    assert_eq!(state.history.list, before, "{why}: the list did not move");
    assert_eq!(state.shown_place(), before_shown, "{why}: nothing moved");
}

#[test]
fn a_press_does_nothing_over_settings_a_dialog_the_switcher_or_the_review_sheet() {
    let (mut state, _, _, _) = walked();
    state.settings.settings_draft = Some(Default::default());
    let before = state.history.list.clone();
    assert_eq!(back(&mut state), Step::None, "Settings (FR-010)");
    assert_eq!(state.history.list, before);
    state.settings.settings_draft = None;

    state.help.about_open = true;
    assert_blocked(&mut state, "a dialog");
    state.help.about_open = false;

    let _ = quick_switch::update(
        &mut state,
        quick_switch::Msg::Toggled {
            terminal_had_focus: true,
        },
    );
    assert!(state.quick_switch.open.is_some());
    assert_blocked(&mut state, "the quick switcher");
    let _ = quick_switch::update(&mut state, quick_switch::Msg::Dismissed);

    send(
        &mut state,
        Message::Sidebar(sidebar::Msg::ReviewChangesRequested(worktree("feat-b"))),
    );
    state
        .changes
        .tabs
        .shown_view_mut()
        .expect("the Review tab is shown")
        .sheet_open = true;
    assert_blocked(&mut state, "the Review sheet");
}

#[test]
fn a_button_press_acts_while_a_field_holds_the_keyboard() {
    let (mut state, _, a, b) = walked();
    state.window.focused_field = Some(micold_client::features::window::FieldId::Composer(b));
    assert!(state.live_focused_field().is_some());
    assert_eq!(
        back(&mut state),
        Step::Show(at_session(worktree("feat-a"), a)),
        "FR-008"
    );
    assert_eq!(state.session.active, Some(a));
}

// ---- U5: gone places, in the active project -----------------------------------------------------

#[test]
fn an_archived_session_is_skipped_and_removed() {
    let (mut state, d, a, b) = walked();
    let session = state
        .workspace
        .sessions
        .get_mut(&repo())
        .and_then(|list| list.iter_mut().find(|s| s.id == a))
        .expect("A");
    session.archive();
    assert_eq!(
        back(&mut state),
        Step::Show(at_session(SessionLocation::Default, d)),
        "US4 AS1"
    );
    assert_eq!(
        entries(&state),
        vec![
            at_session(SessionLocation::Default, d),
            at_session(worktree("feat-b"), b)
        ],
        "FR-007: A removed"
    );
}

#[test]
fn a_removed_worktrees_places_are_skipped_and_removed() {
    let (mut state, d, _, b) = walked();
    state.worktree.worktrees.retain(|w| w.dir_name != "feat-a");
    assert_eq!(
        back(&mut state),
        Step::Show(at_session(SessionLocation::Default, d)),
        "US4 AS2"
    );
    assert_eq!(entries(&state).len(), 2);
    let _ = b;
}

#[test]
fn another_projects_place_is_skipped_and_removed_until_cross_project_moves() {
    let (mut state, d, _, _) = three();
    let other = Place {
        project: PathBuf::from("/other"),
        view: View::Overview,
    };
    let mut list = micold_core::history::History::default();
    list.record(at_session(SessionLocation::Default, d));
    list.record(other.clone());
    list.record(at(View::Overview));
    state.history.list = list;
    state.overview.open = true;
    assert_eq!(
        back(&mut state),
        Step::Show(at_session(SessionLocation::Default, d))
    );
    assert!(!entries(&state).contains(&other), "removed");
}

// ---- U4: showing a target ----------------------------------------------------------------------

#[test]
fn apply_history_sends_each_views_messages() {
    let (mut state, _, a, _) = three();
    let gid = run_group(7, "g", &["feat-a", "feat-b"]).id;
    assert_eq!(state.apply_history(Step::None), vec![]);
    assert_eq!(
        state.apply_history(Step::Show(at_session(worktree("feat-a"), a))),
        vec![Message::Session(session::Msg::Selected(a))]
    );
    let review = || Message::Sidebar(sidebar::Msg::ReviewChangesRequested(worktree("feat-a")));
    let review_place = || Step::Show(at(View::Review(worktree("feat-a"))));
    assert_eq!(state.apply_history(review_place()), vec![review()]);
    assert_eq!(
        state.apply_history(Step::Show(at(View::Overview))),
        vec![Message::Overview(overview::Msg::Opened {
            settings_open: false
        })]
    );
    let compare = || {
        Message::Runs(runs::Msg::CompareOpened {
            group: gid,
            project: repo(),
        })
    };
    assert_eq!(
        state.apply_history(Step::Show(at(View::Compare(gid)))),
        vec![compare()]
    );

    state.overview.open = true;
    state.runs.compare = Some(runs::CompareView {
        group: gid,
        project: repo(),
        reads: Default::default(),
    });
    let closed_overview = Message::Overview(overview::Msg::Closed);
    let closed_compare = Message::Runs(runs::Msg::CompareClosed);
    assert_eq!(
        state.apply_history(review_place()),
        vec![closed_overview.clone(), closed_compare.clone(), review()]
    );
    assert_eq!(
        state.apply_history(Step::Show(at(View::Overview))),
        vec![
            closed_compare,
            Message::Overview(overview::Msg::Opened {
                settings_open: false
            })
        ]
    );
    assert_eq!(
        state.apply_history(Step::Show(at(View::Compare(gid)))),
        vec![closed_overview, compare()]
    );
}

#[test]
fn finishing_a_move_records_only_a_place_other_than_the_target() {
    let (mut state, d, a, _) = walked();
    let step = state.press_history(Msg::Pressed {
        dir: Dir::Back,
        source: Source::Button,
    });
    assert_eq!(step, Step::Show(at_session(worktree("feat-a"), a)));
    assert!(state.history.moving);
    for message in state.apply_history(step) {
        send(&mut state, message);
    }
    state.finish_history_move();
    assert!(!state.history.moving);
    assert_eq!(entries(&state).len(), 3, "landed on the target: nothing new");
    assert_eq!(state.history.list.position(), 1);

    // A move that lands elsewhere than its target: the shown place is recorded.
    let _ = state.press_history(Msg::Pressed {
        dir: Dir::Back,
        source: Source::Button,
    });
    assert!(state.history.moving);
    state.finish_history_move();
    assert_eq!(
        state.history.list.current(),
        Some(&at_session(worktree("feat-a"), a)),
        "A is still shown, so it is recorded after D"
    );
    assert_eq!(state.history.list.position(), 1);
    assert_eq!(entries(&state)[0], at_session(SessionLocation::Default, d));
}

#[test]
fn back_and_forward_walk_the_sessions() {
    let (mut state, d, a, b) = walked();
    back(&mut state);
    assert_eq!(state.session.active, Some(a), "US1 AS1");
    back(&mut state);
    assert_eq!(state.session.active, Some(d));
    assert_eq!(back(&mut state), Step::None, "US1 AS6: at the first");
    press(&mut state, Dir::Forward, Source::Button);
    assert_eq!(state.session.active, Some(a), "US1 AS2");
    press(&mut state, Dir::Forward, Source::Button);
    assert_eq!(state.session.active, Some(b));
    assert_eq!(
        press(&mut state, Dir::Forward, Source::Button),
        Step::None,
        "at the last"
    );
    assert_eq!(entries(&state).len(), 3, "moving recorded nothing (FR-007)");
}

#[test]
fn an_empty_project_has_nothing_to_go_back_to() {
    let mut state = state_with_active();
    state.observe_place();
    assert_eq!(back(&mut state), Step::None);
    assert!(entries(&state).is_empty());
}
