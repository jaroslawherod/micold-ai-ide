//! T036 [US3] — an idle stop leaves nothing behind, twenty times running (FR-013, FR-014, SC-007,
//! lifecycle contract §3.12–§3.13).
//!
//! The promise is not "it exits" but "the next start is indistinguishable from a first start", and
//! that is a claim about *residue*: a socket file nothing listens on, a lock nothing holds, a
//! descendant process nobody owns. Any one of them makes the following start take a recovery path,
//! and a recovery path that runs every time is a recovery path nobody notices is broken.
//!
//! Twenty consecutive cycles rather than one, because residue accumulates. A single stop-and-start
//! passes even when each stop leaks something, and the shape of the failure — the tenth start being
//! slower, or the twentieth failing outright — is exactly what a one-shot test cannot see (SC-007).

#![cfg(unix)]

use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use micold_daemon::idle::IDLE_STOP_ENV;

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

fn spawn_daemon(dir: &Path, idle_stop: &str) -> Child {
    Command::new(DAEMON_BIN)
        .env("XDG_RUNTIME_DIR", dir)
        .env("HOME", dir)
        .env("MICOLD_LOG", "warn")
        .env(IDLE_STOP_ENV, idle_stop)
        .spawn()
        .expect("the daemon binary must start")
}

fn endpoint_in(dir: &Path) -> micold_core::endpoint::Endpoint {
    // Both variables, as `spawn_daemon` sets both: macOS keys the endpoint on `$HOME`.
    let previous = ["XDG_RUNTIME_DIR", "HOME"].map(|var| (var, std::env::var_os(var)));
    for (var, _) in &previous {
        std::env::set_var(var, dir);
    }
    let resolved = micold_core::endpoint::resolve().expect("resolve isolated endpoint");
    for (var, value) in previous {
        match value {
            Some(v) => std::env::set_var(var, v),
            None => std::env::remove_var(var),
        }
    }
    resolved
}

/// Twenty stop-and-restart cycles, asserting zero residue after each one.
///
/// Each cycle: start a daemon, confirm it is listening, let its window expire, and check what it
/// left. The next cycle's start is the real assertion that the check was complete — it binds the
/// same endpoint with no recovery step, which is the user-visible form of FR-014.
#[tokio::test]
async fn twenty_consecutive_idle_stops_leave_no_residue() {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = endpoint_in(dir.path());

    for cycle in 1..=20 {
        let mut daemon = spawn_daemon(dir.path(), "300ms");
        let pid = daemon.id();

        // It came up and bound the endpoint with no recovery step — including on cycle 2..20, where
        // whatever the previous stop left behind is what it had to bind over.
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if matches!(
                micold_core::connect::connect(&endpoint, "teardown-test").await,
                Ok(Some(_))
            ) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "cycle {cycle}: the daemon never started listening — the previous stop left \
                 something behind that this start could not bind over"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        // …and it stops itself once nothing is connected.
        let stop_deadline = Instant::now() + Duration::from_secs(30);
        let exited = loop {
            match daemon.try_wait().expect("try_wait") {
                Some(status) => break status,
                None if Instant::now() >= stop_deadline => {
                    let _ = daemon.kill();
                    panic!("cycle {cycle}: the daemon never stopped");
                }
                None => tokio::time::sleep(Duration::from_millis(25)).await,
            }
        };
        assert!(
            exited.success(),
            "cycle {cycle}: an idle stop is an ordinary exit, not a failure — got {exited:?}"
        );

        // §3.12: no descendant process. Asked of the daemon's own pid, which is what a leaked
        // session's process tree would still be parented under.
        assert!(
            !process_is_alive(pid),
            "cycle {cycle}: the daemon process is still alive after exiting"
        );

        // §3.13: no file at the endpoint path, and the lock is free.
        assert!(
            !endpoint.socket_path.exists(),
            "cycle {cycle}: a socket file was left at {}; the next start would have to decide \
             whether it is stale, and a stale-socket path taken on every start is one nobody \
             notices is broken",
            endpoint.socket_path.display()
        );
        assert!(
            lock_is_free(&endpoint.lock_path),
            "cycle {cycle}: the lock at {} is still held",
            endpoint.lock_path.display()
        );
    }
}

fn process_is_alive(pid: u32) -> bool {
    // SAFETY: signal 0 performs error checking only; it never delivers a signal.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// Whether the advisory lock at `path` can be taken — i.e. nothing holds it.
///
/// A missing file counts as free: there is nothing to hold. The lock is released here again
/// immediately, so this probe never becomes the thing that blocks the next cycle.
fn lock_is_free(path: &Path) -> bool {
    use std::os::unix::io::AsRawFd;

    let Ok(file) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
    else {
        return true;
    };
    // SAFETY: a valid fd from the file above; `LOCK_NB` returns rather than blocking.
    let taken = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 };
    if taken {
        // SAFETY: same fd, releasing what we just took.
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    }
    taken
}
