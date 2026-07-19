//! Session domain model and lifecycle state machine (FR-010, FR-015a, FR-020–023a).
//!
//! Pure and unit-testable; no process spawning (that lives behind
//! [`crate::terminal::TerminalBackend`]). A session is bound to one [`SessionLocation`] — either
//! a worktree (by `dir_name`) or, as of feature 010, the project's own root directory (the
//! "Default" location, constitution v1.3.0) — and one `claude` process; its identity +
//! `claude`-provided title are persisted so it restores across restarts (FR-020). Contract:
//! `contracts/terminal-backend-trait.md`.

use std::fmt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Stable session identity — the app-generated UUID passed to `claude --session-id` and used
/// as the `--resume` handle (research R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Runtime state (transient — not persisted).
    pub lifecycle: SessionLifecycle,
}

impl Session {
    /// A brand-new session at `location`, starting immediately (FR-010).
    pub fn start_new(location: SessionLocation) -> Self {
        Self {
            id: SessionId::new(),
            location,
            label: SessionLabel::Pending,
            lifecycle: SessionLifecycle::Starting,
        }
    }

    /// A persisted session restored from disk — `Idle` until reopened (FR-020, FR-023a).
    pub fn restored(id: SessionId, location: SessionLocation, label: SessionLabel) -> Self {
        Self {
            id,
            location,
            label,
            lifecycle: SessionLifecycle::Idle,
        }
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

    /// Begin (or resume) running: `Idle`/`Failed` → `Starting` (FR-010, FR-023a).
    pub fn start(&mut self) {
        if matches!(
            self.lifecycle,
            SessionLifecycle::Idle | SessionLifecycle::Failed
        ) {
            self.lifecycle = SessionLifecycle::Starting;
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
}
