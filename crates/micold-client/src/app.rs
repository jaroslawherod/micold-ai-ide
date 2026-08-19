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
use crate::features::window::FieldId;
use crate::features::worktree::WorktreeRenameDraft;
use crate::features::worktree_form::WorktreeForm;
use micold_core::notify;
use micold_core::project::{Availability, FolderEntry};
use micold_core::selector::Selector;
use micold_core::session::{Session, SessionId, SessionLocation, ShellInstanceId};
use micold_core::theme::{resolve, ColorScheme, SystemScheme, ThemePreference};
use micold_core::worktree::Worktree;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

/// Minimum sidebar width in pixels (resize lower bound).
pub const SIDEBAR_MIN_WIDTH: u16 = 180;
/// Maximum sidebar width in pixels (resize upper bound).
pub const SIDEBAR_MAX_WIDTH: u16 = 600;
/// Default sidebar width in pixels, used until the user resizes it.
pub const SIDEBAR_DEFAULT_WIDTH: u16 = 300;

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

    // ---- 016 BUG-002: showing a worktree the app does not manage ----
    /// Ask the daemon to show the worktree at this absolute path among the project's own
    /// (FR-027). Raised from the blocked-branch explanation, which is where the user meets a
    /// holder they cannot otherwise reach.
    WorktreeIncludeRequested(PathBuf),
    /// The daemon is now showing it. The row also arrives with the next catalog push; this is what
    /// makes it appear at the moment the user asked rather than at the next refresh.
    WorktreeIncluded(Worktree),
    /// Stop showing an included worktree, by `dir_name` (FR-030). Nothing on disk is touched.
    WorktreeExcludeRequested(String),
    /// The daemon has stopped showing the worktree at this path.
    WorktreeExcluded(PathBuf),
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
    /// Everything the add-worktree wizard says about itself (feature 021, T064 — FR-003).
    ///
    /// **The only nested unit in the application**, and research.md §5 tested every feature against
    /// FR-003's bar to say so: the form is opened, edited across several steps and then submitted
    /// or dismissed as a unit, and no other feature reads its intermediate state. Twenty-two
    /// variants — 17% of this enum — collapse to this one.
    WorktreeForm(crate::features::worktree_form::Msg),
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
    ///
    /// **Emitted nowhere in production** — the daemon owns this transition and publishes it in the
    /// catalog snapshot (FR-006d, `010` BUG-011), which `reconcile_catalog` adopts unconditionally.
    /// Kept as the reducer's own `→ Running` edge, which the state tests drive directly; deleting it
    /// is a separate change from the one that made the daemon report the transition at all.
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
    ///
    /// **Emitted nowhere in production**, like [`Self::SessionRunning`]. The daemon owns this
    /// transition and publishes it as `SessionSummary::live_shells`, which `reconcile_catalog`
    /// adopts (`012` FR-008, BUG-003). Kept as the reducer's own `→ Running` edge: it is the only
    /// lever the integration tests in `tests/` have, since `reconcile_catalog` lives in the binary
    /// crate and those tests can only reach the library.
    ShellInstanceRunning(SessionId, ShellInstanceId),
    /// A Regular Terminal instance's shell process exited (intentional or crash) — never
    /// auto-restarted (FR-008; replaces feature 010's `ShellSessionExited(SessionId)`).
    ///
    /// Emitted nowhere in production, for the same reason and with the same caveat as
    /// [`Self::ShellInstanceRunning`] above.
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
    /// Why entering a project landed on the session it did, from the most recent switch.
    ///
    /// Diagnostic only — nothing renders from it and nothing branches on it. It exists because
    /// "the app forgot which session I was on" is a report with four possible causes, and the one
    /// that matters most (a resolve looking under a key nothing is filed under) is invisible from
    /// the outside. The binary writes it to the client log at the I/O boundary.
    pub last_foreground_choice: Option<crate::features::session::ForegroundChoice>,
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
    pub(crate) fn dismiss_on_scroll_beneath(&mut self) {
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
            Message::HelpMenuToggled => crate::features::help::menu_toggled(self),
            Message::ProjectSwitcherToggled => crate::features::project::switcher_toggled(self),
            Message::AboutOpened => crate::features::help::about_opened(self),
            Message::AboutClosed => crate::features::help::about_closed(self),
            Message::SelectorNavigatedInto(path) => {
                crate::features::project::selector_navigated_into(self, path)
            }
            Message::SelectorNavigatedUp => crate::features::project::selector_navigated_up(self),
            Message::SelectorListingReady(entries) => {
                crate::features::project::selector_listing_ready(self, entries)
            }
            Message::SelectorListingFailed(message) => {
                crate::features::project::selector_listing_failed(self, message)
            }
            Message::ProjectSelectorClosed => crate::features::project::selector_closed(self),
            Message::RenameStarted(path) => crate::features::project::rename_started(self, path),
            Message::FieldFocusChanged(field, focused) => {
                crate::features::window::field_focus_changed(self, field, focused)
            }
            Message::RenameTextChanged(text) => {
                crate::features::project::rename_text_changed(self, text)
            }
            Message::RenameConfirmed => crate::features::project::rename_confirmed(self),
            Message::RenameCancelled => crate::features::project::rename_cancelled(self),
            Message::CursorMoved { x, y } => crate::features::window::cursor_moved(self, x, y),
            Message::WindowResized { width, height } => {
                crate::features::window::resized(self, width, height)
            }
            Message::ProjectMenuToggled(path) => crate::features::project::menu_toggled(self, path),
            Message::ProjectMenuDismissed => crate::features::project::menu_dismissed(self),
            Message::ProjectForgetRequested(path) => {
                crate::features::project::forget_requested(self, path)
            }
            Message::ProjectForgetConfirmed => crate::features::project::forget_confirmed(self),
            Message::ProjectForgetCancelled => crate::features::project::forget_cancelled(self),
            Message::ThemeModeCycled => crate::features::settings::theme_mode_cycled(self),
            Message::ThemePreferenceChanged(pref) => {
                crate::features::settings::theme_preference_changed(self, pref)
            }
            Message::SystemThemeChanged(detected) => {
                crate::features::settings::system_theme_changed(self, detected)
            }
            Message::ProjectOpenRefused(message) => {
                crate::features::project::open_refused(self, message)
            }
            Message::WorktreesLoaded(worktrees) => crate::features::worktree::loaded(self, worktrees),
            Message::WorktreeExpansionToggled(dir) => {
                self.toggle_location(SessionLocation::Worktree(dir));
            }
            Message::DefaultExpansionToggled => {
                self.toggle_location(SessionLocation::Default);
            }
            Message::WorktreeMenuToggled(dir) => crate::features::worktree::menu_toggled(self, dir),
            Message::WorktreeMenuDismissed => crate::features::worktree::menu_dismissed(self),
            // 016 BUG-002. The request itself changes nothing here: the daemon owns the included
            // set, as it owns every other piece of durable state, and answers with the worktree as
            // its own discovery sees it.
            Message::WorktreeIncludeRequested(_) => {}
            Message::WorktreeIncluded(worktree) => crate::features::worktree::included(self, worktree),
            Message::WorktreeExcludeRequested(_) => crate::features::worktree::exclude_requested(self),
            Message::WorktreeExcluded(path) => crate::features::worktree::excluded(self, path),
            Message::WorktreeDeleteRequested(dir) => crate::features::worktree::delete_requested(self, dir),
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
            Message::WorktreeDeleteConfirmed => crate::features::worktree::delete_confirmed(self),
            Message::WorktreeDeleteCancelled => crate::features::worktree::delete_cancelled(self),
            Message::WorktreeDeleteKeepBranchToggled(keep) => {
                crate::features::worktree::delete_keep_branch_toggled(self, keep)
            }
            Message::WorktreeRenameStarted(dir) => crate::features::worktree::rename_started(self, dir),
            Message::WorktreeRenameTextChanged(text) => {
                crate::features::worktree::rename_text_changed(self, text)
            }
            Message::WorktreeRenameConfirmed => crate::features::worktree::rename_confirmed(self),
            Message::WorktreeRenameCancelled => crate::features::worktree::rename_cancelled(self),
            Message::SidebarFilterToggled(filter) => {
                crate::features::sidebar::filter_toggled(self, filter)
            }
            Message::SidebarFiltersCleared => crate::features::sidebar::filters_cleared(self),
            Message::SidebarViewportResized(height) => {
                crate::features::sidebar::viewport_resized(self, height)
            }
            Message::SidebarScrolled(offset) => crate::features::sidebar::scrolled(self, offset),
            Message::ScrolledBeneathOverlay => self.dismiss_on_scroll_beneath(),
            Message::EscapePressed => self.dismiss_topmost(),
            Message::SidebarFilterMenuToggled => {
                crate::features::sidebar::filter_menu_toggled(self)
            }
            Message::ShowAgentWorktreesToggled => {
                crate::features::sidebar::show_agent_worktrees_toggled(self)
            }
            Message::WorktreeHovered(dir) => crate::features::worktree::hovered(self, dir),
            Message::WorktreeUnhovered(dir) => crate::features::worktree::unhovered(self, dir),
            Message::WorktreeForm(msg) => crate::features::worktree_form::update(self, msg),
            Message::SessionStarted(session) => crate::features::session::started(self, session),
            Message::SessionSelected(id) => crate::features::session::selected(self, id),
            Message::SessionRunning(id) => crate::features::session::running(self, id),
            Message::SessionTitleUpdated { id, title } => {
                crate::features::session::title_updated(self, id, title)
            }
            Message::TerminalModeToggled => crate::features::session::mode_toggled(self),
            Message::TerminalRestartRequested => {
                // No pure state to update here — the binary decides which process to spawn based on
                // the current mode. For an AI-CLI session the daemon owns the lifecycle and
                // announces `Running` in the catalog snapshot once the process exists (FR-006d,
                // `010` BUG-011); `reconcile_catalog` adopts it. This comment used to claim a
                // follow-up `SessionRunning` message, which is emitted nowhere — believing it cost
                // BUG-011 a round of investigation, because it made a state bug look like a
                // transport one.
            }
            Message::ShellInstanceOpenRequested => {
                // No session state to update here — the binary decides whether the active session
                // is in Regular mode, opens the instance (`Session::open_shell_instance`), and
                // spawns its process. The daemon then reports it in `SessionSummary::live_shells`
                // and `reconcile_catalog` marks it running (`012` FR-008, BUG-003); this used to
                // claim a follow-up `ShellInstanceRunning` message, which is emitted nowhere and
                // is why every instance sat at `NotStarted` for its whole life.
                // The new instance is what the user will be looking at, so it holds the keyboard
                // (FR-011).
                self.focus_terminal();
            }
            Message::ShellInstanceSelected(id, shell_id) => {
                crate::features::session::shell_instance_selected(self, id, shell_id)
            }
            Message::ShellInstanceCloseRequested(id, shell_id) => {
                crate::features::session::shell_instance_close_requested(self, id, shell_id)
            }
            Message::ShellInstanceRestartRequested(..) => {
                // No pure state to update here — the binary spawns the process, and the daemon's
                // next snapshot reports the instance live (`012` FR-008, BUG-003). Mirrors
                // `TerminalRestartRequested`, including that neither emits a follow-up message.
            }
            Message::ShellInstanceRunning(session_id, shell_id) => {
                crate::features::session::shell_instance_running(self, session_id, shell_id)
            }
            Message::ShellInstanceExited(session_id, shell_id) => {
                crate::features::session::shell_instance_exited(self, session_id, shell_id)
            }
            Message::SessionCloseRequested(id) => crate::features::session::close_requested(self, id),
            Message::SessionMenuToggled(id) => crate::features::session::menu_toggled(self, id),
            Message::SessionMenuDismissed => crate::features::session::menu_dismissed(self),
            Message::SessionRemoveRequested(id) => crate::features::session::remove_requested(self, id),
            Message::SessionRemoveConfirmed => crate::features::session::remove_confirmed(self),
            Message::SessionRemoveCancelled => crate::features::session::remove_cancelled(self),
            Message::TerminalTick => {}
            Message::SidebarToggled => crate::features::sidebar::toggled(self),
            // The handle only speaks while it is being dragged, so there is no flag to consult:
            // an arriving width *is* the drag. Clamped here — how wide the sidebar may be is the
            // application's decision, not the edge's.
            Message::SidebarDragMoved(x) => crate::features::sidebar::drag_moved(self, x),

            // ---- Feature 006 ----
            Message::TerminalFocused => {
                self.focus_terminal();
            }
            Message::TerminalFocusReleased => {
                self.release_terminal();
            }
            Message::TerminalContextMenuOpened { x, y } => {
                crate::features::session::context_menu_opened(self, x, y)
            }
            Message::TerminalContextMenuClosed => crate::features::session::context_menu_closed(self),
            Message::SettingsOpened => crate::features::settings::opened(self),
            Message::SettingsScrollbackChanged(text) => {
                crate::features::settings::scrollback_changed(self, text)
            }
            Message::SettingsEnvIncludeEnabledToggled(enabled) => {
                crate::features::settings::env_include_enabled_toggled(self, enabled)
            }
            Message::SettingsEnvIncludePathChanged(text) => {
                crate::features::settings::env_include_path_changed(self, text)
            }
            Message::SettingsEnvIncludeTimeoutChanged(text) => {
                crate::features::settings::env_include_timeout_changed(self, text)
            }
            Message::SettingsSaved => crate::features::settings::saved(self),
            Message::SettingsCancelled => crate::features::settings::cancelled(self),
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

/// How many outcomes one drain may apply before it gives up (feature 021, T060 — FR-024,
/// contract O4).
///
/// **A bound, not a capacity.** Interpreting one outcome may emit another — the spec's Edge Cases
/// name that case — so the queue has no natural end and a cycle would otherwise hang the UI with
/// no error and no frame. Sixty-four is far above any real cascade (the longest known is two: a
/// worktree delete closing sessions, whose closure raises a notification) and far below anything a
/// person would wait through.
pub const OUTCOME_BUDGET: usize = 64;

/// What one drain did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drained {
    /// How many outcomes were applied.
    pub applied: usize,
    /// Whether the bound was reached with work still queued.
    ///
    /// Unreachable in debug — [`drain`] asserts before it can be returned — so this is what a
    /// release build hands the shell to log. `false` on every ordinary drain.
    pub overflowed: bool,
}

/// Apply outcomes until none remain, following any that interpreting one produces (FR-022, FR-024).
///
/// `apply` is the root's interpretation of a single outcome; whatever it returns is queued behind
/// what is already waiting. The loop is the whole of contracts O4 and O5, and each half is a
/// deliberate choice rather than an implementation detail:
///
/// **Bounded (O4).** Exceeding [`OUTCOME_BUDGET`] trips a `debug_assert`, so a cycle fails loudly
/// in every test run rather than freezing the window. A release build stops at the bound and
/// reports it through [`Drained::overflowed`] instead of panicking at a user — this function has
/// no logger of its own, and inventing one here would put an I/O concern in the reducer.
///
/// **First in, first out (O5).** A stack would let one feature's cascade run to completion ahead
/// of an outcome another feature had already emitted, which makes the interleaving depend on which
/// feature was composed first — exactly what O5 forbids. A queue applies each feature's outcomes in
/// its own emission order no matter where it sits in the composition.
///
/// Generic over `apply` because the properties above are properties of the loop, not of any
/// variant: `tests/outcome_termination.rs` drives it with a cycle no real interpretation can
/// produce, which is the only way to observe the bound being reached at all.
pub fn drain<F>(
    initial: impl IntoIterator<Item = crate::features::Outcome>,
    mut apply: F,
) -> Drained
where
    F: FnMut(crate::features::Outcome) -> Vec<crate::features::Outcome>,
{
    let mut queue: std::collections::VecDeque<crate::features::Outcome> =
        initial.into_iter().collect();
    let mut applied = 0usize;
    while let Some(outcome) = queue.pop_front() {
        if applied == OUTCOME_BUDGET {
            debug_assert!(
                false,
                "outcome interpretation exceeded {OUTCOME_BUDGET} applications with {} still \
                 queued — an outcome cycle (FR-024, contract O4)",
                queue.len() + 1
            );
            return Drained {
                applied,
                overflowed: true,
            };
        }
        applied += 1;
        queue.extend(apply(outcome));
    }
    Drained {
        applied,
        overflowed: false,
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
