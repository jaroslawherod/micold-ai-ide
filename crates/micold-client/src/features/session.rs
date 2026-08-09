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
use crate::app::Overlay;
use crate::app::State;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;
use micold_core::project::canonicalize_best_effort;
use micold_core::session::{Session, SessionId};
use std::path::Path;

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
        (state.overlay == Overlay::ConfirmSessionRemove).then_some(ConfirmSessionRemoveDialog)
    }
}
