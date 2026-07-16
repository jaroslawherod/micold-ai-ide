//! Terminal / process boundary (FR-012–015, FR-019, FR-021/022).
//!
//! Isolates PTY + `claude` spawning from the pure core so session lifecycle and per-session
//! output routing are unit-testable without spawning processes (Constitution Principle I,
//! analyze finding C1). The real `portable-pty` + `alacritty_terminal` impl is gui-side
//! (`src/ui/terminal.rs`); the VT grid is NOT part of this core seam. Contracts:
//! `terminal-backend-trait.md`, `claude-cli.md`.

use crate::session::SessionId;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

/// Whether to start a fresh `claude` session or resume an existing one (research R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    /// `claude --session-id <uuid>` — a brand-new session the app owns the id for.
    Fresh,
    /// `claude --resume <uuid>` — restore/restart an existing session.
    Resume,
}

/// Everything the backend needs to launch `claude` for one session (claude-cli.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    /// Process cwd — the worktree directory; scopes the `claude` session (research R6).
    pub cwd: PathBuf,
    /// The session UUID (`--session-id`/`--resume` value).
    pub session_id: Uuid,
    /// Fresh vs resume.
    pub mode: LaunchMode,
    /// Extra environment (e.g. `TERM=xterm-256color`).
    pub env: Vec<(String, String)>,
}

/// Build the `claude` argument vector for a launch spec (claude-cli.md). Pure, so the
/// fresh/resume flag choice is unit-testable without spawning (FR-013, FR-021).
pub fn claude_args(spec: &LaunchSpec) -> Vec<String> {
    let id = spec.session_id.to_string();
    match spec.mode {
        LaunchMode::Fresh => vec!["--session-id".to_string(), id],
        LaunchMode::Resume => vec!["--resume".to_string(), id],
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

/// Spawns `claude` for a session. The real impl wraps `portable-pty` (gui-side); the fake
/// records specs for assertions.
pub trait TerminalBackend {
    /// Launch per `spec`, returning a live handle.
    fn spawn(&self, spec: LaunchSpec) -> io::Result<Box<dyn TerminalHandle>>;
}

/// Pure per-session output routing seam (analyze finding C1). Routes `PtyOutput { id, chunk }`
/// to the correct session's buffer so isolation/no-cross-talk is unit-testable (FR-019,
/// SC-005) WITHOUT the gui-side VT `Term`.
#[derive(Debug, Default)]
pub struct SessionRouter {
    buffers: HashMap<SessionId, Vec<u8>>,
}

impl SessionRouter {
    /// An empty router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a session so it can receive output.
    pub fn register(&mut self, id: SessionId) {
        self.buffers.entry(id).or_default();
    }

    /// Stop tracking a session (on close).
    pub fn remove(&mut self, id: SessionId) {
        self.buffers.remove(&id);
    }

    /// Route an output chunk to its session. Unknown/unregistered ids are dropped (a closed
    /// session must never leak into another — FR-019).
    pub fn route(&mut self, id: SessionId, chunk: &[u8]) {
        if let Some(buf) = self.buffers.get_mut(&id) {
            buf.extend_from_slice(chunk);
        }
    }

    /// The bytes accumulated for a session so far (test/inspection).
    pub fn buffer(&self, id: SessionId) -> &[u8] {
        self.buffers.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }
}

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
