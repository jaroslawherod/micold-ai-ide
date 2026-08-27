//! Sessions: which one is in the foreground, and what survives a project switch
//! (feature 021, T021).
//!
//! The switch sequence in [`crate::app::State::switch_active`] is the delicate part of this feature and the
//! reason its helpers belong together: the order of record-then-activate-then-restore is
//! load-bearing (data-model.md I1), and the private steps it calls are meaningless apart from it.
//!
//! `SelectKind` is here rather than in `features/project.rs`, where T017 filed it. It is terminal
//! text selection, and terminals belong to sessions; T017 swept it in because it happened to sit in
//! that stretch of `app.rs`. Grouping by line range is what FR-001 argues against.
//!
//! These are `impl State` blocks because `State` is still monolithic in Tier 1. Methods resolve on
//! the type rather than the module, so moving them changed no call site.
//!
//! # The vocabulary this feature declares
//!
//! Thirty-seven transitions in [`Msg`], the largest vocabulary in the application: the session
//! lifecycle (`StartRequested`, `Started`, `Running`, `Selected`, `TitleUpdated`, `CloseRequested`,
//! `RemoveRequested`, `RemoveConfirmed`, `RemoveCancelled`, `MenuToggled`, `MenuDismissed`), the
//! shell instances attached to one (`ShellInstance*`, seven of them), the terminal surface
//! (`Terminal*`, sixteen — bytes, selection, scroll, resize, focus, clipboard, context menu, tick,
//! restart, and the AI CLI picker), and the tab strip's geometry (`TabStripScrolled`,
//! `TabStripViewportResized`, `StripTabMenuRequested`).
//!
//! `Terminal`, `ShellInstance` and `TabStrip` name sub-surfaces inside a session rather than the
//! feature, so they stay; the `Session` prefix eleven variants carried is what went.
//!
//! [`update`] is pure (data-model.md §1.1 shape A) and routes all thirty-seven. Twenty are matched a
//! second time in `main.rs` (M2), because each additionally spawns a process, writes to a PTY,
//! scrolls or resizes it, or reaches the clipboard.
//!
//! # The state this feature remembers (feature 028, contract S1)
//!
//! Twelve fields in [`State`], reached as `state.session` — the largest of the nine feature
//! structs, which matches this being the largest vocabulary. They fall into four groups:
//!
//! - **which session is in front**: `active`, and `reveal_suppressed_for`, the session whose revealed
//!   sidebar row the user closed;
//! - **the terminal surface**: `terminal_released` (has the user handed the keyboard back to the
//!   application), `terminal_context_menu` (the right-click menu's anchor);
//! - **the tab strip**: `shell_instance_menu`, `tab_strip_scroll_offset`, `tab_strip_viewport_width`,
//!   `pending_tab_reveal`;
//! - **the session's own surfaces and history**: `menu_open`, `remove_target`,
//!   `restarted_while_inactive`, and the diagnostic `last_foreground_choice`.
//!
//! Nine keep the names they had flat on the root. Three shed the `session` the qualifier carries:
//! `active_session`, `session_menu_open` and `session_remove_target` are `active`, `menu_open` and
//! `remove_target` (T036).
//!
//! **The sessions themselves are not here.** The records, and which session each project was last
//! on, live in `state.workspace` — see [`crate::app::State::workspace`]. What is here is which of
//! them the user is looking at and what is open over it.

use crate::app::Message;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;
use micold_core::project::canonicalize_best_effort;
use micold_core::session::{AiCli, Session, SessionId, SessionLocation, ShellInstanceId};
use std::collections::BTreeSet;
use std::path::Path;

/// What this feature remembers (feature 028, contract S1).
///
/// Twelve of the seventeen keep the names they had as flat members of `app::State`. Five do not:
/// the qualifier already says `session`, so `session.active_session`, `session.session_menu_open`,
/// `session.session_remove_target`, `session.session_start_menu` and `session.session_start_press`
/// would say it twice, and they are `active`, `menu_open`, `remove_target`, `start_menu` and
/// `start_press` here. That is the same trim `worktree.menu_open` and `worktree.delete_target`
/// took, and the two menus they name are still the mirror of each other they were.
///
/// The other twelve name a sub-surface inside a session rather than the feature — `terminal_`,
/// `shell_instance_`, `tab_strip_` — or name no feature at all, so there is nothing for the
/// qualifier to absorb and they are unchanged.
///
/// The reducers below spell the root's type `crate::app::State` now that `State` here means this
/// struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The currently displayed session, if any (FR-012, FR-015).
    ///
    /// Feature 024: written through [`crate::app::State::set_current_session`] by everything except
    /// `SessionSelected`, because the panel's reveal is a consequence of this field changing
    /// rather than of any particular message being handled (contract §3.0).
    pub active: Option<SessionId>,
    /// Why entering a project landed on the session it did, from the most recent switch.
    ///
    /// Diagnostic only — nothing renders from it and nothing branches on it. It exists because
    /// "the app forgot which session I was on" is a report with four possible causes, and the one
    /// that matters most (a resolve looking under a key nothing is filed under) is invisible from
    /// the outside. The binary writes it to the client log at the I/O boundary.
    pub last_foreground_choice: Option<crate::features::session::ForegroundChoice>,
    /// Whether the marked tab is waiting to be scrolled into view (feature 026 FR-002d).
    ///
    /// A flag, not a target, for the reason [`crate::features::sidebar::State::pending_reveal_scroll`] is one: the offset
    /// cannot be computed when the selection changes, because the viewport's width is not known
    /// until layout. The reducer arms it; the binary computes and applies the scroll on the first
    /// frame where the viewport has a width.
    pub pending_tab_reveal: bool,
    /// Sessions that were auto-restarted while their project was inactive, pending a return
    /// notification. Cleared when the user returns to the owner.
    pub restarted_while_inactive: BTreeSet<SessionId>,
    /// The session whose revealed row the user closed (feature 024, FR-005).
    ///
    /// Scoped to a session rather than to a location, so an old collapse cannot swallow the next
    /// reveal: it is compared against `active` and cleared whenever that changes
    /// (invariant I2). `None` means nothing is suppressed.
    ///
    /// This is the *whole* of the reveal's stored state. Which row is open is otherwise derived
    /// from `active` on every view ([`crate::app::State::location_open`]), which is what makes a
    /// wholesale replacement of the worktree list unable to lose it (FR-001b).
    pub reveal_suppressed_for: Option<SessionId>,
    /// The session whose right-click context menu is open, and where it was opened from (bugfix
    /// BUG-003). At most one is open at a time; `None` means no menu is showing. Mirrors
    /// `worktree.menu_open`.
    pub menu_open: Option<SessionMenu>,
    /// The session pending permanent removal, shown in the confirm dialog (bugfix BUG-003,
    /// FR-015c). Its presence *is* the confirm dialog being shown (T037). Mirrors
    /// `worktree.delete_target`.
    pub remove_target: Option<SessionId>,
    /// Which location's "start a session on…" list is open, if any, and where it hangs from
    /// (feature 026, FR-004).
    ///
    /// The location rather than a boolean: the list's items have to name where the session will
    /// start, and every sidebar row can open its own.
    pub start_menu: Option<StartMenu>,
    /// Where the last press on a start affordance landed, in window pixels (feature 026, T089,
    /// FR-029d).
    ///
    /// The point and the decision arrive in separate messages and, unavoidably, in separate event
    /// phases: `ContextArea` reports the point on `ButtonPressed`, while the `IconButton` it wraps
    /// publishes its own message on the *release*. So the point is known first and has to outlive
    /// the message that carried it — [`start_menu_toggled`] reads it when it opens the list.
    /// `None` before the first press; a list is only ever opened by one.
    ///
    /// Transient: where a row was a moment ago is worth nothing after a restart.
    pub start_press: Option<(u16, u16)>,
    /// Which AI CLIs exist **where sessions run**, in `AiCli::ALL`'s order (feature 026 FR-006 and
    /// T014a; re-sourced by feature 027 FR-023c).
    ///
    /// `None` means the service has not said yet — a state that is neither "none available" nor a
    /// guess, and the reason this is an `Option` rather than an empty `Vec`. An empty *answer* is a
    /// real answer (an image that ships no AI CLI is FR-023b's whole scenario) and has to be
    /// distinguishable from not having asked.
    ///
    /// **The client no longer probes its own `PATH` for this.** It used to, through
    /// `Capabilities::available_providers()`, and that was wrong the moment 027 let the service run
    /// in a container: the client is on the host, the sessions are not, and the host's answer is
    /// plausible enough to look right while describing a different machine. FR-023c says the
    /// question is settled where sessions run, so it is now filled from
    /// `DaemonMsg::AiCliAvailability` — asked on connect and on the same **named events** as
    /// before (the Settings view opening, the override menu opening), never per frame. The answer
    /// is stamped with what it describes as it arrives, so FR-023b's sentence can name it; see
    /// [`CliAvailability`].
    ///
    /// It is here rather than reached for because there is no route to it otherwise, and the three
    /// consumers each lack a different one: `features/` imports nothing from `shell::`;
    /// `ui/settings_view.rs` sees a draft and nothing else; and the sidebar's
    /// `row_actions_cluster` takes narrow arguments rather than the whole state.
    ///
    /// Holding it in memory is not a violation of research R11's rule, which is "never
    /// *persisted*" — an in-memory snapshot refreshed when the choice is offered cannot go stale
    /// in a file.
    pub available_providers: Option<CliAvailability>,
    /// The default AI CLI a new session runs when nothing is chosen for it (feature 026, FR-003).
    ///
    /// Service-owned: this mirrors what the daemon reported in `DaemonSettings`, and is written
    /// only from a `SettingsChanged` or the boot-time settings load.
    pub default_ai_cli: AiCli,
    /// The start failure already reported to the user for each session, by the sentence reported
    /// (feature 026, FR-010, T088).
    ///
    /// This is a *said-it* record, not a copy of the failure — the failure itself lives in the
    /// daemon's snapshot, where `WireLifecycle::Failed { reason, .. }` carries it, and the sidebar
    /// tint and bar text are already derived from that. What the client lacks without this is any
    /// memory of having spoken, and the notification is a one-shot: `reconcile_catalog` runs on
    /// every `CatalogChanged`, an activity badge moving is one of those (T086), and a failure
    /// re-announced on each of them would be a banner every few seconds for as long as the session
    /// stays failed.
    ///
    /// Keyed by session and holding the sentence, so a *different* reason for the same session —
    /// a missing CLI after a conversation that had gone missing, say — is news and is said. The
    /// entry is dropped as soon as the daemon reports that session as anything but failed, which
    /// is what makes the next failure speak again.
    ///
    /// Transient, and deliberately not persisted: it records what this window has said, and a
    /// window that has said nothing yet should say it.
    pub announced_start_failures: std::collections::BTreeMap<SessionId, String>,
    /// The open terminal-tab context menu — which instance it belongs to, and where it was opened
    /// in window pixels — or `None` when no menu is showing (feature 012, BUG-005, FR-010b).
    ///
    /// Carries the instance because the menu acts on the tab it was opened on, **not** on the
    /// active one: restarting a background instance without selecting it first is the whole of
    /// FR-010a, and it is what addressing the restart message by instance id was built for.
    /// Window pixels rather than the pane-local point [`Self::terminal_context_menu`] holds — that
    /// one is drawn on the pane's own overlay because a pane's origin is not known at render time,
    /// and this one is drawn on the window's, where the anchor is already in the right space.
    pub shell_instance_menu: Option<(crate::ui::terminal::StripTab, u16, u16)>,
    /// The tab strip's scroll offset, in whole pixels from its leading edge (feature 026 FR-002e).
    ///
    /// Presentation, not state to persist: FR-002d scrolls the marked tab into view on selection,
    /// and where the user has scrolled to is not remembered across sessions or restarts (spec
    /// Assumptions). It lives here only because the edge fade and the reveal both have to read it,
    /// and only the reducer sees both.
    pub tab_strip_scroll_offset: u32,
    /// The tab strip viewport's laid-out width, in whole pixels. `0` until the first layout, which
    /// reads as "cannot decide yet" and never as "nothing fits" — the same rule
    /// [`crate::features::sidebar::State::viewport_height`] follows, and for the same reason.
    pub tab_strip_viewport_width: u32,
    /// The open terminal right-click context menu's anchor in pane-local pixels, or `None` when
    /// no menu is showing (feature 006, FR-013).
    pub terminal_context_menu: Option<(u16, u16)>,
    /// Whether the user has explicitly handed the keyboard from the terminal back to the
    /// application (feature 023, FR-021). Default `false`.
    ///
    /// **This is not "the terminal is unfocused"** — that question is [`crate::app::State::terminal_focused`],
    /// which is derived. This is the one thing about focus the user decides: the reserved chord or
    /// the release affordance sets it, and any navigation that displays a terminal clears it
    /// (FR-021a). It replaced a stored `terminal_focused: bool` that seven scattered assignments
    /// had to keep correct between them, which is how project switch, mode toggle and instance
    /// switch each ended up missing a case. Written only by `focus_terminal` and
    /// `release_terminal`; `tests/terminal_bar_stability.rs` fails if that stops being
    /// true.
    pub terminal_released: bool,
}

/// What the service answered about AI CLIs, and what that answer describes (FR-023b, FR-023c).
///
/// The two travel together on purpose. FR-023c settles availability **where sessions run**, and
/// FR-023b then has to name the thing that is missing the CLI — "not in this image" and "not
/// installed on this computer" are different sentences with different remedies, and picking the
/// wrong one sends the user to fix the wrong machine.
///
/// The service does not report which image it is running; the client started it and holds that
/// fact already, and a second copy of it is a second thing that can disagree. So the answer is
/// **stamped where it is received**, from the boot plan that is in hand at that moment. What
/// arrives here is therefore self-consistent: the set and its subject cannot drift apart later,
/// however the settings form is edited in the meantime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliAvailability {
    /// Present where sessions run, in `AiCli::ALL`'s order. Empty is a real answer.
    pub available: Vec<AiCli>,
    /// What the set is an answer *about*.
    pub source: AvailabilitySource,
}

/// Where the answer in [`CliAvailability`] was settled.
///
/// A closed enum rather than an `Option<String>` (Principle V): the host case is not "an image
/// whose name we happen not to know", it is a different situation with a different remedy, and the
/// type says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvailabilitySource {
    /// The service runs on this computer, so the answer is this computer's own `PATH`.
    ThisComputer,
    /// The service runs in a container built from this image reference.
    Image(String),
}

impl CliAvailability {
    /// Which CLIs the app knows about that the answer does **not** include, in `AiCli::ALL`'s
    /// order. Empty when everything the app can run is present.
    pub fn missing(&self) -> Vec<AiCli> {
        AiCli::ALL
            .into_iter()
            .filter(|which| !self.available.contains(which))
            .collect()
    }
}

/// Why entering a project landed on the session it did — or on none (feature 008 FR-003).
///
/// Entering a project picks its foreground session, and when it picks nothing the user is dropped
/// on the project overview with no explanation. Four different situations produce that, and they
/// need different answers, so they are told apart here rather than collapsed into `Option::None`:
///
/// - a project that genuinely has no sessions is working as intended;
/// - a project whose sessions have all stopped is also correct, and the count says so;
/// - a resolve that finds **nothing under the key it was given**, while the sidebar is happily
///   listing that project's sessions, means the two are looking under different keys. That reads
///   to a user as "the app forgot my session", and it is invisible to any amount of staring at the
///   foreground logic.
///
/// Carried on the state rather than logged from in here: this module is render-free and does no
/// I/O, so the reducer decides and the binary writes it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForegroundChoice {
    /// The session this project was left on. Restored whether or not it is still running: a
    /// stopped session shows its scrollback and its state, which is what selecting it by hand
    /// does (FR-003a).
    Remembered(SessionId),
    /// No usable remembered session, so the project's first active one was taken. `remembered` is
    /// what was hoped for — `None` when the project had never been left on anything.
    FirstActive {
        /// The session actually chosen.
        chosen: SessionId,
        /// What was remembered but could not be used, if anything.
        remembered: Option<SessionId>,
    },
    /// The project has sessions, but none of them is active, so there is nothing to display.
    NoneActive {
        /// How many sessions the project has, all inactive.
        sessions: usize,
    },
    /// Nothing is filed under the key the resolve was given. Distinct from `NoneActive`: this is
    /// the answer that indicts the *key*, not the sessions.
    NoSessionsForKey,
}

impl ForegroundChoice {
    /// The session to display, if any.
    pub fn session(&self) -> Option<SessionId> {
        match self {
            Self::Remembered(id) => Some(*id),
            Self::FirstActive { chosen, .. } => Some(*chosen),
            Self::NoneActive { .. } | Self::NoSessionsForKey => None,
        }
    }
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

impl crate::app::State {
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

    /// Make `next` the current session — the one write to `active_session` the whole application
    /// goes through, bar one (feature 024, contract §3.0).
    ///
    /// The exception is [`Msg::Selected`]: a session the *user* picked in the panel is
    /// already in front of them, so revealing it would scroll a list they were reading (FR-006).
    /// Every other writer comes here, and `tests/current_session_writers.rs` is what keeps that
    /// true as writers are added — the rule is "any app-initiated transition", not a list of call
    /// sites that a future caller can quietly fall off.
    ///
    /// Order is load-bearing, as [`switch_active`](Self::switch_active)'s already is:
    ///
    /// 1. **Commit the outgoing revealed row** into the user's own set, so ceasing to be current
    ///    takes away the mark and never the open row (FR-001c, invariant I3). Skipped when the
    ///    user had closed that row themselves — committing then would re-open what they closed.
    /// 2. **Clear the suppression**, which was scoped to the outgoing session (invariant I2).
    /// 3. **Move the pointer.**
    /// 4. **Arm the scroll — only for a transition to `Some`** (invariant I5). Arming on a clear
    ///    would leave a scroll armed with no row to scroll to, which invariant I4 then keeps armed
    ///    until it fired against whatever row appeared next; and FR-001a forbids scrolling at all
    ///    when the user closes the session they were on.
    ///
    /// The reveal itself is not written anywhere: which row is open is derived on every view from
    /// the field this sets.
    ///
    /// **Nothing here decides motion, deliberately** (FR-010a). The row opens by exactly the path a
    /// user's own expand opens it — which is instant today, since `TreeView` builds its rows from
    /// the item list and expanding simply adds items. If expansion ever gains a transition, the
    /// reveal inherits it rather than needing one of its own; special-casing the reveal would give
    /// the application two different expansion behaviours depending on who triggered it, which is
    /// the opposite of what that requirement asks for.
    #[must_use = "the outgoing row is committed by draining this, not by `set_current_session`"]
    pub fn set_current_session(
        &mut self,
        next: Option<SessionId>,
    ) -> Vec<crate::features::Outcome> {
        if self.session.active == next {
            crate::reveal_trace::line(format_args!(
                "current session unchanged at {:?}: neither expanded nor armed",
                self.session.active
            ));
            return Vec::new();
        }
        crate::reveal_trace::line(format_args!(
            "current session {:?} -> {next:?}, arming={}",
            self.session.active,
            next.is_some()
        ));
        let mut outcomes = Vec::new();
        // Resolved BEFORE the assignment, so this is the *outgoing* session's location — see
        // `closed` below, which orders itself around exactly that.
        if !self.reveal_suppressed() {
            if let Some(location) = self.current_session_location() {
                outcomes.push(crate::features::Outcome::LocationOpened(location));
            }
        }
        self.session.reveal_suppressed_for = None;
        self.session.active = next;
        if next.is_some() {
            outcomes.push(crate::features::Outcome::RevealScrollArmed);
        }
        outcomes
    }

    /// The user closed or reopened the row holding the current session (feature 024, I2).
    ///
    /// Reached from `Outcome::RevealSuppressed`; the sidebar owns the row, this feature owns what
    /// closing it means.
    pub fn reveal_suppression_set(&mut self, suppressed: bool) {
        self.session.reveal_suppressed_for = if suppressed {
            self.session.active
        } else {
            None
        };
    }

    /// Switch the active project **without stopping any session** (feature 008, FR-001/FR-002).
    ///
    /// Order is load-bearing (see data-model.md I1): (1) record the current (outgoing)
    /// foreground BEFORE activation, so the outgoing project is captured — not the incoming
    /// one; (2) activate the target, which leaves everything unchanged and returns `false` if
    /// the project is unknown/unavailable (FR-008); (3) restore the incoming project's
    /// foreground (stored → first running → `None`) (FR-003); (4) surface a notice if any of
    /// its sessions were restarted while it was inactive (FR-011 / SC-007). No session
    /// lifecycle is mutated.
    /// `None` when the switch was refused; otherwise the outcomes arriving in the project raises.
    #[must_use = "a refused switch is `None`, and the outcomes of one that happened must be drained"]
    pub fn switch_active(&mut self, path: &Path) -> Option<Vec<crate::features::Outcome>> {
        self.record_foreground(); // STEP 1 — before activation
        if !self.workspace.activate(path) {
            // STEP 2 — rejected: leave active project, sessions, and foreground untouched.
            return None;
        }
        Some(self.restore_after_activation(path)) // STEPS 3 + 4
    }

    /// Record the current active project's foreground session for later restore (FR-003).
    ///
    /// Public so callers that move `active` themselves (the `FolderChosen` handler, via
    /// `Workspace::open_or_activate`) can capture the outgoing foreground BEFORE activation
    /// and then call [`restore_after_activation`](Self::restore_after_activation) (I1).
    pub fn record_foreground(&mut self) {
        if let (Some(active), Some(id)) = (self.workspace.active.clone(), self.session.active) {
            // Feature 025: on the workspace, so it is persisted with the sessions it refers to.
            // The client keeps it current in memory; the daemon is what writes it to disk.
            self.workspace.foreground_by_project.insert(active, id);
        }
    }

    /// Finish a switch once `path` is already the active project (steps 3 + 4 of
    /// [`switch_active`](Self::switch_active)): restore its foreground session and surface any
    /// background-restart notice. Pair with a preceding [`record_foreground`](Self::record_foreground).
    #[must_use = "arriving in a project resets view state by draining this (T067a-6)"]
    pub fn restore_after_activation(&mut self, path: &Path) -> Vec<crate::features::Outcome> {
        let key = canonicalize_best_effort(path);
        // Recorded before it is acted on, so the binary can say *why* the app landed where it did
        // — including when it landed nowhere, which is the case a user reports as "it forgot my
        // session" and which `Option::None` alone cannot explain.
        let choice = self.explain_foreground(&key);
        self.session.last_foreground_choice = Some(choice.clone());
        // Feature 024: through `set_current_session`, so arriving in a project reveals the session
        // it drops you into (FR-001) — the reported bug was that the panel showed every row
        // collapsed while the main area showed a session.
        let mut outcomes = self.set_current_session(choice.session()); // STEP 3
                                                                       // Feature 023 (FR-011): arriving in a project puts that session's terminal in front of the
                                                                       // user, so it holds the keyboard. This used to clear focus on the reasoning that arriving is
                                                                       // not the same as asking to type — true of arriving somewhere by accident, but a project
                                                                       // switch is deliberate, and the terminal you are looking at is the one you meant. The two
                                                                       // features agree here: 024 reveals the row, 023 gives its terminal the keyboard.
        outcomes.extend(self.focus_terminal());
        // `default_expanded` is not keyed per project (unlike `expanded`, which is pruned by
        // worktree `dir_name` in `set_worktrees`), and feature 014's reveal of agent worktrees
        // (FR-010e) is remembered nowhere — both would otherwise render in a project that never
        // asked for them.
        //
        // Pushed after the commit above, though the order turns out not to be observable here and
        // an earlier version of this comment wrongly claimed it was load-bearing. Probe F4 swapped
        // them and nothing failed, which the code explains: this runs *after*
        // `Workspace::activate`, so `current_session_location` resolves the outgoing session's id
        // against the **incoming** project's `active_sessions()`, does not find it, and answers
        // `None`. The commit cannot fire on this path at all. Kept in this order because it is the
        // one that stays correct if that ever changes.
        outcomes.push(crate::features::Outcome::ProjectEntered);
        outcomes.extend(self.arm_notice(&key)); // STEP 4
        outcomes
    }

    /// Re-run the active project's foreground resolve when the first one ran before the daemon's
    /// catalog had arrived (`010` BUG-013).
    ///
    /// The client boots, restores its project and asks which session to show — all before the
    /// welcome catalog lands. Sessions live on the daemon, so at that instant the project has none
    /// and [`Self::explain_foreground`] answers [`ForegroundChoice::NoSessionsForKey`], which is
    /// correct. The defect was that it was *final*: the catalog then filed the sessions under the
    /// project and nothing asked again.
    ///
    /// What made it look like data loss rather than a missing selection is the sidebar. A location
    /// row opens when it holds the current session (`features::sidebar::effective_open`), so with
    /// nothing current the Default row stayed shut and the sessions inside it were never drawn —
    /// present in state, listed in the catalog, invisible. The session survived the restart in
    /// every layer except the one the user can see.
    ///
    /// Returns whether anything changed.
    ///
    /// # Why the guard is exactly `NoSessionsForKey`
    ///
    /// FR-007 forbids choosing a session for the user when they are landing on the project
    /// overview, and `on_connected` cannot tell a boot from a mid-session reconnect. So the
    /// condition is not "nothing is current" — it is the *reason* nothing is current.
    /// `NoSessionsForKey` is the one answer that indicts the key rather than the sessions: it
    /// means the resolve looked somewhere empty. `NoneActive` is a real overview landing (the
    /// project has sessions, none running) and is left alone, as is any session already chosen.
    /// # Why this answers `Option<Vec<Outcome>>` rather than `bool`
    ///
    /// It landed on `main` returning `bool` — did it move the pointer — while feature 021 was in
    /// flight, and `set_current_session` became an outcome-returning function in the meantime
    /// (T067a-6). `Some` still means exactly what `true` meant; what it now carries is the reveal
    /// the move produces, which the caller must apply. Dropping it would arm no scroll, and the
    /// row this bug is about would resolve correctly and stay off-screen — the same half-fix
    /// BUG-013 was filed for. Mirrors `switch_active`, whose `None` is likewise the refusal.
    #[must_use = "the reveal of the session this resolved is applied by draining this (010 BUG-013)"]
    pub fn resolve_foreground_after_catalog(&mut self) -> Option<Vec<crate::features::Outcome>> {
        if self.session.active.is_some() {
            return None;
        }
        if !matches!(
            self.session.last_foreground_choice,
            Some(ForegroundChoice::NoSessionsForKey)
        ) {
            return None;
        }
        let active = self.workspace.active.clone()?;
        let key = canonicalize_best_effort(&active);
        let choice = self.explain_foreground(&key);
        // Still nothing filed under the key: the catalog did not bring this project's sessions, so
        // leave the recorded reason as it was rather than overwrite it with the same answer.
        if matches!(choice, ForegroundChoice::NoSessionsForKey) {
            return None;
        }
        let session = choice.session();
        self.session.last_foreground_choice = Some(choice);
        // Through `set_current_session` like every other app-initiated move, so the row the
        // session is in is revealed rather than left shut (feature 024, FR-001) — which is the
        // half of this bug the user actually saw.
        Some(self.set_current_session(session))
    }

    /// The session to display when entering `key`, and why: the session this project was left on
    /// if it still exists and was not closed — running or not (FR-003a) — else the project's first
    /// running one, else none (FR-003).
    ///
    /// Replaces the older `restore_foreground`, which answered the same question and threw the
    /// reason away. There is one function rather than two so the choice and the explanation of it
    /// cannot drift apart — see [`ForegroundChoice`].
    pub fn explain_foreground(&self, key: &Path) -> ForegroundChoice {
        let Some(sessions) = self.workspace.sessions.get(key) else {
            return ForegroundChoice::NoSessionsForKey;
        };
        let remembered = self.workspace.foreground_by_project.get(key).copied();
        if let Some(stored) = remembered {
            // Running or stopped alike (FR-003a, BUG-001): you are put back where you were. The
            // rule used to require the session still be active, which was fair while a
            // backgrounded session was expected to keep running — and wrong once lifecycle turned
            // out not to persist, since after a restart every session is idle and the memory was
            // then discarded in the ordinary case. Clicking that same row selects it with no
            // lifecycle check, so restoring it is consistency rather than indulgence.
            //
            // `archived` is the one condition kept: a closed session is hidden from the sidebar
            // entirely (feature 010 BUG-003), so restoring one would display a session the user
            // cannot see listed.
            if sessions.iter().any(|s| s.id == stored && !s.archived) {
                return ForegroundChoice::Remembered(stored);
            }
        }
        match sessions.iter().find(|s| s.is_active()) {
            Some(s) => ForegroundChoice::FirstActive {
                chosen: s.id,
                remembered,
            },
            None => ForegroundChoice::NoneActive {
                sessions: sessions.len(),
            },
        }
    }

    /// If any session of the just-activated project was restarted while inactive, raise the
    /// return notice and consume those markers (FR-011 / SC-007).
    #[must_use = "the notice reaches the queue by draining this (T067a-9)"]
    fn arm_notice(&mut self, key: &Path) -> Vec<crate::features::Outcome> {
        let restarted: Vec<SessionId> = self
            .workspace
            .sessions
            .get(key)
            .map(|list| {
                list.iter()
                    .map(|s| s.id)
                    .filter(|id| self.session.restarted_while_inactive.contains(id))
                    .collect()
            })
            .unwrap_or_default();
        if !restarted.is_empty() {
            for id in &restarted {
                self.session.restarted_while_inactive.remove(id);
            }
            // Reported through the global surface. The previous dedicated `notice` field was
            // drawn only by `shell::view`, which is the *else* branch of
            // `if state.session.active.is_some()` — and returning to a project restores its
            // foreground session, so the banner was unreachable in exactly the case it exists
            // for (FR-011 / SC-007).
            return vec![crate::features::notifications::info(
                "A background session was restarted while you were away.",
            )];
        }
        Vec::new()
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
                self.session.restarted_while_inactive.insert(id);
            }
        }
    }

    /// Mutable access to a session of the active project by id.
    ///
    /// `pub(crate)` rather than private: seven reducer arms in `app.rs` call it, and the reducer
    /// does not move until Tier 3. The widening is the cost of the boundary (T062 revisits it).
    pub(crate) fn session_mut(&mut self, id: SessionId) -> Option<&mut Session> {
        let path = self.workspace.active.clone()?;
        self.workspace
            .sessions
            .get_mut(&path)?
            .iter_mut()
            .find(|s| s.id == id)
    }
}

/// An open session right-click context menu (BUG-003): which session it acts on, and where to draw
/// it. Mirrors [`crate::features::worktree::WorktreeMenu`], which mirrors feature 015's
/// `ProjectMenu` — one shape for one gesture (018 FR-029d).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionMenu {
    /// The session the menu acts on.
    pub id: micold_core::session::SessionId,
    /// The menu panel's top-left corner, in window pixels (the press point). Clamped at render
    /// time, not here.
    pub anchor: (u16, u16),
}

/// A session row's right-click menu, as a floating surface (feature 021, T031).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionContextMenu;

impl FloatingSurface for SessionContextMenu {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("session_menu")
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu)
            .cancelled_by(Message::Session(Msg::MenuDismissed))
    }
}

impl Registered for SessionContextMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.session.menu_open.map(|_| SessionContextMenu)
    }
}

/// The terminal pane's right-click menu, as a floating surface (feature 021, T031).
///
/// The one surface hosted on a pane's own overlay rather than the window's, because its anchor is
/// pane-local and the pane's origin is not known at render time. That is a fact about where it is
/// *drawn*; what closes it is the same rule as every other menu, which is why it registers here
/// like the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalContextMenu;

impl FloatingSurface for TerminalContextMenu {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("terminal_context_menu")
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu)
            .cancelled_by(Message::Session(Msg::TerminalContextMenuClosed))
    }
}

impl Registered for TerminalContextMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .session
            .terminal_context_menu
            .map(|_| TerminalContextMenu)
    }
}

/// A terminal tab's right-click menu, as a floating surface (feature 012, BUG-005, FR-010b).
///
/// Separate from [`TerminalContextMenu`] because they are different menus on different things — one
/// acts on the terminal's *content* (copy, paste) and one on an *instance* (restart, close) — and
/// because they are hosted differently: the pane's menu is drawn on the pane's own overlay, since
/// its anchor is pane-local, and this one on the window's, since a tab's press point is already in
/// window space. Registering here means it closes by the same rule as every other menu, without
/// anyone maintaining a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellInstanceMenu;

impl FloatingSurface for ShellInstanceMenu {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("shell_instance_menu")
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu)
            .cancelled_by(Message::Session(Msg::ShellInstanceMenuClosed))
    }
}

impl Registered for ShellInstanceMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.session.shell_instance_menu.map(|_| ShellInstanceMenu)
    }
}

/// The confirm-remove-session dialog, as a floating surface (feature 021, T032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmSessionRemoveDialog;

impl FloatingSurface for ConfirmSessionRemoveDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("confirm_session_remove")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog)
            .cancelled_by(Message::Session(Msg::RemoveCancelled))
    }
}

impl Registered for ConfirmSessionRemoveDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .session
            .remove_target
            .map(|_| ConfirmSessionRemoveDialog)
    }
}

/// A session was started (feature 005, FR-020).
///
/// The started session's location joins the user's own open set, as it always has. Feature 024
/// makes that redundant for *display* — `set_current_session` opens the row by derivation anyway —
/// but it is kept, and not only because the assertion freeze forbids rewriting the expectation: it
/// is what the reveal would do a moment later, since the commit in `set_current_session` turns a
/// revealed row into ordinary user-open state on the next change of current session. Writing it
/// here reaches the same place by the same rule, sooner.
///
/// Making a session the displayed session puts the user in front of a terminal, so it holds the
/// keyboard (FR-011).
pub fn started(state: &mut crate::app::State, session: Session) -> Vec<crate::features::Outcome> {
    let id = session.id;
    let location = session.location.clone();
    if let Some(path) = state.workspace.active.clone() {
        state
            .workspace
            .sessions
            .entry(path)
            .or_default()
            .push(session);
    }
    let mut outcomes = vec![crate::features::Outcome::LocationOpened(location)];
    outcomes.extend(state.set_current_session(Some(id)));
    outcomes.extend(state.focus_terminal());
    outcomes
}

/// A session row was clicked (feature 024, contract §3.0).
///
/// **The one writer that does not go through `set_current_session`.** The user clicked a row they
/// could already see, so revealing it would open nothing they had not opened and would scroll a
/// list they were reading (FR-006). `tests/current_session_writers.rs` knows this exemption by
/// name; any other direct write to `active_session` fails that gate.
pub fn selected(state: &mut crate::app::State, id: SessionId) -> Vec<crate::features::Outcome> {
    state.session.active = Some(id);
    // Selecting a session displays its terminal, so it holds the keyboard, clearing any earlier
    // release (FR-011, FR-021a).
    state.focus_terminal()
}

/// The daemon reported a session's process running.
pub fn running(state: &mut crate::app::State, id: SessionId) {
    if let Some(session) = state.session_mut(id) {
        session.mark_running();
    }
}

/// A session's title changed.
pub fn title_updated(state: &mut crate::app::State, id: SessionId, title: String) {
    if let Some(session) = state.session_mut(id) {
        session.set_title(title);
    }
}

/// A shell instance was selected (feature 012).
pub fn shell_instance_selected(
    state: &mut crate::app::State,
    id: SessionId,
    shell_id: ShellInstanceId,
) -> Vec<crate::features::Outcome> {
    if let Some(session) = state.session_mut(id) {
        session.select_shell(shell_id);
        // Feature 027 FR-002: and **display** it. Selecting used to set `active_shell` alone, which
        // from the AI pane changed nothing anyone could see — the indicator and the attached
        // process are both derived from the mode, so all three layers agreed on a no-op. That was
        // survivable only while a mode toggle offered a second way across; the tab is the only one
        // now. Setting rather than flipping, for the reason `ai_cli_selected` records.
        session.set_mode(micold_core::session::TerminalMode::Regular);
    }
    arm_tab_reveal(state); // 026 FR-002d — the newly marked tab has to be in view
    state.focus_terminal() // FR-011
}

/// A new shell instance was opened from the "+" (feature 011 FR-001, feature 027 FR-004).
///
/// The instance itself is opened by the binary — it has a daemon to tell and a process to spawn,
/// and `Session::open_shell_instance` lives on the other side of that call. What is pure here is
/// the consequence: the new instance is the one the user is now looking at, so it holds the
/// keyboard (023 FR-011) and is the newly marked tab (026 FR-002d).
///
/// The reveal is the half that was missing until feature 027's visual pass (T024). It cost nothing
/// while the "+" opened instances into a strip with room for them; feature 027 put the "+" beside a
/// right-aligned strip, and the sixth press then created a tab, marked it, and left it behind the
/// trailing edge fade — the user's own new terminal, and the one thing the bar would not show.
pub fn shell_instance_open_requested(
    state: &mut crate::app::State,
) -> Vec<crate::features::Outcome> {
    arm_tab_reveal(state);
    state.focus_terminal()
}

/// A shell instance was closed (feature 012).
///
/// Whichever instance takes its place is what the user is now looking at (FR-011) — so it is also
/// the newly marked tab, and has to be in view (026 FR-002d). Closing shortens the strip, which
/// moves every tab after the closed one; the mark can land outside the viewport without anything
/// having scrolled.
pub fn shell_instance_close_requested(
    state: &mut crate::app::State,
    id: SessionId,
    shell_id: ShellInstanceId,
) -> Vec<crate::features::Outcome> {
    if let Some(session) = state.session_mut(id) {
        session.close_shell(shell_id);
    }
    arm_tab_reveal(state);
    state.focus_terminal()
}

/// The daemon reported a shell instance live (feature 012, FR-008).
pub fn shell_instance_running(
    state: &mut crate::app::State,
    session_id: SessionId,
    shell_id: ShellInstanceId,
) {
    if let Some(session) = state.session_mut(session_id) {
        session.mark_shell_running(shell_id);
    }
}

/// A shell instance's process ended (feature 012).
pub fn shell_instance_exited(
    state: &mut crate::app::State,
    session_id: SessionId,
    shell_id: ShellInstanceId,
) {
    if let Some(session) = state.session_mut(session_id) {
        session.mark_shell_exited(shell_id);
    }
}

/// A session was closed (bugfix BUG-003, FR-015a).
///
/// Close **archives** the session — kept, hidden from the sidebar via `active_sessions()` — rather
/// than deleting its record outright, so a still-existing `claude` transcript is not reconstructed
/// by reconciliation (FR-020b) on the next project open. The durable provider-side suppression
/// marker (FR-020c) is written by the shell, alongside killing the process.
///
/// Feature 024: the pointer is cleared through `set_current_session` so the row the closed session
/// was in is committed open rather than snapping shut and taking its siblings out of view
/// (FR-001c). Nothing is armed — no session is current to scroll to, and FR-001a forbids moving
/// the panel when the user closes the session they were on. Nothing needs clearing alongside it:
/// with no displayed session `terminal_focused()` is already false (FR-012/FR-016).
pub fn close_requested(
    state: &mut crate::app::State,
    id: SessionId,
) -> Vec<crate::features::Outcome> {
    if let Some(path) = state.workspace.active.clone() {
        if let Some(list) = state.workspace.sessions.get_mut(&path) {
            if let Some(session) = list.iter_mut().find(|s| s.id == id) {
                session.archive();
            }
        }
    }
    if state.session.active == Some(id) {
        return state.set_current_session(None);
    }
    Vec::new()
}

/// A session's right-click menu was toggled (bugfix BUG-003).
///
/// Same session closes; a different one replaces it (only ever one open) — mirrors
/// `worktree::menu_toggled` — and re-anchors at its own press point (018 BUG-008).
pub fn menu_toggled(state: &mut crate::app::State, id: SessionId, anchor: (u16, u16)) {
    state.session.menu_open = match &state.session.menu_open {
        Some(open) if open.id == id => None,
        _ => Some(SessionMenu { id, anchor }),
    };
}

/// The session context menu was dismissed.
pub fn menu_dismissed(state: &mut crate::app::State) {
    state.session.menu_open = None;
}

/// Permanent removal was requested; the confirmation opens (bugfix BUG-003, FR-015c).
pub fn remove_requested(state: &mut crate::app::State, id: SessionId) {
    state.clear_for_dialog();
    state.session.remove_target = Some(id);
}

/// Permanent removal was confirmed (FR-015c, FR-020c).
///
/// Unlike close, which archives, remove drops the record outright — the pre-BUG-003 close
/// behaviour. The shell has already killed the process and recorded the durable suppression marker.
///
/// **The ordering is load-bearing.** Feature 024's commit that keeps the row open (FR-001c)
/// resolves the outgoing session's location by looking it up, and a record already removed has no
/// location to find — so clearing the pointer *after* dropping the record collapses the row and
/// takes its siblings out of view. The close path is not exposed to this, archiving leaving the
/// record in place.
pub fn remove_confirmed(state: &mut crate::app::State) -> Vec<crate::features::Outcome> {
    let Some(id) = state.session.remove_target.take() else {
        return Vec::new();
    };
    let outcomes = if state.session.active == Some(id) {
        state.set_current_session(None)
    } else {
        Vec::new()
    };
    if let Some(path) = state.workspace.active.clone() {
        if let Some(list) = state.workspace.sessions.get_mut(&path) {
            list.retain(|s| s.id != id);
        }
    }
    outcomes
}

/// The removal confirmation was dismissed.
pub fn remove_cancelled(state: &mut crate::app::State) {
    state.session.remove_target = None;
}

/// The terminal's right-click menu opened at a pane-local anchor (feature 006, FR-013).
pub fn context_menu_opened(state: &mut crate::app::State, x: u16, y: u16) {
    state.session.terminal_context_menu = Some((x, y));
}

/// The terminal's right-click menu was dismissed.
pub fn context_menu_closed(state: &mut crate::app::State) {
    state.session.terminal_context_menu = None;
}

/// A terminal *tab* was right-clicked (feature 012, BUG-005, FR-010b).
///
/// Replaces rather than stacks: a second right-click, on this tab or another, moves the one menu.
/// Two open at once would each claim the next click.
///
/// The tab travels with the anchor because the menu acts on the tab it was opened on and **not**
/// on the active one — restarting a background instance without selecting it first is the whole of
/// FR-010a.
///
/// It arrived on `main` as an arm of `State::update` while this feature was in flight; it is a
/// routing call here for the same reason every other arm is (FR-002).
pub fn strip_tab_menu_requested(
    state: &mut crate::app::State,
    tab: crate::ui::terminal::StripTab,
    x: u16,
    y: u16,
) {
    state.session.shell_instance_menu = Some((tab, x, y));
}

/// The terminal-tab context menu was dismissed.
pub fn shell_instance_menu_closed(state: &mut crate::app::State) {
    state.session.shell_instance_menu = None;
}

/// Ask for the marked tab to be scrolled into view on the next laid-out frame (feature 026,
/// FR-002d).
///
/// Called from every reducer arm that can change which tab is marked — the same discipline
/// `terminal_released` imposed on focus after seven scattered assignments had to keep each other
/// correct: **one named intent, called from the arms that mean it**. A flag rather than a scroll,
/// because the viewport's width is not known here and nothing is scrolled on a guess.
///
/// It arrived on `main` as an `impl State` method in `app.rs`. It lives here for the reason
/// T067a-7 gave for `focus_terminal`: the strip draws a *session's* tabs, so which one is marked
/// and whether it is in view are the session's business, and a helper in the root is a helper the
/// write-isolation guard reports against every caller instead of the one function that writes it.
pub(crate) fn arm_tab_reveal(state: &mut crate::app::State) {
    state.session.pending_tab_reveal = true;
}

/// The AI tab was pressed (feature 026, FR-006/FR-007).
///
/// Selecting is **all** this does. It sets the mode and nothing else — no process is started,
/// stopped or restarted, and `active_shell` is left alone so switching back returns to the
/// instance the user was on rather than an arbitrary one.
///
/// It **sets** rather than toggles, which is FR-007: pressing the AI tab while the AI CLI is
/// already displayed must be a no-op with no visible change. A flipping message would switch away,
/// which is the opposite of what the press asked for and the reason this is its own message.
///
/// Arrived on `main` as an arm of `State::update`; routed here like every other arm (FR-002), and
/// the keyboard hand-off it performs is an outcome rather than a reach into `focused_field`
/// (T067a-9).
#[must_use = "the field that holds the keyboard gives it up by draining this (T067a-9)"]
pub fn ai_cli_selected(
    state: &mut crate::app::State,
    id: SessionId,
) -> Vec<crate::features::Outcome> {
    if let Some(session) = state.session_mut(id) {
        session.set_mode(micold_core::session::TerminalMode::AiCli);
    }
    arm_tab_reveal(state);
    // FR-011, as every other pane switch does.
    state.focus_terminal()
}

/// The tab strip was scrolled (feature 026, FR-009).
///
/// A scrollable is a place the ground can move, so this dismisses whatever floats above it — the
/// same rule `sidebar::scrolled` applies, from the second scroll region this application has.
/// Without it the tab menu would hang over a tab that had scrolled out from under it.
pub fn tab_strip_scrolled(state: &mut crate::app::State, offset: u32, width: u32) {
    state.session.tab_strip_scroll_offset = offset;
    state.session.tab_strip_viewport_width = width;
    state.dismiss_on_scroll_beneath();
}

/// The tab strip's viewport was resized.
///
/// Separate from [`tab_strip_scrolled`] because the two answer different questions and fire at
/// different times; a resize moves nothing under an open menu, so it dismisses nothing.
pub fn tab_strip_viewport_resized(state: &mut crate::app::State, width: u32) {
    state.session.tab_strip_viewport_width = width;
}

/// Sessions that no longer exist were dropped (feature 021, T065 — `Outcome::SessionsClosed`).
///
/// The session feature's own answer to a consequence another feature reported. The worktree delete
/// path knows *that* its sessions are gone and must not know *how* a session is dropped — that is
/// the whole of contract O2 — so it names them and this applies it.
///
/// The pointer is cleared before the records are dropped, for the reason `remove_confirmed`
/// records at length: feature 024 resolves the outgoing session's location by looking it up, and a
/// record already removed has no location to find.
pub fn closed(state: &mut crate::app::State, ids: &[SessionId]) -> Vec<crate::features::Outcome> {
    let outcomes = if state.session.active.is_some_and(|id| ids.contains(&id)) {
        state.set_current_session(None)
    } else {
        Vec::new()
    };
    for list in state.workspace.sessions.values_mut() {
        list.retain(|s| !ids.contains(&s.id));
    }
    outcomes
}

impl crate::app::State {
    /// The user is being put in front of a terminal (FR-011, FR-021a, FR-008b).
    ///
    /// Clears the explicit release *and* any text-field focus. The second one matters: a press on
    /// the pane, or a navigation that displays a terminal, is a request for that terminal, and it
    /// must not be defeated by a field that still believes it holds the keyboard. Without it, a
    /// press into the pane made while a rename field had focus would depend on iced's blur
    /// arriving first. FR-018 permits taking the keyboard from a field for exactly this reason —
    /// it is a user press.
    ///
    /// **Moved here from `app.rs` by T067a-7.** T059 recorded the open question — is this a session
    /// operation sitting in the wrong file? — and left it for the burn-down to settle. It is: the
    /// terminal is the session's pane, `terminal_released` is session-owned, and every caller is a
    /// session operation. Leaving it in the root made five session reducers each look like they
    /// wrote window state, when one function does.
    #[must_use = "the field that holds the keyboard gives it up by draining this (T067a-9)"]
    pub(crate) fn focus_terminal(&mut self) -> Vec<crate::features::Outcome> {
        self.session.terminal_released = false;
        vec![crate::features::Outcome::FieldFocusCleared]
    }

    /// The user handed the keyboard back to the application (FR-021) — the reserved chord or the
    /// release affordance. It holds until they give it back or navigate to a terminal.
    pub(crate) fn release_terminal(&mut self) {
        self.session.terminal_released = true;
    }
}

/// Everything a session, its terminal panes and its tab strip say about themselves
/// (feature 028, FR-001).
///
/// The largest feature in the application: thirty-seven variants, a quarter of the root
/// vocabulary before this feature started. The `Session` prefix is gone from the eleven that
/// carried it (contract M1) — the wrapper says it once. `Terminal`, `ShellInstance` and
/// `TabStrip` are not: each names a sub-surface *inside* a session, and `Msg::MenuDismissed`,
/// `Msg::ShellInstanceMenuClosed` and `Msg::TerminalContextMenuClosed` are three different menus
/// that would read as one without them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The terminal tab strip scrolled; carries the offset, the viewport's width and its content's,
    /// all in whole pixels (feature 026 FR-002e).
    ///
    /// Two numbers in one message because they answer one question — "does anything lie beyond this
    /// edge" — and the rendering stack delivers them together. Split across messages there would be
    /// frames where one is stale, and a fade computed from a stale pair points at nothing or fails
    /// to point at something.
    ///
    /// It was three until feature 027. The content width came with them and is no longer carried:
    /// the strip's content is now a function of its viewport (the trailing alignment's slack is
    /// laid out inside it), so a *measured* content width paired with the current viewport is
    /// exactly the stale pair this doc warns about — and it went stale for real, because this
    /// message is the only thing that ever wrote it and it fires only when something scrolls. The
    /// fade derives the width from the tab count instead (`ui::terminal::strip_overflow`).
    TabStripScrolled { offset: u32, width: u32 },
    /// The tab strip's viewport was laid out, or resized (feature 026 FR-002e).
    ///
    /// Separate from [`Self::TabStripScrolled`] because the two answer different questions and fire
    /// at different times — the same split the sidebar's pair makes. This one is what covers the
    /// **first** frame, where nothing has scrolled yet and a strip that already overflows still has
    /// to fade its edge.
    TabStripViewportResized { width: u32 },
    /// Start a new session at the given location — a worktree or, as of feature 010, the
    /// project root ("Default", FR-001) — (FR-010). The binary spawns the named AI CLI.
    StartRequested {
        /// Where the session runs.
        location: SessionLocation,
        /// Which AI CLI to run (feature 026, FR-004). Already resolved — [`State::provider_for_start`]
        /// applied the override or the default before this message was built, so the binary's
        /// handler copies the answer onto the wire and decides nothing.
        provider: AiCli,
    },
    /// Open the "start a session on…" list for a location (feature 026, FR-004). The binary
    /// refreshes the availability set first — this is one of the two named events research R11
    /// means by "when the choice is offered".
    StartMenuOpened {
        /// Where a session started from this list would run.
        location: SessionLocation,
        /// The stored default, when the list is opening because that default is not installed
        /// rather than because the user asked for it (feature 026 BUG-001, FR-002).
        ///
        /// It rides on *this* message rather than one of its own for the reason
        /// `tests/session_start_press.rs` records: the binary re-probes `PATH` on this message, so
        /// a separate one would open the list on a staler set. It says why the press happened,
        /// which is knowledge only the press has; whether it is still true is settled by the
        /// reducer, after that refresh.
        unavailable_default: Option<AiCli>,
    },
    /// Where a press on the start affordance landed, in window pixels (018 BUG-008, FR-029d).
    /// Arrives from the same click as [`Msg::StartMenuOpened`] and **before** it — this one is
    /// published on `ButtonPressed`, that one on the release, because it comes from the wrapped
    /// button's own `on_press`. So this says where the click was, and the open that follows it is
    /// what hangs the list there (feature 026, T089).
    StartMenuAnchored((u16, u16)),
    /// Dismiss the "start a session on…" list without choosing.
    StartMenuDismissed,
    /// A session was started/added for the active project (FR-011).
    Started(Session),
    /// Select a session to display its terminal (FR-015); other sessions keep running.
    Selected(SessionId),
    /// Close/stop a session (FR-015a, bugfix BUG-003). The binary kills the process and records
    /// the durable suppression marker; this archives (not deletes) the record.
    CloseRequested(SessionId),
    /// The session's `claude` process reported it is running (FR-010).
    ///
    /// **Emitted nowhere in production** — the daemon owns this transition and publishes it in the
    /// catalog snapshot (FR-006d, `010` BUG-011), which `reconcile_catalog` adopts unconditionally.
    /// Kept as the reducer's own `→ Running` edge, which the state tests drive directly; deleting it
    /// is a separate change from the one that made the daemon report the transition at all.
    Running(SessionId),
    /// The session's `claude` title became available/changed (FR-011a).
    TitleUpdated { id: SessionId, title: String },

    // ---- Bugfix BUG-003: session Remove (distinct from Close/archive) ----
    /// Open (or close, if already open) a session's right-click context menu, anchored at the
    /// press point in window pixels (018 FR-029d).
    MenuToggled(SessionId, (u16, u16)),
    /// Dismiss the session context menu (outside click, or after an action is chosen).
    MenuDismissed,
    /// Request permanent removal of a session; opens the confirm dialog (FR-015c).
    RemoveRequested(SessionId),
    /// Confirm removal. The binary kills the process (if running) and records the durable
    /// suppression marker, then persists; the reducer drops the record outright.
    RemoveConfirmed,
    /// Dismiss the remove confirmation without removing anything.
    RemoveCancelled,

    // ---- Feature 010: switchable regular terminal mode ----
    // The mode toggle's `TerminalModeToggled` lived here until feature 027 deleted the control.
    // The tab strip is the only route between a session's panes now, so a message meaning "switch
    // to whichever pane is not showing" has no sender and, more to the point, no meaning: a strip
    // names its destination. `TerminalAiCliSelected` and `ShellInstanceSelected` are what remain,
    // and both **set** rather than flip — see FR-007 below for why that distinction is load-bearing.
    /// The manual restart affordance was pressed for the active session's currently-attached,
    /// not-running process — for the AI CLI branch, and for whichever Regular Terminal instance
    /// is currently active (FR-013; contracts/terminal-mode-lifecycle.md).
    TerminalRestartRequested,

    // ---- Feature 011: multiple Regular Terminal instances per session ----
    /// Open an additional Regular Terminal instance for the active session (the "+" control or
    /// the `Ctrl+Shift+T`/`Cmd+Shift+T` shortcut, FR-001, FR-019) — a no-op outside Regular mode.
    /// No pure reducer body: mirrors `TerminalRestartRequested`/`StartRequested`, which
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
    /// Open the context menu for one strip tab, at a window-pixel point (feature 012 BUG-005
    /// FR-010b, widened by feature 026 FR-006a). Dispatched by a secondary (right) press.
    ///
    /// Carries a `StripTab` rather than a `ShellInstanceId` since feature 026, because the AI tab
    /// has a menu too and it is **the same menu** with Close filtered out (FR-004, FR-006a). One
    /// message, one surface, one registration: two would be the shape that lets the two menus drift
    /// into offering different actions for the same reason, which is the thing FR-006a is worded to
    /// prevent.
    StripTabMenuRequested(crate::ui::terminal::StripTab, u16, u16),
    /// Dismiss the terminal-tab context menu.
    ShellInstanceMenuClosed,
    /// Show the session's AI CLI process in the pane (feature 026 FR-006, FR-007).
    ///
    /// **Sets** the mode rather than toggling it, which is FR-007: pressing the AI tab while the AI
    /// CLI is already displayed must be a no-op with no visible change, and a flipping message
    /// would switch away. Carries the session explicitly, for the same reason
    /// `ShellInstanceSelected` does.
    TerminalAiCliSelected(SessionId),
    /// A Regular Terminal instance reported it is running (feature 011; replaces feature 010's
    /// `ShellSessionRunning(SessionId)`, now id-addressed since a session may have more than one
    /// instance).
    ///
    /// **Emitted nowhere in production**, like [`Self::Running`]. The daemon owns this
    /// transition and publishes it as `SessionSummary::live_shells`, which `reconcile_catalog`
    /// adopts (`012` FR-008, BUG-003). Kept as the reducer's own `→ Running` edge, which feature
    /// 023's FR-019 rule (a session reaching `Running` must not move the keyboard) is asserted
    /// through.
    ///
    /// The older reason for keeping it — that it was the *only* lever `tests/` had, because
    /// `reconcile_catalog` sat in the binary crate — no longer holds: the fold now lives in
    /// [`crate::catalog_sync`] and `crates/micold-daemon/tests/catalog_join.rs` drives the real
    /// daemon → wire → client path. That is the coverage this variant was standing in for, and
    /// standing in badly: it let `012` BUG-003 ship an incomplete fix.
    ShellInstanceRunning(SessionId, ShellInstanceId),
    /// A Regular Terminal instance's shell process exited (intentional or crash) — never
    /// auto-restarted (FR-008; replaces feature 010's `ShellSessionExited(SessionId)`).
    ///
    /// Emitted nowhere in production, for the same reason and with the same caveat as
    /// [`Self::ShellInstanceRunning`] above.
    ShellInstanceExited(SessionId, ShellInstanceId),

    /// Periodic redraw tick while a terminal is live (drives streamed-output repaint).
    TerminalTick,

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
}

/// The pure half of this feature's reducer surface: shape A (contract M2).
///
/// All forty arms are here. Fifteen of them additionally need an effect — spawning a
/// process, writing to a PTY, scrolling or resizing it, and the clipboard — and those fifteen are
/// matched a second time in `main.rs`, which runs the effect and lets the message reach here.
/// The split is by *effect*, not by variant, as `worktree_form` established and M2 names as the
/// reference.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::Started(session) => return started(state, session),
        Msg::Selected(id) => return selected(state, id),
        Msg::TerminalAiCliSelected(id) => return ai_cli_selected(state, id),
        Msg::ShellInstanceOpenRequested => {
            // No session state to update here — the binary decides whether the active session
            // is in Regular mode, opens the instance (`Session::open_shell_instance`), and
            // spawns its process. The daemon then reports it in `SessionSummary::live_shells`
            // and `reconcile_catalog` marks it running (`012` FR-008, BUG-003); this used to
            // claim a follow-up `ShellInstanceRunning` message, which is emitted nowhere and
            // is why every instance sat at `NotStarted` for its whole life.
            // The new instance is what the user will be looking at, so it holds the keyboard
            // (FR-011) and is scrolled into view (026 FR-002d) — see
            // `shell_instance_open_requested` for why the second half was missing until feature
            // 027 put the "+" beside the strip it fills.
            return shell_instance_open_requested(state);
        }
        Msg::ShellInstanceSelected(id, shell_id) => {
            return shell_instance_selected(state, id, shell_id)
        }
        Msg::ShellInstanceCloseRequested(id, shell_id) => {
            return shell_instance_close_requested(state, id, shell_id)
        }
        Msg::CloseRequested(id) => return close_requested(state, id),
        Msg::RemoveConfirmed => return remove_confirmed(state),
        Msg::TerminalFocused => return state.focus_terminal(),
        Msg::TabStripScrolled { offset, width } => tab_strip_scrolled(state, offset, width),
        Msg::TabStripViewportResized { width } => tab_strip_viewport_resized(state, width),
        Msg::Running(id) => running(state, id),
        Msg::TitleUpdated { id, title } => title_updated(state, id, title),
        Msg::ShellInstanceRunning(session_id, shell_id) => {
            shell_instance_running(state, session_id, shell_id)
        }
        Msg::ShellInstanceExited(session_id, shell_id) => {
            shell_instance_exited(state, session_id, shell_id)
        }
        Msg::MenuToggled(id, anchor) => menu_toggled(state, id, anchor),
        Msg::MenuDismissed => menu_dismissed(state),
        Msg::RemoveRequested(id) => remove_requested(state, id),
        Msg::RemoveCancelled => remove_cancelled(state),
        Msg::TerminalFocusReleased => state.release_terminal(),
        Msg::TerminalContextMenuOpened { x, y } => context_menu_opened(state, x, y),
        Msg::TerminalContextMenuClosed => context_menu_closed(state),
        Msg::StripTabMenuRequested(tab, x, y) => strip_tab_menu_requested(state, tab, x, y),
        Msg::ShellInstanceMenuClosed => shell_instance_menu_closed(state),
        Msg::TerminalTick => {}
        Msg::TerminalRestartRequested => {
            // No pure state to update here — the binary decides which process to spawn based on
            // the current mode. For an AI-CLI session the daemon owns the lifecycle and
            // announces `Running` in the catalog snapshot once the process exists (FR-006d,
            // `010` BUG-011); `reconcile_catalog` adopts it. This comment used to claim a
            // follow-up `Running` message, which is emitted nowhere — believing it cost
            // BUG-011 a round of investigation, because it made a state bug look like a
            // transport one.
        }
        Msg::ShellInstanceRestartRequested(..) => {
            // No pure state to update here — the binary spawns the process, and the daemon's
            // next snapshot reports the instance live (`012` FR-008, BUG-003). Mirrors
            // `TerminalRestartRequested`, including that neither emits a follow-up message.
        }
        // Performed by the binary at the I/O boundary: PTY spawning, and — for feature 006's
        // terminal gestures — writing to the live PTY, scrolling or resizing it, and the
        // clipboard. No pure reducer effect.
        Msg::StartMenuOpened {
            location,
            unavailable_default,
        } => return start_menu_toggled(state, location, unavailable_default),
        Msg::StartMenuAnchored(anchor) => start_menu_anchored(state, anchor),
        Msg::StartMenuDismissed => start_menu_dismissed(state),
        Msg::StartRequested { .. }
        | Msg::TerminalBytes(_)
        | Msg::TerminalSelectStart { .. }
        | Msg::TerminalSelectUpdate { .. }
        | Msg::TerminalSelectCleared
        | Msg::TerminalScrolled(_)
        | Msg::TerminalScrolledTo(_)
        | Msg::TerminalResized { .. }
        | Msg::TerminalCopyRequested
        | Msg::TerminalPasteRequested => {}
    }
    Vec::new()
}

// ---------------------------------------------------------------------------------------
// Which AI CLI a new session runs (feature 026, T030/T032a/T075 — FR-002, FR-004, FR-006)
// ---------------------------------------------------------------------------------------

/// Which half of the split start affordance the user pressed (feature 026, FR-004).
///
/// The affordance is one control with two press targets: the primary half starts the default in a
/// single interaction exactly as the plain button did, and the secondary half opens the list. The
/// *view* reports which half was hit; everything that follows from that is decided here, because
/// Principle I's GUI exception covers drawing and does not cover branching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressTarget {
    /// The main button — "start a session".
    Primary,
    /// The adjacent control — "start a session on…".
    Secondary,
}

/// What a press should actually do (feature 026, T032a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartIntent {
    /// Start immediately, on this CLI.
    Start(AiCli),
    /// Do not start anything; offer these CLIs to choose from.
    ///
    /// Two different presses land here and they open the same list: the secondary half, which asks
    /// for it, and the primary half when the stored default is not installed, where FR-004 says to
    /// *tell* the user and offer what is available rather than silently substituting or silently
    /// doing nothing.
    ///
    /// They are not the same answer, and BUG-001 is what it cost to treat them as one. The list a
    /// user asked for explains itself; the list they got *instead of* a session does not, and by the
    /// time the offer reached a surface there was nothing left to say why it had opened. So the
    /// reason travels with the offer.
    OfferChoice {
        /// What is installed, in the order the list draws them.
        providers: Vec<AiCli>,
        /// The stored default, when this list is opening because that default cannot be run
        /// (FR-002). `None` for a press that asked for the list.
        ///
        /// Why the CLI and not a bare flag: the sentence names it, and the name has to come from
        /// the same read that decided it was missing. A surface re-deriving "which default?" later
        /// would be answering a question about a different moment.
        unavailable_default: Option<AiCli>,
    },
    /// No AI CLI is installed, so there is nothing to start and nothing to offer (FR-006).
    NothingAvailable,
}

impl State {
    /// The CLI a new session should run: the per-session override if one was chosen, otherwise the
    /// stored default (FR-004).
    ///
    /// Choosing an override does **not** touch the default — that is FR-004's other half, and it is
    /// true here by shape: this function reads `default_ai_cli` and writes nothing.
    pub fn provider_for_start(&self, chosen: Option<AiCli>) -> AiCli {
        chosen.unwrap_or(self.default_ai_cli)
    }

    /// The availability set as far as anything is known, treating "not asked yet" as "nothing".
    ///
    /// Every consumer below wants a slice, and every one of them is a decision about what to
    /// *offer* — where an unanswered service and a service that answered "none" mean the same
    /// thing: there is no CLI to offer. The distinction the `Option` preserves matters exactly
    /// once, in the Settings sentence that has to say "the image provides none" rather than
    /// "nothing is known yet" (FR-023b), and that call site reads the field directly.
    fn known_available(&self) -> &[AiCli] {
        self.available_providers
            .as_ref()
            .map(|a| a.available.as_slice())
            .unwrap_or(&[])
    }

    /// Whether the stored default is currently installed (FR-002).
    ///
    /// A `false` here is **not** a reason to rewrite the preference. The stored value stays as the
    /// user left it and is shown marked, so a temporary `PATH` problem cannot silently discard a
    /// choice (research R11).
    pub fn default_ai_cli_is_available(&self) -> bool {
        self.known_available().contains(&self.default_ai_cli)
    }

    /// The CLIs to offer in a menu — the Settings select and the override list (FR-006, T075).
    ///
    /// A pure function over `State`, deliberately: `features/` cannot see `Capabilities` and must
    /// not learn to, so the availability set arrives as state (T014a) and this reads it.
    pub fn offered_providers(&self) -> Vec<AiCli> {
        self.known_available().to_vec()
    }

    /// Whether the split affordance's secondary half exists at all (FR-006, SC-001).
    ///
    /// Absent when fewer than two CLIs are available: a "choose which one" control that opens a
    /// list of one is a worse single-CLI experience than the plain button it replaced.
    pub fn start_affordance_offers_a_choice(&self) -> bool {
        self.known_available().len() >= 2
    }

    /// Resolve a press into what should happen (T032a).
    ///
    /// The whole branch lives here so `ui/sidebar.rs` only dispatches. The interesting case is the
    /// primary half with an unavailable default: it neither starts the default (FR-002 forbids
    /// substituting silently, and starting a missing binary is FR-010's failure, not FR-004's
    /// answer) nor does nothing.
    pub fn start_intent(&self, target: PressTarget) -> StartIntent {
        if self.known_available().is_empty() {
            return StartIntent::NothingAvailable;
        }
        match target {
            PressTarget::Primary if self.default_ai_cli_is_available() => {
                StartIntent::Start(self.default_ai_cli)
            }
            // The default names a CLI that is not installed: say so, and offer what is.
            PressTarget::Primary => StartIntent::OfferChoice {
                providers: self.offered_providers(),
                unavailable_default: Some(self.default_ai_cli),
            },
            PressTarget::Secondary => StartIntent::OfferChoice {
                providers: self.offered_providers(),
                unavailable_default: None,
            },
        }
    }
}

/// The "start a session on…" list, and where it hangs from (feature 026, FR-004).
///
/// Mirrors [`SessionMenu`]: what the surface acts on, plus the press point it was opened at. The
/// point arrives a message *earlier* than the location and is held on [`State`] until the open
/// reads it — see [`start_menu_anchored`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartMenu {
    /// Where a session started from this list would run.
    pub location: SessionLocation,
    /// The menu panel's top-left corner, in window pixels (the press point). Clamped at render
    /// time, not here.
    pub anchor: (u16, u16),
}

/// The "start a session on…" list was asked for at a location (feature 026, FR-004).
///
/// A second press on the same row's chevron closes it, like every other menu here — the same
/// replace-or-close rule `menu_toggled` applies to the session context menu.
///
/// It opens at the point [`start_menu_anchored`] recorded, which the *same click* published first:
/// the anchor comes from `ContextArea` on `ButtonPressed`, this one from the button it wraps on the
/// release. Writing a constant here and letting the anchor correct it afterwards is what feature
/// 026 shipped — the correction never ran, because there was nothing open when the point arrived
/// and by the time there was, the point had been dropped, so both halves of the split opened the
/// list over the sidebar header (T089).
///
/// `unavailable_default` is why the list is opening, from
/// [`StartIntent::OfferChoice`](StartIntent::OfferChoice): `Some(cli)` when the press was the
/// primary half and the stored default cannot be run, `None` when the user asked for the list.
/// Only the first says anything, and only when it opens — see the notice below (BUG-001).
#[must_use = "the notice reaches the queue by draining this (BUG-001)"]
pub fn start_menu_toggled(
    state: &mut crate::app::State,
    location: SessionLocation,
    unavailable_default: Option<AiCli>,
) -> Vec<crate::features::Outcome> {
    let closing = state
        .session
        .start_menu
        .as_ref()
        .is_some_and(|open| open.location == location);
    state.session.start_menu = if closing {
        None
    } else {
        Some(StartMenu {
            location,
            anchor: state.session.start_press.unwrap_or_default(),
        })
    };
    if closing {
        // The same press closes an open list, and a user dismissing one does not need to be told
        // why it appeared.
        return Vec::new();
    }
    unavailable_default
        .filter(|cli| !state.session.known_available().contains(cli))
        .map(|cli| {
            vec![crate::features::notifications::error(format!(
                "{} isn't installed. Install it, or start this session on another AI CLI.",
                cli.provider().display_name()
            ))]
        })
        .unwrap_or_default()
}

/// Where a press on the start affordance landed (018 BUG-008, FR-029d).
///
/// It records the point and nothing else. The list is not open yet when this runs — the press is
/// this, the open is the release — so there is nothing here to move; [`start_menu_toggled`] is
/// what reads the point back. Every primary press reports one, including the ones that start a
/// session outright and the one that *closes* an open list, which is why this decides nothing: what
/// a press meant is the toggle's answer, and a recorded point that is never read costs nothing.
///
/// It deliberately does not also move an already-open list. Pressing another row's chevron while
/// one is open would then slide the open panel to the new row for the frame between the press and
/// the release, before the toggle replaced it — a visible twitch for no gain.
pub fn start_menu_anchored(state: &mut crate::app::State, anchor: (u16, u16)) {
    state.session.start_press = Some(anchor);
}

/// The "start a session on…" list was dismissed without choosing.
pub fn start_menu_dismissed(state: &mut crate::app::State) {
    state.session.start_menu = None;
}

/// The "start a session on…" list, as a floating surface (feature 026, T033).
///
/// A menu, not a dialog: it is summoned from a row's chevron, it is anchored to the sidebar like
/// the worktree and session context menus, and clicking away is how it goes. It registers here for
/// the same reason they do — what closes a surface is one rule, stated once (FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionStartMenu;

impl FloatingSurface for SessionStartMenu {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("session_start_menu")
    }

    fn layer(&self) -> Layer {
        Layer::ContextMenu
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::ContextMenu)
            .cancelled_by(Message::Session(Msg::StartMenuDismissed))
    }
}

impl Registered for SessionStartMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.session.start_menu.as_ref().map(|_| SessionStartMenu)
    }
}
