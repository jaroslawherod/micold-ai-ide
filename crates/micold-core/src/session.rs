//! Session domain model and lifecycle state machine (FR-010, FR-015a, FR-020–023a).
//!
//! Pure and unit-testable; no process spawning (that lives behind
//! [`crate::terminal::TerminalBackend`]). A session is bound to one [`SessionLocation`] — either
//! a worktree (by `dir_name`) or, as of feature 010, the project's own root directory (the
//! "Default" location, constitution v1.3.0) — and one `claude` process; its identity +
//! `claude`-provided title are persisted so it restores across restarts (FR-020). Contract:
//! `contracts/terminal-backend-trait.md`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Stable session identity — the app-generated UUID passed to `claude --session-id` and used
/// as the `--resume` handle (research R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Generate a fresh random id for a new session.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wrap an existing UUID (persistence / tests).
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The sidebar label for a session — extracted from `claude`, never user-entered (FR-011a).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionLabel {
    /// No title from `claude` yet; show a neutral placeholder.
    Pending,
    /// The `claude`-provided session title.
    Named(String),
}

impl SessionLabel {
    /// The text to display (placeholder while pending).
    pub fn display(&self) -> &str {
        match self {
            SessionLabel::Pending => "New session",
            SessionLabel::Named(name) => name,
        }
    }
}

/// Runtime state of a session's `claude` process (FR-016, FR-022/022a). Never persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLifecycle {
    /// Persisted but no process running (after restart / project close). Reopen resumes it.
    Idle,
    /// The `claude` process is (re)launching.
    Starting,
    /// Running; the terminal is live.
    Running,
    /// Process exited unexpectedly; an auto-restart is pending (FR-022). `attempts` counts
    /// consecutive failed (re)starts for the crash-loop guard.
    Restarting { attempts: u8 },
    /// Auto-restart gave up after repeated quick failures (FR-022a); user may retry manually.
    Failed,
    /// The service restarted (reboot, crash, or a deliberate contract-mismatch restart) and found a
    /// durable record of a session that had a recorded conversation. Presented distinctly from both
    /// `Running` and a deliberately stopped (`Idle`) session, and **never auto-relaunched** — only a
    /// single explicit `start` resumes it (FR-006a/b, data-model L4).
    InterruptedResumable,
}

/// The maximum consecutive auto-restarts before giving up (FR-022a crash-loop guard).
pub const MAX_RESTART_ATTEMPTS: u8 = 3;

/// What the backend should do after a process exit (FR-022/022a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartDecision {
    /// Relaunch via `claude --resume <id>`.
    Resume,
    /// Stop trying; the session is now `Failed`.
    GiveUp,
}

/// Which of a session's two processes is currently attached to its visible terminal pane
/// (feature 010, FR-001–FR-007). Persisted (FR-011); independent of `SessionLifecycle`/
/// `ShellLifecycle` — it says which process is *displayed*, not which are *running*
/// (contracts/terminal-mode-lifecycle.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalMode {
    /// The session's `claude` process is attached (default — today's only behavior).
    #[default]
    AiCli,
    /// The session's plain shell process is attached.
    Regular,
}

// `TerminalMode::other()` — "the mode a single toggle press switches *to*" — was removed with the
// toggle in feature 027. Nothing else ever wanted it: a tab names the mode it selects, so every
// remaining caller assigns a mode outright rather than asking what the opposite one is.

/// Runtime state of a session's shell process (feature 010). Deliberately **not** a copy of
/// [`SessionLifecycle`] — no crash-loop, no `Failed`; restart is always manual (spec
/// Clarifications, 2026-07-18; contracts/terminal-mode-lifecycle.md). Never persisted, mirroring
/// `SessionLifecycle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellLifecycle {
    /// No shell process has been started for this session yet (or the session was just
    /// restored — the process cannot survive an app restart).
    #[default]
    NotStarted,
    /// The shell process is (re)launching.
    Starting,
    /// Running; the shell is live (attached or backgrounded).
    Running,
    /// The shell process exited (intentional `exit` or a crash) — restart is manual only.
    Exited,
}

impl ShellLifecycle {
    /// Whether the shell currently has a running/launching process.
    pub fn is_active(self) -> bool {
        matches!(self, ShellLifecycle::Starting | ShellLifecycle::Running)
    }

    /// Begin (re)starting the shell: `NotStarted`/`Exited` → `Starting`; a no-op if already
    /// `Starting`/`Running` (mirrors `Session::start`'s idempotency, FR-003/FR-004).
    pub fn start_shell(&mut self) {
        if matches!(self, ShellLifecycle::NotStarted | ShellLifecycle::Exited) {
            *self = ShellLifecycle::Starting;
        }
    }

    /// The shell process is up: → `Running`.
    pub fn mark_running(&mut self) {
        *self = ShellLifecycle::Running;
    }

    /// The shell process exited: → `Exited`, unconditionally, with no restart decision (FR-013
    /// — unlike `Session::on_unexpected_exit`, there is no automatic follow-up).
    pub fn mark_exited(&mut self) {
        *self = ShellLifecycle::Exited;
    }
}

/// Identifies one Regular Terminal instance within a single session (feature 011, research R1).
/// Allocated from `Session.next_shell_id`, a per-session monotonic counter — unique only within
/// its owning session, for the lifetime of the running app; never persisted, never compared
/// across sessions. Doubles as the switcher row's display label (spec Assumptions: instances are
/// identified by creation order) — there is no separate "display name" field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ShellInstanceId(pub u32);

/// One of possibly several Regular Terminal instances a session may have open (feature 011,
/// contracts/shell-instance-lifecycle.md). `lifecycle` reuses [`ShellLifecycle`] unchanged — an
/// instance's state machine is identical to the single-shell case feature 010 established, just
/// now scoped to one element of `Session.shells` instead of the session's whole Regular side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellInstance {
    pub id: ShellInstanceId,
    pub lifecycle: ShellLifecycle,
}

/// Where a session's working directory lives (feature 010, data-model.md). A closed enum
/// (Constitution Principle V) so a session can never be ambiguously "maybe a worktree, maybe
/// not" — every session maps to exactly one of these two sanctioned locations (constitution
/// v1.3.0, Principle III).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLocation {
    /// Hosted by a worktree, identified by its `dir_name` (identity, unchanged from before
    /// feature 010).
    Worktree(String),
    /// Hosted directly by the project's own root directory — no worktree. Presented to users
    /// as "Default".
    Default,
}

impl SessionLocation {
    /// The resolved working directory for a session at this location, given the project's
    /// root (`repo`). The single authoritative implementation — every cwd-resolution call site
    /// (`src/main.rs`) and test that needs this decision calls through here rather than
    /// re-deriving the `Worktree`/`Default` branch by hand.
    pub fn cwd(&self, repo: &Path) -> PathBuf {
        match self {
            SessionLocation::Worktree(dir) => repo.join(".claude/worktrees").join(dir),
            SessionLocation::Default => repo.to_path_buf(),
        }
    }

    /// Whether this location is the worktree named `dir` — a borrowing comparison so callers
    /// don't need to allocate a `SessionLocation::Worktree(dir.to_string())` just to compare.
    pub fn is_worktree(&self, dir: &str) -> bool {
        matches!(self, SessionLocation::Worktree(d) if d == dir)
    }
}

/// A unit of work bound to a single [`SessionLocation`], with one embedded terminal
/// (data-model.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Stable identity + `--resume` handle.
    pub id: SessionId,
    /// Where this session's working directory lives — a worktree or the project root.
    pub location: SessionLocation,
    /// Sidebar label (from `claude`).
    pub label: SessionLabel,
    /// Runtime state of the AI CLI process (transient — not persisted).
    pub lifecycle: SessionLifecycle,
    /// Derived activity signal from the daemon (feature 010 US2, FR-016d) — transient, never
    /// persisted, resets to `Unknown` on daemon restart (H3). Drives the sidebar activity badge.
    pub activity: crate::protocol::messages::ActivitySignal,
    /// Which process is attached to the visible pane (feature 010). Persisted (FR-011).
    pub mode: TerminalMode,
    /// Every currently-open Regular Terminal instance, in creation ("open") order (feature 011;
    /// transient — not persisted, mirrors `lifecycle`'s non-persistence). Replaces feature 010's
    /// single `shell_lifecycle: ShellLifecycle` field.
    pub shells: Vec<ShellInstance>,
    /// Which instance is attached to the visible pane when `mode == Regular` (feature 011).
    /// Invariant: `Some(id)` names a live element of `shells`, or `None` iff `shells` is empty —
    /// enforced by construction by the methods below; no other code path writes this field.
    pub active_shell: Option<ShellInstanceId>,
    /// Monotonic counter allocating the next `ShellInstanceId` (feature 011, research R1) —
    /// never decremented or reused, even after every instance is closed.
    pub next_shell_id: u32,
    /// Whether the user closed this session (bugfix BUG-003, FR-015a). An archived session's
    /// process has been stopped and its record is kept, but it is never shown in the sidebar
    /// again (an "invisible tombstone" — no unarchive path). This field is a fast, in-memory
    /// convenience only: the *durable* record that suppresses reconciliation (FR-020c) is a
    /// marker in the AI CLI provider's own storage (`AiCliProvider::mark_archived`), which
    /// survives even if this field's own persisted copy is lost. Set via [`Session::archive`].
    pub archived: bool,
}

impl Session {
    /// A brand-new session at `location`, starting immediately (FR-010). Always starts
    /// attached to the AI CLI (feature 010 FR-010 — today's only behavior for a fresh session).
    pub fn start_new(location: SessionLocation) -> Self {
        Self {
            id: SessionId::new(),
            location,
            label: SessionLabel::Pending,
            lifecycle: SessionLifecycle::Starting,
            activity: crate::protocol::messages::ActivitySignal::Unknown,
            mode: TerminalMode::AiCli,
            shells: Vec::new(),
            active_shell: None,
            next_shell_id: 1,
            archived: false,
        }
    }

    /// A persisted session restored from disk — `Idle` until reopened (FR-020, FR-023a).
    /// `mode` is the persisted value (feature 010 FR-011); `shells` is always empty since no
    /// process survives an app restart (feature 011 FR-017 — at most one instance is ever
    /// restored, lazily, on first switch into Regular mode).
    pub fn restored(
        id: SessionId,
        location: SessionLocation,
        label: SessionLabel,
        mode: TerminalMode,
    ) -> Self {
        Self {
            id,
            location,
            label,
            lifecycle: SessionLifecycle::Idle,
            activity: crate::protocol::messages::ActivitySignal::Unknown,
            mode,
            shells: Vec::new(),
            active_shell: None,
            next_shell_id: 1,
            archived: false,
        }
    }

    /// Set the currently-attached mode (feature 010 FR-002) — always legal, regardless of
    /// either process's running state.
    pub fn set_mode(&mut self, mode: TerminalMode) {
        self.mode = mode;
    }

    /// Open a new Regular Terminal instance: always succeeds (feature 011 FR-001, FR-007;
    /// contracts/shell-instance-lifecycle.md). Allocates the next id, appends a `Starting`
    /// instance to the end of `shells` (append-on-open order), and makes it the active one.
    /// Used both for an explicit "open an additional instance" action and for lazily creating a
    /// session's first-ever instance on its first switch into Regular mode.
    pub fn open_shell_instance(&mut self) -> ShellInstanceId {
        let id = ShellInstanceId(self.next_shell_id);
        self.next_shell_id += 1;
        let mut instance = ShellInstance {
            id,
            lifecycle: ShellLifecycle::NotStarted,
        };
        instance.lifecycle.start_shell();
        self.shells.push(instance);
        self.active_shell = Some(id);
        id
    }

    /// Switch which instance is attached to the visible pane (feature 011 FR-004). A no-op if
    /// `id` does not name a live element of `shells` (e.g. a race with a concurrent close).
    pub fn select_shell(&mut self, id: ShellInstanceId) {
        if self.shells.iter().any(|s| s.id == id) {
            self.active_shell = Some(id);
        }
    }

    /// Close one Regular Terminal instance (feature 011 FR-011–FR-013). A no-op if `id` does not
    /// name a live instance. If the closed instance was the active one, reassigns `active_shell`
    /// to the element now sitting at the removed position (the former *next* instance in list
    /// order) or, if there is none, the new last element (FR-012 — resolved 2026-07-20: "next in
    /// list, else previous"). If `shells` is empty afterward, `active_shell` is already `None`
    /// from that fallback chain, and `mode` reverts to `TerminalMode::AiCli` (FR-013).
    pub fn close_shell(&mut self, id: ShellInstanceId) {
        let Some(pos) = self.shells.iter().position(|s| s.id == id) else {
            return;
        };
        self.shells.remove(pos);
        if self.active_shell == Some(id) {
            self.active_shell = self
                .shells
                .get(pos)
                .or_else(|| self.shells.last())
                .map(|s| s.id);
        }
        if self.shells.is_empty() {
            self.mode = TerminalMode::AiCli;
        }
    }

    /// Manually restart one instance after it exited (feature 011 FR-010) — a no-op if `id` is
    /// absent, else delegates to that instance's unchanged `ShellLifecycle::start_shell`
    /// (idempotent: also a no-op if that instance is already `Starting`/`Running`).
    pub fn restart_shell_instance(&mut self, id: ShellInstanceId) {
        if let Some(instance) = self.shells.iter_mut().find(|s| s.id == id) {
            instance.lifecycle.start_shell();
        }
    }

    /// The shell process for instance `id` is up (feature 011). No-op if `id` is absent.
    pub fn mark_shell_running(&mut self, id: ShellInstanceId) {
        if let Some(instance) = self.shells.iter_mut().find(|s| s.id == id) {
            instance.lifecycle.mark_running();
        }
    }

    /// Instance `id`'s shell process exited — no restart decision (feature 011 FR-008). No-op if
    /// `id` is absent.
    pub fn mark_shell_exited(&mut self, id: ShellInstanceId) {
        if let Some(instance) = self.shells.iter_mut().find(|s| s.id == id) {
            instance.lifecycle.mark_exited();
        }
    }

    /// The currently-active instance's lifecycle, or `None` if `shells` is empty (feature 011;
    /// replaces direct `session.shell_lifecycle` field reads at feature-010-era call sites).
    pub fn active_shell_lifecycle(&self) -> Option<ShellLifecycle> {
        let id = self.active_shell?;
        self.shells.iter().find(|s| s.id == id).map(|s| s.lifecycle)
    }

    /// Whether the session currently has a running/launching process.
    pub fn is_active(&self) -> bool {
        matches!(
            self.lifecycle,
            SessionLifecycle::Starting
                | SessionLifecycle::Running
                | SessionLifecycle::Restarting { .. }
        )
    }

    /// Begin (or resume) running: `Idle`/`Failed`/`InterruptedResumable` → `Starting` (FR-010,
    /// FR-023a). For an `InterruptedResumable` session this is the single explicit action that
    /// resumes the prior conversation (FR-006a) — startup never does it automatically (FR-006b).
    pub fn start(&mut self) {
        if matches!(
            self.lifecycle,
            SessionLifecycle::Idle
                | SessionLifecycle::Failed
                | SessionLifecycle::InterruptedResumable
        ) {
            self.lifecycle = SessionLifecycle::Starting;
        }
    }

    /// Present a durable session as interrupted-but-resumable after a service restart (FR-006a): a
    /// no-op unless it is currently `Idle` (the state every session loads in), so this never
    /// overrides a persisted `Failed`, a live process, or an already-marked session.
    pub fn mark_interrupted_resumable(&mut self) {
        if matches!(self.lifecycle, SessionLifecycle::Idle) {
            self.lifecycle = SessionLifecycle::InterruptedResumable;
        }
    }

    /// The process is up: `Starting`/`Restarting` → `Running` (resets the crash-loop counter).
    pub fn mark_running(&mut self) {
        self.lifecycle = SessionLifecycle::Running;
    }

    /// Update the label from a `claude`-provided title (FR-011a).
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.label = SessionLabel::Named(title.into());
    }

    /// Handle an UNEXPECTED process exit (crash / external kill), applying the crash-loop
    /// guard (FR-022/022a). Returns whether to resume or give up.
    pub fn on_unexpected_exit(&mut self) -> RestartDecision {
        let attempts = match self.lifecycle {
            SessionLifecycle::Restarting { attempts } => attempts,
            _ => 0,
        };
        let next = attempts + 1;
        if next >= MAX_RESTART_ATTEMPTS {
            self.lifecycle = SessionLifecycle::Failed;
            RestartDecision::GiveUp
        } else {
            self.lifecycle = SessionLifecycle::Restarting { attempts: next };
            RestartDecision::Resume
        }
    }

    /// Stop the process intentionally on project **close** → `Idle`, preserving the record; NO
    /// auto-restart applies (FR-023). Note: as of feature 008, merely *switching* the active
    /// project no longer calls this — switched-away sessions keep running in the background.
    pub fn stop_for_project_change(&mut self) {
        self.lifecycle = SessionLifecycle::Idle;
    }

    /// The process exited **cleanly** on its own (status 0 — the user quit `claude`, ran `exit`):
    /// → `Idle`, no auto-restart (FR-004 scenario 3). Distinct from [`Self::on_unexpected_exit`],
    /// which governs crashes with the retry budget. Resets any in-flight crash-loop counter, since
    /// a clean exit is not a failure.
    pub fn record_clean_exit(&mut self) {
        self.lifecycle = SessionLifecycle::Idle;
    }

    /// The user closed this session (bugfix BUG-003, FR-015a): stop the process (`Idle`, no
    /// auto-restart, mirrors [`stop_for_project_change`](Self::stop_for_project_change)) and flag
    /// it `archived` so it is excluded from the sidebar. The record itself is kept — callers must
    /// NOT remove it from the workspace, and must additionally record the durable, provider-side
    /// marker (`AiCliProvider::mark_archived`) so reconciliation (FR-020c) never reconstructs it,
    /// even if this in-memory flag's own persisted copy is later lost.
    pub fn archive(&mut self) {
        self.lifecycle = SessionLifecycle::Idle;
        self.archived = true;
    }
}
