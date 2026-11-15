//! The user's own sidebar groups: creating one, and later renaming, deleting and moving into one
//! (feature 1786, US1; contracts C2, C6a; research R6, R9).
//!
//! # What this feature owns
//!
//! The naming dialog's draft ([`NamingDraft`]): which operation it names, the text, and the
//! refusal shown under the field. The layout itself is the sidebar's (`sidebar.layouts`): this
//! feature computes the changed layout from the held one with the core's operations and returns it
//! as [`Outcome::SidebarLayoutChanged`](crate::features::Outcome::SidebarLayoutChanged); the root
//! keeps it and marks it unsaved, and the shell writes it to the store after the message (R9).
//!
//! # Messages
//!
//! `New`, `Rename`, `Delete`, `TextChanged`, `Confirmed`, `Cancelled`; the sidebar's empty-area menu
//! (`EmptyAreaMenuOpened`, `EmptyAreaMenuDismissed`), whose one item is **New group…** (C6a); the
//! group header's menu (`MenuOpened`, `MenuDismissed`), whose items are **Rename** and **Delete**
//! (C6); and `Move`, the **Move to group** entries of a worktree's or run group's menu
//! ([`move_menu_items`], FR-010, FR-016).

use crate::app::Message;
use crate::features::Outcome;
use crate::floating_surface;
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;
use micold_core::sidebar_layout::{
    DropTarget, LayoutError, LayoutItem, SidebarLayout, UserGroupId,
};

/// What this feature remembers: the open naming dialog, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The naming dialog's draft; its presence *is* the dialog being shown.
    pub naming: Option<NamingDraft>,
    /// The sidebar's empty-area menu, at the window point it was opened from; `None` while closed.
    pub empty_menu: Option<(u16, u16)>,
    /// A group header's menu; `None` while closed.
    pub group_menu: Option<GroupMenu>,
}

/// An open group header menu: which group it acts on and the window point it hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupMenu {
    pub group: UserGroupId,
    pub anchor: (u16, u16),
}

/// The group menu's items, in order (C6). Move up, Move down and Collapse join with the milestones
/// that add them.
pub const GROUP_MENU_ITEMS: [&str; 2] = ["Rename", "Delete"];

/// What the naming dialog names (data-model.md, *NamingDraft*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingTarget {
    /// A new, empty group.
    New,
    /// This group, renamed.
    Rename(UserGroupId),
    /// A new group that takes this row in as its first (**New group…** in a row's menu, US2-3).
    NewAndMove(LayoutItem),
}

/// Where a row's **Move to group** entry puts it (contract C6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveTarget {
    /// The end of this group.
    Group(UserGroupId),
    /// The end of the ungrouped list.
    Ungrouped,
    /// A group the user names next, which the row then joins.
    NewGroup,
}

/// One **Move to group** entry: where it goes and the words naming it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveEntry {
    pub target: MoveTarget,
    /// The group's name, `Ungrouped` or `New group…`.
    pub label: String,
}

impl MoveEntry {
    /// The entry as the menu draws it.
    pub fn menu_label(&self) -> String {
        match self.target {
            MoveTarget::NewGroup => "Move to new group…".to_owned(),
            _ => format!("Move to {}", self.label),
        }
    }
}

/// The **Move to group** entries for `item` (contract C6, FR-010): every group in order, then
/// **Ungrouped**, then **New group…**, which alone is offered while no group exists (US2-3). The
/// project root is never an item, so it is offered none (FR-015). Choosing the place the row is in
/// changes nothing.
pub fn move_menu_items(state: &crate::app::State, item: &LayoutItem) -> Vec<MoveEntry> {
    if matches!(item, LayoutItem::Worktree(dir) if dir.is_empty()) {
        return Vec::new();
    }
    let groups = state.sidebar_layout().map_or(&[][..], |l| &l.groups[..]);
    let mut entries: Vec<MoveEntry> = groups
        .iter()
        .map(|group| MoveEntry {
            target: MoveTarget::Group(group.id),
            label: group.name.clone(),
        })
        .collect();
    if !entries.is_empty() {
        entries.push(MoveEntry {
            target: MoveTarget::Ungrouped,
            label: "Ungrouped".to_owned(),
        });
    }
    entries.push(MoveEntry {
        target: MoveTarget::NewGroup,
        label: "New group…".to_owned(),
    });
    entries
}

/// The naming dialog's draft (contract C2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingDraft {
    pub target: NamingTarget,
    pub text: String,
    /// Why the last confirm was refused, shown under the field until the text changes.
    pub error: Option<String>,
}

/// Everything the user can do here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// **New group…** from the more menu, the sidebar's empty-area menu or the chord.
    New,
    /// **Rename** from a group's menu: opens the naming dialog holding its current name.
    Rename(UserGroupId),
    /// **Delete** from a group's menu: its worktrees return to the ungrouped list.
    Delete(UserGroupId),
    /// A group header was right-clicked at this window point (C6).
    MenuOpened(UserGroupId, (u16, u16)),
    /// The group menu was dismissed.
    MenuDismissed,
    /// The naming field was edited.
    TextChanged(String),
    /// The naming dialog's confirm.
    Confirmed,
    /// The naming dialog was dismissed.
    Cancelled,
    /// **Move to group** from a row's menu: the row goes to the target, or the naming dialog opens
    /// for the group it will join.
    Move(LayoutItem, MoveTarget),
    /// The sidebar's empty area was right-clicked at this window point (C6a).
    EmptyAreaMenuOpened((u16, u16)),
    /// The empty-area menu was dismissed.
    EmptyAreaMenuDismissed,
}

/// The message a refused name shows under the field (FR-004).
pub fn refusal(error: LayoutError, name: &str) -> String {
    match error {
        LayoutError::Blank => "Enter a name for the group.".to_owned(),
        LayoutError::Duplicate => {
            format!(
                "A group named \u{201c}{}\u{201d} already exists.",
                name.trim()
            )
        }
        LayoutError::UnknownGroup => "That group no longer exists.".to_owned(),
        LayoutError::UnknownItem => "That worktree is no longer listed.".to_owned(),
        LayoutError::IsDefault => "Default cannot be put in a group.".to_owned(),
    }
}

/// This feature's reducer (shape A).
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<Outcome> {
    match msg {
        Msg::New => new_started(state),
        Msg::Rename(id) => rename_started(state, id),
        Msg::Delete(id) => return deleted(state, id),
        Msg::MenuOpened(group, anchor) => {
            // Only a group the open project has.
            if !state
                .sidebar_layout()
                .is_some_and(|layout| layout.group(group).is_some())
            {
                return Vec::new();
            }
            state.sidebar_groups.group_menu = Some(GroupMenu { group, anchor });
            return crate::features::surface_opened(true, GroupContextMenu::ID);
        }
        Msg::MenuDismissed => state.sidebar_groups.group_menu = None,
        Msg::TextChanged(text) => {
            if let Some(draft) = &mut state.sidebar_groups.naming {
                draft.text = text;
                draft.error = None;
            }
        }
        Msg::Move(item, target) => return moved(state, item, target),
        Msg::Confirmed => return confirmed(state),
        Msg::Cancelled => state.sidebar_groups.naming = None,
        Msg::EmptyAreaMenuOpened(point) => {
            // With no project open there is nothing to group, so nothing to offer.
            if state.workspace.active.is_none() {
                return Vec::new();
            }
            state.sidebar_groups.empty_menu = Some(point);
            return crate::features::surface_opened(true, SidebarEmptyAreaMenu::ID);
        }
        Msg::EmptyAreaMenuDismissed => state.sidebar_groups.empty_menu = None,
    }
    Vec::new()
}

/// **New group…** opens an empty draft; with no project open there is nothing to group. Not over
/// an open dialog (the chord is heard while one is up): opening this one would close it, and its
/// draft with it. An open menu is fine, it is what the item was chosen from.
fn new_started(state: &mut crate::app::State) {
    if state.workspace.active.is_none() || crate::overlay::registry::open_dialog(state).is_some() {
        return;
    }
    open_draft(state, NamingTarget::New, String::new());
}

/// **Rename** opens the draft holding the group's current name; a group the project no longer has
/// opens nothing. Not over an open dialog, as for **New group…**.
fn rename_started(state: &mut crate::app::State, id: UserGroupId) {
    if crate::overlay::registry::open_dialog(state).is_some() {
        state.sidebar_groups.group_menu = None;
        return;
    }
    let Some(name) = state
        .sidebar_layout()
        .and_then(|layout| layout.group(id))
        .map(|group| group.name.clone())
    else {
        state.sidebar_groups.group_menu = None;
        return;
    };
    open_draft(state, NamingTarget::Rename(id), name);
}

/// Close what is open (a menu included) and show the naming dialog with `text` in its field.
fn open_draft(state: &mut crate::app::State, target: NamingTarget, text: String) {
    state.clear_for_dialog();
    state.sidebar_groups.naming = Some(NamingDraft {
        target,
        text,
        error: None,
    });
}

/// **Delete**: the group goes and its worktrees return to the ungrouped list; no worktree or
/// session is touched (FR-003). A group that is already gone changes nothing.
fn deleted(state: &mut crate::app::State, id: UserGroupId) -> Vec<Outcome> {
    state.sidebar_groups.group_menu = None;
    let Some(project) = state.workspace.active.clone() else {
        return Vec::new();
    };
    let Some(mut layout) = state.sidebar_layout().cloned() else {
        return Vec::new();
    };
    match layout.delete(id) {
        Ok(()) => vec![Outcome::SidebarLayoutChanged { project, layout }],
        Err(_) => Vec::new(),
    }
}

/// **Move to group**: closes the row's menu, then moves the row to the end of the target list. A
/// row the sidebar no longer has, or a group that is gone, changes nothing, and so does choosing
/// the list the row is already in. A row the saved layout does not hold yet is ungrouped, so it is
/// adopted there for [`SidebarLayout::move_item`] to take it out; moving it to *Ungrouped* then has
/// nothing to do.
fn moved(state: &mut crate::app::State, item: LayoutItem, target: MoveTarget) -> Vec<Outcome> {
    let to = match target {
        MoveTarget::Group(id) => DropTarget::Group {
            id,
            index: usize::MAX,
        },
        MoveTarget::Ungrouped => DropTarget::Ungrouped { index: usize::MAX },
        MoveTarget::NewGroup => {
            if crate::overlay::registry::open_dialog(state).is_some() {
                close_row_menus(state);
            } else if state.layout_item_exists(&item) {
                open_draft(state, NamingTarget::NewAndMove(item), String::new());
            } else {
                close_row_menus(state);
            }
            return Vec::new();
        }
    };
    close_row_menus(state);
    let Some(project) = state.workspace.active.clone() else {
        return Vec::new();
    };
    if !state.layout_item_exists(&item) {
        return Vec::new();
    }
    let held = state
        .sidebar_layout()
        .cloned()
        .unwrap_or_else(SidebarLayout::default);
    let list_to = match to {
        DropTarget::Group { id, .. } => Some(id),
        _ => None,
    };
    // Already where it was asked to go: nothing to move, nothing to save.
    if held.list_of(&item).unwrap_or(None) == list_to {
        return Vec::new();
    }
    let mut layout = held;
    layout.adopt(&item);
    if layout.move_item(&item, to).is_err() {
        return Vec::new();
    }
    vec![Outcome::SidebarLayoutChanged { project, layout }]
}

/// A pick from a row's menu closes it, as every other pick does. The menu is found through the
/// overlay registry rather than named, so this feature names no other feature's items (SC-002).
fn close_row_menus(state: &mut crate::app::State) {
    let menus: Vec<SurfaceId> = crate::overlay::registry::open_popovers(state)
        .into_iter()
        .filter(|open| open.layer() == micold_core::overlay::Layer::ContextMenu)
        .map(|open| open.id())
        .collect();
    for id in menus {
        crate::overlay::registry::dismiss(state, id);
    }
}

/// The draft was confirmed: a valid name returns the changed layout and closes the dialog; a
/// refused one keeps the dialog open with the reason, and the layout unchanged (C2).
fn confirmed(state: &mut crate::app::State) -> Vec<Outcome> {
    let (Some(project), Some(draft)) = (
        state.workspace.active.clone(),
        state.sidebar_groups.naming.as_ref(),
    ) else {
        return Vec::new();
    };
    let held = state
        .sidebar_layout()
        .cloned()
        .unwrap_or_else(SidebarLayout::default);
    let mut layout = held.clone();
    let result = match &draft.target {
        NamingTarget::New => layout.create(&draft.text).map(|_| ()),
        NamingTarget::Rename(id) => layout.rename(*id, &draft.text),
        NamingTarget::NewAndMove(item) if !state.layout_item_exists(item) => {
            Err(LayoutError::UnknownItem)
        }
        NamingTarget::NewAndMove(item) => new_and_move(&mut layout, item, &draft.text),
    };
    match result {
        Ok(()) => {
            state.sidebar_groups.naming = None;
            // Confirming a rename without a change writes nothing.
            if layout == held {
                return Vec::new();
            }
            vec![Outcome::SidebarLayoutChanged { project, layout }]
        }
        Err(error) => {
            let message = refusal(error, &draft.text);
            if let Some(draft) = &mut state.sidebar_groups.naming {
                draft.error = Some(message);
            }
            Vec::new()
        }
    }
}

/// Create the group `name` and move `item` into it, together or not at all (`layout` is a copy the
/// caller keeps only on success). A row the layout never held is ungrouped, so it is placed there
/// first for the move to take it out of.
fn new_and_move(
    layout: &mut SidebarLayout,
    item: &LayoutItem,
    name: &str,
) -> Result<(), LayoutError> {
    let id = layout.create(name)?;
    layout.adopt(item);
    layout.move_item(item, DropTarget::Group { id, index: 0 })
}

/// The sidebar's empty-area menu, as a floating surface (C6a): a context menu, so it and the row
/// menus replace each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarEmptyAreaMenu;

impl SidebarEmptyAreaMenu {
    /// This surface's identity, nameable by the surfaces that displace it.
    pub const ID: SurfaceId = SurfaceId::new("sidebar_empty_menu");
}

floating_surface!(SidebarEmptyAreaMenu, id: Self::ID, layer: ContextMenu, cancelled_by: Message::SidebarGroups(Msg::EmptyAreaMenuDismissed));

impl Registered for SidebarEmptyAreaMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .sidebar_groups
            .empty_menu
            .map(|_| SidebarEmptyAreaMenu)
    }
}

/// The group naming dialog, as a floating surface (R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupNamingDialog;

floating_surface!(GroupNamingDialog, id: SurfaceId::new("group_naming"), layer: Dialog, cancelled_by: Message::SidebarGroups(Msg::Cancelled));

impl Registered for GroupNamingDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .sidebar_groups
            .naming
            .as_ref()
            .map(|_| GroupNamingDialog)
    }
}

/// A user group header's menu, as a floating surface (C6): a context menu, so it and the row menus
/// replace each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupContextMenu;

impl GroupContextMenu {
    /// This surface's identity, nameable by the surfaces that displace it.
    pub const ID: SurfaceId = SurfaceId::new("user_group_menu");
}

floating_surface!(GroupContextMenu, id: Self::ID, layer: ContextMenu, cancelled_by: Message::SidebarGroups(Msg::MenuDismissed));

impl Registered for GroupContextMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.sidebar_groups.group_menu.map(|_| GroupContextMenu)
    }
}
