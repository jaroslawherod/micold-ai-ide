//! The sidebar's rows and tag filters (feature 021, T016).
//!
//! A sidebar row is a *projection*: a worktree or the project root, joined with its sessions and
//! with the tags rendered beside its name. Nothing here holds the sidebar's own mutable state —
//! that lives in `State` until Tier 3 — so this module is the row vocabulary and the two pure
//! functions over it, kept together per FR-001.

use micold_core::naming::{ConventionalType, Tag};
use micold_core::session::Session;
use micold_core::worktree::Worktree;
use std::collections::BTreeSet;
use std::path::Path;

/// One row in the sidebar's location list (feature 010): either a worktree or the single
/// "Default" project-root entry. A closed enum (Principle V) so a row can never be ambiguously
/// styled as a worktree when it isn't one (FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarEntry {
    /// A discovered worktree row.
    Worktree(WorktreeNode),
    /// The single project-root row (constitution v1.3.0, Principle III exception).
    Default(DefaultNode),
}

/// The "Default" (project-root) sidebar row, joined with its sessions (feature 010, FR-001,
/// FR-006). Unlike [`WorktreeNode`] it carries no [`Tag`]s and is never subject to the sidebar's
/// tag-filter panel (feature 009) — type/issue/status tags are derived from worktree branch
/// naming and do not apply to the project root (research.md R4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultNode {
    /// Always the literal "Default" (FR-006) — never derived or user-renamable.
    pub display_name: &'static str,
    /// Whether its session sub-items are shown.
    pub expanded: bool,
    /// Sessions with `SessionLocation::Default` for the active project.
    pub sessions: Vec<Session>,
}

/// One worktree row in the sidebar tree, joined with its (expanded) sessions (FR-002/003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeNode {
    /// The worktree itself.
    pub worktree: Worktree,
    /// The human-friendly display name shown on the first line (FR-001, FR-017): the custom
    /// rename override if set, else derived from `dir_name`.
    pub display_name: String,
    /// Color-coded tags shown beneath the name (FR-001..003, FR-011): the conventional type,
    /// an optional Jira issue, and a status tag for non-`Valid` worktrees.
    pub tags: Vec<Tag>,
    /// Whether its session sub-items are shown.
    pub expanded: bool,
    /// The sessions hosted by this worktree (empty unless expanded is irrelevant to data).
    pub sessions: Vec<Session>,
}

/// A tag filter the sidebar can apply (feature 008, FR-024). Typed so an impossible filter is
/// unrepresentable (Principle V); ordered so it lives in a `BTreeSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TagFilter {
    /// Match worktrees of a specific conventional type.
    Type(ConventionalType),
    /// Match worktrees that embed a Jira/issue key.
    HasIssue,
    /// Match worktrees whose name does not follow the convention (no type tag).
    Untyped,
}

/// Whether a worktree with `tags` passes the active `filters` (feature 008, FR-025). An empty
/// filter set shows everything; otherwise a worktree matches if it satisfies ANY active filter
/// (logical OR).
pub fn matches_filters(tags: &[Tag], filters: &BTreeSet<TagFilter>) -> bool {
    if filters.is_empty() {
        return true;
    }
    filters.iter().any(|f| match f {
        TagFilter::Type(t) => tags.iter().any(|tag| matches!(tag, Tag::Type(x) if x == t)),
        TagFilter::HasIssue => tags.iter().any(|tag| matches!(tag, Tag::Issue(_))),
        TagFilter::Untyped => !tags.iter().any(|tag| matches!(tag, Tag::Type(_))),
    })
}

/// The fixed location-tooltip label for the "Default" sidebar entry (feature 010, FR-010) —
/// unlike a worktree's label, this never varies, since the Default entry is always exactly the
/// project root.
pub const DEFAULT_LOCATION_LABEL: &str = "Project root";

/// A worktree's location, expressed relative to the project root, for its sidebar tooltip
/// (feature 010, FR-010, research.md R6). Every worktree lives directly under
/// `<project_root>/.claude/worktrees/`, so a plain `strip_prefix` suffices — no
/// general-purpose relative-path algorithm is needed. Falls back to the absolute path in the
/// unreachable case where a worktree's path is not actually under the project root.
pub fn worktree_location_label(project_root: &Path, worktree: &Worktree) -> String {
    worktree
        .path
        .strip_prefix(project_root)
        .map(|rel| rel.display().to_string())
        .unwrap_or_else(|_| worktree.path.display().to_string())
}
