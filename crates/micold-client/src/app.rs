//! Render-free application core: state, messages, and the `update` reducer.
//!
//! This module has no dependency on iced. All state transitions are pure and
//! unit-testable via `cargo test` (Constitution Principle I). The GUI binary adapts
//! this core to iced's runtime in `src/main.rs`.
//!
//! Side-effectful concerns (reading the filesystem for a directory listing, detecting a
//! folder's git status, and persisting the catalog) are performed by the binary at the
//! I/O boundary; the reducer stays pure. A few messages (`ProjectSelectorOpened`,
//! `FolderChosen`) therefore carry no reducer effect here — they are documented no-ops
//! handled entirely in `src/main.rs`.

use micold_core::naming::{
    derive, dir_name_from_branch, display_name, parse_tags, ConventionalType, DerivedNames,
    NamingError, Tag, WorktreeNaming,
};
use micold_core::project::{canonicalize_best_effort, Availability, FolderEntry, RenameError};
use micold_core::selector::Selector;
use micold_core::session::{Session, SessionId, SessionLocation, ShellInstanceId};
use micold_core::theme::{
    observe_system_scheme, resolve, ColorScheme, SystemScheme, ThemePreference,
};
use micold_core::worktree::{
    BranchCandidate, BranchSituation, CreateMode, CreateStage, Worktree, WorktreeStatus,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The labels of the actions revealed under the "Help" menu, in display order.
///
/// "Help" exposes exactly one action — "About" (FR-003, FR-004).
pub const HELP_ACTIONS: [&str; 1] = ["About"];

/// Minimum sidebar width in pixels (resize lower bound).
pub const SIDEBAR_MIN_WIDTH: u16 = 180;
/// Maximum sidebar width in pixels (resize upper bound).
pub const SIDEBAR_MAX_WIDTH: u16 = 600;
/// Default sidebar width in pixels, used until the user resizes it.
pub const SIDEBAR_DEFAULT_WIDTH: u16 = 300;

/// The actions under the "Help" menu. See [`HELP_ACTIONS`].
pub fn help_actions() -> &'static [&'static str] {
    &HELP_ACTIONS
}

/// Which modal overlay, if any, is currently shown over the main window.
///
/// Modeling the overlay as an enum (rather than a `bool` per dialog) makes
/// "the About dialog is open twice" unrepresentable — satisfying FR-015 at the
/// type level (Constitution Principle V).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overlay {
    /// No modal is open; the main window is fully interactive.
    #[default]
    None,
    /// The About dialog is shown as a modal overlay.
    About,
    /// The in-app project selector (folder browser) is shown as a modal overlay.
    ProjectSelector,
    /// The rename-project dialog is shown as a modal overlay.
    RenameProject,
    /// The add-worktree form is shown as a modal overlay (feature 005, FR-005).
    AddWorktree,
    /// The Settings form is shown as a modal overlay (feature 006, FR-019).
    Settings,
    /// The confirm-delete dialog for a worktree is shown (feature 008, FR-018). The target
    /// worktree is held in [`State::worktree_delete_target`].
    ConfirmWorktreeDelete,
    /// The rename-worktree dialog is shown (feature 008, FR-013/FR-014). The in-progress edit
    /// is held in [`State::worktree_rename_draft`].
    RenameWorktree,
    /// The confirm-remove dialog for a session is shown (bugfix BUG-003, FR-015c). The target
    /// session is held in [`State::session_remove_target`].
    ConfirmSessionRemove,
    /// The confirm-forget dialog for a project is shown (feature 014, FR-002). The target
    /// project is held in [`State::forget_target`].
    ConfirmForgetProject,
}

/// Transient creation status for the add-worktree form (feature 010, research R4). Not
/// persisted — reset to `Editing` whenever the form is (re)opened.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorktreeFormStatus {
    /// The user is filling in the form; no create is in flight.
    #[default]
    Editing,
    /// `WorktreeCreateStarted` was dispatched; the async create (including any submodule
    /// fetch) is running. The form shows a "Creating worktree…" state and disables submission.
    Creating,
}

/// Which half of the add-worktree form is active (feature 016, FR-010).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BranchSource {
    /// Type + ticket + name inputs — a brand-new branch. Today's form.
    #[default]
    New,
    /// Pick from the branches that already exist (User Story 2).
    Existing,
}

/// The conflict-resolution sub-state of the add-worktree form (feature 016, contract
/// `branch-conflict.md` §3).
///
/// Lives INSIDE the form rather than as its own [`Overlay`] variant: `Overlay` holds one modal at
/// a time, so routing the prompt through it would tear down the form — and with it the inputs
/// FR-007 requires to survive a cancel (research R9).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ResolutionState {
    /// No prompt showing.
    #[default]
    Idle,
    /// Pre-flight found something; the user is choosing what to do (FR-002).
    Choosing { situation: BranchSituation },
    /// Overwrite was chosen; the destructive confirmation is showing (FR-005).
    ConfirmingOverwrite { situation: BranchSituation },
}

impl ResolutionState {
    /// The situation being resolved, if any.
    pub fn situation(&self) -> Option<&BranchSituation> {
        match self {
            ResolutionState::Idle => None,
            ResolutionState::Choosing { situation }
            | ResolutionState::ConfirmingOverwrite { situation } => Some(situation),
        }
    }

    /// Whether a prompt is currently awaiting the user.
    pub fn is_prompting(&self) -> bool {
        !matches!(self, ResolutionState::Idle)
    }
}

/// In-progress add-worktree form state, present only while the form overlay is open (FR-005).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeForm {
    /// Selected Conventional-Commits type (FR-005a).
    pub type_: Option<ConventionalType>,
    /// Optional ticket reference (FR-005b).
    pub ticket: String,
    /// Free-text name.
    pub name: String,
    /// The last validation error shown after a rejected submit.
    pub error: Option<NamingError>,
    /// Whether a create is in flight (feature 010, data-model.md).
    pub status: WorktreeFormStatus,
    /// Which half of the form is active (feature 016, FR-010).
    pub source: BranchSource,
    /// Branches that already exist, listed when `source` becomes `Existing` (FR-011). Empty
    /// until then — the listing is not run on every keystroke.
    pub candidates: Vec<BranchCandidate>,
    /// The picked existing branch, if any (FR-014).
    pub selected_branch: Option<BranchCandidate>,
    /// The conflict prompt's state (feature 016, FR-001/FR-005).
    pub resolution: ResolutionState,
    /// The mode the in-flight create is running under. Set when the create is sent, and read
    /// only to word [`Self::stage`] — a reuse must not say "Creating branch" (FR-024).
    pub mode: CreateMode,
    /// The stage the daemon last reported for the in-flight create (feature 016, FR-024).
    /// `None` until the first `OperationProgress` arrives; reset when a new attempt starts.
    pub stage: Option<CreateStage>,
}

impl WorktreeForm {
    /// The live derived directory/branch preview, or the validation error (FR-008a).
    ///
    /// Under [`BranchSource::Existing`] the names come from the selected branch instead of the
    /// type/ticket/name inputs (feature 016, FR-014), so the user sees the directory that will
    /// be created before committing to it.
    pub fn preview(&self) -> Result<DerivedNames, NamingError> {
        match self.source {
            BranchSource::New => derive(&WorktreeNaming {
                type_: self.type_,
                ticket: if self.ticket.trim().is_empty() {
                    None
                } else {
                    Some(self.ticket.clone())
                },
                name: self.name.clone(),
            }),
            BranchSource::Existing => {
                let candidate = self
                    .selected_branch
                    .as_ref()
                    .ok_or(NamingError::EmptyNameAfterSlug)?;
                let dir_name = dir_name_from_branch(&candidate.name);
                if dir_name.is_empty() {
                    return Err(NamingError::EmptyNameAfterSlug);
                }
                Ok(DerivedNames {
                    dir_name,
                    branch: candidate.name.clone(),
                })
            }
        }
    }

    /// Plain-language description of what the create is currently doing (FR-024), or `None`
    /// before the first stage lands.
    pub fn stage_label(&self) -> Option<&'static str> {
        self.stage.map(|s| s.label(&self.mode))
    }

    /// Whether the form can be submitted right now.
    ///
    /// A blocked candidate is deliberately still *selectable* (research R8 — `pick_list` has no
    /// per-item disabling, and forking a list widget is what the Component-reuse gate rejects),
    /// so the refusal happens here, at the point of action (FR-012).
    pub fn can_submit(&self) -> bool {
        if self.status != WorktreeFormStatus::Editing || self.resolution.is_prompting() {
            return false;
        }
        if let Some(candidate) = &self.selected_branch {
            if self.source == BranchSource::Existing && !candidate.is_available() {
                return false;
            }
        }
        self.preview().is_ok()
    }

    /// The mode implied by picking a candidate outright, when no prompt is needed
    /// (contract `branch-picker.md` §5).
    ///
    /// Picking a branch IS the intent to use it — but never the intent to destroy it, so this
    /// can never yield [`CreateMode::Overwrite`].
    /// `preferred_remote` is the remote the user already named by picking a specific row in the
    /// branch list. When the name exists on several remotes and no preference is given, this
    /// returns `None` so the prompt opens and the user chooses — the app must never pick a
    /// remote on the user's behalf (spec Edge Cases).
    pub fn mode_for(
        situation: &BranchSituation,
        preferred_remote: Option<&str>,
    ) -> Option<CreateMode> {
        match situation {
            BranchSituation::Free => Some(CreateMode::NewBranch),
            BranchSituation::LocalAvailable { .. } => Some(CreateMode::ReuseLocal),
            BranchSituation::RemoteOnly { remotes, .. } => {
                let remote = match preferred_remote {
                    // Honour the picked row, but only if that remote really carries the ref.
                    Some(preferred) if remotes.iter().any(|r| r == preferred) => preferred,
                    // Unambiguous: exactly one remote has it.
                    None if remotes.len() == 1 => remotes[0].as_str(),
                    // Ambiguous, or a preference that no longer holds — ask.
                    _ => return None,
                };
                Some(CreateMode::TrackRemote {
                    remote: remote.to_string(),
                })
            }
            BranchSituation::Blocked { .. } | BranchSituation::DirectoryTaken { .. } => None,
        }
    }
}

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

/// The kind of text selection to begin (feature 006, FR-013): single click = character range,
/// double = semantic (word), triple = whole line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectKind {
    /// Character-range selection (single click-drag).
    Simple,
    /// Semantic (word) selection (double click).
    Semantic,
    /// Whole-line selection (triple click).
    Lines,
}

/// In-progress Settings form state, present only while the Settings overlay is open (feature
/// 006, FR-020). The scrollback field is edited as text and validated/parsed on save.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsDraft {
    /// The editable scrollback-limit value (parsed/validated on save).
    pub scrollback_lines: String,
    /// Whether environment-include is enabled (feature 011, FR-001).
    pub env_include_enabled: bool,
    /// The editable environment-include script path (FR-002).
    pub env_include_script_path: String,
    /// The editable environment-include timeout, in seconds as text (parsed/validated on save,
    /// FR-003).
    pub env_include_timeout: String,
    /// The last validation error shown after a rejected save.
    pub error: Option<String>,
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

/// In-progress worktree-rename state, present only while the worktree-rename dialog is open
/// (feature 008, FR-013/FR-014). Mirrors [`RenameDraft`] but is keyed by worktree `dir_name`
/// and only ever changes the displayed name — never the folder or branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRenameDraft {
    /// The worktree being renamed, by `dir_name`.
    pub dir_name: String,
    /// The current editable display name.
    pub text: String,
    /// The last validation error, if the user tried to confirm an invalid name.
    pub error: Option<RenameError>,
}

/// Every user interaction that can change application state.
///
/// No longer `Copy`: some variants carry owned data (`PathBuf`, listing results).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// The "Help" toolbar entry was selected (reveals/collapses its "About" action).
    HelpMenuToggled,
    /// The "About" action was activated.
    AboutOpened,
    /// The About dialog was dismissed (Close button or Esc).
    AboutClosed,
    /// Open the project selector (folder browser). The binary computes the starting
    /// directory and launches the first directory scan (FR-001).
    ProjectSelectorOpened,
    /// Navigate the selector into a subfolder (FR-002).
    SelectorNavigatedInto(PathBuf),
    /// Navigate the selector to the parent directory (FR-002).
    SelectorNavigatedUp,
    /// A directory scan completed; populate the current listing (FR-006).
    SelectorListingReady(Vec<FolderEntry>),
    /// A directory scan failed (e.g. permission denied); show an error, do not crash.
    SelectorListingFailed(String),
    /// Open the currently-browsed folder as a project (FR-003, FR-005). The binary
    /// records git status/availability and persists the catalog.
    FolderChosen(PathBuf),
    /// Dismiss the project selector without choosing (Cancel button or Esc).
    ProjectSelectorClosed,
    /// Reopen a known project from the list without browsing (FR-011). The binary
    /// refreshes its availability, activates it if available (FR-023), and persists.
    KnownProjectReopened(PathBuf),
    /// Begin renaming the given project; opens the rename dialog (FR-017).
    RenameStarted(PathBuf),
    /// The rename dialog's text changed.
    RenameTextChanged(String),
    /// Confirm the rename. Applies it if valid (FR-020); the binary then persists.
    RenameConfirmed,
    /// Dismiss the rename dialog without applying (Cancel or Esc).
    RenameCancelled,

    // ---- Feature 014: forget a project ----
    /// Request to forget the project at this path; opens the confirm dialog (FR-002).
    ProjectForgetRequested(PathBuf),
    /// Confirm forgetting. The binary stops the project's live session processes, the reducer
    /// drops the record + metadata and clears the active working space if it was active, then the
    /// binary persists and deletes the project's per-project state file (FR-003/005/007/008/010).
    ProjectForgetConfirmed,
    /// Dismiss the forget confirmation without removing anything (FR-004).
    ProjectForgetCancelled,

    // ---- Feature 015: forget from the switcher's right-click menu ----
    /// The pointer moved to this window-pixel position. Emitted by the binary only while the
    /// project switcher is open, so a right-click can anchor its menu at the cursor.
    CursorMoved { x: u16, y: u16 },
    /// The window was resized (or reported its initial size). Feeds context-menu clamping.
    WindowResized { width: u16, height: u16 },
    /// Open (or close, if already open) a project's switcher right-click context menu, by path.
    /// Anchored at the last known [`State::cursor`]. The switcher panel stays open behind it;
    /// the other popovers are mutually exclusive.
    ProjectMenuToggled(PathBuf),
    /// Dismiss the project context menu (outside click, or after an action is chosen).
    ProjectMenuDismissed,
    /// The user selected a theme preference (Follow system / Light / Dark) (FR-007, FR-008).
    /// The binary persists the updated preference afterward.
    ThemePreferenceChanged(ThemePreference),
    /// Cycle the theme mode to the next one (Auto → Light → Dark → Auto) from the toolbar
    /// menu's mode toggle. The binary persists the updated preference; the menu stays open.
    ThemeModeCycled,
    /// The OS light/dark preference poll observed a (changed) scheme (FR-006). Transient;
    /// never persisted. Carries the raw detection outcome — `Err(())` for a transient failure
    /// (e.g. `dark_light::detect()` timing out under CPU load) — rather than an
    /// already-resolved `SystemScheme`, specifically so the periodic poll's `Subscription::map`
    /// closure (`os_theme_poll`, `src/main.rs`) does not need to capture the previous scheme:
    /// iced panics if a subscription's mapping closure captures state, since that breaks the
    /// stable identity it relies on to avoid restarting the underlying timer every frame. The
    /// reducer below applies the same last-known fallback (`theme::observe_system_scheme`)
    /// that used to be baked in at the call site instead.
    SystemThemeChanged(Result<SystemScheme, ()>),

    // ---- Feature 005: worktrees, sessions, embedded terminal ----
    /// Opening a directory as a project was refused because it is not a git repo (FR-001a).
    /// The binary performs the `Git::is_repo_root` check and dispatches this on refusal.
    ProjectOpenRefused(String),
    /// The binary discovered/re-discovered the active project's worktrees (FR-018).
    WorktreesLoaded(Vec<Worktree>),
    /// Expand/collapse a worktree's session sub-items (FR-003), by `dir_name`.
    WorktreeExpansionToggled(String),
    /// Expand/collapse the "Default" (project-root) entry's session sub-items (feature 010,
    /// mirrors `WorktreeExpansionToggled`).
    DefaultExpansionToggled,

    // ---- Feature 008: worktree sidebar refinement ----
    /// Open (or close, if already open) a worktree's right-click context menu, by `dir_name`.
    WorktreeMenuToggled(String),
    /// Dismiss the worktree context menu (outside click, or after an action is chosen).
    WorktreeMenuDismissed,
    /// Request deletion of a worktree; opens the confirm dialog (FR-018), by `dir_name`.
    WorktreeDeleteRequested(String),
    /// Confirm deletion. The binary terminates the worktree's sessions, removes its git
    /// worktree + branch and directory, then persists (FR-020); the reducer drops the records.
    WorktreeDeleteConfirmed,
    /// Dismiss the delete confirmation without removing anything (FR-021).
    WorktreeDeleteCancelled,
    /// The delete confirmation's "also delete the branch" choice changed (feature 013,
    /// FR-011/FR-012).
    WorktreeDeleteKeepBranchToggled(bool),
    /// Begin renaming a worktree's displayed name; opens the rename dialog (FR-013), by `dir_name`.
    WorktreeRenameStarted(String),
    /// The worktree-rename dialog's text changed.
    WorktreeRenameTextChanged(String),
    /// Confirm the worktree rename. Applies the display-name override if valid (FR-014); the
    /// binary then persists (FR-015).
    WorktreeRenameConfirmed,
    /// Dismiss the worktree-rename dialog without applying.
    WorktreeRenameCancelled,
    /// Toggle a tag filter on/off in the sidebar (FR-024).
    SidebarFilterToggled(TagFilter),
    /// Clear all active tag filters, restoring the full list (FR-026).
    SidebarFiltersCleared,
    /// Toggle the sidebar's tag-filter panel open/closed (feature 009). Mutually exclusive
    /// with `help_menu_open` and `project_switcher_open`.
    SidebarFilterMenuToggled,
    /// Toggle whether agent-owned worktrees are included in the sidebar list (feature 014,
    /// FR-010). Sole mutation: `show_agent_worktrees`. Never touches the tag filters (FR-010d).
    ShowAgentWorktreesToggled,
    /// Content scrolled underneath an open floating surface (feature 017, FR-009). The third of
    /// the three dismissal triggers, and the one no widget used to report — see
    /// [`micold_core::overlay::Trigger::ScrollBeneath`]. Emitted unconditionally by the scrollable
    /// that moved; deciding whether anything closes is the reducer's job, via the shared rule.
    ScrolledBeneathOverlay,
    /// A dialog has finished animating out (feature 017, FR-011). Emitted by the `Modal` component
    /// itself, which owns the transition, so the binary can release the snapshot it was rendering
    /// from ([`ClosingOverlay`]). The binary used to watch a central progress value for this; the
    /// component now says it, which is the only part of a transition an application still needs.
    OverlayTransitionFinished,
    /// The pointer entered a worktree row (feature 008), by `dir_name`; reveals its row actions.
    WorktreeHovered(String),
    /// The pointer left a worktree row (feature 008), by `dir_name`; hides its row actions.
    WorktreeUnhovered(String),
    /// Copy arbitrary displayed text (e.g. a worktree name) to the system clipboard. The binary
    /// performs the actual clipboard write; the reducer has no state to update.
    TextCopyRequested(String),
    /// Open the add-worktree form (FR-005).
    AddWorktreeOpened,
    /// The form's type selection changed.
    AddWorktreeTypeSelected(ConventionalType),
    /// The form's ticket field changed.
    AddWorktreeTicketChanged(String),
    /// The form's name field changed.
    AddWorktreeNameChanged(String),
    /// Submit the form (FR-006). Validation happens here; the binary performs the git create.
    AddWorktreeSubmitted,
    /// Dismiss the form without creating (Cancel or Esc).
    AddWorktreeCancelled,
    /// Switch between the new-branch and existing-branch halves of the form (feature 016,
    /// FR-010). Switching back to `New` clears any selection (FR-015).
    AddWorktreeSourceChanged(BranchSource),
    /// The binary listed the repository's branches for the picker (feature 016, FR-011).
    AddWorktreeBranchesListed(Vec<BranchCandidate>),
    /// An existing branch was picked from the list (feature 016, FR-014). A blocked candidate is
    /// still selectable — submission is what refuses (FR-012).
    AddWorktreeBranchSelected(BranchCandidate),
    /// Pre-flight found something the user must decide about; raise the prompt (feature 016,
    /// FR-001). Never dispatched for [`BranchSituation::Free`].
    AddWorktreeConflictDetected(BranchSituation),
    /// The user answered the prompt (feature 016, FR-002). The binary performs the create with
    /// the chosen mode. `Overwrite` can only arrive via [`Message::AddWorktreeOverwriteConfirmed`].
    AddWorktreeResolutionChosen(CreateMode),
    /// The user chose Overwrite; show the destructive confirmation first (feature 016, FR-005).
    AddWorktreeOverwriteRequested,
    /// The destructive confirmation was accepted (feature 016, FR-005).
    AddWorktreeOverwriteConfirmed,
    /// Back out of the prompt (or its confirmation) without acting (feature 016, FR-007).
    AddWorktreeResolutionCancelled,
    /// The binary is about to send the `WorktreeCreate` RPC (feature 010; T055); marks the form
    /// `Creating` so it shows an in-progress state until the daemon's reply closes or reopens it.
    /// Carries the mode so the stage display can be worded for it (feature 016, FR-024).
    WorktreeCreateStarted(CreateMode),
    /// The daemon reported that the in-flight create entered a new stage (feature 016, FR-024).
    /// Ignored once the form has closed.
    WorktreeCreateStageChanged(CreateStage),
    /// The daemon created a worktree successfully (FR-007); add it and close the form.
    WorktreeCreated(Worktree),
    /// The daemon reported a worktree create failure (FR-017); show it, keep the form open.
    WorktreeCreateFailed(String),
    /// Start a new session at the given location — a worktree or, as of feature 010, the
    /// project root ("Default", FR-001) — (FR-010). The binary spawns `claude`.
    SessionStartRequested { location: SessionLocation },
    /// A session was started/added for the active project (FR-011).
    SessionStarted(Session),
    /// Select a session to display its terminal (FR-015); other sessions keep running.
    SessionSelected(SessionId),
    /// Close/stop a session (FR-015a, bugfix BUG-003). The binary kills the process and records
    /// the durable suppression marker; this archives (not deletes) the record.
    SessionCloseRequested(SessionId),
    /// The session's `claude` process reported it is running (FR-010).
    SessionRunning(SessionId),
    /// The session's `claude` title became available/changed (FR-011a).
    SessionTitleUpdated { id: SessionId, title: String },

    // ---- Bugfix BUG-003: session Remove (distinct from Close/archive) ----
    /// Open (or close, if already open) a session's right-click context menu.
    SessionMenuToggled(SessionId),
    /// Dismiss the session context menu (outside click, or after an action is chosen).
    SessionMenuDismissed,
    /// Request permanent removal of a session; opens the confirm dialog (FR-015c).
    SessionRemoveRequested(SessionId),
    /// Confirm removal. The binary kills the process (if running) and records the durable
    /// suppression marker, then persists; the reducer drops the record outright.
    SessionRemoveConfirmed,
    /// Dismiss the remove confirmation without removing anything.
    SessionRemoveCancelled,

    // ---- Feature 010: switchable regular terminal mode ----
    /// The mode toggle was pressed for the active session (FR-001–FR-004, FR-010).
    TerminalModeToggled,
    /// The manual restart affordance was pressed for the active session's currently-attached,
    /// not-running process — for the AI CLI branch, and for whichever Regular Terminal instance
    /// is currently active (FR-013; contracts/terminal-mode-lifecycle.md).
    TerminalRestartRequested,

    // ---- Feature 011: multiple Regular Terminal instances per session ----
    /// Open an additional Regular Terminal instance for the active session (the "+" control or
    /// the `Ctrl+Shift+T`/`Cmd+Shift+T` shortcut, FR-001, FR-019) — a no-op outside Regular mode.
    /// No pure reducer body: mirrors `TerminalRestartRequested`/`SessionStartRequested`, which
    /// only trigger binary-side spawn logic.
    ShellInstanceOpenRequested,
    /// Switch the visible pane to a different open Regular Terminal instance of `SessionId`
    /// (FR-004; the instance-switching control in `pane()`). Carries the owning session
    /// explicitly, not just the instance id, because `ShellInstanceId` is only unique within its
    /// own session — resolving against whatever `active_session` happens to be when this message
    /// is processed (rather than the session it was actually raised for) could otherwise apply it
    /// to a same-numbered instance of a different session if the active session changes first.
    ShellInstanceSelected(SessionId, ShellInstanceId),
    /// Close an individual Regular Terminal instance of `SessionId` (FR-011–FR-013) — may flip
    /// that session's `mode` back to `AiCli` if this was the last remaining instance. Carries the
    /// owning session explicitly for the same reason as `ShellInstanceSelected`.
    ShellInstanceCloseRequested(SessionId, ShellInstanceId),
    /// Manually restart a specific Regular Terminal instance of `SessionId` after it exited —
    /// independent of whether that instance is the one currently attached to the pane (FR-010; a
    /// background instance can be restarted without first switching to it). No pure reducer
    /// body: mirrors `TerminalRestartRequested`, which only triggers binary-side spawn logic.
    /// Carries the owning session explicitly for the same reason as `ShellInstanceSelected`.
    ShellInstanceRestartRequested(SessionId, ShellInstanceId),
    /// A Regular Terminal instance reported it is running (feature 011; replaces feature 010's
    /// `ShellSessionRunning(SessionId)`, now id-addressed since a session may have more than one
    /// instance).
    ShellInstanceRunning(SessionId, ShellInstanceId),
    /// A Regular Terminal instance's shell process exited (intentional or crash) — never
    /// auto-restarted (FR-008; replaces feature 010's `ShellSessionExited(SessionId)`).
    ShellInstanceExited(SessionId, ShellInstanceId),

    /// Periodic redraw tick while a terminal is live (drives streamed-output repaint).
    TerminalTick,
    /// Hide or show the sidebar (toggle).
    SidebarToggled,
    /// The resize handle was dragged; carries the pointer x in pixels. The handle owns the drag
    /// itself, so there is no start or end to report — only where the edge now is.
    SidebarDragMoved(u16),
    /// The OS window gained (`true`) or lost (`false`) input focus. Handled by the binary,
    /// which gates the terminal/OS-theme poll subscriptions on it so a backgrounded window
    /// doesn't keep burning CPU on ticks nothing is looking at (idle-CPU fix).
    WindowFocusChanged(bool),

    // ---- Feature 006: real terminal behavior ----
    /// The terminal pane gained input focus (explicit click/action) (FR-010).
    TerminalFocused,
    /// Focus was released back to the app (reserved chord / click-outside / affordance) (FR-011).
    TerminalFocusReleased,
    /// Bytes to write to the focused session's PTY (from `keymap::encode` / paste). The binary
    /// writes them only when the session is Running (FR-008, FR-012a).
    TerminalBytes(Vec<u8>),
    /// Begin a text selection at a viewport grid cell (feature 006 mouse, FR-013/FR-013b).
    TerminalSelectStart {
        col: u16,
        line: u16,
        kind: SelectKind,
    },
    /// Extend the in-progress text selection to a viewport grid cell (FR-013).
    TerminalSelectUpdate { col: u16, line: u16 },
    /// Clear the current text selection.
    TerminalSelectCleared,
    /// Scroll the displayed terminal by N lines (+ up into scrollback) (FR-016).
    TerminalScrolled(i32),
    /// Scroll the displayed terminal to an absolute scrollback offset (0 = live bottom) (FR-016).
    /// Used by the scrollbar drag: the delta is resolved against the live offset at apply time, so
    /// batched drag events set the target instead of accumulating relative deltas (drag flicker).
    TerminalScrolledTo(usize),
    /// The terminal pane's visible size changed; resize the PTY + grid (FR-014, FR-015).
    TerminalResized { cols: u16, rows: u16 },
    /// Copy the current terminal selection to the clipboard (binary handles clipboard) (FR-013).
    TerminalCopyRequested,
    /// Paste clipboard text into the focused session's PTY (binary handles clipboard) (FR-013).
    TerminalPasteRequested,
    /// Open the terminal right-click context menu at a pane-local pixel point (FR-013).
    TerminalContextMenuOpened { x: u16, y: u16 },
    /// Dismiss the terminal context menu (an outside click, or after an item is chosen) (FR-013).
    TerminalContextMenuClosed,
    /// Open the Settings form (from the toolbar menu) (FR-019). The binary seeds the draft with
    /// the current scrollback value.
    SettingsOpened,
    /// The Settings scrollback field changed.
    SettingsScrollbackChanged(String),
    /// The Settings environment-include enabled checkbox was toggled (feature 011, FR-001).
    SettingsEnvIncludeEnabledToggled(bool),
    /// The Settings environment-include script path field changed (FR-002).
    SettingsEnvIncludePathChanged(String),
    /// The Settings environment-include timeout field changed (FR-003).
    SettingsEnvIncludeTimeoutChanged(String),
    /// Save the Settings form (validated + persisted by the binary) (FR-020, FR-021).
    SettingsSaved,
    /// Dismiss the Settings form without saving (Cancel or Esc).
    SettingsCancelled,

    // ---- Feature 008: background project switching ----
    /// Toggle the top-bar project switcher panel (feature 008, FR-004). Mutually exclusive
    /// with the overflow menu.
    ProjectSwitcherToggled,

    // ---- Global notification surface ----
    /// Dismiss the notification at this index. Out-of-range indices are ignored, so a stale
    /// click delivered after the list shrank is harmless.
    NotificationDismissed(usize),

    // ---- Feature 010: daemon connection (client of the daemon-hosted sessions) ----
    /// The daemon connection is up: the binary stores the [`Outbox`] to drive sessions and adopts
    /// the welcome catalog/settings. Handled by the binary (the `Outbox` is runtime, not core).
    DaemonConnected {
        /// Handle for sending `ClientMsg`s to the daemon.
        outbox: crate::daemon::Outbox,
        /// The catalog as of the handshake.
        catalog: micold_core::protocol::messages::CatalogSnapshot,
        /// The service-owned settings.
        settings: micold_core::protocol::messages::DaemonSettings,
    },
    /// A control message pushed by the daemon (catalog/settings changes, operation results, …).
    /// Handled by the binary.
    DaemonEvent(micold_core::protocol::messages::DaemonMsg),
    /// A grid frame for the viewed session (full snapshot or delta). Handled by the binary, applied
    /// into the per-session grid cache.
    DaemonGridFrame(micold_core::protocol::grid::GridFrame),
    /// The daemon connection dropped; the binary clears its outbox until it reconnects.
    DaemonDisconnected,
    /// Connecting to (or spawning) the daemon failed, with a human-facing reason.
    DaemonConnectFailed(String),
    /// The user asked to take the active project back after being displaced (US5, FR-024): re-attach
    /// with `force`. Handled by the binary (attachment is runtime).
    ConnectionTakeoverRequested,
    /// The daemon refused the handshake on a contract mismatch (US6, FR-021): carries both protocol
    /// versions and the daemon build so the client can render an actionable diagnostic. Handled by
    /// the binary.
    DaemonVersionMismatch {
        /// This client's protocol version.
        client: u32,
        /// The running daemon's protocol version.
        daemon: u32,
        /// The running daemon's human-facing build string.
        daemon_build: String,
    },
    /// The daemon refused the handshake on a same-contract package-version difference (US6,
    /// FR-022a, BUG-002): the wire contract matches, but a `.deb` upgrade installed a newer build
    /// than the one still running. Carries both build strings so the client can render a distinct,
    /// lower-severity diagnostic than [`Message::DaemonVersionMismatch`]. Handled by the binary.
    DaemonBuildMismatch {
        /// This client's human-facing build string.
        client_build: String,
        /// The running daemon's human-facing build string.
        daemon_build: String,
    },
    /// The user chose "restart service" after a version or build mismatch (US6, FR-022/022a): stop
    /// the mismatched daemon so the auto-reconnect spawns a matching one. Handled by the binary.
    ConnectionRestartServiceRequested,
    /// A completed side-effecting task that carries nothing to apply (e.g. the daemon-stop task).
    NoOp,
    /// The user asked to see where the session service logs and its recent errors (Phase 10, FR-046).
    /// Handled by the binary: it requests both from the daemon and shows the answers as notices.
    DiagnosticsRequested,
    /// The user asked to make sessions survive logout (US7, FR-038; Linux only). Handled by the
    /// binary, which runs the enable flow off-thread. Never triggered by install — a deliberate choice.
    LogoutSurvivalRequested,
    /// The logout-survival enable flow finished; carries a ready-to-show message (info or error).
    LogoutSurvivalOutcome(String),
}

/// How prominently a [`Notification`] is presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    /// Something happened that the user should know about, but nothing failed.
    Info,
    /// An action the user asked for could not be completed.
    Error,
}

/// A transient, user-visible message not owned by any modal.
///
/// Exists because every feature that needed to report a failure invented its own error field
/// with a single modal-specific render site, and those sites became unreachable as the UI grew
/// (a session that fails to start, a folder that is refused). Notifications render
/// unconditionally in [`crate::ui::view`], outside every branch that can bypass them, so a
/// message pushed here cannot be silently swallowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub level: NoticeLevel,
    pub message: String,
}

/// Root application state for the single main window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The currently displayed modal overlay.
    pub overlay: Overlay,
    /// Whether the Help menu is currently expanded (transient UI affordance).
    pub help_menu_open: bool,
    /// The known-projects catalog and the active working space (persisted). Per-story
    /// selector/rename working state is added alongside those stories.
    pub workspace: micold_core::workspace::Workspace,
    /// The folder-browser state, present only while the project-selector overlay is shown.
    pub selector: Option<Selector>,
    /// The in-progress rename, present only while the rename overlay is shown.
    pub rename_draft: Option<RenameDraft>,
    /// How the app chooses its theme (persisted); defaults to following the OS (FR-005).
    pub theme_pref: ThemePreference,
    /// The last light/dark scheme reported by the OS poll (transient, not persisted).
    pub system_scheme: SystemScheme,
    /// Worktrees discovered for the active project (feature 005, FR-018). Re-derived from git
    /// on open and after each mutation — never persisted.
    pub worktrees: Vec<Worktree>,
    /// Which worktree rows are expanded to reveal their sessions (FR-003). By `dir_name`.
    pub expanded: BTreeSet<String>,
    /// Whether the "Default" (project-root) sidebar row is expanded to reveal its sessions
    /// (feature 010, mirrors `expanded` for worktree rows — a dedicated field rather than a
    /// sentinel key in `expanded`, since there is always exactly one Default row).
    pub default_expanded: bool,
    /// The currently displayed session, if any (FR-012, FR-015).
    pub active_session: Option<SessionId>,
    /// The add-worktree form, present only while its overlay is shown (FR-005).
    pub worktree_form: Option<WorktreeForm>,
    /// A message shown when opening a non-git directory was refused (FR-001a), or a worktree
    /// create failed (FR-017). Transient.
    pub worktree_error: Option<String>,
    /// Whether the sidebar is collapsed/hidden. Default (`false`) is visible.
    pub sidebar_hidden: bool,
    /// The sidebar width in pixels. `0` means "use the default width" (see [`State::sidebar_width_px`]).
    pub sidebar_width: u16,
    /// Whether the embedded terminal holds input focus (feature 006). Default `false`; keys are
    /// delivered to the session process only while `true` (FR-009/FR-010/FR-012).
    pub terminal_focused: bool,
    /// The open terminal right-click context menu's anchor in pane-local pixels, or `None` when
    /// no menu is showing (feature 006, FR-013).
    pub terminal_context_menu: Option<(u16, u16)>,
    /// In-progress Settings form, present only while the Settings overlay is shown (feature 006).
    pub settings_draft: Option<SettingsDraft>,
    /// The session that was in the foreground for each project, remembered so returning to a
    /// project restores it. In-memory only; not persisted (research R2).
    pub foreground_by_project: BTreeMap<PathBuf, SessionId>,
    /// Sessions that were auto-restarted while their project was inactive, pending a return
    /// notification. Cleared when the user returns to the owner.
    pub restarted_while_inactive: BTreeSet<SessionId>,
    /// Global messages, newest last. Rendered unconditionally so no failure can be swallowed by
    /// an unreachable render path — see [`Notification`]. Never persisted.
    ///
    /// A message stays until the user dismisses it or it is evicted by newer ones. Nothing
    /// clears these implicitly: a report that vanishes on unrelated activity (a background
    /// worktree re-scan, say) is how these failures became invisible in the first place.
    pub notifications: Vec<Notification>,
    /// Whether the top-bar project switcher panel is open. Mutually exclusive with
    /// `help_menu_open`.
    pub project_switcher_open: bool,
    /// The open project right-click context menu (feature 015), with the project it acts on and
    /// the cursor anchor to draw it at. At most one is open. Mutually exclusive with the other
    /// popovers, but the switcher panel itself stays open behind it. Transient — not persisted.
    pub project_menu_open: Option<ProjectMenu>,
    /// Last known pointer position in window pixels (feature 015). Tracked only while the
    /// project switcher is open — see the binary's cursor subscription — purely so a right-click
    /// can anchor its context menu at the cursor. Transient — not persisted.
    pub cursor: (u16, u16),
    /// Last known window size in pixels (feature 015), used to clamp a context menu so it cannot
    /// open off-screen. `(0, 0)` means "not reported yet", which disables clamping. Transient.
    pub window_size: (u16, u16),
    /// The worktree whose right-click context menu is open, by `dir_name` (feature 008). At
    /// most one is open at a time; `None` means no menu is showing.
    pub worktree_menu_open: Option<String>,
    /// The worktree pending deletion (its `dir_name`), shown in the confirm dialog (feature
    /// 008, FR-018/FR-019). Present only while [`Overlay::ConfirmWorktreeDelete`] is shown.
    pub worktree_delete_target: Option<String>,
    /// Whether the user has opted to also delete the branch when confirming a worktree delete
    /// (feature 013). Defaults to `false` = delete (today's unconditional behavior), so an
    /// unmodified confirm is unchanged. Reset to `false` on every `WorktreeDeleteRequested`.
    pub worktree_delete_keep_branch: bool,
    /// The in-progress worktree rename, present only while [`Overlay::RenameWorktree`] is shown
    /// (feature 008, FR-013/FR-014).
    pub worktree_rename_draft: Option<WorktreeRenameDraft>,
    /// Active sidebar tag filters (feature 008, FR-024). Empty ⇒ all worktrees shown. Multiple
    /// filters combine with OR (FR-025). Transient — not persisted.
    pub sidebar_filters: BTreeSet<TagFilter>,
    /// Whether the sidebar's tag-filter panel is shown (feature 009, FR-002/FR-003). Mutually
    /// exclusive with `help_menu_open`/`project_switcher_open`. Transient — not persisted;
    /// closing it never alters `sidebar_filters` (FR-007/FR-008).
    pub sidebar_filter_open: bool,
    /// Whether agent-owned worktrees are included in the sidebar list (feature 014, FR-010).
    /// `false` = hidden, the safe default.
    ///
    /// Transient AND project-scoped: never persisted, so every app start begins hidden (FR-010a),
    /// and reset in [`Self::restore_after_activation`] so a project switch begins hidden too
    /// (FR-010e). Deliberately unlike `sidebar_filters`, which survives a switch — view state
    /// switched on for one project must not silently render in another.
    pub show_agent_worktrees: bool,
    /// The worktree row the pointer is currently over, by `dir_name` (feature 008). Drives the
    /// hover-revealed row actions (add-session + delete). Transient.
    pub hovered_worktree: Option<String>,
    /// The session whose right-click context menu is open (bugfix BUG-003). At most one is open
    /// at a time; `None` means no menu is showing. Mirrors `worktree_menu_open`.
    pub session_menu_open: Option<SessionId>,
    /// The session pending permanent removal, shown in the confirm dialog (bugfix BUG-003,
    /// FR-015c). Present only while [`Overlay::ConfirmSessionRemove`] is shown. Mirrors
    /// `worktree_delete_target`.
    pub session_remove_target: Option<SessionId>,
    /// The project pending a forget confirmation, by path (feature 014). Present only while
    /// [`Overlay::ConfirmForgetProject`] is shown. Transient — never persisted. Mirrors
    /// `worktree_delete_target`.
    pub forget_target: Option<PathBuf>,
}

impl State {
    /// The color scheme to render, resolved from the user's preference and the OS scheme
    /// (FR-005, FR-007, FR-018). See [`micold_core::theme::resolve`].
    pub fn color_scheme(&self) -> ColorScheme {
        resolve(self.theme_pref, self.system_scheme)
    }

    /// The most notifications kept at once. Older ones are dropped rather than growing a
    /// banner stack tall enough to crowd out the application.
    const MAX_NOTIFICATIONS: usize = 3;

    /// Surface a failed action to the user (see [`Notification`]).
    ///
    /// Use this for anything the user asked for that could not be completed. Do not add a new
    /// error field with its own render site — that is the pattern that produced the silent
    /// failures this surface replaces.
    pub fn notify_error(&mut self, message: impl Into<String>) {
        self.push_notification(NoticeLevel::Error, message.into());
    }

    /// Surface something the user should know about that is not a failure.
    pub fn notify_info(&mut self, message: impl Into<String>) {
        self.push_notification(NoticeLevel::Info, message.into());
    }

    fn push_notification(&mut self, level: NoticeLevel, message: String) {
        let notification = Notification { level, message };
        // Repeating an action that keeps failing should not stack identical banners.
        if self.notifications.contains(&notification) {
            return;
        }
        self.notifications.push(notification);
        if self.notifications.len() > Self::MAX_NOTIFICATIONS {
            self.notifications.remove(0);
        }
    }

    /// Open a modal overlay, closing any lightweight popover first. The two are meant to be
    /// mutually exclusive (`on_escape` and the keyboard subscription both assume it — feature
    /// 009 code review), but before this helper existed each overlay-opening arm had to
    /// remember to reset the popovers by hand, and none of them reset `sidebar_filter_open`,
    /// so it was possible to open e.g. the Add Worktree form while the filter panel was still
    /// (invisibly) open, leaving Escape's two implementations disagreeing about what to
    /// dismiss. Routing every overlay-open through here makes that reset unconditional.
    pub fn open_overlay(&mut self, overlay: Overlay) {
        self.overlay = overlay;
        self.help_menu_open = false;
        self.project_switcher_open = false;
        self.sidebar_filter_open = false;
        self.project_menu_open = None;
    }

    /// Apply a [`Message`], transitioning the state. Pure and side-effect free.
    pub fn update(&mut self, message: Message) {
        match message {
            // Daemon connection messages are runtime, not pure state — the binary handles them in
            // `update_inner` and never routes them here. Listed explicitly (not a catch-all) so the
            // core reducer stays exhaustive over `Message` and a future variant is a compile error.
            Message::DaemonConnected { .. }
            | Message::DaemonEvent(_)
            | Message::DaemonGridFrame(_)
            | Message::DaemonDisconnected
            | Message::DaemonConnectFailed(_)
            | Message::ConnectionTakeoverRequested
            | Message::DaemonVersionMismatch { .. }
            | Message::DaemonBuildMismatch { .. }
            | Message::ConnectionRestartServiceRequested
            | Message::NoOp
            | Message::DiagnosticsRequested
            | Message::LogoutSurvivalRequested
            | Message::LogoutSurvivalOutcome(_) => {}
            Message::HelpMenuToggled => {
                self.help_menu_open = !self.help_menu_open;
                // The overflow menu, the project switcher, and the sidebar filter panel are
                // mutually exclusive (feature 009).
                self.project_switcher_open = false;
                self.sidebar_filter_open = false;
                // Mutually exclusive with the project context menu (feature 015).
                self.project_menu_open = None;
            }
            Message::ProjectSwitcherToggled => {
                self.project_switcher_open = !self.project_switcher_open;
                self.help_menu_open = false;
                self.sidebar_filter_open = false;
                // Mutually exclusive with the project context menu (feature 015).
                self.project_menu_open = None;
            }
            Message::AboutOpened => {
                // Idempotent: opening while already open keeps a single instance (FR-015).
                self.open_overlay(Overlay::About);
            }
            Message::AboutClosed => {
                // No-op when nothing is open (edge case); otherwise return to the
                // main window unchanged (FR-012).
                self.overlay = Overlay::None;
            }
            Message::SelectorNavigatedInto(path) => {
                if let Some(selector) = &mut self.selector {
                    selector.enter(path);
                }
            }
            Message::SelectorNavigatedUp => {
                if let Some(selector) = &mut self.selector {
                    selector.up();
                }
            }
            Message::SelectorListingReady(entries) => {
                if let Some(selector) = &mut self.selector {
                    selector.listing_ready(entries);
                }
            }
            Message::SelectorListingFailed(message) => {
                if let Some(selector) = &mut self.selector {
                    selector.listing_failed(message);
                }
            }
            Message::ProjectSelectorClosed => {
                self.overlay = Overlay::None;
                self.selector = None;
            }
            Message::RenameStarted(path) => {
                let current = self
                    .workspace
                    .projects
                    .iter()
                    .find(|p| p.path == path)
                    .map(|p| p.display_name.clone());
                if let Some(name) = current {
                    self.rename_draft = Some(RenameDraft {
                        path,
                        text: name,
                        error: None,
                    });
                    self.open_overlay(Overlay::RenameProject);
                }
            }
            Message::RenameTextChanged(text) => {
                if let Some(draft) = &mut self.rename_draft {
                    draft.text = text;
                    draft.error = None;
                }
            }
            Message::RenameConfirmed => {
                if let Some((path, text)) = self
                    .rename_draft
                    .as_ref()
                    .map(|draft| (draft.path.clone(), draft.text.clone()))
                {
                    match self.workspace.rename(&path, &text) {
                        // Renaming never touches disk — only the stored name (FR-018).
                        Ok(()) => {
                            self.overlay = Overlay::None;
                            self.rename_draft = None;
                        }
                        Err(error) => {
                            if let Some(draft) = &mut self.rename_draft {
                                draft.error = Some(error);
                            }
                        }
                    }
                }
            }
            Message::RenameCancelled => {
                self.overlay = Overlay::None;
                self.rename_draft = None;
            }
            Message::CursorMoved { x, y } => {
                self.cursor = (x, y);
            }
            Message::WindowResized { width, height } => {
                self.window_size = (width, height);
            }
            Message::ProjectMenuToggled(path) => {
                // Toggle: the same project closes; a different one replaces (only one open),
                // re-anchored at wherever the pointer now is. The switcher panel stays open
                // behind the menu (so the right-clicked row remains visible), but the other
                // popovers are mutually exclusive with it.
                self.project_menu_open = match &self.project_menu_open {
                    Some(open) if open.path == path => None,
                    _ => Some(ProjectMenu {
                        path,
                        anchor: self.cursor,
                    }),
                };
                self.help_menu_open = false;
                self.sidebar_filter_open = false;
                self.worktree_menu_open = None;
            }
            Message::ProjectMenuDismissed => {
                self.project_menu_open = None;
            }
            Message::ProjectForgetRequested(path) => {
                // Open the confirmation; nothing is removed until confirmed (FR-002).
                self.project_menu_open = None;
                self.forget_target = Some(path);
                self.open_overlay(Overlay::ConfirmForgetProject);
            }
            Message::ProjectForgetConfirmed => {
                // Drop the record + all per-path metadata (FR-003/FR-005). The binary has already
                // stopped the project's live processes and will persist + delete the per-project
                // state file after this pure transition.
                if let Some(path) = self.forget_target.clone() {
                    // If the forgotten project was the active working space, its active session
                    // pointer must be cleared too — `forget` clears `workspace.active`, so the
                    // dangling `active_session` (which only ever referenced the active project)
                    // would otherwise point at a project that no longer exists (FR-008).
                    let was_active = self.workspace.active.as_deref()
                        == Some(canonicalize_best_effort(&path).as_path());
                    self.workspace.forget(&path);
                    if was_active {
                        self.active_session = None;
                    }
                }
                self.forget_target = None;
                self.overlay = Overlay::None;
            }
            Message::ProjectForgetCancelled => {
                self.forget_target = None;
                self.overlay = Overlay::None;
            }
            Message::ThemeModeCycled => {
                // Advance to the next mode; the menu stays open so repeated clicks cycle.
                self.theme_pref = self.theme_pref.next();
            }
            Message::ThemePreferenceChanged(pref) => {
                // Pure state change; the binary persists it at the I/O boundary (FR-009).
                self.theme_pref = pref;
            }
            Message::SystemThemeChanged(detected) => {
                self.system_scheme = observe_system_scheme(detected, self.system_scheme);
            }
            Message::ProjectOpenRefused(message) => {
                // Non-git directory refused (FR-001a); the active project is unchanged.
                // Reported through the global surface, not `worktree_error`: the refusal
                // arrives with the selector already closed, so the Add Worktree modal that
                // owns `worktree_error` is not open and the message was never drawn.
                self.notify_error(message);
            }
            Message::WorktreesLoaded(worktrees) => {
                self.worktree_error = None;
                self.set_worktrees(worktrees);
            }
            Message::WorktreeExpansionToggled(dir) => {
                if !self.expanded.remove(&dir) {
                    self.expanded.insert(dir);
                }
            }
            Message::DefaultExpansionToggled => {
                self.default_expanded = !self.default_expanded;
            }
            Message::WorktreeMenuToggled(dir) => {
                // Toggle: same worktree closes; a different one replaces (only one open).
                self.worktree_menu_open = if self.worktree_menu_open.as_deref() == Some(dir.as_str())
                {
                    None
                } else {
                    Some(dir)
                };
                // Mutually exclusive with the project context menu (feature 015).
                self.project_menu_open = None;
            }
            Message::WorktreeMenuDismissed => {
                self.worktree_menu_open = None;
            }
            Message::WorktreeDeleteRequested(dir) => {
                self.worktree_menu_open = None;
                self.worktree_delete_target = Some(dir);
                // Never carries a choice over from a previously cancelled/confirmed dialog.
                self.worktree_delete_keep_branch = false;
                self.open_overlay(Overlay::ConfirmWorktreeDelete);
            }
            // Confirming *requests* the delete; it does not perform it. The daemon owns the git
            // removal and the session records, and answers with `OperationOk` (which is followed by
            // a `CatalogChanged` carrying git's refreshed truth) or `OperationError`.
            //
            // So this only dismisses the dialog. Dropping the row here instead — the previous
            // behaviour — made every delete *look* like it succeeded: a delete git refused showed
            // the worktree vanishing, then silently reappearing when the next catalog push restored
            // it, which reads as the app resurrecting something the user deleted rather than as the
            // failure it is. Leaving the row alone means a refusal simply leaves it in place, next
            // to the error notification explaining why.
            Message::WorktreeDeleteConfirmed => {
                self.worktree_delete_target = None;
                self.overlay = Overlay::None;
            }
            Message::WorktreeDeleteCancelled => {
                self.worktree_delete_target = None;
                self.overlay = Overlay::None;
            }
            Message::WorktreeDeleteKeepBranchToggled(keep) => {
                self.worktree_delete_keep_branch = keep;
            }
            Message::WorktreeRenameStarted(dir) => {
                let text = self.worktree_display_name(&dir);
                self.worktree_menu_open = None;
                self.worktree_rename_draft = Some(WorktreeRenameDraft {
                    dir_name: dir,
                    text,
                    error: None,
                });
                self.open_overlay(Overlay::RenameWorktree);
            }
            Message::WorktreeRenameTextChanged(text) => {
                if let Some(draft) = &mut self.worktree_rename_draft {
                    draft.text = text;
                    draft.error = None;
                }
            }
            Message::WorktreeRenameConfirmed => {
                if let Some((dir, text)) = self
                    .worktree_rename_draft
                    .as_ref()
                    .map(|d| (d.dir_name.clone(), d.text.clone()))
                {
                    // Changes only the stored display name — never the folder or branch (FR-014).
                    match self.workspace.set_worktree_name(&dir, &text) {
                        Ok(()) => {
                            self.overlay = Overlay::None;
                            self.worktree_rename_draft = None;
                        }
                        Err(error) => {
                            if let Some(draft) = &mut self.worktree_rename_draft {
                                draft.error = Some(error);
                            }
                        }
                    }
                }
            }
            Message::WorktreeRenameCancelled => {
                self.overlay = Overlay::None;
                self.worktree_rename_draft = None;
            }
            Message::SidebarFilterToggled(filter) => {
                if !self.sidebar_filters.remove(&filter) {
                    self.sidebar_filters.insert(filter);
                }
            }
            Message::SidebarFiltersCleared => {
                self.sidebar_filters.clear();
            }
            Message::ScrolledBeneathOverlay => {
                // Ask the shared rule, rather than deciding here: a non-modal surface is transient
                // and the ground moving under it means the user has moved on, while a dialog is
                // anchored to nothing and must survive it (feature 017, FR-009, FR-017).
                use micold_core::overlay::{dismisses, Surface as OverlaySurface, Trigger};
                if dismisses(OverlaySurface::NonModal, Trigger::ScrollBeneath) {
                    self.help_menu_open = false;
                    self.project_switcher_open = false;
                    self.sidebar_filter_open = false;
                    self.project_menu_open = None;
                    self.worktree_menu_open = None;
                    self.session_menu_open = None;
                }
            }
            Message::SidebarFilterMenuToggled => {
                self.sidebar_filter_open = !self.sidebar_filter_open;
                // Mutually exclusive with the other two lightweight popovers (feature 009).
                self.help_menu_open = false;
                self.project_switcher_open = false;
                // Mutually exclusive with the project context menu (feature 015).
                self.project_menu_open = None;
            }
            Message::ShowAgentWorktreesToggled => {
                // Sole mutation (FR-010d): the tag filters, expansion state, and overlays are all
                // left exactly as they were. Nothing is re-discovered either — this is a pure view
                // recomputation, so no git call and no `Task` (FR-008).
                self.show_agent_worktrees = !self.show_agent_worktrees;
            }
            Message::WorktreeHovered(dir) => {
                self.hovered_worktree = Some(dir);
            }
            Message::WorktreeUnhovered(dir) => {
                // Only clear if we're leaving the row we thought was hovered (avoids a stale
                // exit from a previous row clobbering a fresh enter).
                if self.hovered_worktree.as_deref() == Some(dir.as_str()) {
                    self.hovered_worktree = None;
                }
            }
            Message::AddWorktreeOpened => {
                self.open_overlay(Overlay::AddWorktree);
                self.worktree_form = Some(WorktreeForm::default());
                self.worktree_error = None;
            }
            Message::AddWorktreeTypeSelected(type_) => {
                if let Some(form) = &mut self.worktree_form {
                    // Ignored while a create is in flight (feature 010 follow-up) — the form
                    // is inactive until it resolves, not just the submit button.
                    if form.status == WorktreeFormStatus::Editing {
                        form.type_ = Some(type_);
                        form.error = None;
                    }
                }
            }
            Message::AddWorktreeTicketChanged(text) => {
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing {
                        form.ticket = text;
                        form.error = None;
                    }
                }
            }
            Message::AddWorktreeNameChanged(text) => {
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing {
                        form.name = text;
                        form.error = None;
                    }
                }
            }
            Message::AddWorktreeSubmitted => {
                // Validate only (FR-008); the binary performs the git create on a valid form
                // and dispatches WorktreeCreated / WorktreeCreateFailed. A create already in
                // flight (feature 010) makes this a no-op — no double-submit.
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing {
                        if let Err(error) = form.preview() {
                            form.error = Some(error);
                        }
                    }
                }
            }
            Message::AddWorktreeCancelled => {
                self.overlay = Overlay::None;
                self.worktree_form = None;
            }
            // ----- feature 016: existing-branch source + conflict resolution -----
            Message::AddWorktreeSourceChanged(source) => {
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting()
                    {
                        form.source = source;
                        form.error = None;
                        // Leaving the picker drops its selection so no stale branch can be
                        // submitted from the new-branch inputs (FR-015).
                        if source == BranchSource::New {
                            form.selected_branch = None;
                        }
                    }
                }
            }
            Message::AddWorktreeBranchesListed(candidates) => {
                if let Some(form) = &mut self.worktree_form {
                    form.candidates = candidates;
                }
            }
            Message::AddWorktreeBranchSelected(candidate) => {
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting()
                    {
                        form.selected_branch = Some(candidate);
                        form.error = None;
                    }
                }
            }
            Message::AddWorktreeConflictDetected(situation) => {
                if let Some(form) = &mut self.worktree_form {
                    // Invariant 4: a prompt and an in-flight create cannot coexist.
                    if form.status == WorktreeFormStatus::Editing {
                        form.resolution = ResolutionState::Choosing { situation };
                    }
                }
            }
            Message::AddWorktreeOverwriteRequested => {
                if let Some(form) = &mut self.worktree_form {
                    // Only ever from `Choosing`, and only for a situation that HAS a local branch
                    // to overwrite — invariant 1 (FR-005).
                    if let ResolutionState::Choosing { situation } = &form.resolution {
                        if matches!(situation, BranchSituation::LocalAvailable { .. }) {
                            form.resolution = ResolutionState::ConfirmingOverwrite {
                                situation: situation.clone(),
                            };
                        }
                    }
                }
            }
            Message::AddWorktreeOverwriteConfirmed => {
                // The ONLY route to `CreateMode::Overwrite`. The binary picks the resolution up
                // and runs the create; here we just clear the prompt and record the mode.
                if let Some(form) = &mut self.worktree_form {
                    if matches!(form.resolution, ResolutionState::ConfirmingOverwrite { .. }) {
                        form.resolution = ResolutionState::Idle;
                    }
                }
            }
            Message::AddWorktreeResolutionChosen(mode) => {
                if let Some(form) = &mut self.worktree_form {
                    // Overwrite must go through the confirmation, never straight from the choice
                    // (invariant 1) — reject it here rather than trusting call sites.
                    let allowed = !matches!(mode, CreateMode::Overwrite)
                        && matches!(form.resolution, ResolutionState::Choosing { .. });
                    if allowed {
                        form.resolution = ResolutionState::Idle;
                    }
                }
            }
            Message::AddWorktreeResolutionCancelled => {
                if let Some(form) = &mut self.worktree_form {
                    form.resolution = match &form.resolution {
                        // Backing out of the confirmation returns to the choice, not to the form
                        // (invariant 3, US2 AS3).
                        ResolutionState::ConfirmingOverwrite { situation } => {
                            ResolutionState::Choosing {
                                situation: situation.clone(),
                            }
                        }
                        // Cancelling the choice leaves every input exactly as it was (FR-007).
                        _ => ResolutionState::Idle,
                    };
                }
            }
            Message::WorktreeCreateStarted(mode) => {
                if let Some(form) = &mut self.worktree_form {
                    form.status = WorktreeFormStatus::Creating;
                    form.mode = mode;
                    // A new attempt never inherits the previous one's stage.
                    form.stage = None;
                }
            }
            Message::WorktreeCreateStageChanged(stage) => {
                if let Some(form) = &mut self.worktree_form {
                    form.stage = Some(stage);
                }
            }
            Message::WorktreeCreated(worktree) => {
                if !self
                    .worktrees
                    .iter()
                    .any(|w| w.dir_name == worktree.dir_name)
                {
                    self.worktrees.push(worktree);
                    self.worktrees.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
                }
                self.overlay = Overlay::None;
                self.worktree_form = None;
                self.worktree_error = None;
            }
            Message::WorktreeCreateFailed(message) => {
                // Keep the form open so the user can adjust; show the error (FR-017). Reset to
                // Editing (feature 010) so the user can retry instead of staying stuck Creating.
                self.worktree_error = Some(message);
                if let Some(form) = &mut self.worktree_form {
                    form.status = WorktreeFormStatus::Editing;
                }
            }
            Message::SessionStarted(session) => {
                let id = session.id;
                let location = session.location.clone();
                if let Some(path) = self.workspace.active.clone() {
                    self.workspace
                        .sessions
                        .entry(path)
                        .or_default()
                        .push(session);
                }
                match location {
                    SessionLocation::Worktree(dir) => {
                        self.expanded.insert(dir);
                    }
                    SessionLocation::Default => {
                        self.default_expanded = true;
                    }
                }
                self.active_session = Some(id);
                // BUG-001: making a session the displayed session auto-focuses its terminal so the
                // user can interact with the AI CLI immediately (FR-010/FR-010a). The gui path
                // re-asserts focus after any same-click release (see `src/main.rs`).
                self.terminal_focused = true;
            }
            Message::SessionSelected(id) => {
                self.active_session = Some(id);
                // BUG-001: selecting a session auto-focuses its terminal (FR-010/FR-010a).
                self.terminal_focused = true;
            }
            Message::SessionRunning(id) => {
                if let Some(session) = self.session_mut(id) {
                    session.mark_running();
                }
            }
            Message::SessionTitleUpdated { id, title } => {
                if let Some(session) = self.session_mut(id) {
                    session.set_title(title);
                }
            }
            Message::TerminalModeToggled => {
                if let Some(id) = self.active_session {
                    if let Some(session) = self.session_mut(id) {
                        let next = session.mode.other();
                        session.set_mode(next);
                    }
                }
            }
            Message::TerminalRestartRequested => {
                // No pure state to update here — the binary decides which process to spawn
                // based on the current mode and follows up with SessionRunning/
                // ShellInstanceRunning once it's actually up (mirrors SessionStartRequested).
            }
            Message::ShellInstanceOpenRequested => {
                // No pure state to update here — the binary decides whether the active session
                // is in Regular mode, opens the instance (`Session::open_shell_instance`), and
                // spawns its process, following up with `ShellInstanceRunning` once it's up.
            }
            Message::ShellInstanceSelected(id, shell_id) => {
                if let Some(session) = self.session_mut(id) {
                    session.select_shell(shell_id);
                }
            }
            Message::ShellInstanceCloseRequested(id, shell_id) => {
                if let Some(session) = self.session_mut(id) {
                    session.close_shell(shell_id);
                }
            }
            Message::ShellInstanceRestartRequested(..) => {
                // No pure state to update here — the binary spawns the process and follows up
                // with ShellInstanceRunning once it's up (mirrors TerminalRestartRequested).
            }
            Message::ShellInstanceRunning(session_id, shell_id) => {
                if let Some(session) = self.session_mut(session_id) {
                    session.mark_shell_running(shell_id);
                }
            }
            Message::ShellInstanceExited(session_id, shell_id) => {
                if let Some(session) = self.session_mut(session_id) {
                    session.mark_shell_exited(shell_id);
                }
            }
            Message::SessionCloseRequested(id) => {
                // Bugfix BUG-003 (FR-015a): close ARCHIVES the session (kept, hidden from the
                // sidebar via `active_sessions()`) rather than deleting its record outright, so a
                // still-existing `claude` transcript doesn't get reconstructed by reconciliation
                // (FR-020b) on the next project open. The durable, provider-side suppression
                // marker (FR-020c) is written by the caller at the I/O boundary (`src/main.rs`),
                // alongside killing the process.
                if let Some(path) = self.workspace.active.clone() {
                    if let Some(list) = self.workspace.sessions.get_mut(&path) {
                        if let Some(session) = list.iter_mut().find(|s| s.id == id) {
                            session.archive();
                        }
                    }
                }
                if self.active_session == Some(id) {
                    self.active_session = None;
                    // BUG-001 / focus-model.md: no session is displayed, so no terminal is focused.
                    self.terminal_focused = false;
                }
            }
            Message::SessionMenuToggled(id) => {
                // Toggle: same session closes; a different one replaces (only one open) —
                // mirrors `WorktreeMenuToggled` (bugfix BUG-003).
                self.session_menu_open = if self.session_menu_open == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
            Message::SessionMenuDismissed => {
                self.session_menu_open = None;
            }
            Message::SessionRemoveRequested(id) => {
                self.session_menu_open = None;
                self.session_remove_target = Some(id);
                self.open_overlay(Overlay::ConfirmSessionRemove);
            }
            Message::SessionRemoveConfirmed => {
                // Unlike close (archive), remove drops the record outright — the pre-BUG-003
                // close behavior. The binary has already killed the process and recorded the
                // durable suppression marker (FR-015c, FR-020c).
                if let Some(id) = self.session_remove_target.take() {
                    if let Some(path) = self.workspace.active.clone() {
                        if let Some(list) = self.workspace.sessions.get_mut(&path) {
                            list.retain(|s| s.id != id);
                        }
                    }
                    if self.active_session == Some(id) {
                        self.active_session = None;
                        self.terminal_focused = false;
                    }
                }
                self.overlay = Overlay::None;
            }
            Message::SessionRemoveCancelled => {
                self.session_remove_target = None;
                self.overlay = Overlay::None;
            }
            Message::TerminalTick => {}
            Message::SidebarToggled => {
                self.sidebar_hidden = !self.sidebar_hidden;
            }
            // The handle only speaks while it is being dragged, so there is no flag to consult:
            // an arriving width *is* the drag. Clamped here — how wide the sidebar may be is the
            // application's decision, not the edge's.
            Message::SidebarDragMoved(x) => {
                self.sidebar_width = x.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
            }

            // ---- Feature 006 ----
            Message::TerminalFocused => {
                self.terminal_focused = true;
            }
            Message::TerminalFocusReleased => {
                self.terminal_focused = false;
            }
            Message::TerminalContextMenuOpened { x, y } => {
                self.terminal_context_menu = Some((x, y));
            }
            Message::TerminalContextMenuClosed => {
                self.terminal_context_menu = None;
            }
            Message::SettingsOpened => {
                self.open_overlay(Overlay::Settings);
                // The binary seeds the current value; ensure a draft exists for the reducer path.
                if self.settings_draft.is_none() {
                    self.settings_draft = Some(SettingsDraft::default());
                }
            }
            Message::SettingsScrollbackChanged(text) => {
                if let Some(draft) = &mut self.settings_draft {
                    draft.scrollback_lines = text;
                    draft.error = None;
                }
            }
            Message::SettingsEnvIncludeEnabledToggled(enabled) => {
                if let Some(draft) = &mut self.settings_draft {
                    draft.env_include_enabled = enabled;
                    draft.error = None;
                }
            }
            Message::SettingsEnvIncludePathChanged(text) => {
                if let Some(draft) = &mut self.settings_draft {
                    draft.env_include_script_path = text;
                    draft.error = None;
                }
            }
            Message::SettingsEnvIncludeTimeoutChanged(text) => {
                if let Some(draft) = &mut self.settings_draft {
                    draft.env_include_timeout = text;
                    draft.error = None;
                }
            }
            Message::SettingsSaved => {
                // Validation + persistence happen in the binary; the reducer closes the form.
                self.overlay = Overlay::None;
                self.settings_draft = None;
            }
            Message::SettingsCancelled => {
                self.overlay = Overlay::None;
                self.settings_draft = None;
            }
            Message::NotificationDismissed(index) => {
                if index < self.notifications.len() {
                    self.notifications.remove(index);
                }
            }

            // Performed by the binary at the I/O boundary (needs the home directory + a
            // scan task, a FolderScanner, git, persistence, or PTY spawning); no pure
            // reducer effect.
            Message::ProjectSelectorOpened
            | Message::FolderChosen(_)
            | Message::KnownProjectReopened(_)
            | Message::SessionStartRequested { .. }
            // Feature 006: applied to the live terminal by the binary (PTY write/scroll/resize,
            // clipboard); no pure reducer effect.
            | Message::TerminalBytes(_)
            | Message::TerminalSelectStart { .. }
            | Message::TerminalSelectUpdate { .. }
            | Message::TerminalSelectCleared
            | Message::TerminalScrolled(_)
            | Message::TerminalScrolledTo(_)
            | Message::TerminalResized { .. }
            | Message::TerminalCopyRequested
            | Message::TerminalPasteRequested
            | Message::TextCopyRequested(_)
            // The closing dialog's snapshot is a binary-owned render detail (`App::dismissing`),
            // so releasing it is the binary's business; the pure core never knew about it.
            | Message::OverlayTransitionFinished
            // Focus state is tracked by the binary (gui runtime), not the pure core.
            | Message::WindowFocusChanged(_) => {}
        }
    }

    /// The effective sidebar width in pixels: the user's chosen width (clamped), or the
    /// default until they resize it.
    pub fn sidebar_width_px(&self) -> u16 {
        if self.sidebar_width == 0 {
            SIDEBAR_DEFAULT_WIDTH
        } else {
            self.sidebar_width
                .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH)
        }
    }

    /// The project-switcher rows for the current workspace. Pure: one entry per known project,
    /// in catalog order, each carrying its active marker, running background-session count, and
    /// availability. The GUI maps these to rendered rows and the "Add project…" affordance is
    /// added by the view.
    pub fn switcher_entries(&self) -> Vec<SwitcherEntry> {
        self.workspace
            .projects
            .iter()
            .map(|p| SwitcherEntry {
                path: p.path.clone(),
                label: p.display_name.clone(),
                is_active: self.workspace.active.as_ref() == Some(&p.path),
                running_count: self.workspace.running_session_count(&p.path),
                available: p.availability == Availability::Available,
            })
            .collect()
    }

    /// Replace the discovered worktrees and reconcile every piece of state that references a
    /// worktree by `dir_name`, so nothing points at a worktree that no longer exists (feature
    /// 008). The single path used by both the `WorktreesLoaded` reducer and the binary's direct
    /// re-discovery, so a worktree removed in-app OR externally cannot leave stale expansion,
    /// hover, context-menu, delete-confirmation, or rename-override state behind.
    pub fn set_worktrees(&mut self, worktrees: Vec<Worktree>) {
        self.worktrees = worktrees;
        let names: BTreeSet<String> = self.worktrees.iter().map(|w| w.dir_name.clone()).collect();

        self.expanded.retain(|d| names.contains(d));
        if self
            .worktree_menu_open
            .as_deref()
            .is_some_and(|d| !names.contains(d))
        {
            self.worktree_menu_open = None;
        }
        if self
            .hovered_worktree
            .as_deref()
            .is_some_and(|d| !names.contains(d))
        {
            self.hovered_worktree = None;
        }
        if self
            .worktree_delete_target
            .as_deref()
            .is_some_and(|d| !names.contains(d))
        {
            self.worktree_delete_target = None;
            if self.overlay == Overlay::ConfirmWorktreeDelete {
                self.overlay = Overlay::None;
            }
        }
        // Prune rename overrides for the active project's worktrees that are gone (FR-015).
        if let Some(active) = self.workspace.active.clone() {
            if let Some(map) = self.workspace.worktree_names.get_mut(&active) {
                map.retain(|dir, _| names.contains(dir));
            }
        }
    }

    /// Session ids of the active project hosted by the worktree `dir_name` (feature 008
    /// delete): the sessions the binary must terminate before removing the worktree (FR-020).
    pub fn sessions_in_worktree(&self, dir_name: &str) -> Vec<SessionId> {
        self.active_sessions()
            .iter()
            .filter(|s| s.location.is_worktree(dir_name))
            .map(|s| s.id)
            .collect()
    }

    /// Sessions hosted by the active project (FR-011), **including archived ones** — used by
    /// callers that need the raw record (e.g. [`Self::sessions_in_worktree`], which only cares
    /// about location, not visibility). Sidebar-rendering call sites
    /// ([`Self::sidebar_entries`], [`Self::worktree_tree`]) additionally filter out archived
    /// sessions themselves (bugfix BUG-003, FR-015a), so a closed session disappears from the
    /// sidebar. Empty when no project is active.
    pub fn active_sessions(&self) -> &[Session] {
        self.workspace
            .active
            .as_ref()
            .and_then(|path| self.workspace.sessions.get(path))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Switch the active project **without stopping any session** (feature 008, FR-001/FR-002).
    ///
    /// Order is load-bearing (see data-model.md I1): (1) record the current (outgoing)
    /// foreground BEFORE activation, so the outgoing project is captured — not the incoming
    /// one; (2) activate the target, which leaves everything unchanged and returns `false` if
    /// the project is unknown/unavailable (FR-008); (3) restore the incoming project's
    /// foreground (stored → first running → `None`) (FR-003); (4) surface a notice if any of
    /// its sessions were restarted while it was inactive (FR-011 / SC-007). No session
    /// lifecycle is mutated. Returns whether the switch happened.
    pub fn switch_active(&mut self, path: &Path) -> bool {
        self.record_foreground(); // STEP 1 — before activation
        if !self.workspace.activate(path) {
            // STEP 2 — rejected: leave active project, sessions, and foreground untouched.
            return false;
        }
        self.restore_after_activation(path); // STEPS 3 + 4
        true
    }

    /// Record the current active project's foreground session for later restore (FR-003).
    ///
    /// Public so callers that move `active` themselves (the `FolderChosen` handler, via
    /// `Workspace::open_or_activate`) can capture the outgoing foreground BEFORE activation
    /// and then call [`restore_after_activation`](Self::restore_after_activation) (I1).
    pub fn record_foreground(&mut self) {
        if let (Some(active), Some(id)) = (self.workspace.active.clone(), self.active_session) {
            self.foreground_by_project.insert(active, id);
        }
    }

    /// Finish a switch once `path` is already the active project (steps 3 + 4 of
    /// [`switch_active`](Self::switch_active)): restore its foreground session and surface any
    /// background-restart notice. Pair with a preceding [`record_foreground`](Self::record_foreground).
    pub fn restore_after_activation(&mut self, path: &Path) {
        let key = canonicalize_best_effort(path);
        self.active_session = self.restore_foreground(&key); // STEP 3
                                                             // BUG-001 / focus-model.md: switching (or opening) a project does not carry terminal focus
                                                             // across — re-focusing the restored session is a fresh explicit action (or a select/start).
        self.terminal_focused = false;
        // `default_expanded` is not keyed per project (unlike `expanded`, which is pruned by
        // worktree `dir_name` in `set_worktrees`) — reset it explicitly so a Default entry
        // expanded in one project doesn't render pre-expanded in another (feature 010).
        self.default_expanded = false;
        // Feature 014 (FR-010e): same reasoning as `default_expanded` directly above — view state
        // switched on for one project must not render in another. Deliberately unlike
        // `sidebar_filters`, which survives a switch: the filter accordion is collapsed by
        // default, so a sticky reveal would show unexplained agent rows with its cause out of
        // sight. Nothing is remembered per project, so switching back does not restore it.
        self.show_agent_worktrees = false;
        self.arm_notice(&key); // STEP 4
    }

    /// The session to display when entering `key`: the stored foreground if it still exists
    /// and is running, else the project's first running session, else `None` (FR-003).
    fn restore_foreground(&self, key: &Path) -> Option<SessionId> {
        let sessions = self.workspace.sessions.get(key)?;
        if let Some(&stored) = self.foreground_by_project.get(key) {
            if sessions.iter().any(|s| s.id == stored && s.is_active()) {
                return Some(stored);
            }
        }
        sessions.iter().find(|s| s.is_active()).map(|s| s.id)
    }

    /// If any session of the just-activated project was restarted while inactive, raise the
    /// return notice and consume those markers (FR-011 / SC-007).
    fn arm_notice(&mut self, key: &Path) {
        let restarted: Vec<SessionId> = self
            .workspace
            .sessions
            .get(key)
            .map(|list| {
                list.iter()
                    .map(|s| s.id)
                    .filter(|id| self.restarted_while_inactive.contains(id))
                    .collect()
            })
            .unwrap_or_default();
        if !restarted.is_empty() {
            for id in &restarted {
                self.restarted_while_inactive.remove(id);
            }
            // Reported through the global surface. The previous dedicated `notice` field was
            // drawn only by `shell::view`, which is the *else* branch of
            // `if state.active_session.is_some()` — and returning to a project restores its
            // foreground session, so the banner was unreachable in exactly the case it exists
            // for (FR-011 / SC-007).
            self.notify_info("A background session was restarted while you were away.");
        }
    }

    /// Mark a session as auto-restarted while its owning project was inactive (feature 008,
    /// FR-011). No-op when the session's project is the active one (that restart is visible
    /// live and needs no return notice) or the id is unknown.
    pub fn note_background_restart(&mut self, id: SessionId) {
        let owner = self
            .workspace
            .find_session(id)
            .map(|(path, _)| path.to_path_buf());
        if let Some(owner) = owner {
            if self.workspace.active.as_deref() != Some(owner.as_path()) {
                self.restarted_while_inactive.insert(id);
            }
        }
    }

    /// Mutable access to a session of the active project by id.
    fn session_mut(&mut self, id: SessionId) -> Option<&mut Session> {
        let path = self.workspace.active.clone()?;
        self.workspace
            .sessions
            .get_mut(&path)?
            .iter_mut()
            .find(|s| s.id == id)
    }

    /// The display name for a worktree (FR-017): the user's rename override when present,
    /// otherwise the friendly name derived from the directory name. Never touches the folder
    /// or branch on disk (FR-007, FR-014).
    pub fn worktree_display_name(&self, dir_name: &str) -> String {
        self.workspace
            .worktree_name(dir_name)
            .map(str::to_string)
            .unwrap_or_else(|| display_name(dir_name))
    }

    /// The tags for a worktree: the derived type + issue tags, plus a status tag when the
    /// worktree is not `Valid` (FR-002, FR-003, FR-011).
    fn worktree_tags(worktree: &Worktree) -> Vec<Tag> {
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

    /// The worktrees currently shown to the user (feature 014, FR-002/FR-003): all of them while
    /// the reveal control is on, only user-owned ones while it is off.
    ///
    /// The single source every worktree surface reads from — [`Self::worktree_tree`],
    /// [`Self::available_tag_filters`], and the sidebar's empty-state hint — so hiding, counting,
    /// and filtering agree by construction instead of via three separate filters that can drift
    /// (contracts/agent-worktree-classification.md).
    ///
    /// Note what does NOT read this: `set_worktrees`'s pruning and [`Self::sessions_in_worktree`]
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

    /// Build the sidebar tree: worktrees (top level) each joined with their sessions and
    /// expansion state (FR-002, FR-003). Sessions are matched to worktrees by `dir_name`.
    /// Sourced from [`Self::visible_worktrees`], so agent-owned worktrees produce no row while
    /// hidden (feature 014, FR-002).
    pub fn worktree_tree(&self) -> Vec<WorktreeNode> {
        let sessions = self.active_sessions();
        self.visible_worktrees()
            .map(|worktree| WorktreeNode {
                display_name: self.worktree_display_name(&worktree.dir_name),
                tags: Self::worktree_tags(worktree),
                expanded: self.expanded.contains(&worktree.dir_name),
                sessions: sessions
                    .iter()
                    .filter(|s| s.location.is_worktree(&worktree.dir_name) && !s.archived)
                    .cloned()
                    .collect(),
                worktree: worktree.clone(),
            })
            .collect()
    }

    /// The worktree tree narrowed to the active tag filters (feature 008, FR-025). With no
    /// filter active this equals [`Self::worktree_tree`]. Used by the sidebar to render only
    /// matching worktrees; a subsequent add/rename/delete re-runs this so the list stays
    /// consistent (FR-028).
    pub fn filtered_worktree_tree(&self) -> Vec<WorktreeNode> {
        self.worktree_tree()
            .into_iter()
            .filter(|node| matches_filters(&node.tags, &self.sidebar_filters))
            .collect()
    }

    /// The full sidebar location list (feature 010): the "Default" entry first, then worktree
    /// entries narrowed to the active tag filters (`filtered_worktree_tree`). Empty when no
    /// project is open (contracts/sidebar-default-entry.md invariant 1) — mirrors how
    /// `worktree_tree` is empty with no active project. The Default entry is exempt from tag
    /// filtering (FR-011, research.md R4): it is included unconditionally whenever a project is
    /// open, regardless of `sidebar_filters`.
    pub fn sidebar_entries(&self) -> Vec<SidebarEntry> {
        if self.workspace.active.is_none() {
            return Vec::new();
        }
        let default_sessions: Vec<Session> = self
            .active_sessions()
            .iter()
            .filter(|s| s.location == SessionLocation::Default && !s.archived)
            .cloned()
            .collect();
        let mut entries = vec![SidebarEntry::Default(DefaultNode {
            display_name: "Default",
            expanded: self.default_expanded,
            sessions: default_sessions,
        })];
        entries.extend(
            self.filtered_worktree_tree()
                .into_iter()
                .map(SidebarEntry::Worktree),
        );
        entries
    }

    /// The distinct tag filters offered for the current worktrees (feature 008, FR-024): a
    /// `Type` per conventional type present, `HasIssue` if any worktree embeds an issue key,
    /// and `Untyped` if any worktree lacks a type. Order: types first, then HasIssue, Untyped.
    ///
    /// Sourced from [`Self::visible_worktrees`] (feature 014, FR-003): a hidden agent worktree
    /// must not conjure a chip — its machine name has no conventional type, so it would otherwise
    /// offer an `Untyped` filter matching nothing the user can see (research R7).
    pub fn available_tag_filters(&self) -> Vec<TagFilter> {
        let mut types = BTreeSet::new();
        let mut has_issue = false;
        let mut has_untyped = false;
        for worktree in self.visible_worktrees() {
            let tags = Self::worktree_tags(worktree);
            let mut typed = false;
            for tag in &tags {
                match tag {
                    Tag::Type(t) => {
                        types.insert(*t);
                        typed = true;
                    }
                    Tag::Issue(_) => has_issue = true,
                    Tag::Status(_) => {}
                    // Feature 014: label only, never a filter (research R5). Note what the empty
                    // arm implies: carrying no `Type`, a REVEALED agent worktree still counts as
                    // untyped and so can be matched by an `Untyped` chip — correct, and required
                    // by FR-010d (filters apply to revealed rows exactly as to user-created ones).
                    Tag::Agent => {}
                }
            }
            if !typed {
                has_untyped = true;
            }
        }
        let mut out: Vec<TagFilter> = types.into_iter().map(TagFilter::Type).collect();
        if has_issue {
            out.push(TagFilter::HasIssue);
        }
        if has_untyped {
            out.push(TagFilter::Untyped);
        }
        out
    }
}

/// Map an Escape key press to a [`Message`] given the current state.
///
/// Returns `Some(AboutClosed)` only while the About overlay is open (FR-011); returns
/// `None` otherwise, so pressing Esc with no dialog open has no effect (edge case). The
/// iced keyboard subscription in the binary delegates to this pure function.
///
/// Checks the sidebar filter panel first (feature 009): it's a lightweight popover, not a
/// modal `Overlay`, and the two are mutually exclusive in practice, so this takes priority
/// without needing `state.overlay` to be `None`.
pub fn on_escape(state: &State) -> Option<Message> {
    // Matches the keyboard subscription's guard exactly (`ui::subscription()`) — both require
    // no modal `Overlay` before prioritizing the filter panel. `open_overlay()` keeps this
    // combination unreachable in practice, but checking it here too means the two Escape
    // implementations can never disagree even if that invariant is ever violated elsewhere.
    if state.overlay == Overlay::None && state.sidebar_filter_open {
        return Some(Message::SidebarFilterMenuToggled);
    }
    match state.overlay {
        Overlay::About => Some(Message::AboutClosed),
        Overlay::ProjectSelector => Some(Message::ProjectSelectorClosed),
        Overlay::RenameProject => Some(Message::RenameCancelled),
        Overlay::AddWorktree => Some(Message::AddWorktreeCancelled),
        Overlay::Settings => Some(Message::SettingsCancelled),
        Overlay::ConfirmWorktreeDelete => Some(Message::WorktreeDeleteCancelled),
        Overlay::RenameWorktree => Some(Message::WorktreeRenameCancelled),
        Overlay::ConfirmSessionRemove => Some(Message::SessionRemoveCancelled),
        Overlay::ConfirmForgetProject => Some(Message::ProjectForgetCancelled),
        Overlay::None => None,
    }
}

/// Where a decoded key press should go (feature 006, FR-009/FR-011). Pure; see
/// `contracts/focus-model.md`. When the terminal is unfocused every key drives the app; when
/// focused, the key's [`crate::keymap::KeyOutput`] determines the terminal action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRouting {
    /// Let the surrounding application handle the key (terminal unfocused).
    App,
    /// Write these bytes to the focused session's PTY.
    Write(Vec<u8>),
    /// Copy the current terminal selection.
    Copy,
    /// Paste clipboard text into the PTY.
    Paste,
    /// Release terminal focus back to the application.
    ReleaseFocus,
    /// Open a new Regular Terminal instance for the active session (feature 011, FR-019).
    NewTerminalInstance,
    /// Focused, but the key has no terminal meaning — drop it.
    Ignore,
}

/// Route a decoded key press given the current focus (FR-009/FR-011/FR-012).
pub fn route_key(terminal_focused: bool, output: crate::keymap::KeyOutput) -> KeyRouting {
    use crate::keymap::KeyOutput;
    if !terminal_focused {
        return KeyRouting::App;
    }
    match output {
        KeyOutput::Bytes(bytes) => KeyRouting::Write(bytes),
        KeyOutput::Copy => KeyRouting::Copy,
        KeyOutput::Paste => KeyRouting::Paste,
        KeyOutput::ReleaseFocus => KeyRouting::ReleaseFocus,
        KeyOutput::NewTerminalInstance => KeyRouting::NewTerminalInstance,
        KeyOutput::Ignore => KeyRouting::Ignore,
    }
}

/// A snapshot of a just-closed overlay, kept alive by the client so it can keep being rendered
/// while it fades out. The pure core clears the overlay + its draft synchronously on close, so
/// we capture the data here *before* the reducer runs and render from this snapshot during the
/// exit animation. Each variant carries a clone of exactly what that overlay's render function
/// needs (all `Clone`, straight from the core `State`).
#[derive(Debug)]
pub enum ClosingOverlay {
    About,
    Selector(Selector),
    Rename(RenameDraft),
    Worktree(WorktreeForm, Option<String>),
    Settings(SettingsDraft),
    ConfirmDelete(String),
    WorktreeRename(WorktreeRenameDraft),
    ConfirmSessionRemove(String),
    /// Fading-out confirm-forget dialog (feature 014): the project's display name and the
    /// running-session count captured at close time, so the exit animation matches the live view.
    ConfirmForget(String, usize),
}

impl ClosingOverlay {
    /// Which overlay this is a snapshot of.
    ///
    /// The renderer needs a dialog's identity to stay put across the close — a transition that
    /// sees its subject change restarts, and a dialog whose identity vanished the instant it began
    /// closing would jump to hidden instead of animating out. `state.overlay` is already `None` by
    /// then, so the snapshot is the only thing that still knows.
    pub fn overlay(&self) -> Overlay {
        match self {
            ClosingOverlay::About => Overlay::About,
            ClosingOverlay::Selector(_) => Overlay::ProjectSelector,
            ClosingOverlay::Rename(_) => Overlay::RenameProject,
            ClosingOverlay::Worktree(..) => Overlay::AddWorktree,
            ClosingOverlay::Settings(_) => Overlay::Settings,
            ClosingOverlay::ConfirmDelete(_) => Overlay::ConfirmWorktreeDelete,
            ClosingOverlay::WorktreeRename(_) => Overlay::RenameWorktree,
            ClosingOverlay::ConfirmSessionRemove(_) => Overlay::ConfirmSessionRemove,
            ClosingOverlay::ConfirmForget(..) => Overlay::ConfirmForgetProject,
        }
    }
}

/// Whether keystrokes may be written to a session's shell PTY given its `ShellLifecycle`
/// (feature 010, mirrors [`should_write_to`] for the shell process): only while `Running`.
pub fn should_write_to_shell(shell_lifecycle: micold_core::session::ShellLifecycle) -> bool {
    matches!(
        shell_lifecycle,
        micold_core::session::ShellLifecycle::Running
    )
}
