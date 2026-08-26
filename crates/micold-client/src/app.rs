//! Render-free application core: state, messages, and the `update` reducer.
//!
//! This module has no dependency on iced. All state transitions are pure and
//! unit-testable via `cargo test` (Constitution Principle I). The GUI binary adapts
//! this core to iced's runtime in `src/main.rs`.
//!
//! Side-effectful concerns (reading the filesystem for a directory listing, detecting a
//! folder's git status, and persisting the catalog) are performed by the binary at the
//! I/O boundary; the reducer stays pure. A few messages (`project::Msg::SelectorOpened`,
//! `project::Msg::FolderChosen`) therefore carry no reducer effect here — they are documented
//! no-ops handled entirely in `src/main.rs`.

use crate::features::notifications::NoticeLevel;
use crate::features::project::{ProjectMenu, RenameDraft, SwitcherEntry};
use crate::features::session::SessionMenu;
use crate::features::sidebar::TagFilter;
use crate::features::worktree::{WorktreeMenu, WorktreeRenameDraft};
use micold_core::notify;
use micold_core::project::Availability;
use micold_core::selector::Selector;
use micold_core::session::SessionId;
use micold_core::theme::{resolve, ColorScheme};
use micold_core::worktree::Worktree;
use std::collections::BTreeSet;
use std::path::PathBuf;

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
    /// The Help menu and the About dialog it opens (feature 028, FR-001). Three variants moved
    /// behind this one; see [`crate::features::help::Msg`].
    Help(crate::features::help::Msg),
    /// Everything the user or the folder browser can say about a project (feature 028,
    /// FR-001). Nineteen variants moved behind this one; see
    /// [`crate::features::project::Msg`].
    Project(crate::features::project::Msg),

    /// What the window reports about itself (feature 028, FR-001). Two variants moved behind this
    /// one; see [`crate::features::window::Msg`].
    Window(crate::features::window::Msg),

    /// Everything the user can do to their settings (feature 028, FR-001). Ten variants moved
    /// behind this one; see [`crate::features::settings::Msg`].
    Settings(crate::features::settings::Msg),

    // ---- Feature 005: worktrees, sessions, embedded terminal ----
    /// Everything the user or the daemon can say about a worktree (feature 028, FR-001).
    /// Eighteen variants moved behind this one; see [`crate::features::worktree::Msg`].
    Worktree(crate::features::worktree::Msg),
    /// Everything the user can do to the sidebar (feature 028, FR-001). Ten variants moved
    /// behind this one; see [`crate::features::sidebar::Msg`].
    Sidebar(crate::features::sidebar::Msg),

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
    /// Everything a session, its terminal panes and its tab strip say about themselves (feature
    /// 028, FR-001). Thirty-seven variants — the largest feature in the application — moved behind
    /// this one; see [`crate::features::session::Msg`].
    Session(crate::features::session::Msg),
    /// A dialog has finished animating out (feature 017, FR-011). Emitted by the `Modal` component
    /// itself, which owns the transition, so the binary can release the snapshot it was rendering
    /// from ([`crate::overlay::registry::Closing`]). The binary used to watch a central progress
    /// value for this; the
    /// component now says it, which is the only part of a transition an application still needs.
    OverlayTransitionFinished,
    /// Everything the add-worktree wizard says about itself (feature 021, T064 — FR-003).
    ///
    /// **The only nested unit in the application**, and research.md §5 tested every feature against
    /// FR-003's bar to say so: the form is opened, edited across several steps and then submitted
    /// or dismissed as a unit, and no other feature reads its intermediate state. Twenty-two
    /// variants — 17% of this enum — collapse to this one.
    WorktreeForm(crate::features::worktree_form::Msg),
    /// The OS window gained (`true`) or lost (`false`) input focus. Handled by the binary,
    /// which gates the terminal/OS-theme poll subscriptions on it so a backgrounded window
    /// doesn't keep burning CPU on ticks nothing is looking at (idle-CPU fix).
    WindowFocusChanged(bool),

    // ---- Global notification surface ----
    /// What happens to the notification on screen (feature 028, FR-001). Two variants moved behind
    /// this one; see [`crate::features::notifications::Msg`].
    Notifications(crate::features::notifications::Msg),

    // ---- Feature 010: daemon connection (client of the daemon-hosted sessions) ----
    /// Everything the daemon connection reports or is asked to do (feature 028, FR-001). Twelve
    /// variants moved behind this one; see [`crate::features::connection::Msg`].
    ///
    /// All twelve are effects, so this is the one feature whose reducer entry is only in the shell
    /// (data-model §1.1, shape B). `State::update` declines it, as it declined each of the twelve.
    Connection(crate::features::connection::Msg),

    /// A completed side-effecting task that carries nothing to apply (e.g. the daemon-stop task).
    NoOp,
}

/// Root application state for the single main window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// What the help feature remembers -- see [`crate::features::help::State`].
    pub help: crate::features::help::State,
    /// The known-projects catalog and the active working space (persisted). Per-story
    /// selector/rename working state is added alongside those stories.
    pub workspace: micold_core::workspace::Workspace,
    /// The folder-browser state; its presence *is* the project-selector dialog being shown (T037).
    pub selector: Option<Selector>,
    /// The in-progress rename; its presence *is* the rename dialog being shown (T037).
    pub rename_draft: Option<RenameDraft>,
    /// What the window feature remembers -- see [`crate::features::window::State`].
    pub window: crate::features::window::State,
    /// What the settings feature remembers -- see [`crate::features::settings::State`].
    pub settings: crate::features::settings::State,
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
    /// The tab strip's scroll offset, in whole pixels from its leading edge (feature 026 FR-002e).
    ///
    /// Presentation, not state to persist: FR-002d scrolls the marked tab into view on selection,
    /// and where the user has scrolled to is not remembered across sessions or restarts (spec
    /// Assumptions). It lives here only because the edge fade and the reveal both have to read it,
    /// and only the reducer sees both.
    pub tab_strip_scroll_offset: u32,
    /// The tab strip viewport's laid-out width, in whole pixels. `0` until the first layout, which
    /// reads as "cannot decide yet" and never as "nothing fits" — the same rule
    /// [`Self::sidebar_viewport_height`] follows, and for the same reason.
    pub tab_strip_viewport_width: u32,
    /// Whether the marked tab is waiting to be scrolled into view (feature 026 FR-002d).
    ///
    /// A flag, not a target, for the reason [`Self::pending_reveal_scroll`] is one: the offset
    /// cannot be computed when the selection changes, because the viewport's width is not known
    /// until layout. The reducer arms it; the binary computes and applies the scroll on the first
    /// frame where the viewport has a width.
    pub pending_tab_reveal: bool,
    /// What the worktree_form feature remembers -- see [`crate::features::worktree_form::State`].
    pub worktree_form: crate::features::worktree_form::State,
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
    /// The open terminal-tab context menu — which instance it belongs to, and where it was opened
    /// in window pixels — or `None` when no menu is showing (feature 012, BUG-005, FR-010b).
    ///
    /// Carries the instance because the menu acts on the tab it was opened on, **not** on the
    /// active one: restarting a background instance without selecting it first is the whole of
    /// FR-010a, and it is what addressing the restart message by instance id was built for.
    /// Window pixels rather than the pane-local point [`State::terminal_context_menu`] holds — that
    /// one is drawn on the pane's own overlay because a pane's origin is not known at render time,
    /// and this one is drawn on the window's, where the anchor is already in the right space.
    pub shell_instance_menu: Option<(crate::ui::terminal::StripTab, u16, u16)>,
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
    /// What the notifications feature remembers — see
    /// [`crate::features::notifications::State`].
    ///
    /// Rendered unconditionally so no failure can be swallowed by an unreachable render path (see
    /// [`notify::Notification`]).
    pub notifications: crate::features::notifications::State,
    /// How far the worktree sidebar is scrolled, in logical pixels.
    ///
    /// The app bar's elevation derives from this and nothing else (FR-025a) — see
    /// [`Self::app_bar_elevated`] for why a second source would be a defect rather than a feature.
    pub sidebar_scroll_offset: u32,
    /// Whether the top-bar project switcher panel is open. Mutually exclusive with
    /// `help_menu_open`.
    pub project_switcher_open: bool,
    /// The open project right-click context menu (feature 015), with the project it acts on and
    /// the press point to draw it at. At most one is open. Mutually exclusive with the other
    /// popovers, but the switcher panel itself stays open behind it. Transient — not persisted.
    pub project_menu_open: Option<ProjectMenu>,
    /// The worktree whose right-click context menu is open, and where it was opened from
    /// (feature 008). At most one is open at a time; `None` means no menu is showing.
    pub worktree_menu_open: Option<WorktreeMenu>,
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
    /// The session whose right-click context menu is open, and where it was opened from (bugfix
    /// BUG-003). At most one is open at a time; `None` means no menu is showing. Mirrors
    /// `worktree_menu_open`.
    pub session_menu_open: Option<SessionMenu>,
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
        resolve(self.settings.theme_pref, self.settings.system_scheme)
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
        self.notifications
            .queue
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
            && self.window.focused_field.is_none()
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

    /// Apply a [`Message`], transitioning the state. Pure and side-effect free.
    pub fn update(&mut self, message: Message) {
        match message {
            // Daemon connection messages are runtime, not pure state — the binary handles them in
            // `update_inner` and never routes them here. Listed explicitly (not a catch-all) so the
            // core reducer stays exhaustive over `Message` and a future variant is a compile error.
            //
            // Since T011 the whole connection vocabulary arrives under one wrapper, so declining it
            // is one arm rather than twelve. That is not a weaker statement: `Message` is still
            // matched exhaustively, and a thirteenth connection message is a compile error in
            // `shell/connection.rs` — which is the file that would have to decide what to do with
            // it — rather than here, where the answer is and always was "nothing".
            Message::Connection(_) | Message::NoOp => {}
            Message::Help(msg) => {
                let outcomes = crate::features::help::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::Project(msg) => {
                let outcomes = crate::features::project::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::Window(msg) => {
                let outcomes = crate::features::window::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::Settings(msg) => {
                let outcomes = crate::features::settings::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::Worktree(msg) => {
                // The root is the only interpreter (FR-022, contract O3), and this is where the
                // draining loop finally has something to drain.
                let outcomes = crate::features::worktree::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::Sidebar(msg) => {
                let outcomes = crate::features::sidebar::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::Session(msg) => {
                let outcomes = crate::features::session::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::ScrolledBeneathOverlay => self.dismiss_on_scroll_beneath(),
            Message::EscapePressed => self.dismiss_topmost(),
            Message::WorktreeForm(msg) => {
                let outcomes = crate::features::worktree_form::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }
            Message::Notifications(msg) => {
                let outcomes = crate::features::notifications::update(self, msg);
                drain(outcomes, |outcome| interpret(self, outcome));
            }

            // The closing dialog's snapshot is a binary-owned render detail (`App::dismissing`),
            // so releasing it is the binary's business; the pure core never knew about it.
            Message::OverlayTransitionFinished
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
    #[must_use = "the sidebar's expansion is pruned by draining this, not by `set_worktrees` (T066)"]
    pub fn set_worktrees(&mut self, worktrees: Vec<Worktree>) -> Vec<crate::features::Outcome> {
        self.worktrees = worktrees;
        let names: BTreeSet<String> = self.worktrees.iter().map(|w| w.dir_name.clone()).collect();

        if self
            .worktree_menu_open
            .as_ref()
            .is_some_and(|m| !names.contains(&m.dir_name))
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
        // Everything above is worktree-owned. `expanded` is the sidebar's, so it is reported
        // rather than pruned here (FR-020/FR-021, contract O2) — this is the write T066 converted.
        vec![crate::features::Outcome::WorktreesReplaced(names)]
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
/// Apply one outcome, and return anything applying it produced (FR-022, contract O3).
///
/// **The root is the only interpreter of cross-feature consequences**, which is the whole of O3:
/// the feature that emitted `SessionsClosed` does not know how a session is dropped, and the
/// session feature does not know a worktree was deleted. Neither learns anything about the other;
/// this function is the only place the two meet.
///
/// # `ClipboardWrite` is not the root's, and the arm says so
///
/// It is an *effect request* under FR-015a, interpreted by `shell::clipboard::interpret` — the one
/// outcome whose destination is outside the pure core entirely. The shell partitions the queue
/// before draining, so this arm is unreachable in practice; it returns nothing rather than
/// panicking, because an outcome vocabulary that aborts on a variant it does not own would make
/// every future effect request a crash waiting for its shell arm.
///
/// # Returning `Vec<Outcome>` when nothing yet returns any
///
/// Interpreting one outcome may emit another — the spec's Edge Cases name the case, and
/// [`drain`]'s bound exists for it (O4). None of these three does so today. The signature is what
/// makes that a fact about the current interpretations rather than a shape the vocabulary cannot
/// express.
pub fn interpret(
    state: &mut State,
    outcome: crate::features::Outcome,
) -> Vec<crate::features::Outcome> {
    use crate::features::Outcome;
    match outcome {
        Outcome::SessionsClosed(ids) => return crate::features::session::closed(state, &ids),
        Outcome::OverlayDismissed(id) => crate::overlay::registry::dismiss(state, id),
        Outcome::SurfaceOpened(id) => crate::overlay::registry::displace(state, id),
        Outcome::NotificationRaised(notification) => state.notifications.queue.push(notification),
        Outcome::WorktreesReplaced(names) => {
            crate::features::sidebar::worktrees_replaced(state, &names);
            crate::features::worktree_form::worktree_list_changed(state);
        }
        Outcome::WorktreeCreated(worktree) => {
            return crate::features::worktree::created(state, worktree)
        }
        Outcome::LocationOpened(location) => state.location_opened(&location),
        Outcome::RevealScrollArmed => state.reveal_scroll_armed(),
        Outcome::ProjectEntered => state.project_entered(),
        Outcome::RevealSuppressed(suppressed) => state.reveal_suppression_set(suppressed),
        Outcome::FieldFocusCleared => crate::features::window::field_focus_cleared(state),
        Outcome::ClipboardWrite(_) => {}
    }
    Vec::new()
}

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
