//! The three Windows behaviours feature 010's T083 named and nothing exercised (T144, BUG-008 —
//! FR-036, Principle VI, plan Risk 3).
//!
//! Every PTY test in this crate is `#![cfg(unix)]`, and the Windows CI column ran none of them — it
//! compiled the daemon and stopped. So a green Windows build said nothing about the three things
//! research R3 flagged as Windows-specific and untested:
//!
//! 1. **Process-tree teardown.** `portable-pty`'s Windows `kill()` is a bare `TerminateProcess` on the
//!    direct child; a grandchild the session spawned outlives it unless the daemon put the child in a
//!    job object at spawn.
//! 2. **The `0x03` interrupt.** A daemon has no console, so `GenerateConsoleCtrlEvent` cannot reach a
//!    session; writing ETX to the ConPTY input is the only path. Research R3.3 names the trap: add
//!    `CREATE_NEW_PROCESS_GROUP` anywhere and Ctrl-C stops being delivered, *invisibly* — `cmd.exe`
//!    still appears to obey because `ReadConsole` aborts on ETX by itself. So the program interrupted
//!    here is `ping.exe`, which obeys only a real console control event.
//! 3. **The inverted `kill()` result.** `portable-pty` 0.9.0's Windows killer returns `Err` when
//!    `TerminateProcess` *succeeds*. `Child::kill` happens to discard it today, which makes the
//!    daemon's `child.kill()?` harmless — until an upgrade propagates it, at which point every
//!    successful kill reports failure and skips the reap. The observable pinned here is the pair: the
//!    call returns `Ok`, *and* the process is gone.
//!
//! Plus one this file found by existing: dropping a live session must return. On ConPTY the reader
//! only sees end-of-file once the pseudoconsole is closed — conhost holds the output pipe open after
//! the child exits — so a teardown that joins the reader before closing the master waits forever.
//!
//! Every test runs its body under [`within`], because the failure most likely here is a hang, and
//! `cargo test` has no timeout: a hung test is a CI job GitHub cancels hours later, not a red line.

#![cfg(windows)]

use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::session::SessionId;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
};

/// Long enough for a loaded runner; short enough that a hang is reported, not waited out.
const LIMIT: Duration = Duration::from_secs(60);

/// Run `body` on its own thread and fail if it has not returned within `limit`.
///
/// A panic inside `body` still prints its own message when it happens, so an assertion that fails and
/// *then* hangs in the unwinding drop is reported twice — once as itself, once as the hang — rather
/// than only as the hang.
fn within<T: Send + 'static>(
    limit: Duration,
    what: &str,
    body: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(std::panic::catch_unwind(AssertUnwindSafe(body)));
    });
    match rx.recv_timeout(limit) {
        Ok(Ok(value)) => value,
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(_) => panic!("{what} did not return within {limit:?} — a hang, not a slow run"),
    }
}

fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    cond()
}

/// The visible screen as one string.
fn visible_text(session: &PtySession) -> String {
    let term = session.term().lock();
    let grid = term.grid();
    let (cols, rows) = (grid.columns(), grid.screen_lines());
    let mut out = String::new();
    for line in 0..rows {
        for col in 0..cols {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

/// A handle to a process, opened by pid while it is known to be alive.
///
/// Holding the handle is what makes the later "has it exited?" question safe: a pid can be reused the
/// moment its process ends, but a process object cannot be recycled while a handle to it is open.
struct Process(HANDLE);

impl Process {
    fn open(pid: u32) -> Self {
        // Safety: plain FFI; a failure returns a null handle, which is asserted against.
        let handle = unsafe {
            OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            )
        };
        assert!(
            !handle.is_null(),
            "could not open process {pid}: {}",
            std::io::Error::last_os_error()
        );
        Self(handle)
    }

    fn exits_within(&self, limit: Duration) -> bool {
        // Safety: `self.0` is a live handle this value owns, opened with `PROCESS_SYNCHRONIZE`.
        unsafe { WaitForSingleObject(self.0, limit.as_millis() as u32) == WAIT_OBJECT_0 }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // Safety: closing a handle this value owns, exactly once.
        unsafe { CloseHandle(self.0) };
    }
}

/// `powershell.exe -Command <script>`, with the script's own quoting kept to single quotes so it
/// survives `portable-pty`'s command-line quoting unchanged.
fn powershell(script: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    cmd
}

fn ps_literal(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

/// `ping.exe` to loopback for about five minutes — far longer than any test here should take.
fn long_ping() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("ping.exe");
    cmd.args(["-n", "300", "127.0.0.1"]);
    cmd
}

/// Wait for a pid written by the session's own script.
fn read_pid(path: &Path) -> u32 {
    let mut pid = None;
    assert!(
        wait_until(Duration::from_secs(30), || {
            pid = std::fs::read_to_string(path)
                .ok()
                .and_then(|text| text.trim().parse().ok());
            pid.is_some()
        }),
        "the session never recorded its grandchild's pid in {}",
        path.display()
    );
    pid.unwrap()
}

/// Behaviour 1: closing a session reaps what the session started, not only the session's own process.
///
/// PowerShell starts `ping.exe` as its child (so `ping` is the *session's* grandchild), records its
/// pid, and waits on it. Before job objects this was the Windows twin of the orphan
/// `kill_reaps_the_whole_process_group` rules out on Unix: `TerminateProcess` took PowerShell, and
/// `ping` carried on for five minutes with nobody holding it.
#[test]
fn killing_a_session_takes_its_grandchildren_with_it() {
    within(LIMIT, "killing a session with a grandchild", || {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("grandchild.pid");
        let script = format!(
            "$p = Start-Process -FilePath ping.exe -ArgumentList '-n','300','127.0.0.1' \
             -NoNewWindow -PassThru; Set-Content -Path {} -Value $p.Id -Encoding ascii; \
             Wait-Process -Id $p.Id",
            ps_literal(&pidfile)
        );
        let session = PtySession::spawn(SessionId::new(), powershell(&script), 100, None).unwrap();

        let grandchild = Process::open(read_pid(&pidfile));
        assert!(
            !grandchild.exits_within(Duration::ZERO),
            "the grandchild should be running before the session is killed"
        );

        session.kill().unwrap();

        assert!(
            grandchild.exits_within(Duration::from_secs(10)),
            "the grandchild outlived its session — the tree was not torn down"
        );
        drop(session);
    });
}

/// Behaviour 2: `0x03` written to the PTY interrupts the foreground program.
///
/// `ping.exe` is spawned directly as the session's process, and it stops early only on a real
/// `CTRL_C_EVENT` — which ConPTY raises from ETX only when the program shares the pseudoconsole's
/// process group. This is the test that goes red if a `CREATE_NEW_PROCESS_GROUP` ever appears on the
/// spawn path, where one against `cmd.exe` would stay green.
#[test]
fn ctrl_c_written_to_the_pty_interrupts_a_program_that_is_not_cmd() {
    within(LIMIT, "interrupting ping.exe", || {
        let session = PtySession::spawn(SessionId::new(), long_ping(), 100, None).unwrap();

        // Interrupt only once `ping` is demonstrably running and attached: an ETX that ConPTY reads
        // before the program exists has nobody to deliver an event to. The address is in `ping`'s
        // header line in every locale, unlike the word before it.
        assert!(
            wait_until(Duration::from_secs(30), || visible_text(&session)
                .contains("127.0.0.1")),
            "ping never printed its header; screen:\n{}",
            visible_text(&session)
        );
        assert!(session.is_alive(), "ping exited before it was interrupted");

        session.write_input(&[0x03]).unwrap();

        assert!(
            wait_until(Duration::from_secs(10), || !session.is_alive()),
            "ping kept running after 0x03 — Ctrl-C was not delivered through the pseudoconsole"
        );
        drop(session);
    });
}

/// Behaviour 3: `kill()` reports success, and the success is true.
///
/// Both halves, because each alone is satisfied by a broken build. `Ok` with the process still running
/// is a kill that did nothing; the process gone with an `Err` is `portable-pty`'s inversion reaching
/// the daemon, which returns before reaping. The second call is the case the inversion bites hardest
/// — `TerminateProcess` on an exited process fails, which the inverted killer reports as success —
/// so killing a session twice must stay `Ok` on its own merits, not by accident.
#[test]
fn kill_reports_success_and_the_process_is_really_gone() {
    within(LIMIT, "killing a session twice", || {
        let session = PtySession::spawn(SessionId::new(), long_ping(), 100, None).unwrap();
        let pid = session.pid().expect("a spawned session has a pid");
        let child = Process::open(pid);
        assert!(
            !child.exits_within(Duration::ZERO),
            "ping should be running before the kill"
        );

        let first = session.kill();
        assert!(
            first.is_ok(),
            "killing a live session reported failure: {first:?}"
        );
        assert!(
            child.exits_within(Duration::from_secs(10)),
            "kill() returned Ok but the session's process is still running"
        );
        assert!(
            !session.is_alive(),
            "the session still reports its process alive"
        );

        let second = session.kill();
        assert!(
            second.is_ok(),
            "killing an already-dead session reported failure: {second:?}"
        );
        drop(session);
    });
}

/// Dropping a session whose process is still running returns, and takes the process with it.
///
/// This is the teardown every session close and every daemon shutdown performs. On Unix the reader
/// sees end-of-file as soon as the child's side of the PTY closes; on ConPTY it sees it only when the
/// pseudoconsole itself is closed, so the order inside the drop decides between returning and hanging.
#[test]
fn dropping_a_live_session_returns_and_ends_its_process() {
    within(LIMIT, "dropping a live session", || {
        let session = PtySession::spawn(SessionId::new(), long_ping(), 100, None).unwrap();
        let child = Process::open(session.pid().expect("a spawned session has a pid"));
        drop(session);
        assert!(
            child.exits_within(Duration::from_secs(10)),
            "dropping the session left its process running"
        );
    });
}
