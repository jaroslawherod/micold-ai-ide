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
    arranged, deliver, listed, press, run_group, session_at, session_count, state_with_active,
    worktree,
};

fn keys(entries: &[Entry]) -> Vec<SessionLocation> {
    entries
        .iter()
        .map(|e| match &e.candidate.key {
            EntryKey::Location(l) => l.clone(),
            other => panic!("only locations are listed for now, got {other:?}"),
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
    assert_eq!(keys(&entries), location_order(&state));
    assert_eq!(
        entries[0].candidate.key,
        EntryKey::Location(SessionLocation::Default)
    );
    assert!(entries.iter().all(|e| e.candidate.selectable
        && e.candidate.parent.is_none()
        && e.kind == Kind::Location));
}

#[test]
fn names_are_default_or_the_rows_display_name() {
    let state = three();
    let entries = candidates(&state);
    assert_eq!(entries[0].candidate.name, "Default");
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
    assert_eq!(names, shown);
    assert!(names.iter().all(|n| !n.is_empty()));
}

#[test]
fn a_hidden_worktree_is_absent_unless_it_is_the_current_location() {
    let mut state = three();
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Feat));
    assert!(!keys(&candidates(&state)).contains(&worktree("fix-b")));
    let id = session_at(&mut state, worktree("fix-b"));
    state.session.active = Some(id);
    let entries = candidates(&state);
    assert!(keys(&entries).contains(&worktree("fix-b")));
    let current: Vec<_> = entries.iter().filter(|e| e.current).collect();
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
    let current: Vec<_> = entries.iter().filter(|e| e.current).collect();
    assert_eq!(current.len(), 1);
    assert_eq!(
        current[0].candidate.key,
        EntryKey::Location(worktree("feat-c"))
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
    assert_eq!(keys(&candidates(&state)), location_order(&state));
    state.update(micold_client::app::Message::Runs(
        micold_client::features::runs::Msg::GroupToggled(id),
    ));
    assert_eq!(keys(&candidates(&state)), location_order(&state));
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
    assert!(context("feat-PROJ-12_login").starts_with("feat"));
    assert!(context("feat-PROJ-12_login").contains("PROJ-12"));
    assert!(context("fix-b").ends_with("invalid"));
    assert_eq!(context("feat-ok"), "feat");
    assert_eq!(entries[0].context, "", "the project root has no tags");
}

#[test]
fn no_active_project_lists_no_locations() {
    assert!(candidates(&State::default()).is_empty());
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
    assert_eq!(open.query, "");
    assert!(!open.terminal_had_focus);
    assert_eq!(open.keyboard_row_before, Some(row));
    assert_eq!(
        highlighted(&state),
        Some(EntryKey::Location(SessionLocation::Default))
    );
    assert_eq!(open.highlighted, highlighted(&state));
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
        assert!(update(&mut state, close).is_none());
        assert!(state.quick_switch.open.is_none());
        assert_eq!(state.sidebar.keyboard_row, row, "FR-009");
        assert_eq!(
            (state.session.active, state.sidebar.nav_stop.clone()),
            (active, stop)
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
fn two_states_are_independent() {
    let mut a = three();
    let b = three();
    open(&mut a);
    assert!(b.quick_switch.open.is_none());
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
    assert_eq!(entries.len(), 51);
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
    update(&mut state, toggled(true));
    choose(&mut state, worktree("feat-a"));
    assert_eq!(state.session.active, Some(a));
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
    assert_eq!(state.session.active, Some(a));
    assert_eq!(
        state.sidebar.keyboard_row,
        Some(micold_client::features::sidebar::RowRef::Session(a))
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

#[test]
fn opening_filtering_and_closing_start_and_stop_no_session() {
    let (mut state, _, _) = arranged();
    let before = session_count(&state);
    update(&mut state, toggled(true));
    update(&mut state, Msg::QueryChanged("a".into()));
    update(&mut state, Msg::Dismissed);
    assert_eq!(session_count(&state), before, "FR-011");
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

#[test]
fn the_chord_pressed_in_the_field_still_reaches_the_toggle() {
    // Research R8: either the field leaves the press uncaptured (so `app_chord_listener` hears
    // it) or the dialog maps it itself. Exactly one of the two must hold.
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
    let mapped = messages
        .iter()
        .any(|m| matches!(m, Message::QuickSwitch(Msg::Toggled { .. })));
    assert!(
        mapped || status == iced::event::Status::Ignored,
        "the field swallowed the chord: {messages:?}, {status:?}"
    );
}
