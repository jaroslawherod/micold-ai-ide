//! Sessions: which one is in the foreground, and what survives a project switch
//! (feature 021, T021).
//!
//! The switch sequence in [`State::switch_active`] is the delicate part of this feature and the
//! reason its helpers belong together: the order of record-then-activate-then-restore is
//! load-bearing (data-model.md I1), and the private steps it calls are meaningless apart from it.
//!
//! `SelectKind` is here rather than in `features/project.rs`, where T017 filed it. It is terminal
//! text selection, and terminals belong to sessions; T017 swept it in because it happened to sit in
//! that stretch of `app.rs`. Grouping by line range is what FR-001 argues against.
//!
//! These are `impl State` blocks because `State` is still monolithic in Tier 1. Methods resolve on
//! the type rather than the module, so moving them changed no call site.

use crate::app::Message;
use crate::app::State;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;
use micold_core::project::canonicalize_best_effort;
use micold_core::session::{Session, SessionId, ShellInstanceId};
use std::path::Path;

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

impl State {
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
    /// The exception is `Message::SessionSelected`: a session the *user* picked in the panel is
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
        if self.active_session == next {
            return Vec::new();
        }
        let mut outcomes = Vec::new();
        // Resolved BEFORE the assignment, so this is the *outgoing* session's location — see
        // `closed` below, which orders itself around exactly that.
        if !self.reveal_suppressed() {
            if let Some(location) = self.current_session_location() {
                outcomes.push(crate::features::Outcome::LocationOpened(location));
            }
        }
        self.reveal_suppressed_for = None;
        self.active_session = next;
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
        self.reveal_suppressed_for = if suppressed {
            self.active_session
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
        if let (Some(active), Some(id)) = (self.workspace.active.clone(), self.active_session) {
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
        self.last_foreground_choice = Some(choice.clone());
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
        if self.active_session.is_some() {
            return None;
        }
        if !matches!(
            self.last_foreground_choice,
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
        self.last_foreground_choice = Some(choice);
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
                self.restarted_while_inactive.insert(id);
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
        DismissalRules::for_layer(Layer::ContextMenu).cancelled_by(Message::SessionMenuDismissed)
    }
}

impl Registered for SessionContextMenu {
    fn open_in(state: &State) -> Option<Self> {
        state.session_menu_open.map(|_| SessionContextMenu)
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
            .cancelled_by(Message::TerminalContextMenuClosed)
    }
}

impl Registered for TerminalContextMenu {
    fn open_in(state: &State) -> Option<Self> {
        state.terminal_context_menu.map(|_| TerminalContextMenu)
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
        DismissalRules::for_layer(Layer::ContextMenu).cancelled_by(Message::ShellInstanceMenuClosed)
    }
}

impl Registered for ShellInstanceMenu {
    fn open_in(state: &State) -> Option<Self> {
        state.shell_instance_menu.map(|_| ShellInstanceMenu)
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
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::SessionRemoveCancelled)
    }
}

impl Registered for ConfirmSessionRemoveDialog {
    fn open_in(state: &State) -> Option<Self> {
        state
            .session_remove_target
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
pub fn started(state: &mut State, session: Session) -> Vec<crate::features::Outcome> {
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
pub fn selected(state: &mut State, id: SessionId) -> Vec<crate::features::Outcome> {
    state.active_session = Some(id);
    // Selecting a session displays its terminal, so it holds the keyboard, clearing any earlier
    // release (FR-011, FR-021a).
    state.focus_terminal()
}

/// The daemon reported a session's process running.
pub fn running(state: &mut State, id: SessionId) {
    if let Some(session) = state.session_mut(id) {
        session.mark_running();
    }
}

/// A session's title changed.
pub fn title_updated(state: &mut State, id: SessionId, title: String) {
    if let Some(session) = state.session_mut(id) {
        session.set_title(title);
    }
}

/// The displayed session switched between its AI-CLI and Regular modes.
///
/// Switching mode puts a different terminal in front of the user, so it holds the keyboard
/// (FR-011). That is the navigation the reported bug was about: it used to take two presses to
/// reach and then left you looking at a terminal that ignored the keyboard.
pub fn mode_toggled(state: &mut State) -> Vec<crate::features::Outcome> {
    if let Some(id) = state.active_session {
        if let Some(session) = state.session_mut(id) {
            let next = session.mode.other();
            session.set_mode(next);
        }
    }
    state.focus_terminal()
}

/// A shell instance was selected (feature 012).
pub fn shell_instance_selected(
    state: &mut State,
    id: SessionId,
    shell_id: ShellInstanceId,
) -> Vec<crate::features::Outcome> {
    if let Some(session) = state.session_mut(id) {
        session.select_shell(shell_id);
    }
    state.focus_terminal() // FR-011
}

/// A shell instance was closed (feature 012).
///
/// Whichever instance takes its place is what the user is now looking at (FR-011).
pub fn shell_instance_close_requested(
    state: &mut State,
    id: SessionId,
    shell_id: ShellInstanceId,
) -> Vec<crate::features::Outcome> {
    if let Some(session) = state.session_mut(id) {
        session.close_shell(shell_id);
    }
    state.focus_terminal()
}

/// The daemon reported a shell instance live (feature 012, FR-008).
pub fn shell_instance_running(state: &mut State, session_id: SessionId, shell_id: ShellInstanceId) {
    if let Some(session) = state.session_mut(session_id) {
        session.mark_shell_running(shell_id);
    }
}

/// A shell instance's process ended (feature 012).
pub fn shell_instance_exited(state: &mut State, session_id: SessionId, shell_id: ShellInstanceId) {
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
pub fn close_requested(state: &mut State, id: SessionId) -> Vec<crate::features::Outcome> {
    if let Some(path) = state.workspace.active.clone() {
        if let Some(list) = state.workspace.sessions.get_mut(&path) {
            if let Some(session) = list.iter_mut().find(|s| s.id == id) {
                session.archive();
            }
        }
    }
    if state.active_session == Some(id) {
        return state.set_current_session(None);
    }
    Vec::new()
}

/// A session's right-click menu was toggled (bugfix BUG-003).
///
/// Same session closes; a different one replaces it (only ever one open) — mirrors
/// `worktree::menu_toggled`.
pub fn menu_toggled(state: &mut State, id: SessionId) {
    state.session_menu_open = if state.session_menu_open == Some(id) {
        None
    } else {
        Some(id)
    };
}

/// The session context menu was dismissed.
pub fn menu_dismissed(state: &mut State) {
    state.session_menu_open = None;
}

/// Permanent removal was requested; the confirmation opens (bugfix BUG-003, FR-015c).
pub fn remove_requested(state: &mut State, id: SessionId) {
    state.clear_for_dialog();
    state.session_remove_target = Some(id);
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
pub fn remove_confirmed(state: &mut State) -> Vec<crate::features::Outcome> {
    let Some(id) = state.session_remove_target.take() else {
        return Vec::new();
    };
    let outcomes = if state.active_session == Some(id) {
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
pub fn remove_cancelled(state: &mut State) {
    state.session_remove_target = None;
}

/// The terminal's right-click menu opened at a pane-local anchor (feature 006, FR-013).
pub fn context_menu_opened(state: &mut State, x: u16, y: u16) {
    state.terminal_context_menu = Some((x, y));
}

/// The terminal's right-click menu was dismissed.
pub fn context_menu_closed(state: &mut State) {
    state.terminal_context_menu = None;
}

/// A terminal *tab* was right-clicked (feature 012, BUG-005, FR-010b).
///
/// Replaces rather than stacks: a second right-click, on this tab or another, moves the one menu.
/// Two open at once would each claim the next click.
///
/// The instance travels with the anchor because the menu acts on the tab it was opened on and
/// **not** on the active one — restarting a background instance without selecting it first is the
/// whole of FR-010a.
///
/// It arrived on `main` as an arm of `State::update` while this feature was in flight; it is a
/// routing call here for the same reason every other arm is (FR-002).
pub fn shell_instance_menu_requested(state: &mut State, instance: ShellInstanceId, x: u16, y: u16) {
    state.shell_instance_menu = Some((instance, x, y));
}

/// The terminal-tab context menu was dismissed.
pub fn shell_instance_menu_closed(state: &mut State) {
    state.shell_instance_menu = None;
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
pub fn closed(state: &mut State, ids: &[SessionId]) -> Vec<crate::features::Outcome> {
    let outcomes = if state.active_session.is_some_and(|id| ids.contains(&id)) {
        state.set_current_session(None)
    } else {
        Vec::new()
    };
    for list in state.workspace.sessions.values_mut() {
        list.retain(|s| !ids.contains(&s.id));
    }
    outcomes
}

impl State {
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
        self.terminal_released = false;
        vec![crate::features::Outcome::FieldFocusCleared]
    }

    /// The user handed the keyboard back to the application (FR-021) — the reserved chord or the
    /// release affordance. It holds until they give it back or navigate to a terminal.
    pub(crate) fn release_terminal(&mut self) {
        self.terminal_released = true;
    }
}
