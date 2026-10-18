//! The quick switcher over the root state (feature 968): its candidates, reducer and apply path.

use micold_client::app::{Message, State};
use micold_client::features::navigation::location_order;
use micold_client::features::navigation::Step;
use micold_client::features::quick_switch::{
    candidates, list, update, Choice, Empty, Entry, Kind, Msg,
};
use micold_client::features::sidebar::{SidebarEntry, TagFilter};
use micold_core::naming::ConventionalType;
use micold_core::navigation::Dir;
use micold_core::quick_switch::EntryKey;
use micold_core::session::SessionLocation;
use micold_core::worktree::WorktreeStatus;

mod support;
use support::sidebar_fixture::{
    arranged, deliver, foreign_session, listed, press, project, run_group, session_at,
    session_count, state_with_active, worktree,
};

/// The locations among `entries`, in order (sessions are skipped).
fn keys(entries: &[Entry]) -> Vec<SessionLocation> {
    entries
        .iter()
        .filter_map(|e| match &e.candidate.key {
            EntryKey::Location(l) => Some(l.clone()),
            EntryKey::Session(_) => None,
            EntryKey::Project(_) => None,
        })
        .collect()
}

fn three() -> State {
    let mut state = state_with_active();
    deliver(
        &mut state,
        vec![listed("feat-a"), listed("fix-b"), listed("feat-c")],
    );
    state
}

#[test]
fn locations_come_in_location_order_every_one_selectable() {
    let state = three();
    let entries = candidates(&state);
    assert_eq!(
        keys(&entries),
        location_order(&state),
        "the locations come in the #803 navigation order"
    );
    assert_eq!(
        entries[0].candidate.key,
        EntryKey::Location(SessionLocation::Default),
        "Default is listed first"
    );
    assert!(
        entries.iter().all(|e| e.candidate.selectable
            && e.candidate.parent.is_none()
            && e.kind == Kind::Location),
        "every location is selectable, top level, and of kind Location"
    );
    let sessions = candidates(&state_with_active());
    assert!(
        sessions.iter().all(|e| e.kind == Kind::Location),
        "a project with no sessions lists locations only"
    );
}

#[test]
fn names_are_default_or_the_rows_display_name() {
    let state = three();
    let entries = candidates(&state);
    assert_eq!(
        entries[0].candidate.name, "Default",
        "the root is named Default"
    );
    let shown: Vec<String> = state
        .sidebar_entries()
        .into_iter()
        .filter_map(|e| match e {
            SidebarEntry::Worktree(n) => Some(n.display_name),
            _ => None,
        })
        .collect();
    let names: Vec<String> = entries[1..]
        .iter()
        .map(|e| e.candidate.name.clone())
        .collect();
    assert_eq!(names, shown, "a worktree is named as its sidebar row is");
    assert!(names.iter().all(|n| !n.is_empty()), "no row is nameless");
}

#[test]
fn a_hidden_worktree_is_absent_unless_it_is_the_current_location() {
    let mut state = three();
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Feat));
    assert!(
        !keys(&candidates(&state)).contains(&worktree("fix-b")),
        "a worktree the sidebar filter hides is not listed"
    );
    let id = session_at(&mut state, worktree("fix-b"));
    state.session.active = Some(id);
    let entries = candidates(&state);
    assert!(
        keys(&entries).contains(&worktree("fix-b")),
        "a hidden worktree is listed once it is the current location"
    );
    let current: Vec<_> = entries
        .iter()
        .filter(|e| e.current && e.kind == Kind::Location)
        .collect();
    assert_eq!(
        current.len(),
        1,
        "current is set on the current location only"
    );
    assert_eq!(
        current[0].candidate.key,
        EntryKey::Location(worktree("fix-b"))
    );
}

#[test]
fn a_navigation_stop_is_the_current_entry() {
    let mut state = three();
    state.sidebar.nav_stop = Some(worktree("feat-c"));
    let entries = candidates(&state);
    let current: Vec<_> = entries
        .iter()
        .filter(|e| e.current && e.kind == Kind::Location)
        .collect();
    assert_eq!(current.len(), 1, "exactly one location is current");
    assert_eq!(
        current[0].candidate.key,
        EntryKey::Location(worktree("feat-c")),
        "the navigation stop is the current location"
    );
}

#[test]
fn a_collapsed_groups_worktrees_follow_location_order() {
    let mut state = state_with_active();
    deliver(
        &mut state,
        vec![listed("feat-a"), listed("feat-g-1"), listed("feat-g-2")],
    );
    let group = run_group(1, "g", &["feat-g-1", "feat-g-2"]);
    let id = group.id;
    state.update(micold_client::app::Message::Runs(
        micold_client::features::runs::Msg::GroupsChanged(vec![group]),
    ));
    assert_eq!(
        keys(&candidates(&state)),
        location_order(&state),
        "a collapsed group's worktrees are listed in navigation order"
    );
    state.update(micold_client::app::Message::Runs(
        micold_client::features::runs::Msg::GroupToggled(id),
    ));
    assert_eq!(
        keys(&candidates(&state)),
        location_order(&state),
        "expanding the group does not change the order"
    );
}

#[test]
fn context_holds_the_tag_labels_with_the_status_word() {
    let mut state = state_with_active();
    let mut missing = listed("feat-PROJ-12_login");
    missing.status = WorktreeStatus::Missing;
    let mut invalid = listed("fix-b");
    invalid.status = WorktreeStatus::Invalid;
    deliver(&mut state, vec![missing, invalid, listed("feat-ok")]);
    let entries = candidates(&state);
    let context = |dir: &str| {
        entries
            .iter()
            .find(|e| e.candidate.key == EntryKey::Location(worktree(dir)))
            .map(|e| e.context.clone())
            .expect("listed")
    };
    assert!(
        context("feat-PROJ-12_login").ends_with(" · missing"),
        "{}",
        context("feat-PROJ-12_login")
    );
    assert!(
        context("feat-PROJ-12_login").starts_with("feat"),
        "the type tag comes first"
    );
    assert!(
        context("feat-PROJ-12_login").contains("PROJ-12"),
        "the ticket tag is shown"
    );
    assert!(
        context("fix-b").ends_with("invalid"),
        "an invalid worktree says so"
    );
    assert_eq!(
        context("feat-ok"),
        "feat",
        "a valid one shows its tags only"
    );
    assert_eq!(entries[0].context, "", "the project root has no tags");
}

#[test]
fn no_active_project_lists_no_locations() {
    assert!(
        candidates(&State::default()).is_empty(),
        "with no active project there is nothing to list"
    );
}

// ---- the reducer (T010) ----

fn toggled(focus: bool) -> Msg {
    Msg::Toggled {
        terminal_had_focus: focus,
    }
}

fn open(state: &mut State) {
    assert!(update(state, toggled(true)).is_none());
    assert!(state.quick_switch.open.is_some(), "the chord opens it");
}

fn highlighted(state: &State) -> Option<EntryKey> {
    let l = list(state);
    l.highlighted
        .map(|at| l.entries[l.ranked[at].index].candidate.key.clone())
}

fn loc(dir: &str) -> EntryKey {
    EntryKey::Location(worktree(dir))
}

#[test]
fn opening_records_the_start_and_highlights_the_first_entry() {
    let mut state = three();
    let row = micold_client::features::sidebar::RowRef::Location(worktree("feat-a"));
    state.sidebar.keyboard_row = Some(row.clone());
    update(&mut state, toggled(false));
    let open = state.quick_switch.open.as_ref().expect("open");
    assert_eq!(open.query, "", "it opens with an empty query");
    assert!(!open.terminal_had_focus, "the opener's focus is recorded");
    assert_eq!(
        open.keyboard_row_before,
        Some(row),
        "the sidebar's keyboard row is recorded for FR-009"
    );
    assert_eq!(
        highlighted(&state),
        Some(EntryKey::Location(SessionLocation::Default)),
        "the first entry is highlighted"
    );
    assert_eq!(
        open.highlighted,
        highlighted(&state),
        "the stored highlight is the displayed one"
    );
}

#[test]
fn it_does_not_open_over_settings_or_another_surface() {
    let mut state = three();
    state.settings.settings_draft = Some(Default::default());
    update(&mut state, toggled(true));
    assert!(state.quick_switch.open.is_none(), "settings is open");
    state.settings.settings_draft = None;
    state.help.about_open = true;
    update(&mut state, toggled(true));
    assert!(state.quick_switch.open.is_none(), "a dialog is open");
    state.help.about_open = false;
    state.update(micold_client::app::Message::Project(
        micold_client::features::project::Msg::SwitcherToggled,
    ));
    update(&mut state, toggled(true));
    assert!(
        state.quick_switch.open.is_none(),
        "the project panel is open"
    );
}

#[test]
fn toggling_or_dismissing_closes_and_restores_the_keyboard_row() {
    for close in [toggled(true), Msg::Dismissed] {
        let mut state = three();
        let id = session_at(&mut state, worktree("feat-a"));
        state.session.active = Some(id);
        let row = Some(micold_client::features::sidebar::RowRef::Session(id));
        state.sidebar.keyboard_row = row.clone();
        open(&mut state);
        state.sidebar.keyboard_row = None;
        let (active, stop) = (state.session.active, state.sidebar.nav_stop.clone());
        assert!(
            update(&mut state, close).is_none(),
            "closing resolves no choice"
        );
        assert!(state.quick_switch.open.is_none(), "closing closes");
        assert_eq!(state.sidebar.keyboard_row, row, "FR-009");
        assert_eq!(
            (state.session.active, state.sidebar.nav_stop.clone()),
            (active, stop),
            "closing without a choice moves nothing"
        );
    }
}

/// Display names `Login`, `Lobby`, `Logout` (the type prefix is not part of a row's name).
fn lo_state() -> State {
    let mut state = state_with_active();
    deliver(
        &mut state,
        vec![
            listed("feat-login"),
            listed("fix-lobby"),
            listed("feat-logout"),
        ],
    );
    state
}

#[test]
fn typing_reseats_the_highlight_and_moving_stops_at_the_ends() {
    let mut state = lo_state();
    open(&mut state);
    update(&mut state, Msg::Moved(Dir::Next));
    update(&mut state, Msg::QueryChanged("log".into()));
    let l = list(&state);
    assert_eq!(l.entries.len(), 4);
    assert_eq!(l.ranked.len(), 2, "Default and Lobby are filtered out");
    assert_eq!(highlighted(&state), Some(loc("feat-login")));
    update(&mut state, Msg::Moved(Dir::Previous));
    assert_eq!(
        highlighted(&state),
        Some(loc("feat-login")),
        "stops at the top"
    );
    update(&mut state, Msg::Moved(Dir::Next));
    assert_eq!(highlighted(&state), Some(loc("feat-logout")));
    update(&mut state, Msg::Moved(Dir::Next));
    assert_eq!(
        highlighted(&state),
        Some(loc("feat-logout")),
        "stops at the end"
    );
}

#[test]
fn a_query_change_re_seats_the_highlight_on_the_first_match() {
    // FR-005: highlight a later entry, then type a query that still lists it but not first.
    let mut state = lo_state();
    open(&mut state);
    for _ in 0..3 {
        update(&mut state, Msg::Moved(Dir::Next));
    }
    assert_eq!(
        highlighted(&state),
        Some(loc("feat-logout")),
        "the highlight moved to the last entry"
    );
    update(&mut state, Msg::QueryChanged("lo".into()));
    let l = list(&state);
    let names: Vec<&str> = l
        .ranked
        .iter()
        .map(|r| l.entries[r.index].candidate.name.as_str())
        .collect();
    assert!(names.contains(&"Logout"), "still listed: {names:?}");
    assert_ne!(names[0], "Logout", "but not first: {names:?}");
    assert_eq!(
        highlighted(&state),
        Some(l.entries[l.ranked[0].index].candidate.key.clone()),
        "FR-005: the first match is highlighted after the query changes"
    );
}

#[test]
fn choosing_a_location_with_a_session_shows_it_else_stops_there() {
    let mut state = three();
    let id = session_at(&mut state, worktree("feat-a"));
    open(&mut state);
    let choice = update(&mut state, Msg::Chosen(loc("feat-a")));
    assert_eq!(
        choice,
        Some(Choice {
            step: Step::Session(id),
            terminal_had_focus: true,
            close_overview_first: false,
        })
    );
    assert!(state.quick_switch.open.is_none());
    open(&mut state);
    let choice = update(&mut state, Msg::Chosen(loc("feat-c"))).expect("resolves");
    assert_eq!(choice.step, Step::Stop(worktree("feat-c")));
    assert!(!choice.close_overview_first);
    state.overview.open = true;
    open(&mut state);
    let choice = update(&mut state, Msg::Chosen(loc("feat-c"))).expect("resolves");
    assert!(choice.close_overview_first, "a stop with the Overview open");
}

#[test]
fn choosing_what_is_not_listed_keeps_it_open() {
    let mut state = lo_state();
    open(&mut state);
    update(&mut state, Msg::QueryChanged("lo".into()));
    for key in [
        loc("gone"),
        EntryKey::Session(micold_core::session::SessionId::new()),
        EntryKey::Project("/other".into()),
    ] {
        assert!(update(&mut state, Msg::Chosen(key)).is_none());
        let open = state.quick_switch.open.as_ref().expect("stays open");
        assert_eq!(open.query, "lo", "FR-013");
    }
}

#[test]
fn a_list_change_keeps_the_query_and_the_highlighted_key() {
    let mut state = lo_state();
    open(&mut state);
    update(&mut state, Msg::QueryChanged("log".into()));
    update(&mut state, Msg::Moved(Dir::Next));
    assert_eq!(highlighted(&state), Some(loc("feat-logout")));
    let all = |dirs: &[&str]| dirs.iter().map(|d| listed(d)).collect::<Vec<_>>();
    deliver(
        &mut state,
        all(&["feat-login", "fix-lobby", "feat-logout", "feat-logbook"]),
    );
    assert_eq!(
        state.quick_switch.open.as_ref().map(|o| o.query.as_str()),
        Some("log")
    );
    assert_eq!(highlighted(&state), Some(loc("feat-logout")));
    deliver(
        &mut state,
        all(&["feat-login", "fix-lobby", "feat-logbook"]),
    );
    assert_eq!(
        highlighted(&state),
        Some(loc("feat-login")),
        "the first entry when its key leaves"
    );
}

#[test]
fn five_hundred_worktrees_build_and_rank_their_list_within_a_generous_bound() {
    // SC-003 through the app's own entry points (`candidates` and `list`), not `filter` alone.
    // The bound is ten times the spec's 100 ms and the best of five runs, so a loaded runner does
    // not flake it while a quadratic regression still fails it.
    let mut state = state_with_active();
    deliver(
        &mut state,
        (0..500)
            .map(|i| listed(&format!("feat-PROJ-{i}-login-{}", i * 7)))
            .collect(),
    );
    open(&mut state);
    for text in ["", "lo", "logpg", "proj-4", "zzzzqq"] {
        update(&mut state, Msg::QueryChanged(text.into()));
        let (took, kept) = (0..5)
            .map(|_| {
                let started = std::time::Instant::now();
                let kept = list(&state).ranked.len();
                (started.elapsed(), kept)
            })
            .min_by_key(|(took, _)| *took)
            .expect("five runs");
        assert!(
            took < std::time::Duration::from_secs(1),
            "SC-003: listing 501 entries for {text:?} took {took:?} ({kept} kept)"
        );
    }
}

#[test]
fn the_empty_list_says_why() {
    let mut state = three();
    open(&mut state);
    assert_eq!(list(&state).empty, None);
    update(&mut state, Msg::QueryChanged("zzzzqq".into()));
    let l = list(&state);
    assert_eq!(l.empty, Some(Empty::NoMatches));
    assert_eq!(l.highlighted, None);
    assert!(
        update(&mut state, Msg::Chosen(loc("feat-a"))).is_some(),
        "chosen by key, resolved from the fresh list"
    );
    let mut none = State::default();
    open_unchecked(&mut none);
    assert_eq!(list(&none).empty, Some(Empty::NothingToSwitchTo));
}

/// Open the switcher on a state with nothing to list.
fn open_unchecked(state: &mut State) {
    update(state, toggled(false));
    assert!(state.quick_switch.open.is_some());
}

#[test]
fn fifty_locations_are_each_reachable_in_a_short_query() {
    let mut state = state_with_active();
    deliver(
        &mut state,
        (0..50).map(|i| listed(&format!("feat-w{i:02}"))).collect(),
    );
    let entries = candidates(&state);
    assert_eq!(entries.len(), 51, "no sessions: locations only");
    open(&mut state);
    for entry in &entries {
        let name = &entry.candidate.name;
        let query: String = name
            .chars()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        update(&mut state, Msg::QueryChanged(query));
        let l = list(&state);
        let pos = l
            .ranked
            .iter()
            .filter(|r| l.entries[r.index].candidate.selectable)
            .position(|r| l.entries[r.index].candidate.key == entry.candidate.key);
        assert!(matches!(pos, Some(p) if p < 3), "{name}: {pos:?}");
    }
}

// ---- applying a choice (T011) ----

/// Choose `location` in the open switcher and apply the result as the binary does.
fn choose(state: &mut State, location: SessionLocation) -> Choice {
    let choice = update(state, Msg::Chosen(EntryKey::Location(location))).expect("resolves");
    if choice.close_overview_first {
        state.update(Message::Overview(
            micold_client::features::overview::Msg::Closed,
        ));
    }
    press(state, choice.step.clone(), choice.terminal_had_focus);
    choice
}

#[test]
fn choosing_a_location_with_a_session_shows_it_and_hands_the_keyboard_back() {
    let (mut state, _, a) = arranged();
    let before = session_count(&state);
    update(&mut state, toggled(true));
    update(&mut state, Msg::QueryChanged("a".into()));
    choose(&mut state, worktree("feat-a"));
    assert_eq!(state.session.active, Some(a));
    assert_eq!(
        session_count(&state),
        before,
        "FR-011: opening, filtering and choosing a session starts and stops none"
    );
    assert!(state.sidebar.pending_reveal_scroll);
    assert!(
        !state.session.terminal_released,
        "the terminal had it (FR-010)"
    );
    assert_eq!(state.sidebar.keyboard_row, None);
}

#[test]
fn choosing_without_terminal_focus_puts_the_keyboard_on_the_chosen_row() {
    let (mut state, _, a) = arranged();
    update(&mut state, toggled(false));
    choose(&mut state, worktree("feat-a"));
    assert_eq!(state.session.active, Some(a), "the chosen session is shown");
    assert_eq!(
        state.sidebar.keyboard_row,
        Some(micold_client::features::sidebar::RowRef::Session(a)),
        "without terminal focus the keyboard lands on the chosen session's row"
    );
}

#[test]
fn choosing_a_location_without_a_session_starts_none_and_stops_there() {
    let (mut state, default, _) = arranged();
    let before = session_count(&state);
    update(&mut state, toggled(true));
    update(&mut state, Msg::QueryChanged("b".into()));
    choose(&mut state, worktree("feat-b"));
    assert_eq!(state.sidebar.nav_stop, Some(worktree("feat-b")));
    assert_eq!(
        state.sidebar.keyboard_row,
        Some(micold_client::features::sidebar::RowRef::Location(
            worktree("feat-b")
        ))
    );
    assert_eq!(state.session.active, Some(default));
    assert_eq!(session_count(&state), before, "SC-005");
}

#[test]
fn a_stop_closes_the_overview_first_and_reveals_its_row() {
    let (mut state, _, _) = arranged();
    state.overview.open = true;
    update(&mut state, toggled(true));
    let before = session_count(&state);
    let choice = choose(&mut state, worktree("feat-b"));
    assert!(choice.close_overview_first);
    assert!(!state.overview.open);
    assert!(state.sidebar.pending_reveal_scroll);
    assert_eq!(state.sidebar.nav_stop, Some(worktree("feat-b")));
    assert_eq!(session_count(&state), before);
}

// ---- the registry-bound rules (T010 leftovers, T012) ----

#[test]
fn while_open_no_navigation_chord_moves_anything() {
    // FR-012: the switcher is a surface, so `navigation::update`'s gate stops every #803 chord.
    let mut state = three();
    state.sidebar.nav_stop = Some(worktree("feat-a"));
    open(&mut state);
    for (action, _) in micold_client::keymap::NAV_CHORDS {
        assert_eq!(
            micold_client::features::navigation::update(&state, action),
            Step::None,
            "{action:?} moved while the switcher was open"
        );
    }
}

/// Press `key` (with `modifiers`) in the open switcher's dialog, as the runtime would: the dialog
/// is built from the state, its field focused, and the event run through the interface.
fn press_in_dialog(
    state: &State,
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> (Vec<Message>, iced::event::Status) {
    use iced::advanced::{clipboard, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    let mut renderer = support::layout::renderer();
    let outcome = micold_core::env_include::EnvIncludeOutcome::MissingScript;
    let element = micold_client::ui::quick_switcher::dialog(
        state,
        micold_core::theme::ColorScheme::Dark,
        &outcome,
    )
    .expect("the switcher is open");
    let mut ui = UserInterface::build(
        element,
        support::layout::WINDOW,
        Cache::default(),
        &mut renderer,
    );
    let mut focus: Box<dyn iced::advanced::widget::Operation<()>> =
        Box::new(iced::advanced::widget::operation::focusable::focus(
            micold_client::ui::quick_switcher::QUICK_SWITCHER_FIELD_ID.clone(),
        ));
    ui.operate(&renderer, focus.as_mut());
    let event = iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        location: iced::keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    });
    let mut messages = Vec::new();
    let (_, statuses) = ui.update(
        &[event],
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    (messages, statuses[0])
}

#[test]
fn tab_in_the_dialog_dismisses_it() {
    let mut state = three();
    open(&mut state);
    let (messages, _) = press_in_dialog(
        &state,
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
        iced::keyboard::Modifiers::empty(),
    );
    assert_eq!(messages, vec![Message::QuickSwitch(Msg::Dismissed)]);
}

/// What a left click at each height down the middle of the open dialog publishes, in order of
/// first appearance and without repeats: one message per distinct target, so the rows come out in
/// the order they are laid out.
fn clicks_down_the_dialog(state: &State) -> Vec<Message> {
    use iced::advanced::widget::Tree;
    use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
    let renderer = support::layout::renderer();
    let outcome = micold_core::env_include::EnvIncludeOutcome::MissingScript;
    let mut element = micold_client::ui::quick_switcher::dialog(
        state,
        micold_core::theme::ColorScheme::Dark,
        &outcome,
    )
    .expect("the switcher is open");
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(iced::Size::ZERO, support::layout::WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    support::layout::settle(
        &mut element,
        &mut tree,
        &node,
        &renderer,
        std::time::Instant::now(),
        0..support::layout::SETTLE_FRAMES,
        support::layout::WINDOW,
    );
    let bounds = Layout::new(&node).bounds();
    let mut seen: Vec<Message> = Vec::new();
    let mut y = bounds.y;
    while y < bounds.y + bounds.height {
        let at = iced::Point::new(bounds.center_x(), y);
        for event in [
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ] {
            let mut messages: Vec<Message> = Vec::new();
            let mut shell = Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(at),
                &renderer,
                &mut clipboard::Null,
                &mut shell,
                &iced::Rectangle::with_size(support::layout::WINDOW),
            );
            for m in messages {
                if matches!(m, Message::QuickSwitch(Msg::Chosen(_))) && !seen.contains(&m) {
                    seen.push(m);
                }
            }
        }
        y += 2.0;
    }
    seen
}

fn named_key(named: iced::keyboard::key::Named) -> iced::keyboard::Key {
    iced::keyboard::Key::Named(named)
}

#[test]
fn enter_up_and_down_in_the_dialog_choose_and_move() {
    use iced::keyboard::key::Named;
    let mut state = three();
    open(&mut state);
    let none = iced::keyboard::Modifiers::empty();
    let (messages, _) = press_in_dialog(&state, named_key(Named::Enter), none);
    assert_eq!(
        messages,
        vec![Message::QuickSwitch(Msg::Chosen(EntryKey::Location(
            SessionLocation::Default
        )))],
        "Enter chooses the highlighted first row"
    );
    // Up and Down at an end publish nothing, so press them from a middle row.
    update(&mut state, Msg::Moved(Dir::Next));
    let (messages, _) = press_in_dialog(&state, named_key(Named::ArrowUp), none);
    assert_eq!(
        messages,
        vec![Message::QuickSwitch(Msg::Moved(Dir::Previous))],
        "Up moves the highlight to the previous row"
    );
    let (messages, _) = press_in_dialog(&state, named_key(Named::ArrowDown), none);
    assert_eq!(
        messages,
        vec![Message::QuickSwitch(Msg::Moved(Dir::Next))],
        "Down moves the highlight to the next row"
    );
}

#[test]
fn clicking_the_second_row_chooses_that_row() {
    let mut state = three();
    open(&mut state);
    let view = micold_client::ui::quick_switcher::view_of(&state);
    let expected: Vec<Message> = view
        .keys
        .iter()
        .cloned()
        .map(|k| Message::QuickSwitch(Msg::Chosen(k)))
        .collect();
    assert!(expected.len() >= 3, "the fixture lists several rows");
    let clicked = clicks_down_the_dialog(&state);
    assert_eq!(
        clicked.get(1),
        Some(&expected[1]),
        "a click on the second row chooses the second row's key (US1-8): {clicked:?}"
    );
}

#[test]
fn the_dialog_marks_the_current_row_and_disables_an_unavailable_project() {
    use micold_client::ui::quick_switcher::view_of;
    let mut state = with_projects();
    open(&mut state);
    update(&mut state, Msg::QueryChanged("beta".into()));
    let view = view_of(&state);
    let beta = EntryKey::Project(project_at("/beta"));
    assert_eq!(
        view.keys,
        vec![beta],
        "only the unavailable project matches"
    );
    assert!(
        !view.rows[0].enabled,
        "an unavailable project row is disabled (US3-3)"
    );
    assert_eq!(
        view.rows[0].details.as_ref().map(|(text, _)| text.as_str()),
        Some("Project \u{b7} 0 running \u{b7} 0 unread \u{b7} unavailable"),
        "its second line says Project, the counts and unavailable"
    );
    update(&mut state, Msg::QueryChanged("alpha".into()));
    assert!(
        view_of(&state).rows[0].enabled,
        "an available project row is enabled"
    );

    let (mut state, _, helper, _) = with_sessions();
    state.session.active = Some(helper);
    open(&mut state);
    let view = view_of(&state);
    let current = view.current.expect("the active session's row is current");
    assert_eq!(view.keys[current], session_key(helper), "FR-006");
}

#[test]
fn the_dialog_says_why_it_is_empty() {
    use micold_client::ui::quick_switcher::view_of;
    let mut state = three();
    open(&mut state);
    update(&mut state, Msg::QueryChanged("zzzzqq".into()));
    assert_eq!(view_of(&state).empty_message, "No matches");
    let mut none = State::default();
    open_unchecked(&mut none);
    assert_eq!(view_of(&none).empty_message, "Nothing to switch to");
}

#[test]
fn the_chord_pressed_in_the_field_still_reaches_the_toggle() {
    // Research R8: the field leaves the press uncaptured so `app_chord_listener` hears it.
    let mut state = three();
    open(&mut state);
    #[cfg(target_os = "macos")]
    let primary = iced::keyboard::Modifiers::LOGO;
    #[cfg(not(target_os = "macos"))]
    let primary = iced::keyboard::Modifiers::CTRL;
    let (messages, status) = press_in_dialog(
        &state,
        iced::keyboard::Key::Character("p".into()),
        primary | iced::keyboard::Modifiers::SHIFT,
    );
    // One named outcome: the field leaves the press uncaptured and publishes nothing, so the
    // app-level `app_chord_listener` hears it and toggles (its own `app_chord_tests` pin that half).
    assert_eq!(
        status,
        iced::event::Status::Ignored,
        "the field must not swallow the chord: {messages:?}"
    );
    assert!(
        messages.is_empty(),
        "the dialog must not also map the chord (it would toggle twice): {messages:?}"
    );
}

// ---- sessions (M2: T022, T023) ----

/// `Default` with its main session and one helper, `feat-a` with one session, `feat-b` with none.
fn with_sessions() -> (State, SessionId, SessionId, SessionId) {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);
    let main = session_at(&mut state, SessionLocation::Default);
    let helper = session_at(&mut state, SessionLocation::Default);
    let a = session_at(&mut state, worktree("feat-a"));
    state.session.active = Some(main);
    (state, main, helper, a)
}

use micold_core::session::SessionId;

fn session_key(id: SessionId) -> EntryKey {
    EntryKey::Session(id)
}

#[test]
fn sessions_follow_their_location_in_session_order() {
    let (state, main, helper, a) = with_sessions();
    let entries = candidates(&state);
    let order: Vec<(Kind, Option<usize>)> = entries
        .iter()
        .map(|e| (e.kind, e.candidate.parent))
        .collect();
    assert_eq!(
        order,
        vec![
            (Kind::Location, None),
            (Kind::Session, Some(0)),
            (Kind::Session, Some(0)),
            (Kind::Location, None),
            (Kind::Session, Some(3)),
            (Kind::Location, None),
        ],
        "Default, its two sessions, feat-a, its session, feat-b"
    );
    let ids: Vec<EntryKey> = entries
        .iter()
        .filter(|e| e.kind == Kind::Session)
        .map(|e| e.candidate.key.clone())
        .collect();
    assert_eq!(
        ids,
        vec![session_key(main), session_key(helper), session_key(a)],
        "the main session first, then its helper"
    );
}

#[test]
fn a_session_row_is_named_by_its_label_in_its_locations_name() {
    let (state, _, _, _) = with_sessions();
    let entries = candidates(&state);
    for (at, entry) in entries.iter().enumerate() {
        if entry.kind != Kind::Session {
            continue;
        }
        let parent = &entries[entry.candidate.parent.expect("a session has a parent")];
        assert_eq!(entry.context, parent.candidate.name, "row {at}");
        assert_eq!(entry.candidate.name, "New session", "the sidebar's label");
        assert!(entry.candidate.selectable);
    }
}

#[test]
fn only_the_active_session_is_current() {
    let (mut state, main, helper, _) = with_sessions();
    state.session.active = Some(helper);
    let entries = candidates(&state);
    let current: Vec<EntryKey> = entries
        .iter()
        .filter(|e| e.kind == Kind::Session && e.current)
        .map(|e| e.candidate.key.clone())
        .collect();
    assert_eq!(current, vec![session_key(helper)]);
    assert!(!entries
        .iter()
        .any(|e| e.candidate.key == session_key(main) && e.current));
}

#[test]
fn a_closed_session_is_absent() {
    let (mut state, _, helper, _) = with_sessions();
    state
        .workspace
        .sessions
        .get_mut(&std::path::PathBuf::from("/repo"))
        .expect("sessions")
        .retain(|s| s.id != helper);
    assert!(!candidates(&state)
        .iter()
        .any(|e| e.candidate.key == session_key(helper)));
}

#[test]
fn the_sessions_of_a_collapsed_location_are_still_listed() {
    let (state, _, _, a) = with_sessions();
    assert!(
        !state.sidebar.expanded.contains("feat-a"),
        "feat-a's row is not opened"
    );
    assert!(
        candidates(&state)
            .iter()
            .any(|e| e.candidate.key == session_key(a)),
        "a collapsed location's session is still an entry"
    );
}

/// Whether the sidebar draws the Default row's session sub-rows now.
fn default_row_shows_sessions(state: &State) -> bool {
    state.sidebar_entries().iter().any(|e| match e {
        SidebarEntry::Default(node) => node.sessions_shown(),
        _ => false,
    })
}

#[test]
fn choosing_a_session_selects_it_and_reveals_its_sub_row() {
    let (mut state, _, helper, _) = with_sessions();
    open(&mut state);
    let choice = update(&mut state, Msg::Chosen(session_key(helper))).expect("resolves");
    assert_eq!(
        choice.step,
        Step::Session(helper),
        "a session choice resolves to showing that session"
    );
    assert!(
        !choice.close_overview_first,
        "no Overview is open to close first"
    );
    assert!(state.quick_switch.open.is_none(), "choosing closes it");
    press(&mut state, choice.step, choice.terminal_had_focus);
    assert_eq!(
        state.session.active,
        Some(helper),
        "the chosen session is shown"
    );
    assert!(state.sidebar.pending_reveal_scroll, "scrolled into view");
    assert!(default_row_shows_sessions(&state), "its row is opened");
}

#[test]
fn choosing_a_session_without_terminal_focus_leaves_the_keyboard_on_its_sub_row() {
    let (mut state, _, _, a) = with_sessions();
    update(&mut state, toggled(false));
    let choice = update(&mut state, Msg::Chosen(session_key(a))).expect("resolves");
    press(&mut state, choice.step, choice.terminal_had_focus);
    assert_eq!(state.session.active, Some(a), "the chosen session is shown");
    assert_eq!(
        state.sidebar.keyboard_row,
        Some(micold_client::features::sidebar::RowRef::Session(a)),
        "without terminal focus the keyboard lands on the chosen session's row"
    );
}

#[test]
fn a_locations_name_then_down_reaches_its_second_session() {
    let (mut state, _, helper, _) = with_sessions();
    open(&mut state);
    update(&mut state, Msg::QueryChanged("default".into()));
    assert_eq!(
        highlighted(&state),
        Some(EntryKey::Location(SessionLocation::Default)),
        "a location's name matches its own entry first"
    );
    update(&mut state, Msg::Moved(Dir::Next));
    update(&mut state, Msg::Moved(Dir::Next));
    assert_eq!(
        highlighted(&state),
        Some(session_key(helper)),
        "two steps down reach the location's second session"
    );
}

#[test]
fn a_session_created_while_open_appears_and_keeps_query_and_highlight() {
    let (mut state, _, _, _) = with_sessions();
    open(&mut state);
    update(&mut state, Msg::QueryChanged("b".into()));
    let before = highlighted(&state);
    assert_eq!(before, Some(loc("feat-b")), "feat-b is highlighted");
    let made = session_at(&mut state, worktree("feat-b"));
    assert!(
        list(&state)
            .entries
            .iter()
            .any(|e| e.candidate.key == session_key(made)),
        "a session created while open appears"
    );
    let kept: Vec<EntryKey> = list(&state)
        .ranked
        .iter()
        .map(|r| list(&state).entries[r.index].candidate.key.clone())
        .collect();
    assert!(
        kept.contains(&session_key(made)),
        "feat-b matched: {kept:?}"
    );
    assert_eq!(
        state.quick_switch.open.as_ref().map(|o| o.query.as_str()),
        Some("b"),
        "the query is kept"
    );
    assert_eq!(highlighted(&state), before, "the highlight is kept");
}

#[test]
fn choosing_a_session_closed_meanwhile_changes_nothing() {
    let (mut state, _, helper, _) = with_sessions();
    open(&mut state);
    update(&mut state, Msg::Moved(Dir::Next));
    update(&mut state, Msg::Moved(Dir::Next));
    assert_eq!(highlighted(&state), Some(session_key(helper)));
    state
        .workspace
        .sessions
        .get_mut(&std::path::PathBuf::from("/repo"))
        .expect("sessions")
        .retain(|s| s.id != helper);
    assert!(update(&mut state, Msg::Chosen(session_key(helper))).is_none());
    assert!(state.quick_switch.open.is_some(), "FR-013: nothing changes");
}

// ---- projects (T027, T028) ----

use micold_core::project::Availability;

fn project_at(path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(path)
}

/// `/repo` active, then `/alpha`, `/beta` (unavailable) and `/gamma` known, in that order.
fn with_projects() -> State {
    let mut state = three();
    for (name, availability) in [
        ("alpha", Availability::Available),
        ("beta", Availability::Unavailable),
        ("gamma", Availability::Available),
    ] {
        state.workspace.projects.push(project(name, availability));
    }
    state
}

fn project_entries(state: &State) -> Vec<Entry> {
    candidates(state)
        .into_iter()
        .filter(|e| e.kind == Kind::Project)
        .collect()
}

#[test]
fn other_projects_follow_the_active_projects_entries_in_switcher_order() {
    let state = with_projects();
    let all = candidates(&state);
    let first_project = all.iter().position(|e| e.kind == Kind::Project).unwrap();
    assert!(
        all[..first_project].iter().all(|e| e.kind != Kind::Project),
        "no location or session follows a project"
    );
    assert!(
        all[first_project..].iter().all(|e| e.kind == Kind::Project),
        "projects come after every location and session"
    );
    let names: Vec<_> = all[first_project..]
        .iter()
        .map(|e| e.candidate.name.as_str())
        .collect();
    assert_eq!(names, ["alpha", "beta", "gamma"], "unavailable included");
    let keys: Vec<_> = all[first_project..]
        .iter()
        .map(|e| e.candidate.key.clone())
        .collect();
    assert_eq!(
        keys[0],
        EntryKey::Project(project_at("/alpha")),
        "the first project entry is the first other project"
    );
    assert!(
        all[first_project..]
            .iter()
            .all(|e| e.candidate.parent.is_none() && !e.current),
        "project entries are top level and none is current"
    );
}

#[test]
fn a_project_shows_its_counts_and_an_unavailable_one_is_not_selectable() {
    let state = with_projects();
    let entries = project_entries(&state);
    assert_eq!(
        entries[0].context, "0 running · 0 unread",
        "an available project shows its counts"
    );
    assert!(
        entries[0].candidate.selectable,
        "an available project is selectable"
    );
    assert_eq!(
        entries[1].context, "0 running · 0 unread · unavailable",
        "an unavailable project says so after its counts"
    );
    assert!(
        !entries[1].candidate.selectable,
        "an unavailable project is not selectable"
    );
    assert!(entries[2].candidate.selectable, "the next available one is");
}

#[test]
fn the_active_project_and_other_projects_contents_are_not_entries() {
    let mut state = with_projects();
    let other = project_at("/alpha");
    state
        .workspace
        .sessions
        .entry(other)
        .or_default()
        .push(foreign_session());
    let all = candidates(&state);
    assert!(
        !all.iter()
            .any(|e| e.candidate.key == EntryKey::Project(project_at("/repo"))),
        "the active project is not an entry"
    );
    assert_eq!(
        all.iter().filter(|e| e.kind == Kind::Session).count(),
        0,
        "another project's session is not listed (US3-5)"
    );
    assert_eq!(
        all.iter().filter(|e| e.kind == Kind::Project).count(),
        3,
        "the three other projects are listed"
    );
}

#[test]
fn on_the_welcome_screen_every_known_project_is_listed() {
    let mut state = with_projects();
    state.workspace.active = None;
    let entries = candidates(&state);
    assert_eq!(
        entries.len(),
        4,
        "every known project, the former active one too"
    );
    assert!(
        entries.iter().all(|e| e.kind == Kind::Project),
        "only projects are listed"
    );
    assert!(
        candidates(&State::default()).is_empty(),
        "no known project, no entry"
    );
}

#[test]
fn choosing_an_available_project_closes_and_returns_its_step() {
    let mut state = with_projects();
    open(&mut state);
    let choice = update(
        &mut state,
        Msg::Chosen(EntryKey::Project(project_at("/gamma"))),
    )
    .expect("an available project resolves");
    assert_eq!(
        choice.step,
        Step::Project(project_at("/gamma")),
        "an available project resolves to switching to it"
    );
    assert!(!choice.close_overview_first, "no Overview is open");
    assert!(state.quick_switch.open.is_none(), "choosing closes it");
}

#[test]
fn choosing_a_project_with_the_overview_open_leaves_it_to_the_step() {
    let mut state = with_projects();
    state.overview.open = true;
    open(&mut state);
    let choice = update(
        &mut state,
        Msg::Chosen(EntryKey::Project(project_at("/alpha"))),
    )
    .unwrap();
    assert!(
        !choice.close_overview_first,
        "a project switch leaves the Overview to the step"
    );
}

#[test]
fn an_unavailable_project_cannot_be_chosen_and_stays_open() {
    let mut state = with_projects();
    open(&mut state);
    assert!(
        update(
            &mut state,
            Msg::Chosen(EntryKey::Project(project_at("/beta")))
        )
        .is_none(),
        "an unavailable project resolves to nothing"
    );
    assert!(
        state.quick_switch.open.is_some(),
        "and the switcher stays open"
    );
}

#[test]
fn the_highlight_skips_an_unavailable_project() {
    let mut state = with_projects();
    open(&mut state);
    update(&mut state, Msg::QueryChanged("a".into()));
    // Walk to the end; the unavailable one is never highlighted on the way.
    for _ in 0..12 {
        update(&mut state, Msg::Moved(Dir::Next));
        assert_ne!(
            highlighted(&state),
            Some(EntryKey::Project(project_at("/beta"))),
            "the highlight never lands on an unavailable project"
        );
    }
    assert_eq!(
        highlighted(&state),
        Some(EntryKey::Project(project_at("/gamma"))),
        "stays on the last available project"
    );
}

#[test]
fn opening_with_only_an_unavailable_project_first_highlights_the_first_selectable() {
    let mut state = State::default();
    for (name, availability) in [
        ("beta", Availability::Unavailable),
        ("alpha", Availability::Available),
    ] {
        state.workspace.projects.push(project(name, availability));
    }
    open(&mut state);
    assert_eq!(
        highlighted(&state),
        Some(EntryKey::Project(project_at("/alpha"))),
        "opening highlights the first selectable entry, not the unavailable first one"
    );
}

#[test]
fn applying_a_project_choice_activates_it_and_keeps_every_session_running() {
    let mut state = with_projects();
    let before = session_at(&mut state, SessionLocation::Default);
    let remembered = foreign_session();
    let remembered_id = remembered.id;
    state
        .workspace
        .sessions
        .entry(project_at("/gamma"))
        .or_default()
        .push(remembered);
    state
        .workspace
        .foreground_by_project
        .insert(project_at("/gamma"), remembered_id);
    let count = session_count(&state);
    open(&mut state);
    let choice = update(
        &mut state,
        Msg::Chosen(EntryKey::Project(project_at("/gamma"))),
    )
    .unwrap();
    // What the shell does with `Reopened`.
    let _ = state
        .switch_active(&project_at("/gamma"))
        .expect("the switch is accepted");
    state.finish_navigation(&choice.step, choice.terminal_had_focus);
    assert_eq!(
        state.workspace.active,
        Some(project_at("/gamma")),
        "the chosen project is active"
    );
    assert_eq!(state.session.active, Some(remembered_id), "restored");
    assert_eq!(session_count(&state), count, "SC-005");
    assert!(
        state.workspace.sessions[&project_at("/repo")]
            .iter()
            .any(|s| s.id == before),
        "the previous project keeps its session running"
    );
}
