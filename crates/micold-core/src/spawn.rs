//! Detached daemon spawn (research R1.6, FR-003, task T026a).
//!
//! When a client finds no daemon listening it starts one **itself** — no install step, no external
//! supervisor. The spawned process must *outlive the spawning client*, which is the whole point: the
//! user closes the window and the sessions keep running.
//!
//! - **Unix**: `setsid()` in a `pre_exec` hook makes the child a session leader with no controlling
//!   terminal, so it survives both the parent's exit and any terminal signal. The parent never waits,
//!   so on exit the child is reparented to init.
//! - **Windows**: `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` — no inherited console, no Ctrl-C
//!   propagation from the parent's group.
//!
//! Lives in the core (not the client) so the headless test suite can exercise auto-spawn without
//! pulling in iced (FR-040, and the T026b test lives in the daemon's suite).

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::endpoint::Endpoint;

/// Environment variable overriding which daemon binary to spawn. Primarily for tests and for
/// development builds where the binary is not beside the client.
pub const DAEMON_BIN_ENV: &str = "MICOLD_DAEMON_BIN";

/// Environment variable naming the idle rule a daemon should run (feature 028, FR-019/FR-022).
///
/// Here rather than in the daemon, which is where it is *read*, because the two places that
/// **write** it are both outside the daemon: the sandbox's `create` argv (`sandbox::argv`) and the
/// test harnesses that shorten the window. A constant owned by the reader would have to be spelled
/// by hand at every writer, and a variable whose name is spelled in four places is a variable that
/// eventually disagrees with itself.
///
/// `micold_daemon::idle` re-exports it and owns the parsing — the meaning of a value is the
/// reader's business, the spelling of the name is shared.
pub const IDLE_STOP_ENV: &str = "MICOLD_IDLE_STOP";

/// The one [`IDLE_STOP_ENV`] value that turns the idle stop off entirely.
///
/// Every other value is a window, so this is the only one a writer outside the daemon ever needs.
pub const IDLE_STOP_OFF: &str = "off";

/// The daemon executable name.
const DAEMON_BIN: &str = if cfg!(windows) {
    "micold-daemon.exe"
} else {
    "micold-daemon"
};

/// Locate the `micold-daemon` binary: the `MICOLD_DAEMON_BIN` override, then a sibling of the current
/// executable (how a packaged install is laid out), then bare `micold-daemon` for `PATH` lookup.
pub fn daemon_binary() -> OsString {
    if let Some(explicit) = std::env::var_os(DAEMON_BIN_ENV) {
        if !explicit.is_empty() {
            return explicit;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling: PathBuf = dir.join(DAEMON_BIN);
            if sibling.is_file() {
                return sibling.into_os_string();
            }
        }
    }
    OsString::from(DAEMON_BIN)
}

/// Spawn the daemon fully detached, returning its process id. Returns as soon as the child is
/// started — the daemon is ready when it *accepts*, not when `exec` returns, so the caller polls the
/// endpoint (see `connect::connect_or_spawn`).
pub fn spawn_detached_daemon() -> io::Result<u32> {
    let mut command = Command::new(daemon_binary());
    // A detached service owns no terminal; its diagnostics go to its own log sink (FR-045).
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    configure_detached(&mut command);

    // Deliberately not awaited/waited: the child must outlive us (FR-003).
    command.spawn().map(|child| child.id())
}

/// The pid the running daemon recorded in its lock file, if any (`None` when the file is absent,
/// empty, or unparseable — e.g. an older daemon that predates pid recording).
///
/// A record is only believed while the endpoint accepts connections. A daemon that crashed or was
/// killed leaves its record behind, and by then the pid may belong to an unrelated process
/// (feature 030, E4.4).
pub fn running_daemon_pid(endpoint: &Endpoint) -> Option<u32> {
    if !endpoint_is_live(endpoint) {
        return None;
    }
    std::fs::read_to_string(&endpoint.lock_path)
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// `true` iff something accepts connections at the endpoint right now.
fn endpoint_is_live(endpoint: &Endpoint) -> bool {
    use interprocess::local_socket::{prelude::*, GenericFilePath, Stream};
    match endpoint
        .socket_path
        .as_os_str()
        .to_fs_name::<GenericFilePath>()
    {
        Ok(name) => Stream::connect(name).is_ok(),
        Err(_) => false,
    }
}

/// Stop the running daemon so a matching one can be spawned in its place — the client half of the
/// version-mismatch "restart service" action (FR-022). A mismatched client cannot handshake, so this
/// terminates the daemon by the pid it recorded, then the caller's normal `connect_or_spawn` starts a
/// fresh, matching daemon (none will be listening once this one exits). Best-effort and idempotent:
/// returns `Ok(false)` when no pid was recorded (nothing to stop), `Ok(true)` when a stop was issued.
pub fn stop_running_daemon(endpoint: &Endpoint) -> io::Result<bool> {
    match running_daemon_pid(endpoint) {
        Some(pid) => {
            terminate_daemon(pid)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Send a terminate to the daemon process. Unix uses `SIGTERM` so the daemon's `Drop` can unlink its
/// socket cleanly; a still-stuck daemon is superseded anyway once its socket stops accepting.
#[cfg(unix)]
fn terminate_daemon(pid: u32) -> io::Result<()> {
    // SAFETY: `kill` takes a pid and a signal and cannot corrupt this process's memory. An ESRCH
    // (already gone) is success for our purpose — nothing left to stop.
    let rc = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if rc == 0 {
        return Ok(());
    }
    let err = io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(err)
    }
}

/// Terminate the daemon by pid on Windows (research R5). There is no `SIGTERM` to send, so this is a
/// hard stop; the pipe goes with the process and the pid record is superseded by the next daemon.
///
/// The pid comes from a file, and a pid is reused once its process is gone, so the image is checked
/// first: anything that is not `micold-daemon.exe` is refused with `InvalidData` rather than killed
/// (E4.3).
#[cfg(windows)]
fn terminate_daemon(pid: u32) -> io::Result<()> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE,
    };

    /// How long to wait for the terminated process to actually exit.
    const EXIT_WAIT_MS: u32 = 5_000;

    struct Process(HANDLE);
    impl Drop for Process {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful `OpenProcess` and is closed only here.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    // SAFETY: plain call; a null return is the failure signal and is handled.
    let raw = unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    let process = Process(raw);

    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    // SAFETY: `buf` holds `len` UTF-16 units; the call writes at most that and updates `len`.
    if unsafe {
        QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let image = PathBuf::from(OsString::from_wide(&buf[..len as usize]));
    let is_daemon = image
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("micold-daemon.exe"));
    if !is_daemon {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "pid {pid} is {}, not the daemon; refusing to stop it",
                image.display()
            ),
        ));
    }

    // SAFETY: `process` holds PROCESS_TERMINATE and PROCESS_SYNCHRONIZE for this pid.
    unsafe {
        if TerminateProcess(process.0, 1) == 0 {
            return Err(io::Error::last_os_error());
        }
        if WaitForSingleObject(process.0, EXIT_WAIT_MS) == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Stopping the daemon is not implemented on platforms that are neither Unix nor Windows.
#[cfg(not(any(unix, windows)))]
fn terminate_daemon(_pid: u32) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "stopping the daemon is not implemented on this platform",
    ))
}

#[cfg(unix)]
fn configure_detached(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: `setsid` and `close` are async-signal-safe and this is exactly what `pre_exec` is
    // for. It runs in the forked child before `exec`.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                // Already a session leader is fine; anything else is a real failure.
                let err = io::Error::last_os_error();
                if err.raw_os_error() != Some(libc::EPERM) {
                    return Err(err);
                }
            }
            close_inherited_descriptors();
            Ok(())
        });
    }
}

/// Shut every descriptor above stderr before `exec`, so the daemon starts owning nothing but its
/// own three standard streams.
///
/// # Why a detached process has to do this and an ordinary child does not
///
/// Rust opens its own descriptors `CLOEXEC`, so a child normally inherits only what it was given.
/// Descriptors this process inherited from *its* parent carry no such promise, and a daemon that
/// outlives everyone (FR-003) holds them for as long as it runs.
///
/// That is not theoretical. `scripts/build-lock.sh` takes the repository's build lock with a bash
/// `exec 9>`, which is not `CLOEXEC`; running the suite under it therefore handed fd 9 down through
/// cargo, the test binary and into the daemon `tests/autospawn.rs` deliberately leaves running —
/// which then held the lock of every worktree on the machine until it was killed by hand. The same
/// mechanism pins a client's open files and sockets after the client is gone.
///
/// # Safety
///
/// Called from `pre_exec`, between `fork` and `exec`, where only async-signal-safe calls are
/// allowed. `close_range` and `close` are; allocating or locking would not be, which is why this
/// counts rather than enumerating `/proc/self/fd`.
#[cfg(unix)]
unsafe fn close_inherited_descriptors() {
    /// The three the child is entitled to, already redirected to `/dev/null` by the caller.
    const FIRST: libc::c_int = 3;

    // Linux 5.9+ and the modern BSDs close the whole range in one call, without a bound to guess.
    #[cfg(target_os = "linux")]
    {
        if libc::syscall(
            libc::SYS_close_range,
            FIRST as libc::c_uint,
            libc::c_uint::MAX,
            0,
        ) == 0
        {
            return;
        }
    }

    // Otherwise walk to the soft limit. `_SC_OPEN_MAX` is the number of descriptors this process
    // may hold, so nothing above it can be open; a negative answer means the limit is unknown and
    // 1024 is the POSIX minimum, which is the most that can be closed honestly.
    let limit = match libc::sysconf(libc::_SC_OPEN_MAX) {
        n if n > 0 => n as libc::c_int,
        _ => 1024,
    };
    for fd in FIRST..limit {
        libc::close(fd);
    }
}

#[cfg(windows)]
fn configure_detached(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(any(unix, windows)))]
fn configure_detached(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    // Both cases live in ONE test: they mutate the same process-global env var, so splitting them
    // lets cargo's parallel runner clobber one from the other. Sequential here, no races.
    #[test]
    fn daemon_binary_resolution() {
        // Override set → it wins verbatim.
        // SAFETY: this test owns the var for its duration; it is the only test that touches it.
        std::env::set_var(DAEMON_BIN_ENV, "/custom/path/to/daemon");
        assert_eq!(daemon_binary(), OsString::from("/custom/path/to/daemon"));

        // Override cleared → a sibling path or the bare name, never empty.
        std::env::remove_var(DAEMON_BIN_ENV);
        assert!(!daemon_binary().is_empty());
    }

    /// A pid no process can hold (above every platform's pid ceiling), so even a wrong answer
    /// signals nobody.
    const UNUSED_PID: u32 = 0x7FFF_FFF0;

    /// An endpoint in `dir` that nothing listens on.
    fn dead_endpoint(dir: &std::path::Path) -> Endpoint {
        #[cfg(windows)]
        let socket_path = format!(
            r"\\.\pipe\Micold.Test.{}.{}",
            std::process::id(),
            dir.file_name()
                .expect("temp dir has a name")
                .to_string_lossy()
        )
        .into();
        #[cfg(not(windows))]
        let socket_path = dir.join("daemon.sock");
        Endpoint {
            socket_path,
            lock_path: dir.join("daemon.lock"),
        }
    }

    #[test]
    fn stale_pid_record_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = dead_endpoint(dir.path());
        std::fs::write(&endpoint.lock_path, format!("{UNUSED_PID}\n")).unwrap();

        let stopped = stop_running_daemon(&endpoint);

        assert!(
            matches!(stopped, Ok(false)),
            "a pid record whose endpoint is not live is a leftover, not a daemon to stop; got {stopped:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn terminate_refuses_foreign_image() {
        // E4.3: a recorded pid can be reused by an unrelated process between the check and the stop,
        // so only a process whose image is `micold-daemon.exe` is ever terminated.
        let mut foreign = Command::new("cmd")
            .args(["/c", "ping", "-n", "30", "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn a foreign process");

        let refused = terminate_daemon(foreign.id());
        let still_running = matches!(foreign.try_wait(), Ok(None));
        let _ = foreign.kill();
        let _ = foreign.wait();

        assert!(
            matches!(&refused, Err(e) if e.kind() == io::ErrorKind::InvalidData),
            "terminating a pid whose image is not micold-daemon.exe must be refused as InvalidData; got {refused:?}"
        );
        assert!(still_running, "the foreign process must be left running");
    }

    #[cfg(windows)]
    #[test]
    fn terminate_treats_an_exited_pid_as_stopped() {
        // Like ESRCH on Unix: a daemon that exited between the liveness check and the stop leaves
        // nothing to stop. `OpenProcess` on a pid with no process returns ERROR_INVALID_PARAMETER.
        let mut exited = Command::new("cmd")
            .args(["/c", "exit", "0"])
            .spawn()
            .expect("spawn a short-lived process");
        let pid = exited.id();
        exited.wait().unwrap();
        drop(exited);

        let stopped = terminate_daemon(pid);

        assert!(
            stopped.is_ok(),
            "terminating a pid that has already exited is success; got {stopped:?}"
        );
    }
}
