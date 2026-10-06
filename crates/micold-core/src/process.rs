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

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use crate::win_job::JobHandle;

/// `CREATE_NO_WINDOW` process creation flag: the child gets no console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Keep a spawned console program from opening a console window. Returns the command for chaining.
#[cfg(windows)]
pub fn no_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

/// Keep a spawned console program from opening a console window. Returns the command for chaining.
#[cfg(not(windows))]
pub fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}

/// The named mutex a running app holds. `packaging/windows/micold-ai-ide.iss` names the same one in
/// `AppMutex`; `Local\` scopes it to the signed-in session, so another account's app does not block
/// this account's install.
pub const APP_MUTEX_NAME: &str = "Local\\MicoldAIIDE";

/// Proof that this process announced itself to the installer. The announcement ends on drop.
pub struct RunningMarker {
    #[cfg(windows)]
    mutex: windows_sys::Win32::Foundation::HANDLE,
}

/// Announce that an app process is running, for the installer's `AppMutex` check. `None` off
/// Windows, and on Windows when the mutex cannot be created -- the app still runs, the installer
/// just cannot see it.
#[cfg(windows)]
pub fn announce_running() -> Option<RunningMarker> {
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name: Vec<u16> = APP_MUTEX_NAME.encode_utf16().chain(Some(0)).collect();
    // SAFETY: `name` is NUL-terminated and outlives the call; null security attributes are the
    // documented default. Opening an existing mutex (another client instance holds it) also succeeds and
    // returns a handle of our own, which is what keeps the name alive while either process runs.
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if mutex.is_null() {
        return None;
    }
    Some(RunningMarker { mutex })
}

/// Announce that an app process is running, for the installer's `AppMutex` check. `None` off
/// Windows, and on Windows when the mutex cannot be created -- the app still runs, the installer
/// just cannot see it.
#[cfg(not(windows))]
pub fn announce_running() -> Option<RunningMarker> {
    None
}

#[cfg(windows)]
impl Drop for RunningMarker {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful `CreateMutexW` and is closed only here. The
        // name disappears with its last handle.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.mutex);
        }
    }
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

/// Kill every process in `pid`'s process group (Unix only — `cmd` is spawned with
/// `process_group(0)` below, so the group id equals the child's own pid) via a direct `kill(2)`
/// syscall. Deliberately NOT implemented by spawning the `kill(1)` *binary* as a subprocess —
/// that proved unreliable in sandboxed environments during development (the spawned `kill`
/// process reported success without the target group actually dying), whereas the direct syscall
/// from this same process is unambiguous.
#[cfg(unix)]
fn kill_process_group(pid: u32) {
    // Safety: `kill(2)` with a negative pid signals the process group; passing an invalid/already-
    // reaped group id is a documented, safe no-op (returns -1/ESRCH) rather than undefined
    // behavior.
    unsafe {
        libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
    }
}

/// How long after the group kill the readers get to reach EOF before `run_bounded` returns what
/// they have. Covers a descendant that left the group and still holds a pipe.
const READER_GRACE: Duration = Duration::from_millis(500);

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
pub fn run_bounded(mut cmd: Command, timeout: Duration) -> RunOutcome {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => return RunOutcome::SpawnFailed(err.to_string()),
    };
    let stdout_reader = child.stdout.take().map(drain);
    let stderr_reader = child.stderr.take().map(drain);
    // Only the Unix kill needs the pid; binding it unconditionally warns on Windows, and this
    // crate is built with warnings denied.
    #[cfg(unix)]
    let pid = child.id();
    // Windows' counterpart to the process group above, assigned as soon as the child exists.
    #[cfg(windows)]
    let job = JobHandle::capture(&child);

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
    #[cfg(unix)]
    kill_process_group(pid);
    #[cfg(windows)]
    {
        // The job first — it takes the whole tree, including anything holding the pipes read
        // below. `child.kill()` after it, for the case where the job could not be created at all.
        if let Some(job) = &job {
            job.terminate();
        }
        let _ = child.kill();
    }
    #[cfg(not(any(unix, windows)))]
    let _ = child.kill();

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
fn drain(mut pipe: impl Read + Send + 'static) -> Reader {
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&bytes);
    let handle = std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            sink.lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(&chunk[..n]);
        }
    });
    Reader { handle, bytes }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn no_window_sets_flag() {
        // The documented value; a typo here would compile and silently open windows again.
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);

        let status = no_window(Command::new("cmd").args(["/c", "exit 0"]))
            .status()
            .expect("spawn `cmd /c exit 0`");

        assert!(
            status.success(),
            "`cmd /c exit 0` must still run and exit 0, got {status:?}"
        );
    }

    /// The app mutex is machine-session-wide, so the two cases below take turns rather than see each
    /// other's marker.
    #[cfg(windows)]
    static APP_MUTEX_CASES: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Whether a mutex named [`APP_MUTEX_NAME`] exists right now, as the installer's `AppMutex` asks.
    #[cfg(windows)]
    fn app_mutex_exists() -> bool {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};

        let name: Vec<u16> = APP_MUTEX_NAME.encode_utf16().chain(Some(0)).collect();
        // SAFETY: `name` is NUL-terminated and outlives the call; a non-null handle is closed at once.
        unsafe {
            let handle = OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, name.as_ptr());
            if handle.is_null() {
                return false;
            }
            CloseHandle(handle);
            true
        }
    }

    #[cfg(windows)]
    #[test]
    fn announce_running_holds_the_app_mutex() {
        // E7.1: the installer finds a running app by this mutex and asks the user to close it.
        let _turn = APP_MUTEX_CASES.lock().unwrap_or_else(|e| e.into_inner());
        let marker = announce_running();

        assert!(
            app_mutex_exists(),
            "while the marker is held, OpenMutexW({APP_MUTEX_NAME:?}) must succeed"
        );
        drop(marker);
    }

    #[cfg(windows)]
    #[test]
    fn dropping_the_marker_releases_the_app_mutex() {
        // E7.1: once the app exits, the installer must not keep asking the user to close it.
        let _turn = APP_MUTEX_CASES.lock().unwrap_or_else(|e| e.into_inner());
        let marker = announce_running();
        assert!(
            marker.is_some(),
            "announce_running must return a marker on Windows"
        );
        drop(marker);

        assert!(
            !app_mutex_exists(),
            "after the marker is dropped, OpenMutexW({APP_MUTEX_NAME:?}) must fail"
        );
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
