//! Worktrees: which ones are visible, what they are called, and how they are tagged
//! (feature 021, T019).
//!
//! Three of the helpers T019 names are **not** here — `worktree_tree`, `filtered_worktree_tree`
//! and `available_tag_filters` went to `features/sidebar.rs` instead. They are named for worktrees
//! but typed for the sidebar: they return `WorktreeNode` and `TagFilter`, consume
//! `sidebar_filters`, and `worktree_tree`'s own doc comment opens "Build the sidebar tree". Filing
//! them here would group by name where FR-001 asks to group by feature, and SC-010 — "answer
//! 'where does this feature live?' by naming a single module" — is decided by where the sidebar's
//! projections sit, not by what they are called.
//!
//! What is left is worktree-owned: visibility, display name, and tags.
//!
//! These are `impl State` blocks because `State` is still monolithic in Tier 1. Methods resolve on
//! the type rather than the module, so moving them here changed no call site. Tier 3 splits `State`
//! itself, at which point these operate on the worktree feature's own state.

use crate::app::Message;
use crate::app::State;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::naming::{display_name, parse_tags, Tag};
use micold_core::overlay::Layer;
use micold_core::worktree::{Worktree, WorktreeStatus};

/// In-progress worktree-rename state, present only while the worktree-rename dialog is open
/// (feature 008, FR-013/FR-014). Mirrors `RenameDraft` but is keyed by worktree `dir_name`
/// and only ever changes the displayed name — never the folder or branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRenameDraft {
    /// The worktree being renamed, by `dir_name`.
    pub dir_name: String,
    /// The current editable display name.
    pub text: String,
    /// The last validation error, if the user tried to confirm an invalid name.
    pub error: Option<micold_core::project::RenameError>,
}

/// The tags for a worktree: the derived type + issue tags, plus a status tag when the
/// worktree is not `Valid` (FR-002, FR-003, FR-011).
///
/// `pub(crate)` rather than private: the sidebar's row and chip projections both read it from
/// `features/sidebar.rs`. It was private while it and its callers shared one file, and the
/// widening is the cost of the boundary — Tier 3 revisits it (T062).
pub(crate) fn worktree_tags(worktree: &Worktree) -> Vec<Tag> {
    let mut tags = parse_tags(&worktree.dir_name);
    if worktree.status != WorktreeStatus::Valid {
        tags.push(Tag::Status(worktree.status));
    }
    // Feature 014 (FR-010b). Injected here rather than in `parse_tags`, which only sees the
    // directory name and so cannot consult the branch. Only ever *seen* when the reveal
    // control is on, since a hidden worktree produces no row at all.
    if worktree.is_agent_owned() {
        tags.push(Tag::Agent);
    }
    tags
}

impl State {
    /// The display name for a worktree (FR-017): the user's rename override when present,
    /// otherwise the friendly name derived from the directory name. Never touches the folder
    /// or branch on disk (FR-007, FR-014).
    pub fn worktree_display_name(&self, dir_name: &str) -> String {
        self.workspace
            .worktree_name(dir_name)
            .map(str::to_string)
            .unwrap_or_else(|| display_name(dir_name))
    }

    /// The worktrees currently shown to the user (feature 014, FR-002/FR-003): all of them while
    /// the reveal control is on, only user-owned ones while it is off.
    ///
    /// The single source every worktree surface reads from — [`State::worktree_tree`],
    /// [`State::available_tag_filters`], and the sidebar's empty-state hint — so hiding, counting,
    /// and filtering agree by construction instead of via three separate filters that can drift
    /// (contracts/agent-worktree-classification.md).
    ///
    /// Note what does NOT read this: `set_worktrees`'s pruning and [`State::sessions_in_worktree`]
    /// reason about *existence*, not visibility. A hidden worktree still exists, and its rename
    /// override must survive.
    pub fn visible_worktrees(&self) -> impl Iterator<Item = &Worktree> {
        let show_all = self.show_agent_worktrees;
        self.worktrees
            .iter()
            .filter(move |w| show_all || !w.is_agent_owned())
    }

    /// Whether any worktree is currently visible (feature 014, FR-003). Drives the sidebar's
    /// choice between "No worktrees yet" and "No worktrees match the filter": a project whose only
    /// worktrees are agent-owned has none *visible*, so it must get the former — offering a
    /// "Clear filters" action when no filter is active would be nonsense (research R7).
    ///
    /// Lives here rather than in the `gui`-only sidebar so the decision is testable
    /// (Principle I).
    pub fn has_visible_worktrees(&self) -> bool {
        self.visible_worktrees().next().is_some()
    }
}

/// An open worktree right-click context menu (feature 008, FR-013): which worktree it acts on, and
/// where to draw it. Mirrors [`crate::features::project::ProjectMenu`], deliberately — a row is a
/// row, and the two menus differing was BUG-008.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeMenu {
    /// The worktree the menu acts on, by `dir_name`.
    pub dir_name: String,
    /// The menu panel's top-left corner, in window pixels (the press point) — clamped at render
    /// time rather than here, so a resize while the menu is open cannot leave it hanging off the
    /// edge (018 FR-029d).
    pub anchor: (u16, u16),
}

/// A worktree row's right-click menu, as a floating surface (feature 021, T031).
///
/// Anchored at the press point since BUG-008, like every other context menu in the application
/// (018 FR-029d). It is opened over the row it acts on and must not fall behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreeContextMenu;

impl WorktreeContextMenu {
    /// This surface's identity, nameable by the surfaces that displace it or that it
    /// displaces (T067a-2). The declaration has to point at something, and pointing at the
    /// literal string in two places is how the two would come to disagree.
    pub const ID: SurfaceId = SurfaceId::new("worktree_menu");
}

impl FloatingSurface for WorktreeContextMenu {
    fn id(&self) -> SurfaceId {
        Self::ID
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu)
            .cancelled_by(Message::Worktree(Msg::MenuDismissed))
    }
}

impl Registered for WorktreeContextMenu {
    fn open_in(state: &State) -> Option<Self> {
        state
            .worktree_menu_open
            .as_ref()
            .map(|_| WorktreeContextMenu)
    }
}

/// The confirm-delete-worktree dialog, as a floating surface (feature 021, T032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmWorktreeDeleteDialog;

impl FloatingSurface for ConfirmWorktreeDeleteDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("confirm_worktree_delete")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog)
            .cancelled_by(Message::Worktree(Msg::DeleteCancelled))
    }
}

impl Registered for ConfirmWorktreeDeleteDialog {
    fn open_in(state: &State) -> Option<Self> {
        state
            .worktree_delete_target
            .as_ref()
            .map(|_| ConfirmWorktreeDeleteDialog)
    }
}

/// The rename-worktree dialog, as a floating surface (feature 021, T032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenameWorktreeDialog;

impl FloatingSurface for RenameWorktreeDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("rename_worktree")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog)
            .cancelled_by(Message::Worktree(Msg::RenameCancelled))
    }
}

impl Registered for RenameWorktreeDialog {
    fn open_in(state: &State) -> Option<Self> {
        state
            .worktree_rename_draft
            .as_ref()
            .map(|_| RenameWorktreeDialog)
    }
}

/// Discovery answered with the current worktree list (feature 005, FR-018).
pub fn loaded(state: &mut State, worktrees: Vec<Worktree>) -> Vec<crate::features::Outcome> {
    state.set_worktrees(worktrees)
}

/// The shell created a worktree and it joins the list (feature 005, FR-017; T067a-4).
///
/// Idempotent by **directory name**: a create names the directory it made, so that is the identity
/// its caller can vouch for. [`included`] keys on path instead because the daemon answers an
/// include with one. Reached only through `Outcome::WorktreeCreated` — the form that ran the
/// create does not own this list.
pub fn created(state: &mut State, worktree: Worktree) -> Vec<crate::features::Outcome> {
    if !state
        .worktrees
        .iter()
        .any(|w| w.dir_name == worktree.dir_name)
    {
        state.worktrees.push(worktree);
        state.worktrees.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    }
    vec![list_changed(state)]
}

/// What every path that alters the list reports: the `dir_name`s now in it.
fn list_changed(state: &State) -> crate::features::Outcome {
    crate::features::Outcome::WorktreesReplaced(
        state.worktrees.iter().map(|w| w.dir_name.clone()).collect(),
    )
}

/// A worktree's right-click menu was toggled (feature 008).
///
/// Same worktree closes; a different one replaces it (only ever one open). It displaces the
/// project row menu and nothing else — the two context menus replace each other, while a panel
/// popover open elsewhere in the window survives a right-click in the sidebar (T067a-2).
#[must_use = "what an opening menu displaces is the registry's business, not the caller's"]
pub fn menu_toggled(
    state: &mut State,
    dir: String,
    anchor: (u16, u16),
) -> Vec<crate::features::Outcome> {
    state.worktree_menu_open = match &state.worktree_menu_open {
        Some(open) if open.dir_name == dir => None,
        _ => Some(WorktreeMenu {
            dir_name: dir,
            anchor,
        }),
    };
    crate::features::surface_opened(state.worktree_menu_open.is_some(), WorktreeContextMenu::ID)
}

/// The worktree context menu was dismissed.
pub fn menu_dismissed(state: &mut State) {
    state.worktree_menu_open = None;
}

/// The daemon answered an include request with the worktree as its own discovery sees it
/// (016 BUG-002, FR-027/FR-030).
///
/// Idempotent by path, and sorted by directory name so an included worktree lands where the list
/// would have put it rather than at the end.
pub fn included(state: &mut State, worktree: Worktree) -> Vec<crate::features::Outcome> {
    if !state.worktrees.iter().any(|w| w.path == worktree.path) {
        state.worktrees.push(worktree);
        state.worktrees.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    }
    vec![list_changed(state)]
}

/// An exclude was requested; the menu it was chosen from closes (016 BUG-002).
pub fn exclude_requested(state: &mut State) {
    state.worktree_menu_open = None;
}

/// The daemon confirmed a worktree is no longer included (016 BUG-002).
pub fn excluded(state: &mut State, path: std::path::PathBuf) {
    state.worktrees.retain(|w| w.path != path);
}

/// A delete was requested; the confirmation opens (feature 008, FR-018/FR-019).
///
/// The keep-branch choice never carries over from a previously cancelled or confirmed dialog
/// (feature 013).
pub fn delete_requested(state: &mut State, dir: String) {
    state.clear_for_dialog();
    state.worktree_delete_target = Some(dir);
    state.worktree_delete_keep_branch = false;
}

/// A delete was confirmed — which *requests* it rather than performing it.
///
/// The daemon owns the git removal and the session records, and answers with `OperationOk`
/// (followed by a `CatalogChanged` carrying git's refreshed truth) or `OperationError`. So this
/// only dismisses the dialog.
///
/// **Dropping the row here instead — the previous behaviour — made every delete *look* like it
/// succeeded**: a delete git refused showed the worktree vanishing, then silently reappearing when
/// the next catalog push restored it, which reads as the app resurrecting something the user
/// deleted rather than as the failure it is. Leaving the row alone means a refusal simply leaves it
/// in place, beside the error notification explaining why.
pub fn delete_confirmed(state: &mut State) {
    state.worktree_delete_target = None;
}

/// The delete confirmation was dismissed.
pub fn delete_cancelled(state: &mut State) {
    state.worktree_delete_target = None;
}

/// The "also delete the branch" choice was toggled (feature 013).
pub fn delete_keep_branch_toggled(state: &mut State, keep: bool) {
    state.worktree_delete_keep_branch = keep;
}

/// A worktree rename was started, seeded with its current display name (feature 008, FR-013).
pub fn rename_started(state: &mut State, dir: String) {
    let text = state.worktree_display_name(&dir);
    state.clear_for_dialog();
    state.worktree_rename_draft = Some(WorktreeRenameDraft {
        dir_name: dir,
        text,
        error: None,
    });
}

/// The rename field was edited; any pending error is cleared with it.
pub fn rename_text_changed(state: &mut State, text: String) {
    if let Some(draft) = &mut state.worktree_rename_draft {
        draft.text = text;
        draft.error = None;
    }
}

/// The rename was confirmed (feature 008, FR-014).
///
/// Changes only the stored display name — never the folder or the branch on disk. A rejected name
/// leaves the dialog open carrying its reason.
pub fn rename_confirmed(state: &mut State) {
    let Some((dir, text)) = state
        .worktree_rename_draft
        .as_ref()
        .map(|d| (d.dir_name.clone(), d.text.clone()))
    else {
        return;
    };
    match state.workspace.set_worktree_name(&dir, &text) {
        Ok(()) => state.worktree_rename_draft = None,
        Err(error) => {
            if let Some(draft) = &mut state.worktree_rename_draft {
                draft.error = Some(error);
            }
        }
    }
}

/// The rename was dismissed without saving.
pub fn rename_cancelled(state: &mut State) {
    state.worktree_rename_draft = None;
}

/// The pointer entered a worktree row (feature 008).
pub fn hovered(state: &mut State, dir: String) {
    state.hovered_worktree = Some(dir);
}

/// The pointer left a worktree row (feature 008).
///
/// Only clears when leaving the row that was thought to be hovered, so a stale exit from a
/// previous row cannot clobber a fresh enter.
pub fn unhovered(state: &mut State, dir: String) {
    if state.hovered_worktree.as_deref() == Some(dir.as_str()) {
        state.hovered_worktree = None;
    }
}

/// Everything the user or the daemon can say about a worktree (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// Seventeen began with `Worktree` and do not any more — the type says which thing (contract M1),
/// so `WorktreeDeleteKeepBranchToggled` is `Msg::DeleteKeepBranchToggled`. The plural in
/// `WorktreesLoaded` went with it: `Msg::Loaded` is the list arriving, and the list is what this
/// feature is.
///
/// [`Msg::TextCopyRequested`] is the odd one and stays as it is. Its name says nothing about
/// worktrees, but its single emit site copies a worktree's name from the sidebar
/// (research.md §R2), so this is the feature that owns it. Renaming it to something
/// worktree-shaped would claim a generality it does not have; leaving the name alone and letting
/// the wrapper say who owns it is the honest version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The binary discovered/re-discovered the active project's worktrees (FR-018).
    Loaded(Vec<Worktree>),
    /// Open (or close, if already open) a worktree's right-click context menu, by `dir_name`,
    /// anchored at the press point in window pixels (018 FR-029d).
    MenuToggled(String, (u16, u16)),
    /// Dismiss the worktree context menu (outside click, or after an action is chosen).
    MenuDismissed,
    /// Request deletion of a worktree; opens the confirm dialog (FR-018), by `dir_name`.
    DeleteRequested(String),
    /// Ask the daemon to show the worktree at this absolute path among the project's own
    /// (016 BUG-002, FR-027). Raised from the blocked-branch explanation, which is where the user
    /// meets a holder they cannot otherwise reach.
    IncludeRequested(std::path::PathBuf),
    /// The daemon is now showing it. The row also arrives with the next catalog push; this is what
    /// makes it appear at the moment the user asked rather than at the next refresh.
    Included(Worktree),
    /// Stop showing an included worktree, by `dir_name` (FR-030). Nothing on disk is touched.
    ExcludeRequested(String),
    /// The daemon has stopped showing the worktree at this path.
    Excluded(std::path::PathBuf),
    /// Confirm deletion. The binary terminates the worktree's sessions, removes its git
    /// worktree + branch and directory, then persists (FR-020); the reducer drops the records.
    DeleteConfirmed,
    /// Dismiss the delete confirmation without removing anything (FR-021).
    DeleteCancelled,
    /// The delete confirmation's "also delete the branch" choice changed (feature 013,
    /// FR-011/FR-012).
    DeleteKeepBranchToggled(bool),
    /// Begin renaming a worktree's displayed name; opens the rename dialog (FR-013), by `dir_name`.
    RenameStarted(String),
    /// The worktree-rename dialog's text changed.
    RenameTextChanged(String),
    /// Confirm the worktree rename. Applies the display-name override if valid (FR-014); the
    /// binary then persists (FR-015).
    RenameConfirmed,
    /// Dismiss the worktree-rename dialog without applying.
    RenameCancelled,
    /// The pointer entered a worktree row (feature 008), by `dir_name`; reveals its row actions.
    Hovered(String),
    /// The pointer left a worktree row (feature 008), by `dir_name`; hides its row actions.
    Unhovered(String),
    /// Copy arbitrary displayed text (a worktree name) to the system clipboard. The binary
    /// performs the actual clipboard write; the reducer has no state to update.
    TextCopyRequested(String),
}

/// The pure half of this feature's reducer surface: shape A (contract M2).
///
/// All eighteen arms are here. Five of them additionally need an effect — the daemon asked to
/// include, exclude, delete or rename, and the clipboard written — and those five are matched a
/// second time in `main.rs`, which runs the effect and lets the message fall through to here.
/// That is the split `worktree_form` established and M2 names as the reference: by *effect*, not
/// by variant, so nothing about a delete is duplicated between the two halves.
pub fn update(state: &mut State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::Loaded(worktrees) => return loaded(state, worktrees),
        Msg::MenuToggled(dir, anchor) => return menu_toggled(state, dir, anchor),
        Msg::Included(worktree) => return included(state, worktree),
        Msg::MenuDismissed => menu_dismissed(state),
        // The request itself changes nothing here: the daemon owns the included set, as it owns
        // every other piece of durable state, and answers with the worktree as its own discovery
        // sees it (016 BUG-002).
        Msg::IncludeRequested(_) => {}
        // The clipboard is the binary's; the reducer has no state to update.
        Msg::TextCopyRequested(_) => {}
        Msg::ExcludeRequested(_) => exclude_requested(state),
        Msg::Excluded(path) => excluded(state, path),
        Msg::DeleteRequested(dir) => delete_requested(state, dir),
        Msg::DeleteConfirmed => delete_confirmed(state),
        Msg::DeleteCancelled => delete_cancelled(state),
        Msg::DeleteKeepBranchToggled(keep) => delete_keep_branch_toggled(state, keep),
        Msg::RenameStarted(dir) => rename_started(state, dir),
        Msg::RenameTextChanged(text) => rename_text_changed(state, text),
        Msg::RenameConfirmed => rename_confirmed(state),
        Msg::RenameCancelled => rename_cancelled(state),
        Msg::Hovered(dir) => hovered(state, dir),
        Msg::Unhovered(dir) => unhovered(state, dir),
    }
    Vec::new()
}
