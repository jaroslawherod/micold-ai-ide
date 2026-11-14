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
//! `New`, `TextChanged`, `Confirmed`, `Cancelled`; the sidebar's empty-area menu
//! (`EmptyAreaMenuOpened`, `EmptyAreaMenuDismissed`), whose one item is **New group…** (C6a).

use crate::app::Message;
use crate::features::Outcome;
use crate::floating_surface;
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;
use micold_core::sidebar_layout::{LayoutError, SidebarLayout};

/// What this feature remembers: the open naming dialog, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The naming dialog's draft; its presence *is* the dialog being shown.
    pub naming: Option<NamingDraft>,
    /// The sidebar's empty-area menu, at the window point it was opened from; `None` while closed.
    pub empty_menu: Option<(u16, u16)>,
}

/// What the naming dialog names (data-model.md, *NamingDraft*). Renaming a group and creating one
/// to move a row into join this with the milestones that add them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingTarget {
    /// A new, empty group.
    New,
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
    /// The naming field was edited.
    TextChanged(String),
    /// The naming dialog's confirm.
    Confirmed,
    /// The naming dialog was dismissed.
    Cancelled,
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
        Msg::TextChanged(text) => {
            if let Some(draft) = &mut state.sidebar_groups.naming {
                draft.text = text;
                draft.error = None;
            }
        }
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
    state.clear_for_dialog();
    state.sidebar_groups.naming = Some(NamingDraft {
        target: NamingTarget::New,
        text: String::new(),
        error: None,
    });
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
    let mut layout = state
        .sidebar_layout()
        .cloned()
        .unwrap_or_else(SidebarLayout::default);
    let result = match &draft.target {
        NamingTarget::New => layout.create(&draft.text).map(|_| ()),
    };
    match result {
        Ok(()) => {
            state.sidebar_groups.naming = None;
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
