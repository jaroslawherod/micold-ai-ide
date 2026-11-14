//! Background process hygiene (feature 030, FR-005, FR-009, FR-024) and the bounded runner
//! (feature 011, promoted here by feature 034, research R6).
//!
//! - [`no_window`]: a GUI-subsystem app that spawns a console program (`git`, `powershell`, `docker`)
//!   gets a console window flashed up for every call unless the spawn says otherwise.
//! - [`announce_running`]: the installer asks the user to close the app before it replaces the
//!   exes. It finds a running app through a named mutex, which the client holds for its whole
//!   lifetime. The daemon does not: nobody can close a windowless process, so setup stops it itself.
//!
//! Both are no-ops off Windows, so call sites stay free of `cfg`.
//!
//! - [`run_bounded`]: run a child under a hard time limit and kill its whole process tree when it
//!   ends — the environment-include shell (feature 011) and the GitHub CLI (feature 034) both need
//!   exactly this, and the kill is a cross-platform subtlety that was debugged once (011 BUG-003).
//! - [`StreamingChild`]: run a child with no time limit, hand over its output as it prints, and end
//!   its whole process tree from another thread (feature 1289, the setup command). Same tree kill.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

/// Keep a spawned console program from opening a console window. Returns the command for chaining.
pub fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}

/// The named mutex a running app held for the Windows installer's `AppMutex` check (Windows support
/// was dropped in #1897).
pub const APP_MUTEX_NAME: &str = "Local\\MicoldAIIDE";

/// Proof that this process announced itself to the installer. The announcement ends on drop.
pub struct RunningMarker {}

/// Announce that an app process is running, for the installer's `AppMutex` check. `None` off
/// Windows, and on Windows when the mutex cannot be created -- the app still runs, the installer
/// just cannot see it.
pub fn announce_running() -> Option<RunningMarker> {
    None
}

/// How long to poll a spawned child's exit status before considering it hung (feature 011,
/// research R2).
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// How a bounded subprocess run concluded — kept distinct from a caller's own outcome (such as
/// `env_include::EnvIncludeOutcome`) so the caller decides its public category from an unambiguous
/// fact (did it exit, or did we have to kill it) rather than inferring "was this a timeout?" from
/// elapsed time, which would be racy right at the timeout boundary.
#[derive(Debug)]
pub enum RunOutcome {
    /// The process exited on its own within `timeout`.
    Exited {
        /// The exit status, or -1 when there is none (killed by a signal).
        code: i32,
        /// Everything the process wrote to stdout.
        stdout: Vec<u8>,
        /// Everything the process wrote to stderr, lossily decoded.
        stderr: String,
    },
    /// The process was still running at `timeout` and was killed.
    TimedOut {
        /// What it had written to stderr by then.
        stderr: String,
    },
    /// The process could not even be spawned (e.g. the interpreter binary is missing).
    SpawnFailed(String),
}

/// Kill every process in `pid`'s process group (Unix only — the child is spawned with
/// `process_group(0)` by [`TreeKill::prepare`], so the group id equals the child's own pid) via a
/// direct `kill(2)` syscall. Deliberately NOT implemented by spawning the `kill(1)` *binary* as a
/// subprocess — that proved unreliable in sandboxed environments during development (the spawned
/// `kill` process reported success without the target group actually dying), whereas the direct
/// syscall from this same process is unambiguous.
#[cfg(unix)]
fn kill_process_group(pid: u32) {
    // A descendant can leave the group (mise starts rustup with its own process group, #1642), and
    // once its parent dies it is reparented and no longer found from `pid`: list them first.
    let mut found = vec![pid];
    let mut next = 0;
    while let Some(&parent) = found.get(next) {
        next += 1;
        // ponytail: `pgrep -P` per node, a /proc walk or libproc if trees get large; no pgrep: the
        // group kill alone.
        let children = Command::new("pgrep")
            .args(["-P", &parent.to_string()])
            .stderr(Stdio::null())
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        found.extend(
            children
                .split_whitespace()
                .filter_map(|p| p.parse::<u32>().ok()),
        );
    }
    // Safety: `kill(2)` with a negative pid signals the process group; passing an invalid/already-
    // reaped group id is a documented, safe no-op (returns -1/ESRCH) rather than undefined
    // behavior. The same call with a positive pid signals one process, and is as safe.
    unsafe {
        libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
        for descendant in &found[1..] {
            libc::kill(*descendant as libc::pid_t, libc::SIGKILL);
        }
    }
}

/// What ends a spawned child together with everything it started: its process group on Unix, its
/// job on Windows (011 BUG-003). [`run_bounded`] and [`StreamingChild`] share it.
struct TreeKill {
    #[cfg(unix)]
    pid: u32,
}

impl TreeKill {
    /// Make `cmd` spawn as the head of a tree this can kill (its own process group on Unix).
    fn prepare(cmd: &mut Command) {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
    }

    /// The tree of `child`, captured as soon as it exists.
    fn capture(child: &std::process::Child) -> Self {
        Self {
            #[cfg(unix)]
            pid: child.id(),
        }
    }

    /// Kill the whole tree from any thread, without the child (a [`Terminator`]). On Windows
    /// without a job this reaches nothing; [`Self::kill`] covers that case.
    fn kill_tree(&self) {
        #[cfg(unix)]
        kill_process_group(self.pid);
    }

    /// Whether the tree's head has exited and is a zombie not yet reaped (Unix: `waitid` with
    /// `WNOWAIT` looks without reaping). Elsewhere false: the job handle cannot name another
    /// process, so the tree is killed after the reap instead.
    fn leader_exited(&self) -> bool {
        #[cfg(unix)]
        {
            // SAFETY: an all-zero `siginfo_t` is valid; `waitid` writes it, and `WNOWAIT` leaves
            // the child waitable for `try_wait`.
            unsafe {
                let mut info: libc::siginfo_t = std::mem::zeroed();
                let pid = libc::id_t::from(self.pid);
                let looked = libc::waitid(
                    libc::P_PID,
                    pid,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                );
                // A failed look (ECHILD: already reaped) counts as exited, so `try_wait` gets to
                // say so instead of the run loop polling forever.
                looked != 0 || info.si_pid() != 0
            }
        }
    }

    /// Kill the whole tree, then `child` itself, for the case where the job could not be created.
    fn kill(&self, child: &mut std::process::Child) {
        self.kill_tree();
        #[cfg(unix)]
        let _ = child;
    }
}

/// How long after the group kill the readers get to reach EOF before `run_bounded` returns what
/// they have. Covers a descendant that left the group and still holds a pipe.
const READER_GRACE: Duration = Duration::from_millis(500);

/// How long past its deadline a bounded run may still return: one poll, then [`READER_GRACE`] for
/// the pipes. A caller that waits for a run by the same deadline gives the run this much less.
pub const RUN_OVERRUN: Duration = POLL_INTERVAL.saturating_add(READER_GRACE);

/// Run `cmd`, waiting up to `timeout` for it to exit (research R2: `spawn()` + `try_wait()`
/// poll, not the blocking `.output()`). On Unix, `cmd` is spawned in its own process group so
/// that on timeout — or even after a natural exit — the ENTIRE group (not just the top-level
/// process) is killed before reading its piped stdout/stderr: a sourced rc file MAY background a
/// process (an agent daemon, a version-manager helper); if such a grandchild inherits the pipe
/// and outlives its parent, reading to EOF would otherwise block forever (this exact deadlock was
/// observed with a `sleep`-under-`source`-under-command-substitution during development).
///
/// stdout and stderr are drained on two reader threads **while** the child runs (feature 034,
/// research R6): a child that writes more than the OS pipe buffer (64 KiB on Linux, less on macOS
/// and Windows) would otherwise block on its write, never exit, and be killed as a timeout. The
/// readers are joined after the group kill above, which brings them to EOF, for at most
/// [`READER_GRACE`]: a descendant that left the group (`setsid`) and holds a pipe never sends EOF,
/// and the call returns what was read instead of waiting on it.
pub fn run_bounded(cmd: Command, timeout: Duration) -> RunOutcome {
    run_bounded_with_input(cmd, timeout, None)
}

/// [`run_bounded`] with `input` written to the child's stdin on its own thread (git answers as it
/// reads, so a reply that filled its pipe while we were still writing would stop both sides). A
/// child that exits without reading it all is not an error. `None`: the caller's stdin setting
/// stands.
pub fn run_bounded_with_input(
    mut cmd: Command,
    timeout: Duration,
    input: Option<Vec<u8>>,
) -> RunOutcome {
    use std::io::Write;
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    }
    TreeKill::prepare(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => return RunOutcome::SpawnFailed(err.to_string()),
    };
    if let (Some(input), Some(mut pipe)) = (input, child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = pipe.write_all(&input);
        });
    }
    let stdout_reader = child.stdout.take().map(drain);
    let stderr_reader = child.stderr.take().map(drain);
    let tree = TreeKill::capture(&child);

    let start = Instant::now();
    // `Err` holds a failed wait. The group is still killed and the readers still joined below,
    // so nothing outlives the call.
    let waited: Result<bool, std::io::Error> = loop {
        match child.try_wait() {
            Ok(Some(_)) => break Ok(false),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    break Ok(true);
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(err) => break Err(err),
        }
    };

    // Kill the whole group unconditionally (whether it exited on its own or timed out) so no
    // orphaned grandchild can keep the pipes open — see the doc comment above.
    tree.kill(&mut child);

    // Bounded: a descendant that left the group (`setsid`) still holds the pipes, so EOF may
    // never come. Take what was read once the grace period is up and leave the readers behind.
    let reader_deadline = Instant::now() + READER_GRACE;
    let stdout = joined(stdout_reader, reader_deadline);
    let stderr = String::from_utf8_lossy(&joined(stderr_reader, reader_deadline)).into_owned();

    let timed_out = match waited {
        Ok(timed_out) => timed_out,
        Err(err) => {
            // Reap it: the group is dead, so this returns at once, and no zombie is left behind.
            let _ = child.wait();
            return RunOutcome::SpawnFailed(err.to_string());
        }
    };
    if timed_out {
        RunOutcome::TimedOut { stderr }
    } else {
        match child.wait() {
            Ok(status) => RunOutcome::Exited {
                code: status.code().unwrap_or(-1),
                stdout,
                stderr,
            },
            Err(err) => RunOutcome::SpawnFailed(err.to_string()),
        }
    }
}

/// A [`drain`] thread and what it has read so far.
struct Reader {
    handle: std::thread::JoinHandle<()>,
    bytes: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

/// Read `pipe` to EOF on its own thread, so the child never blocks on a full pipe.
fn drain(pipe: impl Read + Send + 'static) -> Reader {
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&bytes);
    let handle = std::thread::spawn(move || {
        read_chunks(pipe, |chunk| {
            sink.lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(chunk);
            true
        });
    });
    Reader { handle, bytes }
}

/// Hand each chunk `pipe` reads to `each` until end of file, a read error or `each` returning
/// false. A read interrupted by a signal (`EINTR`) is retried, not taken for end of file.
fn read_chunks(mut pipe: impl Read, mut each: impl FnMut(&[u8]) -> bool) {
    let mut chunk = [0u8; 8192];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if !each(&chunk[..n]) {
                    break;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
}

/// What a [`drain`] thread read: everything at EOF, else what it had by `deadline` (the thread is
/// left to end on its own). Nothing when there was no pipe.
fn joined(reader: Option<Reader>, deadline: Instant) -> Vec<u8> {
    let Some(reader) = reader else {
        return Vec::new();
    };
    while !reader.handle.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut bytes = reader.bytes.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *bytes)
}

/// Which of a child's output pipes a chunk came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// How a [`StreamingChild`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitKind {
    /// It exited with this status.
    Code(i32),
    /// A signal ended it (Unix).
    Signal(i32),
    /// Waiting for it failed; the tree was killed.
    WaitFailed(String),
}

/// What a [`StreamingChild`] and its [`Terminator`]s share.
struct Shared {
    tree: TreeKill,
    /// Set once the child is reaped, under the lock a terminate takes: after that its group id
    /// could name someone else's processes, so a late terminate sends nothing.
    reaped: Mutex<bool>,
    /// A terminate was asked for; the run loop also kills the child itself (Windows without a job).
    requested: AtomicBool,
}

/// A child whose stdout and stderr are handed over as they are read, with no input and no time
/// limit, and whose whole process tree a [`Terminator`] ends from another thread (feature 1289,
/// research R6: the setup command). The tree kill is the one [`run_bounded`] uses.
pub struct StreamingChild {
    child: std::process::Child,
    shared: Arc<Shared>,
}

/// A handle that ends a [`StreamingChild`]'s whole process tree from any thread.
#[derive(Clone)]
pub struct Terminator(Arc<Shared>);

impl Terminator {
    /// Kill the child and everything it started. Does nothing once the child has been reaped.
    pub fn terminate(&self) {
        self.0.requested.store(true, Ordering::SeqCst);
        let reaped = self.0.reaped.lock().unwrap_or_else(|e| e.into_inner());
        if !*reaped {
            self.0.tree.kill_tree();
        }
    }
}

impl StreamingChild {
    /// Spawn `cmd` with stdin at end of file (null) and stdout and stderr piped, at the head of its
    /// own process tree. `Err` when it cannot be spawned (a missing program, a missing folder).
    pub fn spawn(mut cmd: Command) -> std::io::Result<Self> {
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        TreeKill::prepare(&mut cmd);
        let child = cmd.spawn()?;
        let tree = TreeKill::capture(&child);
        Ok(Self {
            child,
            shared: Arc::new(Shared {
                tree,
                reaped: Mutex::new(false),
                requested: AtomicBool::new(false),
            }),
        })
    }

    /// A handle that ends this child's tree from another thread.
    #[must_use]
    pub fn terminator(&self) -> Terminator {
        Terminator(Arc::clone(&self.shared))
    }

    /// Hand every chunk the child prints to `on_chunk`, in the order each stream printed it, until
    /// the child exits; then kill what is left of its tree and wait at most [`READER_GRACE`] for
    /// the pipes to reach end of file (a descendant that left the tree may hold them).
    pub fn run(mut self, mut on_chunk: impl FnMut(Stream, &[u8])) -> ExitKind {
        let (tx, rx) = mpsc::channel();
        let mut open = 0;
        if let Some(pipe) = self.child.stdout.take() {
            stream_to(pipe, Stream::Stdout, tx.clone());
            open += 1;
        }
        if let Some(pipe) = self.child.stderr.take() {
            stream_to(pipe, Stream::Stderr, tx.clone());
            open += 1;
        }
        drop(tx);
        let mut deliver = |message: Option<(Stream, Vec<u8>)>, open: &mut usize| match message {
            Some((stream, bytes)) => on_chunk(stream, &bytes),
            None => *open -= 1,
        };

        let waited = loop {
            if open > 0 {
                if let Ok(message) = rx.recv_timeout(POLL_INTERVAL) {
                    deliver(message, &mut open);
                }
            } else {
                std::thread::sleep(POLL_INTERVAL);
            }
            if self.shared.requested.load(Ordering::SeqCst) {
                self.shared.tree.kill(&mut self.child);
            }
            let mut reaped = self.shared.reaped.lock().unwrap_or_else(|e| e.into_inner());
            // As in `run_bounded`: whatever the child left running would hold the pipes open, so
            // the tree is killed once the child exits, and before it is reaped: until then its
            // pid, and so its group id, cannot name anyone else's processes.
            let exited = self.shared.tree.leader_exited();
            if exited {
                self.shared.tree.kill_tree();
            }
            #[cfg(test)]
            tests::after_look(exited);
            // Unix reaps only a child the look saw exited, after the kill above: one that exits
            // between the look and the reap would be reaped with its tree still running (#1856).
            // It is seen on the next pass instead.
            if !exited {
                continue;
            }
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    *reaped = true;
                    break Ok(status);
                }
                Ok(None) => {}
                Err(err) => {
                    // Not reaped: the tree is killed, then the child is reaped, under the lock.
                    self.shared.tree.kill(&mut self.child);
                    let _ = self.child.wait();
                    *reaped = true;
                    break Err(err);
                }
            }
        };

        let deadline = Instant::now() + READER_GRACE;
        while open > 0 {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(message) => deliver(message, &mut open),
                Err(_) => break,
            }
        }

        match waited {
            Ok(status) => exit_kind(status),
            Err(err) => ExitKind::WaitFailed(err.to_string()),
        }
    }
}

impl Drop for StreamingChild {
    /// Dropped without [`Self::run`] (or with `run` unwound): the tree is killed and the child
    /// reaped, so neither a process nor a zombie outlives it.
    fn drop(&mut self) {
        let mut reaped = self.shared.reaped.lock().unwrap_or_else(|e| e.into_inner());
        if !*reaped {
            self.shared.tree.kill(&mut self.child);
            let _ = self.child.kill();
            let _ = self.child.wait();
            *reaped = true;
        }
    }
}

/// Send what `pipe` reads to `tx` as it arrives, then `None` at end of file, on its own thread.
fn stream_to(
    pipe: impl Read + Send + 'static,
    stream: Stream,
    tx: mpsc::Sender<Option<(Stream, Vec<u8>)>>,
) {
    std::thread::spawn(move || {
        read_chunks(pipe, |chunk| {
            tx.send(Some((stream, chunk.to_vec()))).is_ok()
        });
        let _ = tx.send(None);
    });
}

fn exit_kind(status: std::process::ExitStatus) -> ExitKind {
    if let Some(code) = status.code() {
        return ExitKind::Code(code);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return ExitKind::Signal(signal);
        }
    }
    ExitKind::WaitFailed(format!("no exit status: {status}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pipe whose reads are interrupted by a signal before each chunk (EINTR).
    struct Interrupted(Vec<&'static [u8]>, bool);

    impl Read for Interrupted {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.1 = !self.1;
            if self.1 {
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            let Some(chunk) = self.0.pop() else {
                return Ok(0);
            };
            buf[..chunk.len()].copy_from_slice(chunk);
            Ok(chunk.len())
        }
    }

    #[test]
    fn an_interrupted_read_is_retried_not_taken_for_end_of_file() {
        let pipe = || Interrupted(vec![b"two", b"one "], false);
        let deadline = Instant::now() + Duration::from_secs(5);
        assert_eq!(joined(Some(drain(pipe())), deadline), b"one two");

        let (tx, rx) = mpsc::channel();
        stream_to(pipe(), Stream::Stdout, tx);
        let got: Vec<u8> = rx
            .iter()
            .map_while(|message| message)
            .flat_map(|(_, bytes)| bytes)
            .collect();
        assert_eq!(got, b"one two");
    }

    /// #1642: mise starts rustup in a process group of its own, so the group kill alone left the
    /// stopped setup's downloads running into the next run. A descendant in another group dies too.
    #[cfg(unix)]
    #[test]
    fn terminate_also_ends_a_descendant_in_its_own_process_group() {
        let mut cmd = Command::new("perl");
        // The forked `sleep` moves to a group of its own and the parent prints its pid.
        cmd.args([
            "-e",
            "$|=1; my $p=fork; if(!$p){setpgrp(0,0); exec q(sleep),q(300)} print qq($p\\n); waitpid($p,0)",
        ]);
        let child = StreamingChild::spawn(cmd).unwrap();
        let terminator = child.terminator();
        let (tx, rx) = mpsc::channel();
        let run = std::thread::spawn(move || {
            child.run(|_, bytes| {
                let _ = tx.send(String::from_utf8_lossy(bytes).trim().to_string());
            })
        });
        let pid: libc::pid_t = std::iter::from_fn(|| rx.recv_timeout(Duration::from_secs(10)).ok())
            .find_map(|chunk| chunk.parse().ok())
            .expect("perl prints the sleep's pid");
        terminator.terminate();
        run.join().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        // SAFETY: signal 0 only checks that the process exists.
        while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        // SAFETY: as above.
        let alive = unsafe { libc::kill(pid, 0) } == 0;
        if alive {
            // SAFETY: clean up the leaked sleep.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        assert!(!alive, "the descendant {pid} outlived the tree kill");
    }

    type Hook = Box<dyn Fn(bool)>;

    thread_local! {
        static AFTER_LOOK: std::cell::RefCell<Option<Hook>> =
            const { std::cell::RefCell::new(None) };
    }

    /// Test seam: runs on the run loop's thread between the look at the child and the reap.
    pub(super) fn after_look(exited: bool) {
        AFTER_LOOK.with(|hook| {
            if let Some(hook) = hook.borrow().as_ref() {
                hook(exited);
            }
        });
    }

    /// #1856: a child that exits between the loop's look and its reap was reaped with its tree
    /// still running. The hook holds that gap open until the child is gone.
    #[cfg(unix)]
    #[test]
    fn a_child_exiting_between_the_look_and_the_reap_still_loses_its_tree() {
        AFTER_LOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(|exited| {
                if !exited {
                    std::thread::sleep(Duration::from_millis(600));
                }
            }));
        });
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 300 >/dev/null 2>&1 & echo $!; sleep 0.3"]);
        let mut out = String::new();
        let exit = StreamingChild::spawn(cmd)
            .unwrap()
            .run(|_, bytes| out.push_str(&String::from_utf8_lossy(bytes)));
        AFTER_LOOK.with(|hook| *hook.borrow_mut() = None);
        assert_eq!(exit, ExitKind::Code(0));
        let pid: libc::pid_t = out.trim().parse().expect("the sleep's pid");
        let deadline = Instant::now() + Duration::from_secs(5);
        // SAFETY: signal 0 only checks that the process exists.
        while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        // SAFETY: as above.
        let alive = unsafe { libc::kill(pid, 0) } == 0;
        if alive {
            // SAFETY: clean up the leaked sleep.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        assert!(!alive, "the sleep {pid} outlived the run");
    }

    #[cfg(unix)]
    #[test]
    fn no_window_is_noop_elsewhere() {
        let status = no_window(&mut Command::new("true"))
            .status()
            .expect("spawn `true`");

        assert!(
            status.success(),
            "`true` must still run and exit 0, got {status:?}"
        );
    }
}
