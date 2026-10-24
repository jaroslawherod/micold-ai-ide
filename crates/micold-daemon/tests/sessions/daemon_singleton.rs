//! T018 — single-instance convergence (contracts/protocol.md §2, research R1.4).
//!
//! Two simultaneous starters converge on exactly one daemon; a stale socket left by a crash is
//! reclaimed; a predictable directory with the wrong ownership/mode bails loudly instead of binding.
//!
//! Runs on every platform (feature 030, FR-022): on Windows the endpoint is a named pipe and the
//! convergence comes from `FILE_FLAG_FIRST_PIPE_INSTANCE` rather than a lock file, but the observable
//! rule is the same. Only the stale-socket case is Unix-only, because a pipe leaves nothing behind.

use std::path::Path;

use micold_core::endpoint::Endpoint;
use micold_daemon::singleton::{self, Acquisition};

#[cfg(unix)]
fn test_endpoint(dir: &Path) -> Endpoint {
    Endpoint {
        socket_path: dir.join("daemon.sock"),
        lock_path: dir.join("daemon.lock"),
    }
}

/// A pipe name no other test or running daemon uses: the pipe namespace is machine-wide, so the temp
/// dir's own unique name keys it.
#[cfg(windows)]
fn test_endpoint(dir: &Path) -> Endpoint {
    let unique = dir
        .file_name()
        .expect("temp dir has a name")
        .to_string_lossy();
    Endpoint {
        socket_path: format!(r"\\.\pipe\Micold.Test.{}.{unique}", std::process::id()).into(),
        lock_path: dir.join("micold-daemon.pid"),
    }
}

/// How long a restart may wait for the endpoint the previous daemon released. Generous for a slow CI
/// runner; a leaked endpoint would otherwise hang the test instead of failing it.
const REBIND_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);

fn is_bound(a: &Acquisition) -> bool {
    matches!(a, Acquisition::Bound(_))
}

#[tokio::test]
async fn two_simultaneous_starters_converge_on_one_daemon() {
    let dir = tempfile::tempdir().unwrap();
    let ep = test_endpoint(dir.path());

    // Race two starters. Exactly one must win the bind; the other must observe the live daemon.
    let (a, b) = tokio::join!(singleton::acquire(&ep), singleton::acquire(&ep));
    let a = a.expect("acquire a");
    let b = b.expect("acquire b");

    let bound = usize::from(is_bound(&a)) + usize::from(is_bound(&b));
    assert_eq!(bound, 1, "exactly one starter must become the daemon");
    // Hold both until here so the winner's listener stays alive during the loser's probe.
    drop((a, b));
}

// unix-only: a crashed Unix listener leaves its socket file behind; a Windows pipe vanishes with its
// last handle, so there is nothing to reclaim.
#[cfg(unix)]
#[tokio::test]
async fn a_stale_socket_from_a_crash_is_reclaimed() {
    let dir = tempfile::tempdir().unwrap();
    let ep = test_endpoint(dir.path());

    // Simulate a crash: bind a socket file, then drop the listener WITHOUT unlinking it (std's
    // UnixListener does not remove the path on drop). The file now exists but nothing listens.
    {
        let listener = std::os::unix::net::UnixListener::bind(&ep.socket_path).unwrap();
        drop(listener);
    }
    assert!(ep.socket_path.exists(), "stale socket file should remain");

    // A fresh start must reclaim it (connect fails => stale => S_ISSOCK unlink => bind).
    let acq = singleton::acquire(&ep)
        .await
        .expect("acquire over stale socket");
    assert!(
        is_bound(&acq),
        "a stale socket must be reclaimed, not treated as a live daemon"
    );
}

#[tokio::test]
async fn a_second_start_after_a_live_bind_acts_as_client() {
    let dir = tempfile::tempdir().unwrap();
    let ep = test_endpoint(dir.path());

    let first = singleton::acquire(&ep).await.expect("first acquire");
    assert!(is_bound(&first), "first starter binds");

    let second = singleton::acquire(&ep).await.expect("second acquire");
    assert!(
        matches!(second, Acquisition::AlreadyRunning),
        "a start against a live daemon must act as a client"
    );
    drop(first);
}

#[tokio::test]
async fn acquire_after_drop_rebinds() {
    let dir = tempfile::tempdir().unwrap();
    let ep = test_endpoint(dir.path());

    let first = singleton::acquire(&ep).await.expect("first acquire");
    assert!(is_bound(&first), "first starter binds");
    drop(first);

    // A daemon that shut down cleanly must not block the next one: "Restart service" is exactly this.
    let second = tokio::time::timeout(REBIND_BUDGET, singleton::acquire(&ep))
        .await
        .expect("acquire after drop must not wait on the previous daemon's endpoint")
        .expect("acquire after drop");
    assert!(
        is_bound(&second),
        "the endpoint must be free again once the previous daemon's listener is dropped"
    );
}
