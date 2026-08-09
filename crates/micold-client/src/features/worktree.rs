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

/// A worktree row's right-click menu, as a floating surface (feature 021, T031).
///
/// Anchored beside the sidebar rather than at the cursor, but a context menu all the same: it is
/// opened over the row it acts on and must not fall behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreeContextMenu;

impl FloatingSurface for WorktreeContextMenu {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("worktree_menu")
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu).cancelled_by(Message::WorktreeMenuDismissed)
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
