//! A detached daemon starts owning nothing it was handed (FR-003, FR-045).
//!
//! # The defect this exists for
//!
//! `spawn_detached_daemon` is the one place in the workspace that starts a process meant to
//! *outlive its parent*. Everything Rust opens is `CLOEXEC`, so the usual child inherits nothing —
//! but a descriptor this process inherited from *its* own parent carries no such promise, and the
//! daemon then holds it for as long as it runs, which is indefinitely.
//!
//! That is how the repository's build lock got wedged. `scripts/build-lock.sh` takes it with a bash
//! `exec 9>`, which is not `CLOEXEC`; `cargo test --workspace` under that wrapper handed fd 9 down
//! through cargo and the test binary into the daemon `autospawn.rs` deliberately leaves running,
//! and every worktree on the machine then blocked on a lock held by an orphan. Nothing in the
//! suite could see it: both processes behaved exactly as specified.
//!
//! Linux-only because it reads the answer out of `/proc`. The property is not Linux-only, but a
//! test that cannot see descriptors cannot assert about them, and one platform checking it is what
//! keeps the `pre_exec` hook honest.

#![cfg(target_os = "linux")]

use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use micold_core::spawn::{spawn_detached_daemon, DAEMON_BIN_ENV};

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// Everything `/proc/<pid>/fd` currently resolves to.
fn descriptors_of(pid: u32) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| std::fs::read_link(e.path()).ok())
        .collect()
}

/// Wait until the child has actually `exec`'d the daemon — before that it is still a fork of this
/// test binary, whose descriptors say nothing about the daemon's.
fn wait_for_exec(pid: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if std::fs::read_link(format!("/proc/{pid}/exe"))
            .is_ok_and(|exe| exe == Path::new(DAEMON_BIN))
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn a_spawned_daemon_holds_none_of_its_spawners_descriptors() {
    let dir = tempfile::tempdir().unwrap();
    let inherited = dir.path().join("inherited-by-accident");
    let file = std::fs::File::create(&inherited).unwrap();

    // `dup` is the point of the test: it produces a descriptor with `CLOEXEC` *cleared*, which is
    // precisely the shape of the one bash's `exec 9>` hands down. Opening a file the ordinary way
    // would prove nothing — std would have marked it close-on-exec itself.
    // SAFETY: `dup` takes a valid descriptor and returns a new one or -1; it cannot corrupt memory.
    let raw = unsafe { libc::dup(file.as_raw_fd()) };
    assert!(raw >= 0, "could not duplicate a descriptor to inherit");

    // SAFETY: this is the only test in this binary and all three writes happen before any spawn,
    // so nothing reads the environment concurrently.
    std::env::set_var(DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", dir.path());
    std::env::set_var("MICOLD_LOG", "warn");

    let pid = spawn_detached_daemon().expect("spawn the daemon");
    assert!(wait_for_exec(pid), "the daemon never reached its own image");

    let held = descriptors_of(pid);
    let leaked: Vec<&PathBuf> = held.iter().filter(|p| p.as_path() == inherited).collect();

    // SAFETY: `kill` takes a pid and a signal. Done before the assertion so a failure still cleans
    // up — a leaked daemon here would hold the very lock this test is about.
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
        libc::close(raw);
    }

    assert!(
        leaked.is_empty(),
        "the daemon inherited {inherited:?} and will hold it for as long as it runs; it holds \
         {held:?}"
    );
}
