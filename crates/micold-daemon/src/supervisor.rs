//! PTY spawn + child supervision (plan W3, task T031).
//!
//! Each [`PtySession`] owns one child process in a pseudo-terminal and the [`Term`] that interprets
//! its output — both live in the daemon, so the process and its interpreted grid **outlive every
//! client** (FR-001). A per-session reader thread absorbs the blocking PTY read (research R4): it
//! streams raw bytes into the VT parser under the shared [`FairMutex`], and on EOF records that the
//! child is gone. Nothing here depends on a client being attached; a session with no viewer keeps
//! running and its grid keeps updating.
//!
//! Detached sizing (Edge: resize while detached): a session always has a defined grid size — seeded
//! at spawn and only changed by an explicit [`PtySession::resize`], which the client drives on
//! attach via `SessionResize`. No attached client is required for the size to be valid.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use micold_core::provider::{AiCliProvider, ClaudeProvider};
use micold_core::session::SessionId;
use micold_core::terminal::{claude_args, default_shell_command, LaunchSpec};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use crate::supervision::ExitOutcome;
use crate::terminal::{DaemonListener, SharedTerm, SharedWriter, VtSignals};

/// The grid size a session starts with before any client reports its real pane dimensions
/// (bugfix: a new terminal must not start at 1×1). Matches the client's historical seed.
const INIT_COLS: u16 = 100;
const INIT_ROWS: u16 = 30;

/// Fixed grid dimensions handed to `Term` at construction / resize (alacritty needs a `Dimensions`).
struct TermDimensions {
    rows: usize,
    cols: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// One daemon-hosted terminal session: a PTY child plus its VT emulator, alive independently of any
/// client connection (FR-001).
pub struct PtySession {
    id: SessionId,
    /// The VT emulator, shared with the reader thread and the framer (plan W3).
    term: SharedTerm,
    /// The child process. Behind a mutex so liveness checks / kill take `&self`.
    child: Mutex<Box<dyn Child + Send + Sync>>,
    /// The PTY master — used for resize. Behind a `Mutex` only so [`PtySession`] is `Sync` (the
    /// trait object is `Send` but not `Sync`); this lets the session live in the shared daemon
    /// registry. `resize` takes `&self` and the lock is never held across an `.await`.
    master: Mutex<Box<dyn MasterPty + Send>>,
    /// The PTY writer, shared with the [`DaemonListener`] (VT replies use the same writer as user
    /// input, so both hold this).
    writer: SharedWriter,
    /// Out-of-band signals (dirty flag, title, bell, child-exit) the framer/catalog read.
    signals: VtSignals,
    /// The current grid size, shared with the listener (answers `TextAreaSizeRequest`).
    size: Arc<Mutex<WindowSize>>,
    /// Set true when the reader thread sees EOF on the PTY (the child's output ended).
    reader_done: Arc<AtomicBool>,
    /// The child's exit outcome once reaped — `Some(success)` after [`Self::is_alive`] or
    /// [`Self::exit_outcome`] observes the child gone, `None` while it is still running. Cached
    /// because `try_wait` only yields the status once; the supervisor reads it after the child dies
    /// to classify a clean exit vs a crash (US4, FR-005).
    exit: Arc<Mutex<Option<bool>>>,
    /// The reader thread handle, joined on drop.
    reader: Option<JoinHandle<()>>,
}

impl PtySession {
    /// Spawn `claude` for `spec` as a daemon-owned session (FR-006). `scrollback_lines` sets the VT
    /// history depth; `initial_size` seeds the grid (falls back to the standard seed).
    pub fn spawn_claude(
        id: SessionId,
        spec: &LaunchSpec,
        scrollback_lines: usize,
        initial_size: Option<(u16, u16)>,
    ) -> io::Result<Self> {
        let mut cmd = CommandBuilder::new(ClaudeProvider.command());
        cmd.cwd(&spec.cwd);
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        for arg in claude_args(spec) {
            cmd.arg(arg);
        }
        Self::spawn(id, cmd, scrollback_lines, initial_size)
    }

    /// Spawn the platform's plain interactive shell in `cwd` as a daemon-owned session
    /// (contracts/shell-process.md). No `claude`-specific args apply.
    pub fn spawn_shell(
        id: SessionId,
        cwd: &std::path::Path,
        env: &[(String, String)],
        scrollback_lines: usize,
        initial_size: Option<(u16, u16)>,
    ) -> io::Result<Self> {
        let shell = std::env::var("SHELL").ok();
        let comspec = std::env::var("COMSPEC").ok();
        let command = default_shell_command(shell.as_deref(), comspec.as_deref());
        let mut cmd = CommandBuilder::new(command);
        cmd.cwd(cwd);
        for (k, v) in env {
            cmd.env(k, v);
        }
        Self::spawn(id, cmd, scrollback_lines, initial_size)
    }

    /// Open a PTY, spawn `cmd` as its child, start the reader thread, and build the VT emulator with
    /// its daemon listener. Shared plumbing for [`Self::spawn_claude`] / [`Self::spawn_shell`].
    pub fn spawn(
        id: SessionId,
        cmd: CommandBuilder,
        scrollback_lines: usize,
        initial_size: Option<(u16, u16)>,
    ) -> io::Result<Self> {
        let (cols, rows) = initial_size.unwrap_or((INIT_COLS, INIT_ROWS));
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(io::Error::other)?;

        let child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
        // Drop the slave so the PTY reports EOF once the child (and any of its children holding the
        // slave open) exit — otherwise the reader thread would block forever.
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let writer: SharedWriter = Arc::new(Mutex::new(
            pair.master.take_writer().map_err(io::Error::other)?,
        ));

        let size = Arc::new(Mutex::new(WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 0,
            cell_height: 0,
        }));
        let signals = VtSignals::default();

        let listener = DaemonListener::new(Arc::clone(&writer), Arc::clone(&size), signals.clone());
        let dims = TermDimensions {
            rows: rows as usize,
            cols: cols as usize,
        };
        let config = Config {
            scrolling_history: scrollback_lines,
            ..Config::default()
        };
        let term: SharedTerm = Arc::new(FairMutex::new(Term::new(config, &dims, listener)));

        // The reader thread: absorb the blocking PTY read, feeding bytes into the VT parser under
        // the shared lock. It, and only it, advances the parser (research R4).
        let reader_term = Arc::clone(&term);
        let reader_dirty = signals.clone();
        let reader_done = Arc::new(AtomicBool::new(false));
        let done_flag = Arc::clone(&reader_done);
        let reader = std::thread::Builder::new()
            .name(format!("vt-reader-{id}"))
            .spawn(move || {
                let mut parser: Processor = Processor::new();
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let mut term = reader_term.lock();
                            parser.advance(&mut *term, &buf[..n]);
                            drop(term);
                            reader_dirty.mark_dirty();
                        }
                    }
                }
                done_flag.store(true, Ordering::Release);
                reader_dirty.mark_dirty();
            })?;

        Ok(Self {
            id,
            term,
            child: Mutex::new(child),
            master: Mutex::new(pair.master),
            writer,
            signals,
            size,
            reader_done,
            exit: Arc::new(Mutex::new(None)),
            reader: Some(reader),
        })
    }

    /// The session's stable identity.
    pub fn id(&self) -> SessionId {
        self.id
    }

    /// The shared VT emulator (the framer reads the grid through this).
    pub fn term(&self) -> &SharedTerm {
        &self.term
    }

    /// Out-of-band VT signals (dirty flag, title, bell, child-exit).
    pub fn signals(&self) -> &VtSignals {
        &self.signals
    }

    /// Deliver keystrokes to the child (FR-014/FR-019). The writer is shared with VT replies, so
    /// user input and terminal replies serialize on the one PTY writer.
    pub fn write_input(&self, bytes: &[u8]) -> io::Result<()> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("writer poisoned"))?;
        w.write_all(bytes)?;
        w.flush()
    }

    /// Resize the PTY and the VT grid to `cols`×`rows`. Valid whether or not a client is attached
    /// (detached sizing): the size is authoritative daemon state.
    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        if cols == 0 || rows == 0 {
            return Ok(());
        }
        self.master
            .lock()
            .map_err(|_| io::Error::other("pty master mutex poisoned"))?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(io::Error::other)?;
        {
            let mut term = self.term.lock();
            term.resize(TermSizeShim {
                cols: cols as usize,
                rows: rows as usize,
            });
        }
        if let Ok(mut s) = self.size.lock() {
            s.num_cols = cols;
            s.num_lines = rows;
        }
        Ok(())
    }

    /// Whether the child is still running, via a non-blocking `try_wait` reap. Authoritative for
    /// liveness (the reader-thread EOF flag is only a hint — a child can close stdout yet linger).
    /// On observing the child gone, caches its exit success bit for [`Self::exit_outcome`].
    pub fn is_alive(&self) -> bool {
        match self.child.lock() {
            Ok(mut child) => match child.try_wait() {
                Ok(Some(status)) => {
                    self.cache_exit(status.success());
                    false
                }
                Ok(None) => true,
                // A reap error means we can no longer track the child; treat it as gone (a crash,
                // so supervision can react) rather than pretend it is alive forever.
                Err(_) => {
                    self.cache_exit(false);
                    false
                }
            },
            Err(_) => false,
        }
    }

    /// Record the child's exit success bit the first time it is observed (idempotent — later reaps
    /// never overwrite the first-seen outcome).
    fn cache_exit(&self, success: bool) {
        if let Ok(mut e) = self.exit.lock() {
            e.get_or_insert(success);
        }
    }

    /// The child's exit outcome once it has exited, or `None` while it is still running. Reaps via
    /// [`Self::is_alive`] if not yet observed, so a caller can poll this directly. Drives the
    /// restart supervision policy (US4, FR-005): a clean exit stops the session, a crash restarts.
    pub fn exit_outcome(&self) -> Option<ExitOutcome> {
        if self.exit.lock().ok().and_then(|e| *e).is_none() {
            // Not yet observed — attempt a reap now.
            let _ = self.is_alive();
        }
        self.exit
            .lock()
            .ok()
            .and_then(|e| *e)
            .map(ExitOutcome::from_success)
    }

    /// Whether the PTY reader has seen EOF (the child's output stream ended). A hint for the framer;
    /// [`Self::is_alive`] is authoritative for process liveness.
    pub fn output_ended(&self) -> bool {
        self.reader_done.load(Ordering::Acquire)
    }

    /// The child's OS process id, if known.
    pub fn pid(&self) -> Option<u32> {
        self.child.lock().ok().and_then(|c| c.process_id())
    }

    /// Terminate and reap the child (FR-015a / session close). Signals the child's whole process
    /// **group** first so grandchildren the session forked don't orphan (FR-036, T061), then reaps
    /// the direct child.
    pub fn kill(&self) -> io::Result<()> {
        if let Some(pid) = self.pid() {
            crate::platform::terminate_process_tree(pid);
        }
        if let Ok(mut child) = self.child.lock() {
            child.kill()?;
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Best-effort teardown: kill the child so the reader thread hits EOF, then join it so no
        // thread outlives the session.
        let _ = self.kill();
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
    }
}

/// `Term::resize` takes any `Dimensions`; this is the resize-time shim (distinct from the
/// construction-time `TermDimensions` only to keep call sites obvious).
struct TermSizeShim {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermSizeShim {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A `sh -c "<script>"` command builder for a real, short-lived child.
    fn sh(script: &str) -> CommandBuilder {
        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(script);
        cmd
    }

    /// Poll `exit_outcome` until the child is reaped (bounded), returning the classification.
    fn wait_for_exit(session: &PtySession) -> ExitOutcome {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(outcome) = session.exit_outcome() {
                return outcome;
            }
            assert!(Instant::now() < deadline, "child did not exit in time");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn a_zero_exit_is_classified_clean() {
        let s = PtySession::spawn(SessionId::new(), sh("exit 0"), 100, None).unwrap();
        assert_eq!(wait_for_exit(&s), ExitOutcome::Clean);
    }

    #[test]
    fn a_nonzero_exit_is_classified_crashed() {
        let s = PtySession::spawn(SessionId::new(), sh("exit 3"), 100, None).unwrap();
        assert_eq!(wait_for_exit(&s), ExitOutcome::Crashed);
    }

    #[test]
    fn a_live_child_has_no_exit_outcome_yet() {
        let s = PtySession::spawn(SessionId::new(), sh("sleep 5"), 100, None).unwrap();
        assert!(s.is_alive());
        assert_eq!(s.exit_outcome(), None);
        let _ = s.kill();
    }

    #[test]
    fn the_first_seen_outcome_is_cached_and_stable() {
        let s = PtySession::spawn(SessionId::new(), sh("exit 0"), 100, None).unwrap();
        let first = wait_for_exit(&s);
        // Repeated reads never flip the outcome (try_wait only yields the status once).
        assert_eq!(s.exit_outcome(), Some(first));
        assert_eq!(s.exit_outcome(), Some(ExitOutcome::Clean));
    }

    /// FR-036 / T061: tearing down a session reaps its whole process group, not just the direct
    /// child — a backgrounded grandchild must not orphan.
    #[test]
    fn kill_reaps_the_whole_process_group() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("grandchild.pid");
        // sh backgrounds a `sleep` (a grandchild in the same process group), records its pid, then
        // waits. Killing the group must take the grandchild with it.
        let script = format!("sleep 300 & echo $! > {} ; wait", pidfile.display());
        let session = PtySession::spawn(SessionId::new(), sh(&script), 100, None).unwrap();

        // Read the grandchild pid once the shell has recorded it.
        let deadline = Instant::now() + Duration::from_secs(10);
        let grandchild: i32 = loop {
            if let Ok(text) = std::fs::read_to_string(&pidfile) {
                if let Ok(pid) = text.trim().parse::<i32>() {
                    break pid;
                }
            }
            assert!(Instant::now() < deadline, "grandchild pid never recorded");
            std::thread::sleep(Duration::from_millis(20));
        };
        // SAFETY: `kill(pid, 0)` sends no signal; it only probes whether the process exists.
        assert_eq!(
            unsafe { libc::kill(grandchild, 0) },
            0,
            "grandchild should be alive before teardown"
        );

        session.kill().unwrap();

        // The grandchild is gone once the group teardown reaches it (bounded — reparenting +
        // reaping is not instantaneous).
        let deadline = Instant::now() + Duration::from_secs(10);
        while unsafe { libc::kill(grandchild, 0) } == 0 {
            assert!(
                Instant::now() < deadline,
                "grandchild survived the process-group teardown"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
