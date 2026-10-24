//! What a navigation press resolves to (feature 803, contracts/ui.md U1, U2 location row).

use micold_client::app::{Message, State};
use micold_client::features::navigation::{location_order, update, Step};
use micold_client::features::sidebar::TagFilter;
use micold_client::keymap::{NavAction, NavLevel};
use micold_core::naming::ConventionalType;
use micold_core::navigation::Dir;
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};

mod support;
use support::sidebar_fixture::{
    deliver, listed, run_group, session_at, state_with_active, worktree,
};

const NEXT: NavAction = NavAction {
    level: NavLevel::Location,
    dir: Dir::Next,
};
const PREVIOUS: NavAction = NavAction {
    level: NavLevel::Location,
    dir: Dir::Previous,
};

/// Default and three worktrees, each with one session; returns the sessions in that order.
fn four_locations() -> (State, Vec<SessionId>) {
    let mut state = state_with_active();
    deliver(
        &mut state,
        vec![listed("feat-a"), listed("feat-b"), listed("feat-c")],
    );
    let ids = vec![
        session_at(&mut state, SessionLocation::Default),
        session_at(&mut state, worktree("feat-a")),
        session_at(&mut state, worktree("feat-b")),
        session_at(&mut state, worktree("feat-c")),
    ];
    (state, ids)
}

#[test]
fn the_location_order_is_default_then_the_worktrees_as_drawn() {
    let (state, _) = four_locations();
    assert_eq!(
        location_order(&state),
        vec![
            SessionLocation::Default,
            worktree("feat-a"),
            worktree("feat-b"),
            worktree("feat-c")
        ]
    );
}

#[test]
fn filtered_worktrees_are_skipped_but_the_current_ones_location_stays() {
    let (mut state, ids) = four_locations();
    // `untyped-x` has no conventional type, so the Feat filter hides it.
    deliver(
        &mut state,
        vec![
            listed("feat-a"),
            listed("untyped-x"),
            listed("feat-b"),
            listed("feat-c"),
        ],
    );
    let hidden = session_at(&mut state, worktree("untyped-x"));
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Feat));
    state.session.active = Some(ids[1]);
    assert!(
        !location_order(&state).contains(&worktree("untyped-x")),
        "a worktree the tag filter hides is skipped (US1 AS7)"
    );
    state.session.active = Some(hidden);
    assert!(
        location_order(&state).contains(&worktree("untyped-x")),
        "the current session's location stays listed, as the sidebar keeps it shown"
    );
}

#[test]
fn a_collapsed_run_groups_worktrees_are_skipped_and_an_expanded_ones_are_not() {
    let (mut state, _) = four_locations();
    deliver(
        &mut state,
        vec![
            listed("feat-a"),
            listed("feat-b"),
            listed("feat-c"),
            listed("feat-g-1"),
            listed("feat-g-2"),
        ],
    );
    let group = run_group(1, "g", &["feat-g-1", "feat-g-2"]);
    let id = group.id;
    state.update(Message::Runs(
        micold_client::features::runs::Msg::GroupsChanged(vec![group]),
    ));
    let order = location_order(&state);
    assert!(
        order.contains(&worktree("feat-g-1")) && order.contains(&worktree("feat-g-2")),
        "an expanded group's runs are locations: {order:?}"
    );
    state.update(Message::Runs(
        micold_client::features::runs::Msg::GroupToggled(id),
    ));
    let order = location_order(&state);
    assert!(
        !order.contains(&worktree("feat-g-1")) && !order.contains(&worktree("feat-g-2")),
        "a collapsed group's runs are skipped: {order:?}"
    );
}

#[test]
fn next_and_previous_step_from_the_middle_and_wrap_at_both_ends() {
    let (mut state, ids) = four_locations();
    state.session.active = Some(ids[1]);
    assert_eq!(update(&state, NEXT), Step::Session(ids[2]), "US1 AS1");
    assert_eq!(update(&state, PREVIOUS), Step::Session(ids[0]));
    state.session.active = Some(ids[3]);
    assert_eq!(
        update(&state, NEXT),
        Step::Session(ids[0]),
        "wrap to Default (AS2)"
    );
    state.session.active = Some(ids[0]);
    assert_eq!(
        update(&state, PREVIOUS),
        Step::Session(ids[3]),
        "wrap to the last (AS2)"
    );
}

#[test]
fn no_current_session_starts_at_the_first_or_last_location() {
    let (state, ids) = four_locations();
    assert_eq!(update(&state, NEXT), Step::Session(ids[0]));
    assert_eq!(update(&state, PREVIOUS), Step::Session(ids[3]));
}

#[test]
fn a_default_only_project_or_no_project_changes_nothing() {
    let mut state = state_with_active();
    let id = session_at(&mut state, SessionLocation::Default);
    state.session.active = Some(id);
    assert_eq!(
        update(&state, NEXT),
        Step::None,
        "one location: stays on it"
    );
    assert_eq!(update(&state, PREVIOUS), Step::None);
    let state = State::default();
    assert_eq!(
        update(&state, NEXT),
        Step::None,
        "no active project: empty order"
    );
}

#[test]
fn a_location_without_a_session_is_a_stop_and_the_next_press_steps_on_from_it() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);
    let default = session_at(&mut state, SessionLocation::Default);
    let b = session_at(&mut state, worktree("feat-b"));
    state.session.active = Some(default);
    assert_eq!(
        update(&state, NEXT),
        Step::Stop(worktree("feat-a")),
        "an empty location becomes a stop (US1 AS5)"
    );
    state.sidebar.nav_stop = Some(worktree("feat-a"));
    assert_eq!(
        update(&state, NEXT),
        Step::Session(b),
        "the stop is the current location for the next press"
    );
    state.sidebar.nav_stop = Some(worktree("gone"));
    assert_eq!(
        update(&state, NEXT),
        Step::Stop(worktree("feat-a")),
        "a stop no longer in the order is ignored"
    );
}

#[test]
fn a_missing_worktree_without_a_session_is_a_stop() {
    let mut state = state_with_active();
    let mut missing = listed("feat-a");
    missing.status = micold_core::worktree::WorktreeStatus::Missing;
    deliver(&mut state, vec![missing]);
    let default = session_at(&mut state, SessionLocation::Default);
    state.session.active = Some(default);
    assert_eq!(update(&state, NEXT), Step::Stop(worktree("feat-a")));
}

#[test]
fn nothing_happens_while_settings_or_a_surface_is_open() {
    let (mut state, ids) = four_locations();
    state.session.active = Some(ids[1]);
    let open: [fn(&mut State); 4] = [
        |s| s.settings.settings_draft = Some(Default::default()),
        |s| s.sidebar.more_menu_open = true,
        |s| s.project.switcher_open = true,
        |s| s.help.help_menu_open = true,
    ];
    for (i, open) in open.iter().enumerate() {
        let mut state = state.clone();
        open(&mut state);
        assert_eq!(
            update(&state, NEXT),
            Step::None,
            "surface {i} open (FR-012)"
        );
    }
}

// ---- The session level (US2, contracts/ui.md U2 session row) ----

const NEXT_SESSION: NavAction = NavAction {
    level: NavLevel::Session,
    dir: Dir::Next,
};
const PREVIOUS_SESSION: NavAction = NavAction {
    level: NavLevel::Session,
    dir: Dir::Previous,
};

/// A worktree `feat-a` holding a main session and two helpers; returns them in list order.
fn main_and_two_helpers() -> (State, [SessionId; 3]) {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("feat-a")]);
    let location = worktree("feat-a");
    let mut main = Session::start_new(location.clone(), AiCli::ClaudeCode);
    main.main = true;
    let first = Session::start_new(location.clone(), AiCli::ClaudeCode);
    let second = Session::start_new(location, AiCli::ClaudeCode);
    let ids = [main.id, first.id, second.id];
    state
        .workspace
        .sessions
        .entry(std::path::PathBuf::from("/repo"))
        .or_default()
        .extend([main, first, second]);
    (state, ids)
}

#[test]
fn next_session_steps_through_main_then_helpers_and_wraps() {
    let (mut state, [main, first, second]) = main_and_two_helpers();
    state.session.active = Some(main);
    assert_eq!(update(&state, NEXT_SESSION), Step::Session(first), "AS1");
    state.session.active = Some(first);
    assert_eq!(update(&state, NEXT_SESSION), Step::Session(second));
    state.session.active = Some(second);
    assert_eq!(update(&state, NEXT_SESSION), Step::Session(main), "wrap");
}

#[test]
fn previous_session_wraps_backwards() {
    let (mut state, [main, first, second]) = main_and_two_helpers();
    state.session.active = Some(main);
    assert_eq!(update(&state, PREVIOUS_SESSION), Step::Session(second));
    state.session.active = Some(second);
    assert_eq!(update(&state, PREVIOUS_SESSION), Step::Session(first));
}

#[test]
fn one_session_no_current_session_or_a_stop_changes_nothing() {
    let (state, ids) = four_locations();
    let mut one = state.clone();
    one.session.active = Some(ids[1]);
    assert_eq!(update(&one, NEXT_SESSION), Step::None, "AS2");
    assert_eq!(update(&one, PREVIOUS_SESSION), Step::None);
    let (mut many, [main, ..]) = main_and_two_helpers();
    assert_eq!(update(&many, NEXT_SESSION), Step::None, "AS4: no current");
    many.session.active = Some(main);
    many.sidebar.nav_stop = Some(worktree("feat-a"));
    assert_eq!(
        update(&many, NEXT_SESSION),
        Step::None,
        "a stop is not a session"
    );
}

#[test]
fn a_helper_chosen_on_a_collapsed_row_opens_it() {
    let (mut state, [main, first, _]) = main_and_two_helpers();
    state.session.active = Some(main);
    state.sidebar.expanded.clear();
    let Step::Session(id) = update(&state, NEXT_SESSION) else {
        panic!("a step onto a helper");
    };
    assert_eq!(id, first);
    state.update(Message::Session(
        micold_client::features::session::Msg::Selected(id),
    ));
    assert!(state.location_open(&worktree("feat-a")), "AS3 (FR-009)");
}

// ---- Project level (feature 803, US3) ----

const NEXT_PROJECT: NavAction = NavAction {
    level: NavLevel::Project,
    dir: Dir::Next,
};
const PREVIOUS_PROJECT: NavAction = NavAction {
    level: NavLevel::Project,
    dir: Dir::Previous,
};

fn project_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from("/").join(name)
}

/// Projects `/a`, `/b`, `/c` in that order with `/b` unavailable; `/a` active.
fn three_projects() -> State {
    use micold_core::project::{Availability, Project};
    let mut state = State::default();
    for (name, availability) in [
        ("a", Availability::Available),
        ("b", Availability::Unavailable),
        ("c", Availability::Available),
    ] {
        state.workspace.projects.push(Project {
            path: project_path(name),
            display_name: name.to_string(),
            is_git_repo: true,
            availability,
        });
    }
    state.workspace.active = Some(project_path("a"));
    state
}

#[test]
fn project_steps_skip_unavailable_and_wrap_at_both_ends() {
    let mut state = three_projects();
    assert_eq!(
        update(&state, NEXT_PROJECT),
        Step::Project(project_path("c")),
        "AS3: /b is skipped"
    );
    assert_eq!(
        update(&state, PREVIOUS_PROJECT),
        Step::Project(project_path("c")),
        "AS2: wrap backwards"
    );
    state.workspace.active = Some(project_path("c"));
    assert_eq!(
        update(&state, NEXT_PROJECT),
        Step::Project(project_path("a")),
        "AS2: wrap forwards"
    );
}

#[test]
fn one_available_project_or_none_changes_nothing() {
    let mut state = three_projects();
    state.workspace.projects.remove(2);
    assert_eq!(update(&state, NEXT_PROJECT), Step::None, "AS4");
    assert_eq!(update(&state, PREVIOUS_PROJECT), Step::None, "AS4");
    state.workspace.projects.clear();
    state.workspace.active = None;
    assert_eq!(update(&state, NEXT_PROJECT), Step::None);
}

#[test]
fn the_welcome_screen_steps_to_the_first_or_last_project() {
    let mut state = three_projects();
    state.workspace.active = None;
    assert_eq!(
        update(&state, NEXT_PROJECT),
        Step::Project(project_path("a"))
    );
    assert_eq!(
        update(&state, PREVIOUS_PROJECT),
        Step::Project(project_path("c"))
    );
}

#[test]
fn a_project_step_reopens_the_project_and_lands_on_its_restored_session() {
    let mut state = three_projects();
    state.overview.open = true;
    let step = Step::Project(project_path("c"));
    let messages = state.apply_navigation(step.clone(), true);
    assert_eq!(
        messages,
        vec![Message::Project(
            micold_client::features::project::Msg::Reopened(project_path("c"))
        )]
    );
    assert!(state.overview.open, "closing it is the project switch's");
    // The switch ran: `/c` holds a session, which became current.
    state.workspace.active = Some(project_path("c"));
    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let id = session.id;
    state
        .workspace
        .sessions
        .entry(project_path("c"))
        .or_default()
        .push(session);
    state.session.active = Some(id);
    state.finish_navigation(&step, false);
    assert_eq!(
        state.sidebar.keyboard_row,
        Some(micold_client::features::sidebar::RowRef::Session(id))
    );
    state.session.active = None;
    state.finish_navigation(&step, false);
    assert_eq!(state.sidebar.keyboard_row, None, "no session restored");
    // A refused switch (the project is not active) leaves the keyboard where it was.
    state.session.active = Some(id);
    state.sidebar.keyboard_row = None;
    state.workspace.active = Some(project_path("a"));
    state.finish_navigation(&step, false);
    assert_eq!(
        state.sidebar.keyboard_row, None,
        "the switch did not happen"
    );
}

#[test]
fn an_unavailable_active_project_steps_to_the_available_ones() {
    let mut state = three_projects();
    state.workspace.projects[0].availability = micold_core::project::Availability::Unavailable;
    assert_eq!(
        update(&state, NEXT_PROJECT),
        Step::Project(project_path("c")),
        "/a is active but gone: /b is skipped, /c is next"
    );
}
