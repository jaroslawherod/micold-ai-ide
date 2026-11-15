//! Creating, renaming and deleting a user group from the sidebar (feature 1786, US1; contracts
//! C2, C6, C6a; test list A1 to A5, U30, U31).

use crate::support::sidebar_fixture::{deliver, listed, session_at, state_with_active, worktree};
use micold_client::app::{Message, State};
use micold_client::features::sidebar::{more_menu_items, MoreAction};
use micold_client::features::sidebar_group_actions::{update, Msg, NamingTarget, GROUP_MENU_ITEMS};
use micold_client::keymap::{is_new_group_chord, Key, Mods};
use micold_core::sidebar_layout::{LayoutItem, SidebarLayout, UserGroupId};
use std::path::PathBuf;

const PROJECT: &str = "/repo";

fn send(state: &mut State, msg: Msg) {
    state.update(Message::SidebarGroups(msg));
}

fn named(state: &mut State, name: &str) {
    send(state, Msg::New);
    send(state, Msg::TextChanged(name.into()));
    send(state, Msg::Confirmed);
}

fn group_names(state: &State) -> Vec<String> {
    state
        .sidebar_layout()
        .map(|layout| layout.groups.iter().map(|g| g.name.clone()).collect())
        .unwrap_or_default()
}

#[test]
fn new_group_opens_an_empty_naming_draft() {
    let mut state = state_with_active();
    update(&mut state, Msg::New);
    let draft = state
        .sidebar_groups
        .naming
        .as_ref()
        .expect("New group opens the naming dialog");
    assert_eq!(draft.target, NamingTarget::New);
    assert_eq!(draft.text, "");
    assert_eq!(draft.error, None);
}

#[test]
fn a_valid_name_adds_a_header_holding_no_worktree_and_saves() {
    let mut state = state_with_active();
    named(&mut state, "  Work  ");
    assert!(state.sidebar_groups.naming.is_none(), "the dialog closes");
    let layout = state.sidebar_layout().expect("the project has a layout");
    assert_eq!(layout.groups.len(), 1);
    assert_eq!(layout.groups[0].name, "Work", "the name is trimmed");
    assert!(
        layout.groups[0].items.is_empty(),
        "a new group holds no worktree"
    );
    assert!(
        state
            .sidebar
            .layouts_unsaved
            .contains(&PathBuf::from(PROJECT)),
        "the changed layout waits for the shell's save (R9)"
    );
}

#[test]
fn a_blank_name_is_refused_and_the_state_stays() {
    let mut state = state_with_active();
    named(&mut state, "   ");
    let draft = state
        .sidebar_groups
        .naming
        .as_ref()
        .expect("the dialog stays open");
    assert_eq!(draft.error.as_deref(), Some("Enter a name for the group."));
    assert!(group_names(&state).is_empty());
    assert!(state.sidebar.layouts_unsaved.is_empty());
}

#[test]
fn a_duplicate_name_is_refused_with_a_visible_message() {
    let mut state = state_with_active();
    named(&mut state, "Work");
    state.sidebar.layouts_unsaved.clear();
    named(&mut state, "work");
    let draft = state
        .sidebar_groups
        .naming
        .as_ref()
        .expect("the dialog stays open");
    assert_eq!(
        draft.error.as_deref(),
        Some("A group named \u{201c}work\u{201d} already exists.")
    );
    assert_eq!(group_names(&state), ["Work"]);
    assert!(state.sidebar.layouts_unsaved.is_empty());
}

#[test]
fn editing_the_name_clears_the_message_and_cancel_closes() {
    let mut state = state_with_active();
    named(&mut state, "");
    send(&mut state, Msg::TextChanged("W".into()));
    assert_eq!(
        state
            .sidebar_groups
            .naming
            .as_ref()
            .and_then(|d| d.error.clone()),
        None
    );
    send(&mut state, Msg::Cancelled);
    assert!(state.sidebar_groups.naming.is_none());
    assert!(group_names(&state).is_empty());
}

#[test]
fn no_project_open_offers_no_new_group() {
    let mut state = State::default();
    update(&mut state, Msg::New);
    assert!(state.sidebar_groups.naming.is_none());
    let item = more_menu_items(false, false)
        .into_iter()
        .find(|item| item.action == MoreAction::NewGroup)
        .expect("the more menu lists New group");
    assert!(!item.enabled, "with no project, New group is not offered");
}

#[test]
fn new_group_is_on_the_more_menu_with_no_worktree_needed() {
    let item = more_menu_items(true, false)
        .into_iter()
        .find(|item| item.action == MoreAction::NewGroup)
        .expect("the more menu lists New group");
    assert_eq!(item.label, "New group…");
    assert!(item.enabled);
    // No worktree is listed in this state, and the dialog still opens.
    let mut state = state_with_active();
    update(&mut state, Msg::New);
    assert!(state.sidebar_groups.naming.is_some());
}

#[test]
fn the_new_group_chord_is_primary_shift_g_and_on_the_shortcut_sheet() {
    let primary_shift = if cfg!(target_os = "macos") {
        Mods {
            shift: true,
            logo: true,
            ..Mods::NONE
        }
    } else {
        Mods {
            shift: true,
            ctrl: true,
            ..Mods::NONE
        }
    };
    assert!(is_new_group_chord(&Key::Char('g'), primary_shift));
    assert!(is_new_group_chord(&Key::Char('G'), primary_shift));
    assert!(!is_new_group_chord(&Key::Char('g'), Mods::NONE));
    let sheet = micold_client::features::changes::sheet_model(true, false);
    assert!(
        sheet
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .any(|(_, text)| *text == "New group"),
        "the shortcut sheet lists the chord"
    );
}

#[test]
fn the_empty_area_menu_opens_at_the_press_and_its_new_group_opens_the_dialog() {
    let mut state = state_with_active();
    send(&mut state, Msg::EmptyAreaMenuOpened((40, 300)));
    assert_eq!(state.sidebar_groups.empty_menu, Some((40, 300)));
    let roles = micold_core::tokens::roles(micold_core::theme::ColorScheme::Light);
    assert!(
        micold_client::ui::sidebar::empty_area_menu(&state, roles).is_some(),
        "an open empty-area menu is drawn"
    );
    // Its one item is New group…, which opens the naming dialog and closes the menu.
    send(&mut state, Msg::New);
    assert!(state.sidebar_groups.naming.is_some());
    assert_eq!(state.sidebar_groups.empty_menu, None);
}

#[test]
fn the_empty_area_menu_dismisses_and_offers_nothing_with_no_project() {
    let mut state = state_with_active();
    send(&mut state, Msg::EmptyAreaMenuOpened((40, 300)));
    send(&mut state, Msg::EmptyAreaMenuDismissed);
    assert_eq!(state.sidebar_groups.empty_menu, None);
    let mut none = State::default();
    send(&mut none, Msg::EmptyAreaMenuOpened((40, 300)));
    assert_eq!(none.sidebar_groups.empty_menu, None);
}

#[test]
fn new_group_over_an_open_dialog_leaves_that_dialog_and_its_draft_alone() {
    let mut state = state_with_active();
    send(&mut state, Msg::New);
    send(&mut state, Msg::TextChanged("Half typed".into()));
    // The chord again, while the naming dialog is up.
    send(&mut state, Msg::New);
    assert_eq!(
        state
            .sidebar_groups
            .naming
            .as_ref()
            .map(|d| d.text.as_str()),
        Some("Half typed"),
        "the open dialog keeps its draft"
    );
}

/// A project with worktrees `a` and `b` in a group "Work" (id 0) and `c` ungrouped, each with a
/// running session, and the layout already saved.
fn grouped() -> State {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("a"), listed("b"), listed("c")]);
    for dir in ["a", "b", "c"] {
        session_at(&mut state, worktree(dir));
    }
    let mut layout = SidebarLayout::default();
    let id = layout.create("Work").unwrap();
    layout.groups[0].items = vec![
        LayoutItem::Worktree("a".into()),
        LayoutItem::Worktree("b".into()),
    ];
    layout.ungrouped = vec![LayoutItem::Worktree("c".into())];
    assert_eq!(id, UserGroupId(0));
    state.sidebar_layout_loaded(PathBuf::from(PROJECT), layout);
    state
}

fn rename_to(state: &mut State, name: &str) {
    send(state, Msg::Rename(UserGroupId(0)));
    send(state, Msg::TextChanged(name.into()));
    send(state, Msg::Confirmed);
}

#[test]
fn rename_opens_a_draft_holding_the_current_name() {
    let mut state = grouped();
    send(&mut state, Msg::Rename(UserGroupId(0)));
    let draft = state
        .sidebar_groups
        .naming
        .as_ref()
        .expect("a rename draft");
    assert_eq!(draft.target, NamingTarget::Rename(UserGroupId(0)));
    assert_eq!(draft.text, "Work");
}

#[test]
fn renaming_a_group_changes_the_header_and_keeps_its_worktrees_and_saves() {
    let mut state = grouped();
    rename_to(&mut state, " Office ");
    assert!(state.sidebar_groups.naming.is_none(), "the dialog closes");
    let layout = state.sidebar_layout().unwrap();
    assert_eq!(layout.groups[0].name, "Office");
    assert_eq!(layout.groups[0].items.len(), 2, "its worktrees stay in it");
    assert!(state
        .sidebar
        .layouts_unsaved
        .contains(&PathBuf::from(PROJECT)));
}

#[test]
fn a_blank_or_duplicate_rename_is_refused_with_a_message_and_the_state_stays() {
    let mut state = grouped();
    named(&mut state, "Home");
    state.sidebar.layouts_unsaved.clear();
    rename_to(&mut state, "  ");
    assert_eq!(
        state
            .sidebar_groups
            .naming
            .as_ref()
            .and_then(|d| d.error.as_deref()),
        Some("Enter a name for the group.")
    );
    send(&mut state, Msg::TextChanged("home".into()));
    send(&mut state, Msg::Confirmed);
    assert_eq!(
        state
            .sidebar_groups
            .naming
            .as_ref()
            .and_then(|d| d.error.as_deref()),
        Some("A group named \u{201c}home\u{201d} already exists.")
    );
    assert_eq!(group_names(&state), ["Work", "Home"]);
    assert!(state.sidebar.layouts_unsaved.is_empty());
    // Keeping its own name is not a duplicate.
    send(&mut state, Msg::TextChanged("work".into()));
    send(&mut state, Msg::Confirmed);
    assert_eq!(group_names(&state), ["work", "Home"]);
}

#[test]
fn renaming_a_group_that_vanished_is_refused_with_its_reason() {
    let mut state = grouped();
    send(&mut state, Msg::Rename(UserGroupId(0)));
    let layout = state.sidebar_layout().cloned().unwrap();
    let mut gone = layout;
    gone.delete(UserGroupId(0)).unwrap();
    state.sidebar_layout_loaded(PathBuf::from(PROJECT), gone);
    send(&mut state, Msg::Confirmed);
    assert_eq!(
        state
            .sidebar_groups
            .naming
            .as_ref()
            .and_then(|d| d.error.as_deref()),
        Some("That group no longer exists.")
    );
}

#[test]
fn deleting_a_group_returns_its_worktrees_and_touches_no_worktree_or_session() {
    let mut state = grouped();
    let worktrees = state.worktree.worktrees.clone();
    let sessions = state.workspace.sessions.clone();
    send(&mut state, Msg::Delete(UserGroupId(0)));
    let layout = state.sidebar_layout().unwrap();
    assert!(layout.groups.is_empty(), "the header disappears");
    assert_eq!(
        layout.ungrouped,
        vec![
            LayoutItem::Worktree("c".into()),
            LayoutItem::Worktree("a".into()),
            LayoutItem::Worktree("b".into()),
        ]
    );
    assert_eq!(
        state.worktree.worktrees, worktrees,
        "SC-004: no worktree is deleted"
    );
    assert_eq!(
        state.workspace.sessions, sessions,
        "FR-003: no session is stopped"
    );
    assert!(state
        .sidebar
        .layouts_unsaved
        .contains(&PathBuf::from(PROJECT)));
}

#[test]
fn the_group_menu_offers_rename_and_delete_and_acts_on_its_group() {
    assert_eq!(GROUP_MENU_ITEMS, ["Rename", "Delete"]);
    let mut state = grouped();
    send(&mut state, Msg::MenuOpened(UserGroupId(0), (40, 60)));
    let roles = micold_core::tokens::roles(micold_core::theme::ColorScheme::Light);
    assert!(
        micold_client::ui::sidebar::group_menu(&state, roles).is_some(),
        "an open group menu is drawn, its items reachable by keyboard like any menu's"
    );
    assert!(micold_client::overlay::registry::open_popovers(&state)
        .iter()
        .any(|open| open.id().as_str() == "user_group_menu"));
    send(&mut state, Msg::Rename(UserGroupId(0)));
    assert!(
        state.sidebar_groups.group_menu.is_none(),
        "choosing an item closes the menu"
    );
    assert!(state.sidebar_groups.naming.is_some());
    send(&mut state, Msg::Cancelled);
    send(&mut state, Msg::MenuOpened(UserGroupId(0), (40, 60)));
    send(&mut state, Msg::Delete(UserGroupId(0)));
    assert!(state.sidebar_groups.group_menu.is_none());
    assert!(state.sidebar_layout().unwrap().groups.is_empty());
    send(&mut state, Msg::MenuOpened(UserGroupId(0), (40, 60)));
    send(&mut state, Msg::MenuDismissed);
    assert!(state.sidebar_groups.group_menu.is_none());
}

#[test]
fn the_group_menu_opens_only_for_a_group_the_project_has() {
    let mut state = grouped();
    send(&mut state, Msg::MenuOpened(UserGroupId(7), (40, 60)));
    assert!(state.sidebar_groups.group_menu.is_none());
    let mut none = State::default();
    send(&mut none, Msg::MenuOpened(UserGroupId(0), (40, 60)));
    assert!(none.sidebar_groups.group_menu.is_none());
}

#[test]
fn confirming_a_rename_without_a_change_writes_nothing() {
    let mut state = grouped();
    send(&mut state, Msg::Rename(UserGroupId(0)));
    send(&mut state, Msg::Confirmed);
    assert!(state.sidebar_groups.naming.is_none(), "the dialog closes");
    assert!(state.sidebar.layouts_unsaved.is_empty());
}

#[test]
fn rename_over_an_open_dialog_closes_the_menu_and_keeps_that_dialog() {
    let mut state = grouped();
    send(&mut state, Msg::New);
    send(&mut state, Msg::MenuOpened(UserGroupId(0), (40, 60)));
    send(&mut state, Msg::Rename(UserGroupId(0)));
    assert!(state.sidebar_groups.group_menu.is_none());
    assert_eq!(
        state.sidebar_groups.naming.as_ref().map(|d| &d.target),
        Some(&NamingTarget::New)
    );
}

// ---- Move to group from the menu (US2; contract C6; test list A6 to A9, U27) ----

use micold_client::features::overview::{MenuOrigin, Msg as OverviewMsg};
use micold_client::features::sidebar_group_actions::{move_menu_items, MoveTarget};
use micold_client::ui::overview::card_menu_items;

fn wt_item(dir: &str) -> LayoutItem {
    LayoutItem::Worktree(dir.into())
}

fn move_to(state: &mut State, dir: &str, target: MoveTarget) {
    send(state, Msg::Move(wt_item(dir), target));
}

fn items_of(state: &State, group: u64) -> Vec<LayoutItem> {
    state
        .sidebar_layout()
        .and_then(|layout| layout.group(UserGroupId(group)))
        .map(|group| group.items.clone())
        .unwrap_or_default()
}

fn ungrouped(state: &State) -> Vec<LayoutItem> {
    state
        .sidebar_layout()
        .map(|layout| layout.ungrouped.clone())
        .unwrap_or_default()
}

fn labels(state: &State, item: &LayoutItem) -> Vec<String> {
    move_menu_items(state, item)
        .into_iter()
        .map(|entry| entry.label)
        .collect()
}

#[test]
fn the_move_menu_lists_every_group_then_ungrouped_then_new_group() {
    let mut state = grouped();
    named(&mut state, "Later");
    let entries = move_menu_items(&state, &wt_item("c"));
    assert_eq!(
        entries
            .iter()
            .map(|e| (e.target.clone(), e.label.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (MoveTarget::Group(UserGroupId(0)), "Work"),
            (MoveTarget::Group(UserGroupId(1)), "Later"),
            (MoveTarget::Ungrouped, "Ungrouped"),
            (MoveTarget::NewGroup, "New group…"),
        ]
    );
}

#[test]
fn with_no_group_the_move_menu_offers_only_new_group() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("a")]);
    assert_eq!(labels(&state, &wt_item("a")), vec!["New group…"]);
}

#[test]
fn the_default_row_is_offered_no_move() {
    let state = grouped();
    assert!(move_menu_items(&state, &wt_item("")).is_empty());
}

#[test]
fn moving_a_worktree_to_a_group_puts_it_under_that_group_and_saves() {
    let mut state = grouped();
    move_to(&mut state, "c", MoveTarget::Group(UserGroupId(0)));
    assert_eq!(
        items_of(&state, 0),
        vec![wt_item("a"), wt_item("b"), wt_item("c")]
    );
    assert!(ungrouped(&state).is_empty());
    assert!(state
        .sidebar
        .layouts_unsaved
        .contains(&PathBuf::from(PROJECT)));
    let drawn = state.sidebar_list();
    assert_eq!(drawn.len(), 5, "Default, the header and its three rows");
}

#[test]
fn moving_to_ungrouped_takes_the_row_out_of_its_group() {
    let mut state = grouped();
    move_to(&mut state, "a", MoveTarget::Ungrouped);
    assert_eq!(items_of(&state, 0), vec![wt_item("b")]);
    assert_eq!(ungrouped(&state), vec![wt_item("c"), wt_item("a")]);
}

#[test]
fn moving_a_row_the_layout_never_held_still_lands_in_exactly_one_list() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("a"), listed("b")]);
    named(&mut state, "Work");
    move_to(&mut state, "b", MoveTarget::Group(UserGroupId(0)));
    assert_eq!(items_of(&state, 0), vec![wt_item("b")]);
    let layout = state.sidebar_layout().unwrap();
    assert!(
        !layout.ungrouped.contains(&wt_item("b")),
        "removed from the ungrouped list before it is inserted"
    );
}

#[test]
fn moving_an_unplaced_row_to_ungrouped_changes_nothing() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("a"), listed("b")]);
    named(&mut state, "Work");
    let before = state.sidebar_layout().cloned();
    state.sidebar.layouts_unsaved.clear();
    move_to(&mut state, "b", MoveTarget::Ungrouped);
    assert_eq!(state.sidebar_layout().cloned(), before);
    assert!(state.sidebar.layouts_unsaved.is_empty(), "nothing to save");
}

#[test]
fn a_move_of_a_vanished_row_or_group_changes_nothing() {
    let mut state = grouped();
    let before = state.sidebar_layout().cloned();
    move_to(&mut state, "gone", MoveTarget::Group(UserGroupId(0)));
    move_to(&mut state, "c", MoveTarget::Group(UserGroupId(9)));
    move_to(&mut state, "", MoveTarget::Group(UserGroupId(0)));
    assert_eq!(state.sidebar_layout().cloned(), before);
    assert!(state.sidebar.layouts_unsaved.is_empty());
}

#[test]
fn new_group_from_the_move_menu_creates_the_group_and_moves_the_row_into_it() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("a"), listed("b")]);
    move_to(&mut state, "b", MoveTarget::NewGroup);
    let draft = state
        .sidebar_groups
        .naming
        .as_ref()
        .expect("the dialog opens");
    assert_eq!(draft.target, NamingTarget::NewAndMove(wt_item("b")));
    assert!(
        state.sidebar_layout().is_none_or(|l| l.groups.is_empty()),
        "nothing yet"
    );
    send(&mut state, Msg::TextChanged("Fresh".into()));
    send(&mut state, Msg::Confirmed);
    assert!(state.sidebar_groups.naming.is_none());
    assert_eq!(group_names(&state), vec!["Fresh"]);
    assert_eq!(items_of(&state, 0), vec![wt_item("b")]);
}

#[test]
fn a_refused_name_keeps_the_move_pending_and_the_layout_unchanged() {
    let mut state = grouped();
    move_to(&mut state, "c", MoveTarget::NewGroup);
    send(&mut state, Msg::TextChanged("work".into()));
    send(&mut state, Msg::Confirmed);
    let draft = state.sidebar_groups.naming.as_ref().expect("stays open");
    assert!(draft.error.is_some());
    assert_eq!(draft.target, NamingTarget::NewAndMove(wt_item("c")));
    assert_eq!(group_names(&state), vec!["Work"]);
    assert_eq!(ungrouped(&state), vec![wt_item("c")]);
}

#[test]
fn a_run_group_row_moves_as_one_unit_with_its_runs() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("g-1"), listed("g-2"), listed("z")]);
    let group = crate::support::sidebar_fixture::run_group(1, "g", &["g-1", "g-2"]);
    let run = LayoutItem::Run(group.id);
    state.update(Message::Runs(
        micold_client::features::runs::Msg::GroupsChanged(vec![group]),
    ));
    named(&mut state, "Work");
    assert_eq!(
        labels(&state, &run),
        vec!["Work", "Ungrouped", "New group…"]
    );
    send(
        &mut state,
        Msg::Move(run.clone(), MoveTarget::Group(UserGroupId(0))),
    );
    assert_eq!(items_of(&state, 0), vec![run], "one item, not its runs");
    assert!(!ungrouped(&state).contains(&wt_item("g-1")));
}

#[test]
fn the_worktree_menu_offers_move_to_group_above_delete() {
    let state = grouped();
    let items = micold_client::ui::location_menu_items(&state, &worktree_location("c"));
    let drawn: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    for expected in ["Move to Work", "Move to Ungrouped", "Move to new group…"] {
        assert!(drawn.contains(&expected), "{expected} in {drawn:?}");
    }
    assert_eq!(drawn.last(), Some(&"Delete"), "Delete stays last");
}

fn worktree_location(dir: &str) -> micold_core::session::SessionLocation {
    micold_core::session::SessionLocation::Worktree(dir.to_string())
}

#[test]
fn the_default_rows_menu_has_no_move_to_group() {
    let state = grouped();
    let items = micold_client::ui::location_menu_items(
        &state,
        &micold_core::session::SessionLocation::Default,
    );
    assert!(items.iter().all(|i| !i.label.starts_with("Move to")));
}

#[test]
fn a_keyboard_user_moves_a_row_through_the_card_menu_with_no_pointer() {
    let mut state = grouped();
    state.overview.open = true;
    state.update(Message::Overview(OverviewMsg::CardMenuOpened {
        location: worktree_location("c"),
        anchor: (10, 10),
        origin: MenuOrigin::Keyboard,
    }));
    let items = card_menu_items(&state, &worktree_location("c"));
    let at = items
        .iter()
        .position(|item| item.label == "Move to Work")
        .expect("the card menu offers the move");
    // Down from the first entry, one at a time, as the arrow key does.
    for _ in 0..at {
        state.update(Message::Overview(OverviewMsg::CardMenuMoved {
            forward: true,
            items: items.len(),
        }));
    }
    let highlighted = state
        .overview
        .card_menu
        .as_ref()
        .and_then(|m| m.highlighted);
    assert_eq!(
        highlighted,
        Some(at),
        "the entry is reached with Down alone"
    );
    let chosen = items[at].message.clone().expect("a pressable entry");
    state.update(chosen.clone());
    let Message::Overview(OverviewMsg::CardMenuEntryChosen(inner)) = chosen else {
        panic!("a card menu entry is wrapped so the root closes the menu");
    };
    state.update(*inner);
    assert_eq!(
        items_of(&state, 0),
        vec![wt_item("a"), wt_item("b"), wt_item("c")]
    );
}

// ---- The header's count (M3 decision D7) ----

fn header_of(state: &State, id: u64) -> micold_client::features::sidebar::GroupHeaderNode {
    state
        .sidebar_list()
        .into_iter()
        .find_map(|entry| match entry {
            micold_client::features::sidebar::SidebarEntry::UserGroup(h)
                if h.id == UserGroupId(id) =>
            {
                Some(h)
            }
            _ => None,
        })
        .expect("the header is listed")
}

#[test]
fn a_header_counts_the_rows_shown_and_says_how_many_a_filter_hides() {
    use micold_client::features::sidebar::TagFilter;
    use micold_core::naming::ConventionalType;
    let mut state = state_with_active();
    deliver(
        &mut state,
        vec![listed("feat-a"), listed("fix-b"), listed("fix-c")],
    );
    named(&mut state, "Work");
    move_to(&mut state, "feat-a", MoveTarget::Group(UserGroupId(0)));
    move_to(&mut state, "fix-b", MoveTarget::Group(UserGroupId(0)));
    let all = header_of(&state, 0);
    assert_eq!(
        (all.count, all.hidden),
        (2, 0),
        "no filter: every member shown"
    );
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Feat));
    let some = header_of(&state, 0);
    assert_eq!(
        (some.count, some.hidden),
        (1, 1),
        "one member shown, one hidden"
    );
    state.sidebar.filters.clear();
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Docs));
    let none = header_of(&state, 0);
    assert_eq!(
        (none.count, none.hidden),
        (0, 2),
        "not empty: its members are hidden"
    );
    assert_eq!(items_of(&state, 0).len(), 2, "their stored places stay");
}

#[test]
fn a_group_holding_no_member_has_nothing_hidden() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("a")]);
    named(&mut state, "Work");
    let header = header_of(&state, 0);
    assert_eq!((header.count, header.hidden), (0, 0));
}

#[test]
fn a_member_whose_worktree_is_gone_is_not_counted_as_hidden() {
    let mut state = grouped();
    deliver(&mut state, vec![listed("a"), listed("c")]);
    let header = header_of(&state, 0);
    assert_eq!(
        (header.count, header.hidden),
        (1, 0),
        "`b` is gone, not filtered"
    );
}

#[test]
fn choosing_the_list_the_row_is_already_in_changes_and_saves_nothing() {
    let mut state = grouped();
    let before = state.sidebar_layout().cloned();
    move_to(&mut state, "a", MoveTarget::Group(UserGroupId(0)));
    move_to(&mut state, "c", MoveTarget::Ungrouped);
    assert_eq!(state.sidebar_layout().cloned(), before, "`a` stays first");
    assert!(state.sidebar.layouts_unsaved.is_empty());
}

#[test]
fn new_group_from_a_move_menu_leaves_an_open_dialog_and_its_draft_alone() {
    let mut state = grouped();
    send(&mut state, Msg::Rename(UserGroupId(0)));
    send(&mut state, Msg::TextChanged("Half typed".into()));
    move_to(&mut state, "c", MoveTarget::NewGroup);
    let draft = state.sidebar_groups.naming.as_ref().expect("still open");
    assert_eq!(draft.target, NamingTarget::Rename(UserGroupId(0)));
    assert_eq!(draft.text, "Half typed");
}
