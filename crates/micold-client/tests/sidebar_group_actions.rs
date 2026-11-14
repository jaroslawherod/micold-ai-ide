//! Creating a user group from the sidebar (feature 1786, US1; contracts C2, C6a; test list A1,
//! A4, A5, U31).

use crate::support::sidebar_fixture::state_with_active;
use micold_client::app::{Message, State};
use micold_client::features::sidebar::{more_menu_items, MoreAction};
use micold_client::features::sidebar_group_actions::{update, Msg, NamingTarget};
use micold_client::keymap::{is_new_group_chord, Key, Mods};
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
