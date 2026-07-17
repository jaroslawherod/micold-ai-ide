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

use crate::naming::{derive, ConventionalType, DerivedNames, NamingError, WorktreeNaming};
use crate::project::{canonicalize_best_effort, Availability, FolderEntry, RenameError};
use crate::selector::Selector;
use crate::session::{Session, SessionId};
use crate::theme::{resolve, ColorScheme, SystemScheme, ThemePreference};
use crate::worktree::Worktree;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The labels of the top toolbar's entries, in display order.
///
/// The shell deliberately exposes exactly one entry — "Help" (FR-002, FR-003).
pub const TOOLBAR_ENTRIES: [&str; 1] = ["Help"];

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

/// The top toolbar's entry labels. See [`TOOLBAR_ENTRIES`].
pub fn toolbar_entries() -> &'static [&'static str] {
    &TOOLBAR_ENTRIES
}

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
}

impl WorktreeForm {
    /// The live derived directory/branch preview, or the validation error (FR-008a).
    pub fn preview(&self) -> Result<DerivedNames, NamingError> {
        derive(&WorktreeNaming {
            type_: self.type_,
            ticket: if self.ticket.trim().is_empty() {
                None
            } else {
                Some(self.ticket.clone())
            },
            name: self.name.clone(),
        })
    }
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

/// One worktree row in the sidebar tree, joined with its (expanded) sessions (FR-002/003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeNode {
    /// The worktree itself.
    pub worktree: Worktree,
    /// Whether its session sub-items are shown.
    pub expanded: bool,
    /// The sessions hosted by this worktree (empty unless expanded is irrelevant to data).
    pub sessions: Vec<Session>,
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
    /// The user selected a theme preference (Follow system / Light / Dark) (FR-007, FR-008).
    /// The binary persists the updated preference afterward.
    ThemePreferenceChanged(ThemePreference),
    /// Cycle the theme mode to the next one (Auto → Light → Dark → Auto) from the toolbar
    /// menu's mode toggle. The binary persists the updated preference; the menu stays open.
    ThemeModeCycled,
    /// The OS light/dark preference poll observed a (changed) scheme (FR-006). Transient;
    /// never persisted.
    SystemThemeChanged(SystemScheme),

    // ---- Feature 005: worktrees, sessions, embedded terminal ----
    /// Opening a directory as a project was refused because it is not a git repo (FR-001a).
    /// The binary performs the `Git::is_repo_root` check and dispatches this on refusal.
    ProjectOpenRefused(String),
    /// The binary discovered/re-discovered the active project's worktrees (FR-018).
    WorktreesLoaded(Vec<Worktree>),
    /// Expand/collapse a worktree's session sub-items (FR-003), by `dir_name`.
    WorktreeExpansionToggled(String),
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
    /// The binary created a worktree successfully (FR-007); add it and close the form.
    WorktreeCreated(Worktree),
    /// The binary reported a worktree create failure (FR-017); show it, keep the form open.
    WorktreeCreateFailed(String),
    /// Start a new session on the given worktree (FR-010). The binary spawns `claude`.
    SessionStartRequested { worktree_dir: String },
    /// A session was started/added for the active project (FR-011).
    SessionStarted(Session),
    /// Select a session to display its terminal (FR-015); other sessions keep running.
    SessionSelected(SessionId),
    /// Close/stop a session (FR-015a). The binary kills the process; this drops the record.
    SessionCloseRequested(SessionId),
    /// The session's `claude` process reported it is running (FR-010).
    SessionRunning(SessionId),
    /// The session's `claude` title became available/changed (FR-011a).
    SessionTitleUpdated { id: SessionId, title: String },
    /// Periodic redraw tick while a terminal is live (drives streamed-output repaint).
    TerminalTick,
    /// Hide or show the sidebar (toggle).
    SidebarToggled,
    /// The user began dragging the sidebar resize handle.
    SidebarDragStarted,
    /// The resize drag moved; carries the new intended width in pixels (cursor x).
    SidebarDragMoved(u16),
    /// The resize drag ended.
    SidebarDragEnded,
    /// The pointer entered (`true`) or left (`false`) the sidebar resize handle; drives its
    /// animated hover highlight (handled by the binary).
    SidebarHandleHovered(bool),
    /// Animation clock tick (drives fade/slide progress; handled by the binary).
    AnimationTick,

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
    /// Save the Settings form (validated + persisted by the binary) (FR-020, FR-021).
    SettingsSaved,
    /// Dismiss the Settings form without saving (Cancel or Esc).
    SettingsCancelled,

    // ---- Feature 008: background project switching ----
    /// Toggle the top-bar project switcher panel (feature 008, FR-004). Mutually exclusive
    /// with the overflow menu.
    ProjectSwitcherToggled,
    /// Dismiss the "restarted while you were away" notice banner (feature 008, SC-007).
    NoticeDismissed,
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
    pub workspace: crate::workspace::Workspace,
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
    /// Whether a sidebar resize drag is in progress (transient).
    pub sidebar_dragging: bool,
    /// Whether the embedded terminal holds input focus (feature 006). Default `false`; keys are
    /// delivered to the session process only while `true` (FR-009/FR-010/FR-012).
    pub terminal_focused: bool,
    /// The open terminal right-click context menu's anchor in pane-local pixels, or `None` when
    /// no menu is showing (feature 006, FR-013).
    pub terminal_context_menu: Option<(u16, u16)>,
    /// In-progress Settings form, present only while the Settings overlay is shown (feature 006).
    pub settings_draft: Option<SettingsDraft>,
    /// The session that was in the foreground for each project, remembered so returning to a
    /// project restores it (feature 008, FR-003). In-memory only; not persisted (research R2).
    pub foreground_by_project: BTreeMap<PathBuf, SessionId>,
    /// Sessions that were auto-restarted while their project was inactive, pending a return
    /// notification (feature 008, FR-011 / SC-007). Cleared when the user returns to the owner.
    pub restarted_while_inactive: BTreeSet<SessionId>,
    /// A transient banner shown on return when a background restart occurred (feature 008,
    /// SC-007). Presentation-only; cleared by [`Message::NoticeDismissed`] or the next switch.
    pub notice: Option<String>,
    /// Whether the top-bar project switcher panel is open (feature 008, FR-004). Mutually
    /// exclusive with `help_menu_open`.
    pub project_switcher_open: bool,
}

impl State {
    /// The color scheme to render, resolved from the user's preference and the OS scheme
    /// (FR-005, FR-007, FR-018). See [`crate::theme::resolve`].
    pub fn color_scheme(&self) -> ColorScheme {
        resolve(self.theme_pref, self.system_scheme)
    }

    /// Apply a [`Message`], transitioning the state. Pure and side-effect free.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::HelpMenuToggled => {
                self.help_menu_open = !self.help_menu_open;
                // The overflow menu and the project switcher are mutually exclusive.
                self.project_switcher_open = false;
            }
            Message::ProjectSwitcherToggled => {
                self.project_switcher_open = !self.project_switcher_open;
                self.help_menu_open = false;
            }
            Message::AboutOpened => {
                // Idempotent: opening while already open keeps a single instance (FR-015).
                self.overlay = Overlay::About;
                self.help_menu_open = false;
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
                    self.overlay = Overlay::RenameProject;
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
            Message::ThemeModeCycled => {
                // Advance to the next mode; the menu stays open so repeated clicks cycle.
                self.theme_pref = self.theme_pref.next();
            }
            Message::ThemePreferenceChanged(pref) => {
                // Pure state change; the binary persists it at the I/O boundary (FR-009).
                self.theme_pref = pref;
            }
            Message::SystemThemeChanged(scheme) => {
                self.system_scheme = scheme;
            }
            Message::ProjectOpenRefused(message) => {
                // Non-git directory refused (FR-001a); the active project is unchanged.
                self.worktree_error = Some(message);
            }
            Message::WorktreesLoaded(worktrees) => {
                self.worktrees = worktrees;
                self.worktree_error = None;
                // Drop expansion state for worktrees that no longer exist.
                let names: BTreeSet<String> =
                    self.worktrees.iter().map(|w| w.dir_name.clone()).collect();
                self.expanded.retain(|d| names.contains(d));
            }
            Message::WorktreeExpansionToggled(dir) => {
                if !self.expanded.remove(&dir) {
                    self.expanded.insert(dir);
                }
            }
            Message::AddWorktreeOpened => {
                self.overlay = Overlay::AddWorktree;
                self.worktree_form = Some(WorktreeForm::default());
                self.worktree_error = None;
            }
            Message::AddWorktreeTypeSelected(type_) => {
                if let Some(form) = &mut self.worktree_form {
                    form.type_ = Some(type_);
                    form.error = None;
                }
            }
            Message::AddWorktreeTicketChanged(text) => {
                if let Some(form) = &mut self.worktree_form {
                    form.ticket = text;
                    form.error = None;
                }
            }
            Message::AddWorktreeNameChanged(text) => {
                if let Some(form) = &mut self.worktree_form {
                    form.name = text;
                    form.error = None;
                }
            }
            Message::AddWorktreeSubmitted => {
                // Validate only (FR-008); the binary performs the git create on a valid form
                // and dispatches WorktreeCreated / WorktreeCreateFailed.
                if let Some(form) = &mut self.worktree_form {
                    if let Err(error) = form.preview() {
                        form.error = Some(error);
                    }
                }
            }
            Message::AddWorktreeCancelled => {
                self.overlay = Overlay::None;
                self.worktree_form = None;
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
                // Keep the form open so the user can adjust; show the error (FR-017).
                self.worktree_error = Some(message);
            }
            Message::SessionStarted(session) => {
                let id = session.id;
                let worktree_dir = session.worktree_dir.clone();
                if let Some(path) = self.workspace.active.clone() {
                    self.workspace
                        .sessions
                        .entry(path)
                        .or_default()
                        .push(session);
                }
                self.expanded.insert(worktree_dir);
                self.active_session = Some(id);
            }
            Message::SessionSelected(id) => {
                self.active_session = Some(id);
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
            Message::SessionCloseRequested(id) => {
                if let Some(path) = self.workspace.active.clone() {
                    if let Some(list) = self.workspace.sessions.get_mut(&path) {
                        list.retain(|s| s.id != id);
                    }
                }
                if self.active_session == Some(id) {
                    self.active_session = None;
                }
            }
            Message::TerminalTick => {}
            Message::SidebarToggled => {
                self.sidebar_hidden = !self.sidebar_hidden;
            }
            Message::SidebarDragStarted => {
                self.sidebar_dragging = true;
            }
            Message::SidebarDragMoved(x) => {
                if self.sidebar_dragging {
                    self.sidebar_width = x.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
                }
            }
            Message::SidebarDragEnded => {
                self.sidebar_dragging = false;
            }
            // Animation progress is tracked by the binary (gui runtime), not the pure core.
            Message::AnimationTick => {}

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
                self.overlay = Overlay::Settings;
                self.help_menu_open = false;
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
            Message::SettingsSaved => {
                // Validation + persistence happen in the binary; the reducer closes the form.
                self.overlay = Overlay::None;
                self.settings_draft = None;
            }
            Message::SettingsCancelled => {
                self.overlay = Overlay::None;
                self.settings_draft = None;
            }
            Message::NoticeDismissed => {
                self.notice = None;
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
            | Message::SidebarHandleHovered(_) => {}
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

    /// The project-switcher rows for the current workspace (feature 008, FR-005–FR-008). Pure:
    /// one entry per known project, in catalog order, each carrying its active marker, running
    /// background-session count, and availability. The GUI maps these to rendered rows and the
    /// "Add project…" affordance is added by the view (FR-009).
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

    /// Sessions hosted by the active project (FR-011). Empty when no project is active.
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
            self.notice =
                Some("A background session was restarted while you were away.".to_string());
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

    /// Build the sidebar tree: worktrees (top level) each joined with their sessions and
    /// expansion state (FR-002, FR-003). Sessions are matched to worktrees by `dir_name`.
    pub fn worktree_tree(&self) -> Vec<WorktreeNode> {
        let sessions = self.active_sessions();
        self.worktrees
            .iter()
            .map(|worktree| WorktreeNode {
                expanded: self.expanded.contains(&worktree.dir_name),
                sessions: sessions
                    .iter()
                    .filter(|s| s.worktree_dir == worktree.dir_name)
                    .cloned()
                    .collect(),
                worktree: worktree.clone(),
            })
            .collect()
    }
}

/// Map an Escape key press to a [`Message`] given the current state.
///
/// Returns `Some(AboutClosed)` only while the About overlay is open (FR-011); returns
/// `None` otherwise, so pressing Esc with no dialog open has no effect (edge case). The
/// iced keyboard subscription in the binary delegates to this pure function.
pub fn on_escape(state: &State) -> Option<Message> {
    match state.overlay {
        Overlay::About => Some(Message::AboutClosed),
        Overlay::ProjectSelector => Some(Message::ProjectSelectorClosed),
        Overlay::RenameProject => Some(Message::RenameCancelled),
        Overlay::AddWorktree => Some(Message::AddWorktreeCancelled),
        Overlay::Settings => Some(Message::SettingsCancelled),
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
        KeyOutput::Ignore => KeyRouting::Ignore,
    }
}

/// Whether keystrokes may be written to a session's PTY given its lifecycle (FR-012a): only
/// while `Running`. In other states input is discarded (no buffering); focus/scroll/copy still
/// work.
pub fn should_write_to(lifecycle: crate::session::SessionLifecycle) -> bool {
    matches!(lifecycle, crate::session::SessionLifecycle::Running)
}
