//! Terminal / process boundary (FR-012–015, FR-019, FR-021/022).
//!
//! Isolates PTY + AI-CLI spawning from the pure core so session lifecycle and per-session
//! output routing are unit-testable without spawning processes (Constitution Principle I,
//! analyze finding C1). The real `portable-pty` + `alacritty_terminal` impl is daemon-side; the VT
//! grid is NOT part of this core seam. *Which* CLI a spec launches is the spec's own
//! [`LaunchSpec::provider`] since feature 026 — this module names no provider type. Contracts:
//! `terminal-backend-trait.md`, and the per-provider profiles in `contracts/`.

use crate::session::AiCli;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

/// Whether to start a fresh session or resume an existing one (research R6). Each provider spells
/// the two out in its own [`crate::provider::AiCliProvider::launch_args`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    /// A brand-new session, under an id the application owns.
    Fresh,
    /// Restore/restart an existing session by its id — never "the most recent one here".
    Resume,
}

/// Everything the backend needs to launch a session's AI CLI (the provider profile in
/// `contracts/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    /// Process cwd — the worktree directory; scopes the session (research R6).
    pub cwd: PathBuf,
    /// The session UUID (`--session-id`/`--resume` value).
    pub session_id: Uuid,
    /// Which AI CLI to launch (feature 026, FR-007) — taken from the session's own record, never
    /// re-derived. Without this field the seam does not reach the spawn at all: the argument
    /// builder below had no way to be told, so it named one provider and every session got that
    /// one's argv.
    pub provider: AiCli,
    /// Fresh vs resume.
    pub mode: LaunchMode,
    /// Extra environment (e.g. `TERM=xterm-256color`).
    pub env: Vec<(String, String)>,
}

/// Build the AI CLI argument vector for a launch spec. Pure, so the fresh/resume flag choice is
/// unit-testable without spawning (FR-013, FR-021).
///
/// Driven by `spec.provider` through the registry, so the argument shape lives in one place behind
/// the provider seam (FR-024, feature 026 FR-020). It used to be called `claude_args` and ignore
/// the spec entirely — the rename is the point, not decoration: a function named for one CLI is a
/// decision, and this one is now the session's.
pub fn launch_args(spec: &LaunchSpec) -> Vec<String> {
    spec.provider
        .provider()
        .launch_args(spec.session_id, spec.mode)
}

/// Resolve the platform's default interactive shell command (feature 010, research R3;
/// contracts/shell-process.md). Pure and argument-driven — the impure `std::env::var("SHELL")`
/// / `std::env::var("COMSPEC")` reads happen at the call site, not here, so this is testable
/// under `--no-default-features` without touching process env. An empty env value is treated
/// the same as absent (the convention every `AiCliProvider::config_dir` follows).
pub fn default_shell_command(shell_env: Option<&str>, comspec_env: Option<&str>) -> String {
    if cfg!(windows) {
        comspec_env
            .filter(|s| !s.is_empty())
            .unwrap_or("cmd.exe")
            .to_string()
    } else {
        shell_env
            .filter(|s| !s.is_empty())
            .unwrap_or("/bin/sh")
            .to_string()
    }
}

/// A live handle to one running session's terminal. `Send` so its reader can live on a worker
/// thread (research R4).
pub trait TerminalHandle: Send {
    /// Deliver keystrokes to the PTY writer (FR-014).
    fn write_input(&mut self, bytes: &[u8]) -> io::Result<()>;
    /// Resize the PTY to `rows`×`cols`.
    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()>;
    /// Terminate and reap the child process (FR-015a).
    fn kill(&mut self) -> io::Result<()>;
}

/// Spawns a session's AI CLI. The real impl wraps `portable-pty` (daemon-side); the fake records
/// specs for assertions.
pub trait TerminalBackend {
    /// Launch per `spec`, returning a live handle.
    fn spawn(&self, spec: LaunchSpec) -> io::Result<Box<dyn TerminalHandle>>;
}

// NOTE: the pure `SessionRouter` byte-routing seam was removed with the W3 migration (T030): the
// daemon now owns a real per-session `alacritty_terminal::Term`, so per-session isolation is a
// property of separate `Term` instances, covered end-to-end by the daemon's `session_isolation`
// integration test rather than an in-memory byte-buffer approximation.

// ---------------------------------------------------------------------------------------
// In-memory fake for tests (public so `tests/` can share it). No real process.
// ---------------------------------------------------------------------------------------

use std::sync::{Arc, Mutex};

/// Records launches for assertions and hands back a [`FakeHandle`] (contract).
#[derive(Debug, Default)]
pub struct FakeTerminalBackend {
    specs: std::cell::RefCell<Vec<LaunchSpec>>,
}

impl FakeTerminalBackend {
    /// A fresh fake backend.
    pub fn new() -> Self {
        Self::default()
    }

    /// All launch specs seen, in order.
    pub fn specs(&self) -> Vec<LaunchSpec> {
        self.specs.borrow().clone()
    }

    /// The most recent launch spec, if any.
    pub fn last_spec(&self) -> Option<LaunchSpec> {
        self.specs.borrow().last().cloned()
    }
}

impl TerminalBackend for FakeTerminalBackend {
    fn spawn(&self, spec: LaunchSpec) -> io::Result<Box<dyn TerminalHandle>> {
        self.specs.borrow_mut().push(spec);
        Ok(Box::new(FakeHandle::default()))
    }
}

/// Records writes/resizes/kills for a fake session (Send via `Arc<Mutex<_>>`).
#[derive(Debug, Clone, Default)]
pub struct FakeHandle {
    /// Bytes written via `write_input`.
    pub written: Arc<Mutex<Vec<u8>>>,
    /// Last `(rows, cols)` resize.
    pub last_resize: Arc<Mutex<Option<(u16, u16)>>>,
    /// Whether `kill` was called.
    pub killed: Arc<Mutex<bool>>,
}

impl TerminalHandle for FakeHandle {
    fn write_input(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.written.lock().unwrap().extend_from_slice(bytes);
        Ok(())
    }
    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()> {
        *self.last_resize.lock().unwrap() = Some((rows, cols));
        Ok(())
    }
    fn kill(&mut self) -> io::Result<()> {
        *self.killed.lock().unwrap() = true;
        Ok(())
    }
}
