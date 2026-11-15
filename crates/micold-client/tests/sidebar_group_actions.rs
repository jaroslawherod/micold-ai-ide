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
