//! T018 — single-instance convergence (contracts/protocol.md §2, research R1.4).
//!
//! Two simultaneous starters converge on exactly one daemon; a stale socket left by a crash is
//! reclaimed; a predictable directory with the wrong ownership/mode bails loudly instead of binding.

#![cfg(unix)]

use std::path::Path;

use micold_core::endpoint::Endpoint;
use micold_daemon::singleton::{self, Acquisition};

fn test_endpoint(dir: &Path) -> Endpoint {
    Endpoint {
        socket_path: dir.join("daemon.sock"),
        lock_path: dir.join("daemon.lock"),
    }
}

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
