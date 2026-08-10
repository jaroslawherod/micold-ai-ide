//! Project switching, its right-click menu, and the rename draft (feature 021, T017).
//!
//! One feature, three shapes: the switcher row the top bar draws, the context menu a right-click
//! opens over it, and the in-progress rename that menu can start. `clamp_menu_anchor` lives here
//! rather than beside the view because it is a decision about the menu, not about how the menu is
//! painted (FR-001, FR-006).
//!
//! `SelectKind` sat in this stretch of `app.rs` and is named by T017, but it is terminal text
//! selection (feature 006, FR-013) and has nothing to do with projects. Grouping it here would
//! have followed the line range rather than the feature — the exact mistake FR-001 exists to
//! correct — so it stays put for the session module (T021).

use crate::app::Message;
use crate::app::State;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;
use micold_core::project::RenameError;
use std::path::PathBuf;

/// An open project right-click context menu (feature 015): which project it acts on, and where
/// to draw it. The anchor is the pointer position at the moment of the right-click, in window
/// pixels, so the menu opens under the cursor like a normal desktop context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMenu {
    /// The project the menu acts on.
    pub path: PathBuf,
    /// The menu panel's top-left corner, in window pixels (the click point).
    pub anchor: (u16, u16),
}

/// Clamp a context-menu anchor so the whole panel stays inside the window (feature 015).
///
/// `menu` and `window` are `(width, height)` in pixels. The panel is drawn from its top-left
/// corner, so an anchor near the right/bottom edge would otherwise push it off-screen; this
/// slides it back just far enough to fit. A window smaller than the menu, or a window size not
/// known yet (either dimension `0`), leaves the anchor untouched — clamping against a bogus
/// size would be worse than not clamping at all.
pub fn clamp_menu_anchor(anchor: (u16, u16), menu: (u16, u16), window: (u16, u16)) -> (u16, u16) {
    if window.0 == 0 || window.1 == 0 {
        return anchor;
    }
    (
        anchor.0.min(window.0.saturating_sub(menu.0)),
        anchor.1.min(window.1.saturating_sub(menu.1)),
    )
}

/// One row in the top-bar project switcher (feature 008), computed purely from the workspace
/// so the switcher's contents (active marker, running count, unavailable state) are
/// unit-testable without the GUI (FR-005–FR-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherEntry {
    /// The project's path (payload for selecting/activating it).
    pub path: PathBuf,
    /// The project's display name.
    pub label: String,
    /// Whether this is the currently active project (FR-006).
    pub is_active: bool,
    /// Number of running background sessions this project holds (FR-007).
    pub running_count: usize,
    /// Whether the folder is available; unavailable projects are shown but not selectable (FR-008).
    pub available: bool,
}

/// In-progress rename state, present only while the rename dialog is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDraft {
    /// The project being renamed, identified by its (canonical) path.
    pub path: PathBuf,
    /// The current editable text in the dialog.
    pub text: String,
    /// The last validation error, if the user tried to confirm an invalid name (FR-020).
    pub error: Option<RenameError>,
}

/// The project-switcher panel, as a floating surface (feature 021, T031).
///
/// A marker: the rows are projected from the workspace at render time, so "open" is all the
/// panel itself stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectSwitcher;

impl FloatingSurface for ProjectSwitcher {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("project_switcher")
    }

    fn layer(&self) -> Layer {
        Layer::Popover
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Popover).cancelled_by(Message::ProjectSwitcherToggled)
    }
}

impl Registered for ProjectSwitcher {
    fn open_in(state: &State) -> Option<Self> {
        state.project_switcher_open.then_some(ProjectSwitcher)
    }
}

/// A project row's right-click menu, as a floating surface (feature 021, T031).
///
/// [`Layer::ContextMenu`] rather than [`Layer::Popover`]: it is opened *over* the switcher it was
/// right-clicked in, and the row it acts on has to stay visible behind it. That was a property of
/// the order `ui::view` happened to build the two in; declaring the band makes it a property of
/// what the menu is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectContextMenu;

impl FloatingSurface for ProjectContextMenu {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("project_menu")
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu).cancelled_by(Message::ProjectMenuDismissed)
    }
}

impl Registered for ProjectContextMenu {
    fn open_in(state: &State) -> Option<Self> {
        state.project_menu_open.as_ref().map(|_| ProjectContextMenu)
    }
}

/// The folder-browser dialog that adds a project, as a floating surface
/// (feature 021, T032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectSelectorDialog;

impl FloatingSurface for ProjectSelectorDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("project_selector")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::ProjectSelectorClosed)
    }
}

impl Registered for ProjectSelectorDialog {
    fn open_in(state: &State) -> Option<Self> {
        state.selector.as_ref().map(|_| ProjectSelectorDialog)
    }
}

/// The rename-project dialog, as a floating surface (feature 021, T032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenameProjectDialog;

impl FloatingSurface for RenameProjectDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("rename_project")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::RenameCancelled)
    }
}

impl Registered for RenameProjectDialog {
    fn open_in(state: &State) -> Option<Self> {
        state.rename_draft.as_ref().map(|_| RenameProjectDialog)
    }
}

/// The confirm-forget-project dialog, as a floating surface (feature 021, T032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmForgetProjectDialog;

impl FloatingSurface for ConfirmForgetProjectDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("confirm_forget_project")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::ProjectForgetCancelled)
    }
}

impl Registered for ConfirmForgetProjectDialog {
    fn open_in(state: &State) -> Option<Self> {
        state
            .forget_target
            .as_ref()
            .map(|_| ConfirmForgetProjectDialog)
    }
}
