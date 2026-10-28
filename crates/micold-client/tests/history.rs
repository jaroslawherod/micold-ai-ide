//! Back/Forward on the root `State`, with no window (feature 1465, contracts/ui.md U1, U3–U5).
//!
//! Every message goes through `State::update` and then `observe_place`, as the binary's `update`
//! does; a press resolves, dispatches `apply_history`'s follow-ups the same way and finishes the
//! move (U2).

use std::path::{Path, PathBuf};

use micold_client::app::{Message, State};
use micold_client::features::history::{Lookup, Msg, Source, Step};
use micold_client::features::{
    navigation, overview, project, quick_switch, runs, session, sidebar,
};
use micold_client::keymap::{NavAction, NavLevel};
use micold_core::history::{Dir, Place, View};
use micold_core::project::Availability;
use micold_core::quick_switch::EntryKey;
use micold_core::session::{AiCli, Session, SessionId, SessionLifecycle, SessionLocation};

mod support;
use support::sidebar_fixture::{
    arranged, deliver, listed, run_group, session_at, state_with_active, worktree,
};

/// No project other than the active one has worktrees.
fn no_worktrees(_: &Path) -> Option<Vec<String>> {
    Some(Vec::new())
}

/// A message as the binary sends it: the reducer, then the history observes.
fn send(state: &mut State, message: Message) {
    send_with(state, message, &no_worktrees);
}

/// As [`send`]; a project switch (`Project(Reopened)`, the shell's) runs as
/// `on_known_project_reopened` does, its worktrees read from `lookup`.
fn send_with(state: &mut State, message: Message, lookup: Lookup) {
    if let Message::Project(project::Msg::Reopened(path)) = &message {
        let arrival = state.switch_active(path).expect("the switch is accepted");
        micold_client::app::drain(arrival, |o| micold_client::app::interpret(state, o));
        let worktrees = lookup(path)
            .unwrap_or_default()
            .iter()
            .map(|dir| listed(dir))
            .collect();
        deliver(state, worktrees);
    } else {
        state.update(message);
    }
    state.observe_place();
}

/// A press as the binary's `shell::history::update` runs it (U2).
fn press(state: &mut State, dir: Dir, source: Source) -> Step {
    press_with(state, dir, source, &no_worktrees)
}

fn press_with(state: &mut State, dir: Dir, source: Source, lookup: Lookup) -> Step {
    let step = state.press_history(Msg::Pressed { dir, source }, lookup);
    for message in state.apply_history(step.clone()) {
        send_with(state, message, lookup);
    }
    if matches!(step, Step::Show(_)) {
        state.finish_history_move(source);
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
    send(
        &mut state,
        Message::Session(session::Msg::CloseRequested(a)),
    );
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
    assert_eq!(
        entries(&state),
        vec![
            at_session(SessionLocation::Default, d),
            at_session(worktree("feat-b"), b)
        ],
        "FR-007: feat-a's place removed"
    );
}

#[test]
fn forward_passes_over_the_shown_place_and_on_past_a_gone_one() {
    // TDD audit finding 1: D → A → D → B, Back three times to the first D, A archived. Forward
    // removes A, passes over the second D (the place shown) and lands on B.
    let (mut state, d, a, b) = three();
    for id in [a, d, b] {
        select(&mut state, id);
    }
    for _ in 0..3 {
        assert!(matches!(back(&mut state), Step::Show(_)));
    }
    assert_eq!(state.history.list.position(), 0, "on the first D");
    state
        .workspace
        .sessions
        .get_mut(&repo())
        .and_then(|list| list.iter_mut().find(|s| s.id == a))
        .expect("A")
        .archive();
    assert_eq!(
        press(&mut state, Dir::Forward, Source::Button),
        Step::Show(at_session(worktree("feat-b"), b)),
        "FR-006, FR-007"
    );
    assert_eq!(
        entries(&state),
        vec![
            at_session(SessionLocation::Default, d),
            at_session(worktree("feat-b"), b)
        ],
        "A removed and the two Ds merged"
    );
}

#[test]
fn the_shown_place_is_none_while_settings_is_open() {
    // TDD audit finding 3: Settings is not a place (FR-010, spec Assumptions).
    let (mut state, _, _, b) = walked();
    assert_eq!(state.shown_place(), Some(at_session(worktree("feat-b"), b)));
    state.settings.settings_draft = Some(Default::default());
    assert_eq!(state.shown_place(), None, "Settings shows no place");
}

// ---- Across projects (US4, research R5) ---------------------------------------------------------

fn other() -> PathBuf {
    PathBuf::from("/other")
}

fn in_other(view: View) -> Place {
    Place {
        project: other(),
        view,
    }
}

/// [`three`] plus a known, available project `/other` with an idle session at its root (O) and one
/// in its worktree `w1` (W), never shown.
fn two_projects() -> (State, SessionId, SessionId, SessionId) {
    let (mut state, d, _, _) = three();
    state
        .workspace
        .projects
        .push(support::sidebar_fixture::project(
            "other",
            Availability::Available,
        ));
    let mut sessions = Vec::new();
    let mut ids = Vec::new();
    for location in [SessionLocation::Default, worktree("w1")] {
        let mut session = Session::start_new(location, AiCli::ClaudeCode);
        session.lifecycle = SessionLifecycle::Idle;
        ids.push(session.id);
        sessions.push(session);
    }
    state.workspace.sessions.insert(other(), sessions);
    (state, d, ids[0], ids[1])
}

/// `/other`'s worktrees: `w1` only.
fn w1_only(project: &Path) -> Option<Vec<String>> {
    (project == other()).then(|| vec!["w1".to_string()])
}

/// The list `places`, then the Overview of `/repo` shown and recorded last.
fn walked_through(state: &mut State, places: &[Place]) {
    let mut list = micold_core::history::History::default();
    for place in places {
        list.record(place.clone());
    }
    list.record(at(View::Overview));
    state.history.list = list;
    state.overview.open = true;
}

#[test]
fn back_reaches_another_projects_session_by_switching_project() {
    let (mut state, d, o, _) = two_projects();
    let target = in_other(View::Session {
        location: SessionLocation::Default,
        id: o,
    });
    walked_through(
        &mut state,
        &[at_session(SessionLocation::Default, d), target.clone()],
    );
    let before = entries(&state);
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(target.clone())
    );
    assert_eq!(state.workspace.active, Some(other()), "switched project");
    assert_eq!(state.session.active, Some(o));
    assert_eq!(entries(&state), before, "R2: nothing recorded on the way");
    assert_eq!(state.history.list.position(), 1);
    press_with(&mut state, Dir::Forward, Source::Button, &w1_only);
    assert_eq!(state.workspace.active, Some(repo()));
    assert!(state.overview.open, "Forward returns to /repo's Overview");
    assert_eq!(entries(&state), before);
}

#[test]
fn another_projects_listed_worktree_and_default_exist() {
    let (mut state, _, o, w) = two_projects();
    let at_w = in_other(View::Session {
        location: worktree("w1"),
        id: w,
    });
    walked_through(
        &mut state,
        &[
            in_other(View::Session {
                location: SessionLocation::Default,
                id: o,
            }),
            at_w.clone(),
        ],
    );
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(at_w)
    );
    assert_eq!(state.session.active, Some(w));
}

#[test]
fn another_projects_worktree_missing_from_its_lookup_is_skipped_and_removed() {
    let (mut state, d, _, w) = two_projects();
    let at_w = in_other(View::Session {
        location: worktree("w1"),
        id: w,
    });
    // `w1` is gone from disk: the lookup lists no worktree of /other.
    walked_through(
        &mut state,
        &[at_session(SessionLocation::Default, d), at_w.clone()],
    );
    assert_eq!(
        back(&mut state),
        Step::Show(at_session(SessionLocation::Default, d)),
        "US4 AS2"
    );
    assert!(!entries(&state).contains(&at_w), "FR-007: removed");

    // Unreadable (`None`): its worktree places are gone, its root is not.
    let (mut state, _, o, w) = two_projects();
    let at_w = in_other(View::Session {
        location: worktree("w1"),
        id: w,
    });
    let at_o = in_other(View::Session {
        location: SessionLocation::Default,
        id: o,
    });
    walked_through(&mut state, &[at_o.clone(), at_w.clone()]);
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &|_| None),
        Step::Show(at_o),
        "the Default of a known, available project always exists"
    );
    assert!(!entries(&state).contains(&at_w));
    assert_eq!(state.workspace.active, Some(other()));
}

#[test]
fn an_unavailable_or_unknown_projects_places_are_skipped_and_removed() {
    let (mut state, d, o, _) = two_projects();
    let at_o = in_other(View::Session {
        location: SessionLocation::Default,
        id: o,
    });
    let elsewhere = Place {
        project: PathBuf::from("/forgotten"),
        view: View::Overview,
    };
    state.workspace.projects[1].availability = Availability::Unavailable;
    walked_through(
        &mut state,
        &[
            at_session(SessionLocation::Default, d),
            elsewhere.clone(),
            at_o.clone(),
        ],
    );
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(at_session(SessionLocation::Default, d)),
        "US4 AS2, AS3"
    );
    assert_eq!(state.workspace.active, Some(repo()), "never switched");
    assert!(!entries(&state).contains(&at_o), "unavailable: removed");
    assert!(!entries(&state).contains(&elsewhere), "unknown: removed");
}

#[test]
fn compare_of_another_project_reopens_it_with_its_last_group_list() {
    let (mut state, _, o, _) = two_projects();
    let group = run_group(7, "g", &["feat-a", "feat-b"]);
    let gid = group.id;
    send(
        &mut state,
        Message::Runs(runs::Msg::GroupsChanged(vec![group])),
    );
    send(
        &mut state,
        Message::Runs(runs::Msg::CompareOpened {
            group: gid,
            project: repo(),
        }),
    );
    // To /other through the switcher; its own group list (empty) arrives and closes Compare.
    send(
        &mut state,
        Message::Project(project::Msg::Reopened(other())),
    );
    send(
        &mut state,
        Message::Runs(runs::Msg::GroupsChanged(Vec::new())),
    );
    select(&mut state, o);
    assert_eq!(state.workspace.active, Some(other()));
    assert!(state.runs.compare.is_none());

    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(at(View::Compare(gid)))
    );
    assert_eq!(state.workspace.active, Some(repo()), "FR-006");
    assert_eq!(state.runs.compare.as_ref().map(|c| c.group), Some(gid));
    let o_place = in_other(View::Session {
        location: SessionLocation::Default,
        id: o,
    });
    assert_eq!(
        entries(&state).last(),
        Some(&o_place),
        "FR-007: the forward entries are kept"
    );
    press_with(&mut state, Dir::Forward, Source::Button, &w1_only);
    assert_eq!(state.workspace.active, Some(other()));
    assert_eq!(state.session.active, Some(o));
}

#[test]
fn compare_of_another_project_whose_group_is_gone_is_skipped_and_removed() {
    let (mut state, d, _, _) = two_projects();
    let gid = run_group(7, "g", &["feat-a", "feat-b"]).id;
    // /other's last list holds no group 7 (it was dismissed there).
    state.runs.known.insert(other(), Vec::new());
    let gone = in_other(View::Compare(gid));
    walked_through(
        &mut state,
        &[at_session(SessionLocation::Default, d), gone.clone()],
    );
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(at_session(SessionLocation::Default, d))
    );
    assert!(!entries(&state).contains(&gone));
    assert_eq!(state.workspace.active, Some(repo()));
}

/// [`two_projects`] plus a session in `/other`'s included worktree `w1-2`, a `dir_name` `reconcile`
/// uniquified (decision 7): only the daemon's list names it, never this machine's discovery.
fn with_included(state: &mut State) -> SessionId {
    let mut session = Session::start_new(worktree("w1-2"), AiCli::ClaudeCode);
    session.lifecycle = SessionLifecycle::Idle;
    let id = session.id;
    state
        .workspace
        .sessions
        .get_mut(&other())
        .expect("two_projects lists /other's sessions")
        .push(session);
    id
}

/// `/other` was active once and its authoritative list (the daemon's push) named `w1` and the
/// included `w1-2`; then the user went back to `/repo`'s Overview.
fn visited_other_with_included(state: &mut State) {
    send_with(
        state,
        Message::Project(project::Msg::Reopened(other())),
        &|_| Some(vec!["w1".to_string(), "w1-2".to_string()]),
    );
    send_with(
        state,
        Message::Project(project::Msg::Reopened(repo())),
        &|_| Some(vec!["feat-a".to_string(), "feat-b".to_string()]),
    );
}

#[test]
fn another_projects_included_worktree_from_its_last_list_exists() {
    // Review A M4 F1: the local discovery (`w1_only`) never lists an included worktree.
    let (mut state, d, _, _) = two_projects();
    let i = with_included(&mut state);
    visited_other_with_included(&mut state);
    let at_i = in_other(View::Session {
        location: worktree("w1-2"),
        id: i,
    });
    walked_through(
        &mut state,
        &[at_session(SessionLocation::Default, d), at_i.clone()],
    );
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(at_i.clone()),
        "FR-006: switching to /other shows it once the daemon's list arrives"
    );
    assert!(entries(&state).contains(&at_i), "FR-007: not removed");
    assert_eq!(state.workspace.active, Some(other()));
}

#[test]
fn a_remote_projects_worktree_from_its_last_list_exists() {
    // Review A M4 F1: a remote project (or no local git) reads nothing locally: `None`.
    let (mut state, d, _, w) = two_projects();
    visited_other_with_included(&mut state);
    let at_w = in_other(View::Review(worktree("w1")));
    let at_ws = in_other(View::Session {
        location: worktree("w1"),
        id: w,
    });
    walked_through(
        &mut state,
        &[
            at_session(SessionLocation::Default, d),
            at_ws.clone(),
            at_w.clone(),
        ],
    );
    let unreadable = |_: &Path| None;
    assert_eq!(
        state.press_history(
            Msg::Pressed {
                dir: Dir::Back,
                source: Source::Button
            },
            &unreadable
        ),
        Step::Show(at_w.clone())
    );
    assert!(entries(&state).contains(&at_ws), "FR-007: nothing removed");
}

#[test]
fn a_press_reads_each_projects_worktrees_once() {
    // Review A M4 F2: the lookup is a `git worktree list` on the UI thread.
    let (mut state, d, _, w) = two_projects();
    walked_through(
        &mut state,
        &[
            at_session(SessionLocation::Default, d),
            in_other(View::Session {
                location: worktree("w1"),
                id: w,
            }),
            in_other(View::Review(worktree("w1"))),
            in_other(View::Review(worktree("w9"))),
        ],
    );
    let calls = std::cell::Cell::new(0);
    let counting = |_: &Path| {
        calls.set(calls.get() + 1);
        Some(Vec::new())
    };
    assert_eq!(
        state.press_history(
            Msg::Pressed {
                dir: Dir::Back,
                source: Source::Button
            },
            &counting
        ),
        Step::Show(at_session(SessionLocation::Default, d)),
        "all three of /other's worktree places are gone"
    );
    assert_eq!(calls.get(), 1, "one read of /other for the press");
}

#[test]
fn compare_left_open_across_a_switch_is_not_recorded_under_the_new_project() {
    // Review A M4 F4: until /other's list arrives, Compare still shows /repo's group.
    let (mut state, _, _, _) = two_projects();
    let group = run_group(7, "g", &["feat-a", "feat-b"]);
    let gid = group.id;
    send(
        &mut state,
        Message::Runs(runs::Msg::GroupsChanged(vec![group])),
    );
    send(
        &mut state,
        Message::Runs(runs::Msg::CompareOpened {
            group: gid,
            project: repo(),
        }),
    );
    send_with(
        &mut state,
        Message::Project(project::Msg::Reopened(other())),
        &w1_only,
    );
    assert!(
        state.runs.compare.is_some(),
        "still open: /other's list has not arrived"
    );
    assert_ne!(
        state.shown_place().map(|p| p.view),
        Some(View::Compare(gid)),
        "/repo's group is not a place of /other"
    );
    assert!(!entries(&state).contains(&in_other(View::Compare(gid))));
}

#[test]
fn a_compare_dismissed_in_the_background_is_reopened_then_closed_by_its_list() {
    // Review A M4 F3, accepted (data-model.md): `known[P]` updates only while P is active, so a
    // group dismissed while P was in the background still counts as existing. Back opens it from
    // the stale list; P's own list closes it; the place shown then is recorded and cuts the
    // forward entries off, as any new place does.
    let (mut state, d, o, _) = two_projects();
    let gid = run_group(7, "g", &["feat-a", "feat-b"]).id;
    state
        .runs
        .known
        .insert(other(), vec![run_group(7, "g", &["w1", "w2"])]);
    let stale = in_other(View::Compare(gid));
    let at_o = in_other(View::Session {
        location: SessionLocation::Default,
        id: o,
    });
    walked_through(
        &mut state,
        &[
            at_o.clone(),
            stale.clone(),
            at_session(SessionLocation::Default, d),
        ],
    );
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(at_session(SessionLocation::Default, d))
    );
    assert_eq!(
        press_with(&mut state, Dir::Back, Source::Button, &w1_only),
        Step::Show(stale.clone())
    );
    assert_eq!(state.runs.compare.as_ref().map(|c| c.group), Some(gid));
    // /other's own list arrives without the group (dismissed in the background).
    send(
        &mut state,
        Message::Runs(runs::Msg::GroupsChanged(Vec::new())),
    );
    assert!(state.runs.compare.is_none(), "closed by its project's list");
    select(&mut state, o);
    assert_eq!(
        entries(&state),
        vec![at_o.clone(), stale, at_o],
        "the place shown next is recorded; the forward entries are cut off"
    );
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
fn apply_history_switches_project_first_for_another_projects_place() {
    let (state, _, o, _) = two_projects();
    let reopened = || Message::Project(project::Msg::Reopened(other()));
    assert_eq!(
        state.apply_history(Step::Show(in_other(View::Session {
            location: SessionLocation::Default,
            id: o,
        }))),
        vec![reopened(), Message::Session(session::Msg::Selected(o))]
    );
    assert_eq!(
        state.apply_history(Step::Show(in_other(View::Overview))),
        vec![
            reopened(),
            Message::Overview(overview::Msg::Opened {
                settings_open: false
            })
        ]
    );
    let gid = run_group(7, "g", &["feat-a", "feat-b"]).id;
    assert_eq!(
        state.apply_history(Step::Show(in_other(View::Compare(gid)))),
        vec![
            reopened(),
            Message::Runs(runs::Msg::CompareOpened {
                group: gid,
                project: other(),
            })
        ]
    );
}

#[test]
fn finishing_a_move_records_only_a_place_other_than_the_target() {
    let (mut state, d, a, _) = walked();
    let step = state.press_history(
        Msg::Pressed {
            dir: Dir::Back,
            source: Source::Button,
        },
        &no_worktrees,
    );
    assert_eq!(step, Step::Show(at_session(worktree("feat-a"), a)));
    assert!(state.history.moving);
    for message in state.apply_history(step) {
        send(&mut state, message);
    }
    state.finish_history_move(Source::Button);
    assert!(!state.history.moving);
    assert_eq!(
        entries(&state).len(),
        3,
        "landed on the target: nothing new"
    );
    assert_eq!(state.history.list.position(), 1);

    // A move that lands elsewhere than its target: the shown place is recorded.
    let _ = state.press_history(
        Msg::Pressed {
            dir: Dir::Back,
            source: Source::Button,
        },
        &no_worktrees,
    );
    assert!(state.history.moving);
    state.finish_history_move(Source::Button);
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
fn back_with_nothing_shown_returns_to_the_place_just_left() {
    // Review A M1 F1: D → A → B, then nothing is shown (a stop with no session): Back shows B,
    // where the user just was, not A.
    let (mut state, _, a, b) = walked();
    state.session.active = None;
    state.observe_place();
    assert_eq!(state.shown_place(), None, "nothing shown");
    assert_eq!(
        back(&mut state),
        Step::Show(at_session(worktree("feat-b"), b))
    );
    assert_eq!(state.session.active, Some(b));
    back(&mut state);
    assert_eq!(state.session.active, Some(a), "then on as usual");
    assert_eq!(entries(&state).len(), 3, "moving recorded nothing (FR-007)");
}

#[test]
fn an_empty_project_has_nothing_to_go_back_to() {
    let mut state = state_with_active();
    state.observe_place();
    assert_eq!(back(&mut state), Step::None);
    assert!(entries(&state).is_empty());
}

// ---- U3: the keys (M2, FR-009, FR-012) ---------------------------------------------------------

fn key_back(state: &mut State) -> Step {
    press(state, Dir::Back, Source::Key)
}

/// A key press that changes nothing: no step, the list and the shown place as they were.
fn assert_key_ignored(state: &mut State, why: &str) {
    let before_shown = state.shown_place();
    let before = state.history.list.clone();
    assert_eq!(key_back(state), Step::None, "{why}");
    assert_eq!(state.history.list, before, "{why}: the list did not move");
    assert_eq!(state.shown_place(), before_shown, "{why}: nothing moved");
}

/// D → A → B recorded, B shown, and the user gave the terminal's keyboard back to the app (B5).
fn walked_released() -> (State, SessionId, SessionId, SessionId) {
    let (mut state, d, a, b) = walked();
    send(
        &mut state,
        Message::Session(session::Msg::TerminalFocusReleased),
    );
    assert!(!state.terminal_focused(), "released");
    (state, d, a, b)
}

#[test]
fn a_key_press_does_nothing_while_a_field_holds_the_keyboard() {
    use micold_client::features::window::FieldId;
    let (mut state, _, _, b) = walked_released();
    state.window.focused_field = Some(FieldId::Composer(b));
    assert!(state.live_focused_field().is_some());
    assert_key_ignored(&mut state, "a focused composer (US2 AS2)");
}

#[test]
fn a_key_press_acts_over_a_closed_comment_boxs_stale_focus() {
    use micold_client::features::window::FieldId;
    let (mut state, _, a, _) = walked_released();
    state.window.focused_field = Some(FieldId::ReviewComposer);
    assert_eq!(state.live_focused_field(), None, "no comment box is open");
    assert_eq!(
        key_back(&mut state),
        Step::Show(at_session(worktree("feat-a"), a))
    );
}

#[test]
fn a_key_press_acts_over_a_session_fields_stale_focus_behind_the_review_tab() {
    // TDD audit finding 2 (research R8): a session's composer or queued message left focused when
    // the Review tab replaced it never reports the blur; the keys are the app's again.
    use micold_client::features::window::FieldId;
    for field in [FieldId::Composer, FieldId::QueuedMessage] {
        let (mut state, _, _, b) = walked_released();
        send(
            &mut state,
            Message::Sidebar(sidebar::Msg::ReviewChangesRequested(worktree("feat-b"))),
        );
        assert!(state.review_shown().is_some(), "the Review tab is shown");
        assert!(!state.terminal_focused(), "precondition");
        state.window.focused_field = Some(field(b));
        assert_eq!(
            state.live_focused_field(),
            None,
            "the field is behind the tab"
        );
        assert_eq!(
            key_back(&mut state),
            Step::Show(at_session(worktree("feat-b"), b)),
            "Back leaves the Review tab for B (US2 AS2 boundary, FR-009)"
        );
    }
}

#[test]
fn a_key_press_does_nothing_while_the_terminal_holds_the_keyboard() {
    let (mut state, _, _, _) = walked();
    assert!(state.terminal_focused(), "selecting B focused its terminal");
    assert_key_ignored(&mut state, "a focused terminal (US2 AS3)");
}

#[test]
fn a_key_press_does_nothing_over_settings() {
    let (mut state, _, _, _) = walked_released();
    state.settings.settings_draft = Some(Default::default());
    let before = state.history.list.clone();
    assert_eq!(key_back(&mut state), Step::None, "Settings (FR-010)");
    assert_eq!(state.history.list, before);
}

#[test]
fn three_key_presses_step_back_three_entries() {
    // US2 AS4, FR-012: each press or repeat steps one more entry, through sessions too, so a key
    // move leaves the keyboard with the app rather than in the terminal it shows.
    let (mut state, d, a, b) = walked();
    send(
        &mut state,
        Message::Overview(overview::Msg::Toggled {
            settings_open: false,
        }),
    );
    assert_eq!(state.shown_place(), Some(at(View::Overview)));
    for (want, id) in [("B", b), ("A", a), ("D", d)] {
        assert!(
            matches!(key_back(&mut state), Step::Show(_)),
            "back to {want}"
        );
        assert_eq!(state.session.active, Some(id), "{want} shown");
        assert!(!state.overview.open, "the Overview closed");
        assert!(!state.terminal_focused(), "the keyboard stays with the app");
    }
    assert_eq!(key_back(&mut state), Step::None, "until there is none");
    assert_eq!(entries(&state).len(), 4, "moving recorded nothing (FR-007)");
}

#[test]
fn a_button_move_lands_on_the_terminal_as_a_click_would() {
    // Only a key move keeps the keyboard with the app; a side button lands as a sidebar click.
    let (mut state, _, a, _) = walked_released();
    assert!(matches!(back(&mut state), Step::Show(_)));
    assert_eq!(state.session.active, Some(a));
    assert!(state.terminal_focused(), "A's terminal has the keyboard");
}

// ---- FR-011: the Review tab's own Back first (T024) --------------------------------------------

/// Opens a file in the shown Review tab; the root declines `Message::Changes`, the shell runs it.
fn browse_a_file(state: &mut State) {
    let file = micold_core::review::RelPath::from_native("src/a.rs").expect("relative path");
    micold_client::features::changes::update(
        &mut state.changes,
        micold_client::features::changes::Msg::BrowseOpen(file),
    );
    state.observe_place();
}

/// `feat-a` shown as a Review tab, after a session was shown; with `browse`, a file is opened in
/// it (one Back frame on its stack).
fn in_review(browse: bool) -> (State, SessionId) {
    let (mut state, _, a, _) = three();
    select(&mut state, a);
    send(
        &mut state,
        Message::Sidebar(sidebar::Msg::ReviewChangesRequested(worktree("feat-a"))),
    );
    if browse {
        browse_a_file(&mut state);
    }
    (state, a)
}

#[test]
fn back_in_a_browsing_review_tab_is_the_tabs_own_back() {
    for source in [Source::Button, Source::Key] {
        let (mut state, _) = in_review(true);
        let before = entries(&state);
        let position = state.history.list.position();
        let step = state.press_history(
            Msg::Pressed {
                dir: Dir::Back,
                source,
            },
            &no_worktrees,
        );
        assert_eq!(step, Step::ReviewBack, "{source:?}");
        assert_eq!(state.history.list.position(), position, "{source:?}");
        assert_eq!(entries(&state), before, "{source:?}");
        assert!(
            !state.history.moving,
            "{source:?}: nothing is being moved to"
        );
        assert_eq!(
            state.apply_history(step),
            vec![Message::Changes(
                micold_client::features::changes::Msg::BrowseBack
            )],
            "{source:?}"
        );
        // Applied as the shell runs it, then observed: still nothing recorded or moved.
        micold_client::features::changes::update(
            &mut state.changes,
            micold_client::features::changes::Msg::BrowseBack,
        );
        state.observe_place();
        assert_eq!(state.history.list.position(), position, "{source:?}");
        assert_eq!(entries(&state), before, "{source:?}: applied");
    }
}

#[test]
fn back_in_a_review_tab_without_a_browse_frame_is_the_global_back() {
    let (mut state, a) = in_review(false);
    let step = back(&mut state);
    assert_eq!(step, Step::Show(at_session(worktree("feat-a"), a)));
}

#[test]
fn forward_from_a_browsing_review_tab_is_the_global_forward() {
    let (mut state, a) = in_review(true);
    // D, A, Review(feat-a): leave the browse, go Back to A, then Forward is the Review tab again.
    micold_client::features::changes::update(
        &mut state.changes,
        micold_client::features::changes::Msg::BrowseBack,
    );
    back(&mut state);
    assert_eq!(state.session.active, Some(a));
    let step = press(&mut state, Dir::Forward, Source::Button);
    assert_eq!(step, Step::Show(at(View::Review(worktree("feat-a")))));
    // Browsing there again: Forward with nothing ahead stays the global Forward (does nothing).
    browse_a_file(&mut state);
    assert_eq!(
        press(&mut state, Dir::Forward, Source::Key),
        Step::None,
        "Forward never reaches the tab's own Back"
    );
}
