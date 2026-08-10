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

use crate::features::notifications::NoticeLevel;
use crate::features::project::{ProjectMenu, RenameDraft, SwitcherEntry};
use crate::features::session::SelectKind;
use crate::features::settings::SettingsDraft;
use crate::features::sidebar::TagFilter;
use crate::features::worktree::WorktreeRenameDraft;
use crate::features::worktree_form::{
    BranchSource, ResolutionState, WorktreeForm, WorktreeFormStatus,
};
use micold_core::naming::ConventionalType;
use micold_core::notify;
use micold_core::project::{canonicalize_best_effort, Availability, FolderEntry};
use micold_core::selector::Selector;
use micold_core::session::{Session, SessionId, SessionLocation, ShellInstanceId};
use micold_core::theme::{
    observe_system_scheme, resolve, ColorScheme, SystemScheme, ThemePreference,
};
use micold_core::typeahead::{move_highlight, Direction};
use micold_core::worktree::{BranchCandidate, BranchSituation, CreateMode, CreateStage, Worktree};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

/// Minimum sidebar width in pixels (resize lower bound).
pub const SIDEBAR_MIN_WIDTH: u16 = 180;
/// Maximum sidebar width in pixels (resize upper bound).
pub const SIDEBAR_MAX_WIDTH: u16 = 600;
/// Default sidebar width in pixels, used until the user resizes it.
pub const SIDEBAR_DEFAULT_WIDTH: u16 = 300;

/// Which text field holds the keyboard, when one does (BUG-003).
///
/// A filled field's whole focus affordance — the label floating clear of the value, the active
/// indicator thickening to the accent, the focus state layer (§7.7, FR-031, FR-035) — is decided
/// when the field is *built*, from a flag its caller supplies. Nothing supplied it. The component
/// honoured the flag, every anatomy gate proved it honoured the flag, and in the running
/// application every field was drawn permanently at rest.
///
/// One enum for the whole application rather than a focus flag on each of the four drafts: at most
/// one field can hold the keyboard, and this is the shape that says so. `Option<FieldId>` also makes "two fields focused at once" unrepresentable
/// (Principle V), where four booleans would have needed a rule keeping them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldId {
    /// The rename-project dialog's name field.
    RenameProjectName,
    /// The rename-worktree dialog's name field.
    RenameWorktreeName,
    /// The add-worktree form's optional ticket field.
    AddWorktreeTicket,
    /// The add-worktree form's branch-name field.
    AddWorktreeName,
    /// Settings: the terminal scrollback limit.
    SettingsScrollback,
    /// The confirm-worktree-delete dialog's "also delete the branch" checkbox.
    ConfirmDeleteAlsoBranch,
    /// Settings: the environment-include on/off checkbox. Not a text field — the checkbox now
    /// takes the keyboard too, and this is the same fact about the same dialog (BUG-003).
    SettingsEnvIncludeEnabled,
    /// Settings: the environment-include script path.
    SettingsEnvIncludePath,
    /// Settings: the environment-include timeout.
    SettingsEnvIncludeTimeout,
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
    /// A text field took or lost the keyboard (BUG-003). Emitted by the field's own container,
    /// which asks the input rather than guessing from the pointer — see
    /// `material::FormField::on_focus_change`. Sole mutation: [`State::focused_field`].
    FieldFocusChanged(FieldId, bool),
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
    /// Escape was pressed with the terminal unfocused (feature 021, T034). The first of the three
    /// dismissal triggers to be reported the same way the third already was: as *what happened*,
    /// not as what should close.
    ///
    /// The keyboard subscription used to name the message itself, which meant a nine-arm match
    /// over the overlay enum in the view layer and a hand-written priority rule above it. It now
    /// emits this, and the reducer asks the registry which surface Escape reaches — so a surface
    /// added tomorrow is dismissed without the subscription hearing of it, and the decision is
    /// made against the state Escape actually lands in rather than the state that was last
    /// rendered.
    EscapePressed,
    /// The worktree sidebar scrolled to this vertical offset.
    ///
    /// Carries the offset rather than being a bare notification, because the app bar's elevation
    /// derives from it (FR-025a) — and the sidebar is the only scroll region beneath the bar, so it
    /// is the only thing that can answer "is content passing under it".
    SidebarScrolled(u32),
    /// The sidebar's scroll viewport was laid out at this height, in whole logical pixels
    /// (feature 024).
    ///
    /// Distinct from [`Self::SidebarScrolled`] because the two answer different questions and fire
    /// at different times: an offset changes when the user scrolls, and a viewport height changes
    /// when the window does — including on the very first layout, where nothing has scrolled and
    /// the reveal still has to decide whether its row is on screen.
    SidebarViewportResized(u32),
    /// A dialog has finished animating out (feature 017, FR-011). Emitted by the `Modal` component
    /// itself, which owns the transition, so the binary can release the snapshot it was rendering
    /// from ([`crate::overlay::registry::Closing`]). The binary used to watch a central progress
    /// value for this; the
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
    /// An existing branch was picked from the list (feature 016, FR-014).
    ///
    /// A blocked candidate is **ignored entirely** (feature 021, FR-012a): it does not become the
    /// selection and does not close the list. Feature 016 let it be selected and refused at the
    /// point of creating, because the list widget of the day could not disable a row; the
    /// type-ahead can, so the refusal moved to the point of choosing.
    AddWorktreeBranchSelected(BranchCandidate),
    /// The branch search field took focus, so the list opens on what is already on offer (feature
    /// 021, FR-001b). Not a query change: focusing is not typing.
    AddWorktreeBranchFocused,
    /// The branch search text changed (feature 021, FR-001, FR-005).
    AddWorktreeBranchQueryChanged(String),
    /// The keyboard moved through the results (feature 021, FR-017). The saturating rule lives in
    /// `micold_core::typeahead`, not here — this arm applies its answer.
    AddWorktreeBranchHighlightMoved(Direction),
    /// The result list closed without a pick — Escape, a press outside it, or Tab taking focus out
    /// of the field (feature 021, FR-001b). Three triggers, one effect.
    AddWorktreeBranchDismissed,
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
    /// The daemon reported progress on the in-flight create: a new stage (feature 016, FR-024), or
    /// — with the stage unchanged — its latest live output line, rate-limited daemon-side so a long
    /// stage reads as moving rather than frozen (BUG-009, T123). Ignored once the form has closed.
    WorktreeCreateStageChanged(CreateStage, Option<String>),
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
    /// Dismiss the visible notification, promoting the next immediately (FR-032b).
    ///
    /// No index: exactly one is visible, so there is nothing to identify. The index this used to
    /// carry was a position in a stack that no longer exists.
    NotificationDismissed,
    /// Time passed while a notification was on screen, in milliseconds.
    ///
    /// Subscribed to only while the queue is active (`Queue::is_active`), so nothing ticks at rest
    /// (SC-017).
    NotificationsAdvanced(u32),

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

/// Root application state for the single main window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Whether the About dialog is open.
    ///
    /// The last remnant of the `Overlay` enum (feature 021, T037). That enum was a single slot
    /// naming which of nine dialogs was showing, and eight of the nine already had a field of
    /// their own saying the same thing — `selector`, `rename_draft`, `worktree_form`,
    /// `settings_draft`, and the four confirm-dialog targets. The slot was a second copy of a fact
    /// the state already held, kept in step by hand at twenty-five reducer sites. Each dialog now
    /// reads its own state, and About, which had none, gets this.
    ///
    /// **What the enum bought and this does not**: two dialogs open at once was unrepresentable
    /// (FR-015, Principle V). That invariant now belongs to [`State::clear_for_dialog`], which
    /// closes whatever dialog is open before the next one is set up, and to
    /// `one_dialog_at_a_time` in `tests/overlay_registry.rs`. A mechanism where there was a type;
    /// recorded rather than glossed.
    pub about_open: bool,
    /// Whether the Help menu is currently expanded (transient UI affordance).
    pub help_menu_open: bool,
    /// The known-projects catalog and the active working space (persisted). Per-story
    /// selector/rename working state is added alongside those stories.
    pub workspace: micold_core::workspace::Workspace,
    /// The folder-browser state; its presence *is* the project-selector dialog being shown (T037).
    pub selector: Option<Selector>,
    /// The in-progress rename; its presence *is* the rename dialog being shown (T037).
    pub rename_draft: Option<RenameDraft>,
    /// Which text field holds the keyboard, if any (BUG-003). Transient — never persisted.
    ///
    /// Held here rather than on each draft because it is one fact about the application, not four:
    /// see [`FieldId`]. Every filled field's focus chrome is drawn from this and nothing else.
    pub focused_field: Option<FieldId>,
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
    ///
    /// Feature 024: written through [`Self::set_current_session`] by everything except
    /// `SessionSelected`, because the panel's reveal is a consequence of this field changing
    /// rather than of any particular message being handled (contract §3.0).
    pub active_session: Option<SessionId>,
    /// The session whose revealed row the user closed (feature 024, FR-005).
    ///
    /// Scoped to a session rather than to a location, so an old collapse cannot swallow the next
    /// reveal: it is compared against `active_session` and cleared whenever that changes
    /// (invariant I2). `None` means nothing is suppressed.
    ///
    /// This is the *whole* of the reveal's stored state. Which row is open is otherwise derived
    /// from `active_session` on every view ([`Self::location_open`]), which is what makes a
    /// wholesale replacement of the worktree list unable to lose it (FR-001b).
    pub reveal_suppressed_for: Option<SessionId>,
    /// The sidebar scroll viewport's laid-out height in whole logical pixels (feature 024).
    ///
    /// Reported by the `Scrollable`'s viewport sensor. `0` until the first layout, which reads as
    /// "cannot decide visibility yet" and never as "zero tall" — nothing is scrolled on a guess
    /// (contract §6.3).
    ///
    /// `u32` rather than `f32` for two reasons that happen to agree: `State` derives `Eq`, and the
    /// offset this is compared against ([`Self::sidebar_scroll_offset`]) is already whole pixels.
    /// Keeping both in the same unit is what stops the scroll arithmetic from having a rounding
    /// seam in the middle of it.
    pub sidebar_viewport_height: u32,
    /// Whether a reveal is waiting to scroll its row into view (feature 024, FR-008).
    ///
    /// A flag, not a target. The offset cannot be computed when the reveal is armed: the incoming
    /// project's worktree list may not have arrived yet, and the viewport height is not known
    /// until layout. So the reducer arms this, and the binary computes and applies the scroll on
    /// the first frame where a row for the current session actually exists (research R7,
    /// invariant I4).
    pub pending_reveal_scroll: bool,
    /// The add-worktree form, present only while its overlay is shown (FR-005).
    pub worktree_form: Option<WorktreeForm>,
    /// A message shown when opening a non-git directory was refused (FR-001a), or a worktree
    /// create failed (FR-017). Transient.
    pub worktree_error: Option<String>,
    /// Whether the sidebar is collapsed/hidden. Default (`false`) is visible.
    pub sidebar_hidden: bool,
    /// The sidebar width in pixels. `0` means "use the default width" (see [`State::sidebar_width_px`]).
    pub sidebar_width: u16,
    /// Whether the user has explicitly handed the keyboard from the terminal back to the
    /// application (feature 023, FR-021). Default `false`.
    ///
    /// **This is not "the terminal is unfocused"** — that question is [`State::terminal_focused`],
    /// which is derived. This is the one thing about focus the user decides: the reserved chord or
    /// the release affordance sets it, and any navigation that displays a terminal clears it
    /// (FR-021a). It replaced a stored `terminal_focused: bool` that seven scattered assignments
    /// had to keep correct between them, which is how project switch, mode toggle and instance
    /// switch each ended up missing a case. Written only by [`State::focus_terminal`] and
    /// [`State::release_terminal`]; `tests/terminal_bar_stability.rs` fails if that stops being
    /// true.
    pub terminal_released: bool,
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
    /// an unreachable render path — see [`notify::Notification`]. Never persisted.
    ///
    /// A message stays until the user dismisses it or it is evicted by newer ones. Nothing
    /// clears these implicitly: a report that vanishes on unrelated activity (a background
    /// worktree re-scan, say) is how these failures became invisible in the first place.
    /// The notification queue: one visible, the rest waiting (FR-032a).
    ///
    /// A `micold_core::notify::Queue` rather than a `Vec` that renders all at once. Which one is
    /// visible, how long it stays and what is behind it are decisions with no pixels in them, so
    /// they live in the render-free core and the view draws whatever is currently visible.
    pub notify: notify::Queue,
    /// How far the worktree sidebar is scrolled, in logical pixels.
    ///
    /// The app bar's elevation derives from this and nothing else (FR-025a) — see
    /// [`Self::app_bar_elevated`] for why a second source would be a defect rather than a feature.
    pub sidebar_scroll_offset: u32,
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
    /// 008, FR-018/FR-019). Its presence *is* the confirm dialog being shown (T037).
    pub worktree_delete_target: Option<String>,
    /// Whether the user has opted to also delete the branch when confirming a worktree delete
    /// (feature 013). Defaults to `false` = delete (today's unconditional behavior), so an
    /// unmodified confirm is unchanged. Reset to `false` on every `WorktreeDeleteRequested`.
    pub worktree_delete_keep_branch: bool,
    /// The in-progress worktree rename; its presence *is* the rename dialog being shown (T037)
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
    /// FR-015c). Its presence *is* the confirm dialog being shown (T037). Mirrors
    /// `worktree_delete_target`.
    pub session_remove_target: Option<SessionId>,
    /// The project pending a forget confirmation, by path (feature 014). Its presence *is* the
    /// confirm dialog being shown (T037). Transient — never persisted. Mirrors
    /// `worktree_delete_target`.
    pub forget_target: Option<PathBuf>,
}

/// The sidebar's reported offset as the app bar reads it: whole pixels, never above the top.
///
/// Whole pixels because [`State`] derives `Eq`, and an `f32` field would take that from every type
/// that holds one. Nothing is lost — the bar asks only whether anything is under it at all.
///
/// Clamped at zero because an overscroll bounce reports *past* the origin, and treating that as
/// "scrolled" would raise the bar for a gesture that moved content the wrong way. A non-finite
/// reading is treated the same: it means the viewport has not settled, not that the list moved.
pub fn scroll_offset_px(reported: f32) -> u32 {
    if reported.is_finite() && reported > 0.0 {
        reported.round() as u32
    } else {
        0
    }
}

impl State {
    /// The color scheme to render, resolved from the user's preference and the OS scheme
    /// (FR-005, FR-007, FR-018). See [`micold_core::theme::resolve`].
    pub fn color_scheme(&self) -> ColorScheme {
        resolve(self.theme_pref, self.system_scheme)
    }

    /// Whether the app bar sits raised over content passing beneath it (FR-025a, contract §7.1).
    ///
    /// **Derived, never stored.** The flag exists only as a reading of the sidebar's offset, so
    /// there is no second field for an unrelated message to forget to update — which is how a bar
    /// ends up raised or flat according to whichever write happened last.
    ///
    /// The sidebar is the only scroll region beneath the bar, which is what makes one offset the
    /// whole answer. A bar that also raised itself for the terminal's scrollback would flicker
    /// between states that have nothing to do with what is under it.
    pub fn app_bar_elevated(&self) -> bool {
        self.sidebar_scroll_offset > 0
    }

    /// Close the transient popovers when the ground moves under them (FR-009, FR-017).
    ///
    /// Asks the shared rule rather than deciding here: a non-modal surface is transient and the
    /// ground moving under it means the user has moved on, while a dialog is anchored to nothing
    /// and must survive it. Shared by the two messages that can report a scroll so the rule has one
    /// caller-visible answer rather than two that can drift.
    fn dismiss_on_scroll_beneath(&mut self) {
        // Which surfaces the trigger reaches is the registry's answer now, not a list here. It
        // used to name six of the seven popovers; the seventh, the terminal's context menu, is
        // included from T031 — it is a non-modal surface and the core rule has always said so.
        crate::overlay::registry::close_on_scroll_beneath(self);
    }

    /// Dismiss whatever Escape reaches: the topmost open surface, and no other (contract D1).
    ///
    /// The counterpart of [`Self::dismiss_on_scroll_beneath`], and deliberately its opposite
    /// shape — a scroll invalidates every anchored menu at once, Escape is a single decision
    /// aimed at whatever holds the user's attention.
    fn dismiss_topmost(&mut self) {
        crate::overlay::registry::close_topmost(self);
    }

    /// Surface a failed action to the user (see [`notify::Notification`]).
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
        // Dedup and the retention cap moved into the queue with the rest of the discipline, so
        // this is now only the level translation: `NoticeLevel` stays the *banner's* vocabulary
        // (FR-032c keeps that a separate component) while the queue speaks the core's.
        self.notify
            .push(notify::Notification::new(level.to_queue_level(), message));
    }

    /// Clear the way for a dialog about to open: close whatever is already floating.
    ///
    /// Popovers and modals are meant to be mutually exclusive (`on_escape` and the keyboard
    /// subscription both assume it — feature 009 code review), but before this helper existed each
    /// overlay-opening arm had to reset the popovers by hand, and none of them reset
    /// `sidebar_filter_open`, so it was possible to open e.g. the Add Worktree form while the
    /// filter panel was still (invisibly) open, leaving Escape's two implementations disagreeing
    /// about what to dismiss. Routing every dialog-open through here makes that reset
    /// unconditional. Since T031 the popovers are closed by asking the registry which are open
    /// rather than by assigning to four remembered fields — so the three that list had never
    /// mentioned (`worktree_menu_open`, `session_menu_open`, `terminal_context_menu`) are closed
    /// too.
    ///
    /// **It now closes an open dialog as well** (T037), and that is the point of the rename: it
    /// used to *be* the assignment that opened one, `self.overlay = overlay`, and a single slot
    /// cannot hold two dialogs, so replacing the slot's contents closed the previous dialog for
    /// free — while quietly leaving its draft behind, since nothing read the draft once the slot
    /// said otherwise. With the slot gone the draft is what says a dialog is open, so a leftover
    /// one *is* a second open dialog. The same registry call that closes the popovers closes it,
    /// by sending the cancellation the dialog itself declared, so this is still not a list of
    /// dialogs anybody has to maintain.
    ///
    /// Callers must invoke it **before** setting up the dialog they are opening — otherwise it
    /// closes the one they just prepared. The eight call sites that did it the other way round
    /// were reordered at T037.
    pub fn clear_for_dialog(&mut self) {
        crate::overlay::registry::close_dialogs(self);
        crate::overlay::registry::close_popovers(self);
        // A dialog opens with nothing focused. The fields that reported focus belong to a widget
        // tree that is being torn down and will never report losing it, so a remembered focus would
        // outlive them — and reopening the same dialog would draw its field focused over an input
        // that has not been clicked (BUG-003).
        self.focused_field = None;
    }

    /// The one surface that does not take the keyboard — see [`State::any_surface_takes_keyboard`].
    /// Matched by id because the registry is the list; a typo here would silently make the
    /// terminal yield to its own right-click menu, so `the_terminals_own_context_menu_is_furniture`
    /// in `tests/terminal_focus.rs` is what notices.
    const TERMINAL_CONTEXT_MENU: &str = "terminal_context_menu";

    /// Whether the displayed session's terminal holds the keyboard (feature 023, FR-009).
    ///
    /// **Derived, never stored.** The rule in one line: *the displayed terminal holds the keyboard
    /// unless the user gave it away or something that types took it.* Everything else follows
    /// rather than being remembered — a dialog closes and this reads true again with no restore
    /// stack (FR-010); a session goes away and nothing has to clear a flag (FR-012, FR-016); the
    /// window loses and regains focus and nothing is written at all, so nothing needs restoring
    /// (FR-013–FR-015). Only the displayed session is ever named here, which is what makes "at most
    /// one terminal, and only that one" structural rather than a rule (FR-020).
    ///
    /// See `specs/023-terminal-focus-flow/contracts/focus-model.md` (v2).
    pub fn terminal_focused(&self) -> bool {
        self.active_session.is_some()
            && !self.terminal_released
            && self.focused_field.is_none()
            && !self.any_surface_takes_keyboard()
    }

    /// Any floating surface that takes the keyboard while it is open (FR-004, FR-017).
    ///
    /// Every dialog, and every popover **except** the terminal's own right-click menu: that one is
    /// pane furniture (FR-007), drawn inside the pane, offering the pane's own Copy and Paste — a
    /// right-click that stopped the user typing would be the same defect this feature exists to
    /// remove.
    ///
    /// It asks the registry rather than naming flags. A hand-written list of popovers is exactly
    /// the list nobody remembers to extend, and feature 024 already built the one that is
    /// maintained — "one line per surface, and this is the only such list" (024 FR-009) — so a
    /// surface registered later participates in terminal focus without anyone touching this.
    fn any_surface_takes_keyboard(&self) -> bool {
        use crate::overlay::{registry, SurfaceId};
        registry::open_dialog(self).is_some()
            || registry::open_popovers(self)
                .iter()
                .any(|open| open.id() != SurfaceId::new(Self::TERMINAL_CONTEXT_MENU))
    }

    /// The user is being put in front of a terminal (FR-011, FR-021a, FR-008b).
    ///
    /// Clears the explicit release *and* any text-field focus. The second one matters: a press on
    /// the pane, or a navigation that displays a terminal, is a request for that terminal, and it
    /// must not be defeated by a field that still believes it holds the keyboard. Without it, a
    /// press into the pane made while a rename field had focus would depend on iced's blur
    /// arriving first. FR-018 permits taking the keyboard from a field for exactly this reason —
    /// it is a user press.
    pub(crate) fn focus_terminal(&mut self) {
        self.terminal_released = false;
        self.focused_field = None;
    }

    /// The user handed the keyboard back to the application (FR-021) — the reserved chord or the
    /// release affordance. It holds until they give it back or navigate to a terminal.
    pub(crate) fn release_terminal(&mut self) {
        self.terminal_released = true;
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
                self.clear_for_dialog();
                self.about_open = true;
            }
            Message::AboutClosed => {
                // No-op when nothing is open (edge case); otherwise return to the
                // main window unchanged (FR-012).
                self.about_open = false;
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
                    self.clear_for_dialog();
                    self.rename_draft = Some(RenameDraft {
                        path,
                        text: name,
                        error: None,
                    });

                }
            }
            // A blur is only believed from the field that currently holds focus. Gaining and losing
            // are reported by two different widgets and arrive in whichever order the frame
            // produced them, so an unguarded `None` on the way out of one field would erase the
            // focus the next one had already claimed — and clicking straight from one field to
            // another would leave both at rest.
            Message::FieldFocusChanged(field, focused) => {
                if focused {
                    self.focused_field = Some(field);
                } else if self.focused_field == Some(field) {
                    self.focused_field = None;
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
                self.clear_for_dialog();
                self.forget_target = Some(path);
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
                        // Feature 024: an app-initiated clear like any other, so it goes through
                        // the same function (contract §3's table). It arms nothing — there is no
                        // session and, after a forget, no project either.
                        self.set_current_session(None);
                    }
                }
                self.forget_target = None;
            }
            Message::ProjectForgetCancelled => {
                self.forget_target = None;
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
                self.toggle_location(SessionLocation::Worktree(dir));
            }
            Message::DefaultExpansionToggled => {
                self.toggle_location(SessionLocation::Default);
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
                self.clear_for_dialog();
                self.worktree_delete_target = Some(dir);
                // Never carries a choice over from a previously cancelled/confirmed dialog.
                self.worktree_delete_keep_branch = false;
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
            }
            Message::WorktreeDeleteCancelled => {
                self.worktree_delete_target = None;
            }
            Message::WorktreeDeleteKeepBranchToggled(keep) => {
                self.worktree_delete_keep_branch = keep;
            }
            Message::WorktreeRenameStarted(dir) => {
                let text = self.worktree_display_name(&dir);
                self.clear_for_dialog();
                self.worktree_rename_draft = Some(WorktreeRenameDraft {
                    dir_name: dir,
                    text,
                    error: None,
                });

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
            Message::SidebarViewportResized(height) => {
                self.sidebar_viewport_height = height;
            }
            Message::SidebarScrolled(offset) => {
                self.sidebar_scroll_offset = offset;
                // The sidebar's scroll is *also* the dismissal trigger, and the rendering stack
                // gives a scrollable one message per event — so this arm does both rather than the
                // view trying to emit two. Same rule, one call, no second copy of it.
                self.dismiss_on_scroll_beneath();
            }
            Message::ScrolledBeneathOverlay => self.dismiss_on_scroll_beneath(),
            Message::EscapePressed => self.dismiss_topmost(),
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
                self.clear_for_dialog();
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
                        // …and takes the search with it, so returning never resumes someone
                        // else's half-finished query (feature 021).
                        form.reset_branch_search();
                    }
                }
            }
            Message::AddWorktreeBranchesListed(candidates) => {
                if let Some(form) = &mut self.worktree_form {
                    form.candidates = candidates;
                    // The results describe the current query, whenever the candidates arrive.
                    form.rematch_branches();
                }
            }
            Message::AddWorktreeBranchSelected(candidate) => {
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting()
                    {
                        // FR-012a: a branch held elsewhere cannot be chosen. Silently, and without
                        // closing the list — a press that does nothing must not look like a press
                        // that did something.
                        if !candidate.is_available() {
                            return;
                        }
                        form.selected_branch = Some(candidate);
                        form.error = None;
                        form.branch_list_open = false;
                        // The query is deliberately left alone (FR-014a).
                    }
                }
            }
            Message::AddWorktreeBranchFocused => {
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting()
                    {
                        form.branch_list_open = true;
                    }
                }
            }
            Message::AddWorktreeBranchQueryChanged(text) => {
                if let Some(form) = &mut self.worktree_form {
                    if form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting()
                    {
                        form.branch_query = text;
                        form.branch_list_open = true;
                        form.rematch_branches();
                    }
                }
            }
            Message::AddWorktreeBranchHighlightMoved(direction) => {
                if let Some(form) = &mut self.worktree_form {
                    // Saturating, not wrapping — and the rule itself is `micold_core`'s, not this
                    // arm's (FR-017a, FR-021). An empty list has nowhere to land, so the highlight
                    // is left exactly as it was.
                    let rows = form.branch_matches.len();
                    if let Some(next) = move_highlight(form.branch_highlight, direction, rows) {
                        form.branch_highlight = Some(next);
                    }
                }
            }
            Message::AddWorktreeBranchDismissed => {
                if let Some(form) = &mut self.worktree_form {
                    form.branch_list_open = false;
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
                    form.stage_detail = None;
                }
            }
            Message::WorktreeCreateStageChanged(stage, detail) => {
                if let Some(form) = &mut self.worktree_form {
                    // Entering a stage clears the previous stage's trailing line — it described
                    // work that is over. A detail-only push keeps the stage and replaces the line.
                    if form.stage != Some(stage) {
                        form.stage = Some(stage);
                        form.stage_detail = None;
                    }
                    if detail.is_some() {
                        form.stage_detail = detail;
                    }
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
                // The started session's location joins the user's own open set, as it always has.
                //
                // Feature 024 makes this redundant for *display* — `set_current_session` below
                // opens the row by derivation anyway — but it is kept, and not only because
                // feature 021's assertion freeze forbids rewriting the expectation. It is also
                // what the reveal would do a moment later: the commit in `set_current_session`
                // turns a revealed row into ordinary user-open state on the next change of current
                // session, so writing it here reaches the same place by the same rule, sooner.
                match location {
                    SessionLocation::Worktree(dir) => {
                        self.expanded.insert(dir);
                    }
                    SessionLocation::Default => {
                        self.default_expanded = true;
                    }
                }
                self.set_current_session(Some(id));
                // Making a session the displayed session puts the user in front of a terminal,
                // so it holds the keyboard (FR-011). No re-assertion from the gui path any more:
                // nothing releases focus on the same click, so there is no race to win.
            }
            Message::SessionSelected(id) => {
                // Feature 024: the ONE writer that does not go through `set_current_session`
                // (contract §3.0). The user clicked a row they could already see, so revealing it
                // would open nothing they had not opened and scroll a list they were reading
                // (FR-006). `tests/current_session_writers.rs` knows about this exemption by name;
                // any other direct write to `active_session` fails that gate.
                self.active_session = Some(id);
                // Selecting a session displays its terminal, so it holds the keyboard, clearing
                // any earlier release (FR-011, FR-021a).
                self.focus_terminal();
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
                // Switching mode puts a different terminal in front of the user, so it holds the
                // keyboard (FR-011). This is the navigation the reported bug was about: it used to
                // take two presses to reach and then left you looking at a terminal that ignored
                // the keyboard.
                self.focus_terminal();
            }
            Message::TerminalRestartRequested => {
                // No pure state to update here — the binary decides which process to spawn
                // based on the current mode and follows up with SessionRunning/
                // ShellInstanceRunning once it's actually up (mirrors SessionStartRequested).
            }
            Message::ShellInstanceOpenRequested => {
                // No session state to update here — the binary decides whether the active session
                // is in Regular mode, opens the instance (`Session::open_shell_instance`), and
                // spawns its process, following up with `ShellInstanceRunning` once it's up.
                // The new instance is what the user will be looking at, so it holds the keyboard
                // (FR-011).
                self.focus_terminal();
            }
            Message::ShellInstanceSelected(id, shell_id) => {
                if let Some(session) = self.session_mut(id) {
                    session.select_shell(shell_id);
                }
                self.focus_terminal(); // FR-011
            }
            Message::ShellInstanceCloseRequested(id, shell_id) => {
                if let Some(session) = self.session_mut(id) {
                    session.close_shell(shell_id);
                }
                // Whichever instance takes its place is what the user is now looking at (FR-011).
                self.focus_terminal();
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
                    // Feature 024: through `set_current_session` so the row the closed session was
                    // in is committed open rather than snapping shut and taking its siblings out of
                    // view (FR-001c). Nothing is armed — no session is current to scroll to, and
                    // FR-001a forbids moving the panel when the user closes the session they were
                    // on.
                    //
                    // Nothing to clear alongside it: with no displayed session `terminal_focused()`
                    // is already false (feature 023, FR-012/FR-016).
                    self.set_current_session(None);
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
                self.clear_for_dialog();
                self.session_remove_target = Some(id);
            }
            Message::SessionRemoveConfirmed => {
                // Unlike close (archive), remove drops the record outright — the pre-BUG-003
                // close behavior. The binary has already killed the process and recorded the
                // durable suppression marker (FR-015c, FR-020c).
                if let Some(id) = self.session_remove_target.take() {
                    // Feature 024: clear the pointer BEFORE dropping the record. The commit that
                    // keeps the row open (FR-001c) resolves the outgoing session's location by
                    // looking it up — and a record already removed has no location to find, so
                    // ordering it the other way collapses the row and takes its siblings out of
                    // view. The close arm above is not exposed to this: archiving leaves the
                    // record in place.
                    if self.active_session == Some(id) {
                        self.set_current_session(None);
                    }
                    if let Some(path) = self.workspace.active.clone() {
                        if let Some(list) = self.workspace.sessions.get_mut(&path) {
                            list.retain(|s| s.id != id);
                        }
                    }
                }
            }
            Message::SessionRemoveCancelled => {
                self.session_remove_target = None;
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
                self.focus_terminal();
            }
            Message::TerminalFocusReleased => {
                self.release_terminal();
            }
            Message::TerminalContextMenuOpened { x, y } => {
                self.terminal_context_menu = Some((x, y));
            }
            Message::TerminalContextMenuClosed => {
                self.terminal_context_menu = None;
            }
            Message::SettingsOpened => {
                self.clear_for_dialog();
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
                self.settings_draft = None;
            }
            Message::SettingsCancelled => {
                self.settings_draft = None;
            }
            Message::NotificationDismissed => self.notify.dismiss(),
            Message::NotificationsAdvanced(elapsed_ms) => {
                self.notify.advance(Duration::from_millis(u64::from(elapsed_ms)));
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
            // Clearing the target *is* closing the dialog since T037; there is no second slot
            // left to reset.
            self.worktree_delete_target = None;
        }
        // Prune rename overrides for the active project's worktrees that are gone (FR-015).
        if let Some(active) = self.workspace.active.clone() {
            if let Some(map) = self.workspace.worktree_names.get_mut(&active) {
                map.retain(|dir, _| names.contains(dir));
            }
        }
    }
}

/// Map an Escape key press to a [`Message`] given the current state.
///
/// Returns `Some(AboutClosed)` only while the About overlay is open (FR-011); returns
/// `None` otherwise, so pressing Esc with no dialog open has no effect (edge case). The
/// iced keyboard subscription in the binary delegates to this pure function.
///
/// Escape goes to the topmost open surface, whichever that is: a dialog outranks the sidebar
/// filter panel and every other popover (contract D1), and with nothing open it goes nowhere.
///
/// **No longer a match** (feature 021, T033). This asked the enum for the open dialog's cancel
/// message and hand-checked the one popover it knew about; the priority between the two was
/// written out here and mirrored, by hand, in the keyboard subscription. It is now the registry's
/// [`escape`](crate::overlay::registry::escape), which reads the band ordering the core already
/// declares — so a surface added tomorrow is reached without this function hearing of it.
///
/// The function itself survives only as the name the scrim and the tests already call; T034
/// collapses the keyboard subscription onto the same call and this goes with it.
pub fn on_escape(state: &State) -> Option<Message> {
    crate::overlay::registry::escape(state)
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

/// Whether keystrokes may be written to a session's shell PTY given its `ShellLifecycle`
/// (feature 010, mirrors [`should_write_to`] for the shell process): only while `Running`.
pub fn should_write_to_shell(shell_lifecycle: micold_core::session::ShellLifecycle) -> bool {
    matches!(
        shell_lifecycle,
        micold_core::session::ShellLifecycle::Running
    )
}
