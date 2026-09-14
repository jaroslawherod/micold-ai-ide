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
use micold_core::session::SessionId;
use micold_core::terminal::{default_shell_command, launch_args, LaunchSpec};
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
    /// The child process and its process tree. Behind a mutex so liveness checks / kill take
    /// `&self`, and one mutex for both so that reaping the child and forgetting its pid happen
    /// together — see [`Supervised`].
    child: Mutex<Supervised>,
    /// The PTY master — used for resize. Behind a `Mutex` only so [`PtySession`] is `Sync` (the
    /// trait object is `Send` but not `Sync`); this lets the session live in the shared daemon
    /// registry. `resize` takes `&self` and the lock is never held across an `.await`.
    ///
    /// An `Option` only so [`Drop`] can close it before joining the reader; it is `Some` for the
    /// whole of the session's public life.
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    /// The PTY writer, shared with the [`DaemonListener`] (VT replies use the same writer as user
    /// input, so both hold this).
    writer: SharedWriter,
    /// Out-of-band signals (dirty flag, title, bell, child-exit) the framer/catalog read.
    signals: VtSignals,
    /// The current grid size, shared with the listener (answers `TextAreaSizeRequest`).
    size: Arc<Mutex<WindowSize>>,
    /// Set true when the reader thread sees EOF on the PTY (the child's output ended).
    reader_done: Arc<AtomicBool>,
    /// The child's exit outcome once reaped — `Some(outcome)` after [`Self::is_alive`] or
    /// [`Self::exit_outcome`] observes the child gone, `None` while it is still running. Cached
    /// because `try_wait` only yields the status once; the supervisor reads it after the child dies
    /// to classify a clean exit vs a crash (US4, FR-005).
    exit: Arc<Mutex<Option<ExitOutcome>>>,
    /// The reader thread handle, joined on drop.
    reader: Option<JoinHandle<()>>,
}

/// Refuse a spawn whose working directory does not exist (FR-006c, BUG-012).
///
/// [`CommandBuilder`] does **not** fail on a missing `cwd`: it filters the path out and substitutes
/// the user's home directory, which also changes how the command binary itself is resolved. So a
/// session whose worktree was deleted outside the application would otherwise start — silently, and
/// against `$HOME` rather than the project it names. For an AI-CLI session, which is a thing the
/// user gives instructions to, that is the wrong tree to act on rather than a cosmetic detail.
///
/// Checked against the filesystem at spawn time rather than against the daemon's cached worktree
/// statuses: the cache is refreshed on its own schedule, and the directory can vanish between a
/// refresh and this call. The cache remains the right source for the row's `missing` badge.
fn ensure_cwd_exists(cwd: &std::path::Path) -> io::Result<()> {
    if cwd.is_dir() {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "session working directory does not exist: {}",
            cwd.display()
        ),
    ))
}

/// A session's child together with the process tree its teardown reaches.
///
/// They share a lock because on Unix the tree is reached through the child's pid, and that pid stops
/// naming the child the moment the child is reaped. Every reap below is followed, under the same
/// guard, by [`Supervised::reaped`]; every tree teardown happens under it too. A liveness check on
/// another thread therefore cannot reap between a teardown reading the pid and signalling it.
struct Supervised {
    child: Box<dyn Child + Send + Sync>,
    /// The child and everything it starts, as the platform reaches them at teardown — a process
    /// group on Unix, a job object on Windows (FR-036). Adopted at spawn, because a job has to
    /// exist before the child starts the processes it should contain.
    tree: crate::platform::ProcessTree,
    /// Whether the child has been reaped. After that nothing may signal its pid — including
    /// `portable-pty`'s own `Child::kill`, which on Unix sends `SIGHUP` to the pid without asking
    /// whether it has already been waited on.
    reaped: bool,
}

impl Supervised {
    /// The child's exit status if it has exited, `None` while it runs. On Unix an exited child is
    /// left unreaped, so teardown can still reach what it left running (see
    /// [`crate::platform::ProcessTree::exit_status`]).
    fn exit_status(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        let status = self.tree.exit_status(self.child.as_mut());
        if status.is_err() {
            self.reaped();
        }
        status
    }

    /// End the child's whole tree, then the child, and reap it. Does nothing to a child already
    /// reaped but the tree teardown, which forgets a reaped Unix child's group and on Windows still
    /// ends whatever is left in the job.
    fn terminate(&mut self) -> io::Result<()> {
        self.tree.terminate();
        if !self.reaped {
            self.child.kill()?;
            let _ = self.child.wait();
            self.reaped();
        }
        Ok(())
    }

    /// The child has been waited on — or can no longer be, which is treated the same way: a pid
    /// that cannot be vouched for is not one to signal.
    fn reaped(&mut self) {
        self.reaped = true;
        self.tree.leader_reaped();
    }
}

impl PtySession {
    /// Spawn `spec`'s AI CLI as a daemon-owned session (FR-006). `scrollback_lines` sets the VT
    /// history depth; `initial_size` seeds the grid (falls back to the standard seed).
    ///
    /// Refused if `spec.cwd` does not exist (FR-006c, BUG-012).
    ///
    /// Was `spawn_claude`, and the rename is the substance rather than tidying (feature 026,
    /// T016a): the command and the argv now come from `spec.provider` through the seam, so this
    /// function names no CLI and decides nothing about which one runs.
    pub fn spawn_ai_cli(
        id: SessionId,
        spec: &LaunchSpec,
        scrollback_lines: usize,
        initial_size: Option<(u16, u16)>,
        activity_args: &[std::ffi::OsString],
    ) -> io::Result<Self> {
        ensure_cwd_exists(&spec.cwd)?;
        let mut cmd = CommandBuilder::new(spec.provider.provider().command());
        cmd.cwd(&spec.cwd);
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        for arg in launch_args(spec) {
            cmd.arg(arg);
        }
        // Whatever wires this session's activity reporting, appended after the launch arguments:
        // `claude`'s per-session `--settings` file (contracts/hooks.md §Configuration, T046), or
        // Pi's `-e <component>` (feature 029, FR-012a). Empty when there is nothing to wire — no
        // hook receiver, a declined component, or a provider that needs neither. Which it is was
        // the caller's decision (`DaemonState::activity_launch_for`), not this function's.
        for arg in activity_args {
            cmd.arg(arg);
        }
        Self::spawn(id, cmd, scrollback_lines, initial_size)
    }

    /// Spawn the platform's plain interactive shell in `cwd` as a daemon-owned session
    /// (contracts/shell-process.md). No AI-CLI args apply.
    ///
    /// Refused if `cwd` does not exist (FR-006c, BUG-012).
    pub fn spawn_shell(
        id: SessionId,
        cwd: &std::path::Path,
        env: &[(String, String)],
        scrollback_lines: usize,
        initial_size: Option<(u16, u16)>,
    ) -> io::Result<Self> {
        ensure_cwd_exists(cwd)?;
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
    /// its daemon listener. Shared plumbing for [`Self::spawn_ai_cli`] / [`Self::spawn_shell`].
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

        let mut child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
        // Before anything else: on Windows this is what puts the child in a job object, and every
        // instant between the spawn and this call is one in which a grandchild would escape it.
        let tree = match child.process_id() {
            Some(pid) => crate::platform::ProcessTree::adopt(pid),
            None => {
                // Neither platform's `portable-pty` child can lack a pid; if one ever does, end it
                // rather than hand back a session whose tree nothing could reap.
                let _ = child.kill();
                return Err(io::Error::other(
                    "the PTY child has no process id, so its process tree cannot be reaped",
                ));
            }
        };
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
            child: Mutex::new(Supervised {
                child,
                tree,
                reaped: false,
            }),
            master: Mutex::new(Some(pair.master)),
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
            .as_ref()
            .ok_or_else(|| io::Error::other("pty master already closed"))?
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

    /// Whether the child is still running, via a non-blocking exit check. Authoritative for
    /// liveness (the reader-thread EOF flag is only a hint — a child can close stdout yet linger).
    /// On observing the child gone, caches its exit success bit for [`Self::exit_outcome`].
    ///
    /// It does not reap on Unix: the exited child stays a zombie until [`Self::kill`], so the
    /// descendants it left running are still reachable then (FR-036).
    pub fn is_alive(&self) -> bool {
        match self.child.lock() {
            Ok(mut supervised) => match supervised.exit_status() {
                Ok(Some(status)) => {
                    self.cache_exit(ExitOutcome::from_status(&status));
                    false
                }
                Ok(None) => true,
                // An error means we can no longer track the child; treat it as gone (a crash, so
                // supervision can react) rather than pretend it is alive forever.
                Err(_) => {
                    self.cache_exit(ExitOutcome::crashed("could not be reaped"));
                    false
                }
            },
            Err(_) => false,
        }
    }

    /// Record how the child exited the first time it is observed (idempotent — later reaps never
    /// overwrite the first-seen outcome).
    fn cache_exit(&self, outcome: ExitOutcome) {
        if let Ok(mut e) = self.exit.lock() {
            e.get_or_insert(outcome);
        }
    }

    /// The child's exit outcome once it has exited, or `None` while it is still running. Reaps via
    /// [`Self::is_alive`] if not yet observed, so a caller can poll this directly. Drives the
    /// restart supervision policy (US4, FR-005): a clean exit stops the session, a crash restarts.
    pub fn exit_outcome(&self) -> Option<ExitOutcome> {
        if self.exit.lock().ok().is_some_and(|e| e.is_none()) {
            // Not yet observed — attempt a reap now.
            let _ = self.is_alive();
        }
        self.exit.lock().ok().and_then(|e| e.clone())
    }

    /// Whether the PTY reader has seen EOF (the child's output stream ended). A hint for the framer;
    /// [`Self::is_alive`] is authoritative for process liveness.
    pub fn output_ended(&self) -> bool {
        self.reader_done.load(Ordering::Acquire)
    }

    /// The child's OS process id, if known.
    pub fn pid(&self) -> Option<u32> {
        self.child.lock().ok().and_then(|s| s.child.process_id())
    }

    /// Terminate and reap the child (FR-015a / session close). Terminates the child's whole process
    /// tree first — its process group on Unix, its job object on Windows — so processes the session
    /// started don't orphan (FR-036, T061), then kills and reaps the direct child.
    ///
    /// On Windows `child.kill()` is `portable-pty`'s, whose killer reports `TerminateProcess`
    /// inverted (research R3.4). `Child::kill` discards that result in 0.9.0, which is the only
    /// reason the `?` in [`Supervised::terminate`] is sound there; `tests/windows_process_tree.rs` pins both halves — `Ok`,
    /// and the process really gone — so an upgrade that starts propagating it fails a test rather
    /// than every session close.
    ///
    /// The tree is terminated under the child's lock, so it cannot be racing a reap that would free
    /// the pid it signals through. A second `kill()` — the drop after a session close — finds the
    /// child reaped and signals nothing through its pid.
    pub fn kill(&self) -> io::Result<()> {
        match self.child.lock() {
            Ok(mut supervised) => supervised.terminate(),
            Err(_) => Ok(()),
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Best-effort teardown: kill the child, close the master, then join the reader so no thread
        // outlives the session.
        //
        // The master must close *before* the join. On Unix the reader sees end-of-file as soon as
        // the child's side of the PTY is gone, so the order never mattered there. On ConPTY it sees
        // it only when the pseudoconsole is closed — `conhost` keeps the output pipe open after the
        // child exits — and dropping the master is what closes it (the slave was dropped at spawn).
        // Joining first waited forever. The reader stays running meanwhile, which matters too:
        // before Windows 11 24H2 `ClosePseudoConsole` blocks until pending output is drained.
        let _ = self.kill();
        drop(self.master.get_mut().ok().and_then(Option::take));
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
    fn a_nonzero_exit_is_classified_crashed_and_keeps_the_status() {
        let s = PtySession::spawn(SessionId::new(), sh("exit 3"), 100, None).unwrap();
        // The status, not just the classification: it is the only place the give-up reason can come
        // from, and it is gone the moment this cache drops it (BUG-017).
        assert_eq!(wait_for_exit(&s), ExitOutcome::crashed("exit status 3"));
    }

    #[test]
    fn a_signalled_child_is_named_by_its_signal() {
        let s = PtySession::spawn(SessionId::new(), sh("kill -TERM $$"), 100, None).unwrap();
        let ExitOutcome::Crashed { how } = wait_for_exit(&s) else {
            panic!("a signalled child is not a clean exit");
        };
        // `portable_pty` gives the signal's *description*, not its `SIG…` name — a `kill -TERM`
        // reads `killed by Terminated`. Asserting the prefix rather than that exact word keeps
        // this from depending on libc's wording, while still failing if the signal is reduced to
        // the code 1 the same child also reports (which is what `exit status 1` would say).
        assert!(
            how.starts_with("killed by ") && !how.contains("exit status"),
            "a signal reported as a bare code reads as an ordinary failure; got {how:?}"
        );
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
        assert_eq!(s.exit_outcome(), Some(first.clone()));
        assert_eq!(s.exit_outcome(), Some(ExitOutcome::Clean));
    }

    /// The pid a teardown would signal the process group of.
    fn signalled_pid(session: &PtySession) -> Option<u32> {
        session.child.lock().unwrap().tree.signal_target()
    }

    /// Teardown reaches a live session's group through the child's own pid — the only handle Unix
    /// offers, and a sound one while the child is unreaped, because until then the kernel cannot
    /// give that pid to anything else.
    #[test]
    fn a_live_child_is_signalled_through_its_own_pid() {
        let s = PtySession::spawn(SessionId::new(), sh("sleep 5"), 100, None).unwrap();
        assert!(s.is_alive());
        assert_eq!(signalled_pid(&s), s.pid());
        let _ = s.kill();
    }

    /// A liveness check that sees the child exited must not reap it: reaping is what frees the pid,
    /// and the pid is the only way teardown can still reach the child's group — the descendants it
    /// left running. Unreaped, the child is a zombie holding that pid, so no other process can be
    /// given it and signalling through it stays sound until teardown reaps.
    #[test]
    fn a_child_seen_exited_by_a_liveness_check_keeps_its_pid_until_teardown() {
        let s = PtySession::spawn(SessionId::new(), sh("exit 0"), 100, None).unwrap();
        wait_for_exit(&s);
        assert!(!s.is_alive(), "a later liveness check still sees the exit");
        assert_eq!(signalled_pid(&s), s.pid());
    }

    /// Once teardown has reaped the child, its pid belongs to the kernel again and can be handed to
    /// an unrelated process — which, as a group leader, would have its whole group `SIGKILL`ed by the
    /// session's drop, which kills again. So nothing may be left to signal.
    ///
    /// Reuse itself cannot be provoked here: pids are allocated cyclically up to `pid_max`, and
    /// forcing a particular one needs `CAP_SYS_ADMIN` in a pid namespace. What this pins is the
    /// precondition — no pid is retained past the reap — without which reuse is only a matter of load.
    #[test]
    fn a_child_reaped_by_kill_leaves_no_pid_to_signal() {
        let s = PtySession::spawn(SessionId::new(), sh("sleep 300"), 100, None).unwrap();
        s.kill().unwrap();
        assert_eq!(signalled_pid(&s), None);
    }

    /// Every session close is a `kill()` and then the session's drop, which kills again — by then
    /// on a reaped child. `portable-pty`'s Unix `Child::kill` sends `SIGHUP` to the pid without
    /// checking for that, so it must not be reached: the pid may already be someone else's. Unreused,
    /// the stray signal fails with `ESRCH`, which is what this can see.
    #[test]
    fn a_second_kill_signals_nothing_through_the_reaped_pid() {
        let s = PtySession::spawn(SessionId::new(), sh("sleep 300"), 100, None).unwrap();
        s.kill().unwrap();
        s.kill().unwrap();
    }

    /// Spawn `sh -c "<script>"` after substituting `{pidfile}` with a file the script writes a
    /// grandchild's pid to; return the session and that pid once it has been recorded.
    fn spawn_with_grandchild(script: &str) -> (PtySession, i32, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("grandchild.pid");
        let script = script.replace("{pidfile}", &pidfile.display().to_string());
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
        (session, grandchild, dir)
    }

    /// Whether a process with `pid` exists.
    fn exists(pid: i32) -> bool {
        // SAFETY: `kill(pid, 0)` sends no signal; it only probes whether the process exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// Wait (bounded — reparenting + reaping is not instantaneous) for `pid` to be gone.
    fn assert_gone(pid: i32, what: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while exists(pid) {
            assert!(Instant::now() < deadline, "{what}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// FR-036 / T061: tearing down a session reaps its whole process group, not just the direct
    /// child — a backgrounded grandchild must not orphan.
    #[test]
    fn kill_reaps_the_whole_process_group() {
        // sh backgrounds a `sleep` (a grandchild in the same process group), records its pid, then
        // waits. Killing the group must take the grandchild with it.
        let (session, grandchild, _dir) =
            spawn_with_grandchild("sleep 300 & echo $! > {pidfile} ; wait");
        assert!(
            exists(grandchild),
            "grandchild should be alive before teardown"
        );

        session.kill().unwrap();

        assert_gone(grandchild, "grandchild survived the process-group teardown");
    }

    /// FR-036, and parity with Windows (Principle VI): a grandchild still running after the child
    /// exited *on its own* — and after supervision has noticed the exit — is ended at teardown too.
    /// A Windows job holds it regardless of the child; on Unix the group is reachable only through
    /// the child's pid, which a liveness check that reaped would give up.
    ///
    /// The grandchild ignores `SIGHUP`, as anything started under `nohup` does. One that does not is
    /// already gone by teardown: a session leader's exit hangs up its terminal, and the kernel sends
    /// `SIGHUP` to the terminal's foreground group — which, with no job control, is the whole group.
    #[test]
    fn a_grandchild_outliving_its_exited_parent_dies_at_teardown() {
        // The grandchild records its own pid once it ignores SIGHUP, and the child exits only after
        // that: the child leads the session, so its exit hangs up the terminal, and a grandchild
        // not yet past its `trap` died of it (seen on macOS).
        let (session, grandchild, _dir) = spawn_with_grandchild(
            "sh -c 'trap \"\" HUP; echo $$ > {pidfile}; exec sleep 300' & \
             until [ -s {pidfile} ]; do sleep 0.01; done; exit 0",
        );
        assert_eq!(wait_for_exit(&session), ExitOutcome::Clean);
        assert!(
            exists(grandchild),
            "the grandchild outlives its parent's exit"
        );

        session.kill().unwrap();

        assert_gone(
            grandchild,
            "grandchild of an exited child survived teardown",
        );
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    /// How long the grandchild may take to go once its session is killed (E5.1).
    const REAP_BUDGET: Duration = Duration::from_secs(5);

    /// FR-020 / E5.1, the Windows counterpart of `kill_reaps_the_whole_process_group`: a process the
    /// session's shell started must not outlive the session.
    #[test]
    fn kill_reaps_grandchild() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("grandchild.pid");
        // PowerShell starts `ping` as its own child (the session's grandchild), records its pid, then
        // waits on it. `-n 300` bounds a leak to five minutes if the test fails before cleanup.
        let script = format!(
            "$p = Start-Process ping -ArgumentList '-n','300','127.0.0.1' -NoNewWindow -PassThru; \
             Set-Content -Encoding ascii -Path '{}' -Value $p.Id; Wait-Process -Id $p.Id",
            pidfile.display()
        );
        let mut cmd = CommandBuilder::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        let session = PtySession::spawn(SessionId::new(), cmd, 100, None).unwrap();

        let deadline = Instant::now() + Duration::from_secs(30);
        let grandchild: u32 = loop {
            if let Ok(text) = std::fs::read_to_string(&pidfile) {
                if let Ok(pid) = text.trim().parse() {
                    break pid;
                }
            }
            assert!(Instant::now() < deadline, "grandchild pid never recorded");
            std::thread::sleep(Duration::from_millis(50));
        };
        // SAFETY: a plain handle open on a pid; it is closed below on every path that reaches it.
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, 0, grandchild) };
        assert!(
            !handle.is_null(),
            "grandchild {grandchild} should be alive before teardown: {}",
            io::Error::last_os_error()
        );

        session.kill().unwrap();

        // SAFETY: `handle` is a live process handle opened with SYNCHRONIZE and TERMINATE above.
        let reaped = unsafe { WaitForSingleObject(handle, REAP_BUDGET.as_millis() as u32) };
        if reaped != WAIT_OBJECT_0 {
            // Do not leave the survivor behind for the rest of the suite.
            // SAFETY: as above.
            unsafe { TerminateProcess(handle, 1) };
        }
        // SAFETY: closed exactly once.
        unsafe { CloseHandle(handle) };
        assert_eq!(
            reaped, WAIT_OBJECT_0,
            "grandchild {grandchild} survived the session kill by {REAP_BUDGET:?}"
        );
    }
}
