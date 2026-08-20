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
use micold_core::project::{canonicalize_best_effort, FolderEntry, RenameError};
use micold_core::selector::Selector;
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

impl ProjectSwitcher {
    /// This surface's identity, nameable by the surfaces that displace it or that it
    /// displaces (T067a-2). The declaration has to point at something, and pointing at the
    /// literal string in two places is how the two would come to disagree.
    pub const ID: SurfaceId = SurfaceId::new("project_switcher");
}

impl FloatingSurface for ProjectSwitcher {
    fn id(&self) -> SurfaceId {
        Self::ID
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

impl ProjectContextMenu {
    /// This surface's identity, nameable by the surfaces that displace it or that it
    /// displaces (T067a-2). The declaration has to point at something, and pointing at the
    /// literal string in two places is how the two would come to disagree.
    pub const ID: SurfaceId = SurfaceId::new("project_menu");
}

impl FloatingSurface for ProjectContextMenu {
    fn id(&self) -> SurfaceId {
        Self::ID
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

/// The top-bar switcher panel was toggled (feature 015).
///
/// Mutually exclusive with the other two lightweight popovers and with the project context menu.
/// The writes into `help` and `sidebar` data are the shared toolbar rule, catalogued in
/// `tests/feature_write_isolation.rs`; `help::menu_toggled` carries the same rule from its side.
#[must_use = "what an opening popover displaces is the registry's business, not the caller's"]
pub fn switcher_toggled(state: &mut State) -> Vec<crate::features::Outcome> {
    state.project_switcher_open = !state.project_switcher_open;
    crate::features::surface_opened(state.project_switcher_open, ProjectSwitcher::ID)
}

/// The folder browser descended into `path`.
pub fn selector_navigated_into(state: &mut State, path: PathBuf) {
    with_selector(state, |selector| selector.enter(path));
}

/// The folder browser went up one level.
pub fn selector_navigated_up(state: &mut State) {
    with_selector(state, Selector::up);
}

/// A folder listing arrived.
pub fn selector_listing_ready(state: &mut State, entries: Vec<FolderEntry>) {
    with_selector(state, |selector| selector.listing_ready(entries));
}

/// A folder listing failed.
pub fn selector_listing_failed(state: &mut State, message: String) {
    with_selector(state, |selector| selector.listing_failed(message));
}

/// Apply an operation to the open folder browser, if one is showing.
///
/// Its presence *is* the dialog being shown (T037), so every one of these arms is a no-op with the
/// dialog closed — stated once rather than four times.
fn with_selector<T>(state: &mut State, change: impl FnOnce(&mut Selector) -> T) {
    if let Some(selector) = &mut state.selector {
        let _ = change(selector);
    }
}

/// The project selector was dismissed.
pub fn selector_closed(state: &mut State) {
    state.selector = None;
}

/// A rename was started for `path` (feature 015, FR-018).
///
/// Seeded with the project's current display name. A path with no project is a no-op: nothing is
/// opened, so an empty dialog cannot appear over a project that is gone.
pub fn rename_started(state: &mut State, path: PathBuf) {
    let current = state
        .workspace
        .projects
        .iter()
        .find(|p| p.path == path)
        .map(|p| p.display_name.clone());
    if let Some(name) = current {
        state.clear_for_dialog();
        state.rename_draft = Some(RenameDraft {
            path,
            text: name,
            error: None,
        });
    }
}

/// The rename field was edited.
///
/// Clearing the error matters: a stale message beside a name the user has since corrected is the
/// dialog arguing with them after they fixed it.
pub fn rename_text_changed(state: &mut State, text: String) {
    if let Some(draft) = &mut state.rename_draft {
        draft.text = text;
        draft.error = None;
    }
}

/// The rename was confirmed (feature 015, FR-018).
///
/// Renaming never touches disk — only the stored name. A rejected name leaves the dialog open
/// carrying its reason, rather than discarding what was typed.
pub fn rename_confirmed(state: &mut State) {
    let Some((path, text)) = state
        .rename_draft
        .as_ref()
        .map(|draft| (draft.path.clone(), draft.text.clone()))
    else {
        return;
    };
    match state.workspace.rename(&path, &text) {
        Ok(()) => state.rename_draft = None,
        Err(error) => {
            if let Some(draft) = &mut state.rename_draft {
                draft.error = Some(error);
            }
        }
    }
}

/// The rename was dismissed without saving.
pub fn rename_cancelled(state: &mut State) {
    state.rename_draft = None;
}

/// A project's right-click menu was toggled (feature 015).
///
/// The same project closes; a different one replaces it (only ever one open), re-anchored at
/// wherever the pointer now is. The switcher panel stays open behind the menu so the right-clicked
/// row remains visible; the other popovers do not.
#[must_use = "what an opening menu displaces is the registry's business, not the caller's"]
pub fn menu_toggled(state: &mut State, path: PathBuf) -> Vec<crate::features::Outcome> {
    state.project_menu_open = match &state.project_menu_open {
        Some(open) if open.path == path => None,
        _ => Some(ProjectMenu {
            path,
            anchor: state.cursor,
        }),
    };
    crate::features::surface_opened(state.project_menu_open.is_some(), ProjectContextMenu::ID)
}

/// The project context menu was dismissed.
pub fn menu_dismissed(state: &mut State) {
    state.project_menu_open = None;
}

/// A forget was requested (feature 014, FR-002).
///
/// Opens the confirmation; nothing is removed until confirmed.
pub fn forget_requested(state: &mut State, path: PathBuf) {
    state.clear_for_dialog();
    state.forget_target = Some(path);
}

/// A forget was confirmed (feature 014, FR-003/FR-005).
///
/// Drops the record and all per-path metadata. The shell has already stopped the project's live
/// processes and persists the deletion after this pure transition.
///
/// **Clearing the session pointer is the subtle half.** `Workspace::forget` clears
/// `workspace.active`, and `active_session` only ever referenced the active project — so without
/// this it would point into a project that no longer exists (FR-008).
pub fn forget_confirmed(state: &mut State) -> Vec<crate::features::Outcome> {
    let mut outcomes = Vec::new();
    if let Some(path) = state.forget_target.clone() {
        let was_active =
            state.workspace.active.as_deref() == Some(canonicalize_best_effort(&path).as_path());
        state.workspace.forget(&path);
        if was_active {
            // Feature 024: an app-initiated clear like any other, so it goes through the same
            // function (contract §3's table). It arms nothing — there is no session and, after a
            // forget, no project either.
            outcomes = state.set_current_session(None);
        }
    }
    state.forget_target = None;
    outcomes
}

/// A forget was dismissed.
pub fn forget_cancelled(state: &mut State) {
    state.forget_target = None;
}

/// Opening a folder was refused because it is not a git repository (FR-001a).
///
/// The active project is unchanged. Reported through the global surface rather than
/// `worktree_error`: the refusal arrives with the selector already closed, so the Add Worktree
/// modal that owns `worktree_error` is not open and the message would never be drawn.
pub fn open_refused(_state: &mut State, message: String) -> Vec<crate::features::Outcome> {
    vec![crate::features::notifications::error(message)]
}
