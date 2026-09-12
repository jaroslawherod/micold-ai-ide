//! A newer application meeting a service left running by an older one resolves it (028, FR-023).
//!
//! Updating on macOS is a drag-and-replace: the user swaps the bundle in `Applications` while the
//! previous version's session daemon is still running from the copy that was just overwritten. The
//! new client then handshakes with a daemon that speaks the old contract, and the handshake is a
//! strict exact match with no negotiation — so the connection is refused. That is correct, and it
//! is not the requirement. FR-023 is about what happens *next*: the mismatch has to resolve
//! "without the user having to find and stop a background process by hand".
//!
//! Two properties make that true, and neither was covered:
//!
//!   1. The running daemon records its pid where a client that cannot talk to it can still read
//!      it. A mismatched client has no protocol left to ask over, so the pid file is the only
//!      version-independent handle on the process — without it, the remedy really is `ps | grep`.
//!   2. Stopping it is enough. Nothing has to be un-registered, no service manager consulted, and
//!      the very next connection brings up a daemon matching the client that asked for it.
//!
//! `handshake_flow.rs` covers the refusal itself; this covers the recovery the refusal hands the
//! user, end to end, against the real binary Cargo built.
//!
//! # One test, deliberately
//!
//! The setup is process-wide environment (`XDG_RUNTIME_DIR`, `MICOLD_DAEMON_BIN`) because the
//! spawned child has to resolve the same endpoint this process does, and Cargo runs a file's tests
//! on parallel threads. A second `#[test]` here would race the first's `set_var`, so the
//! idempotence check is folded into the one flow rather than split out — see `autospawn.rs`, which
//! is one test for the same reason.

#![cfg(unix)]

use std::time::Duration;

use micold_core::connect::{connect, connect_or_spawn, Connected};
use micold_core::spawn::{running_daemon_pid, stop_running_daemon, DAEMON_BIN_ENV};

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// Poll until nothing is listening on `endpoint`. SIGTERM is asynchronous: the daemon unlinks its
/// socket in `Drop`, so "stopped" is observable only by asking, and only after a moment.
async fn wait_until_nothing_listens(endpoint: &micold_core::endpoint::Endpoint) {
    for _ in 0..200 {
        if matches!(connect(endpoint, "probe").await, Ok(None)) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the stale daemon was still accepting 10s after it was stopped");
}

#[tokio::test]
async fn a_stale_daemon_is_stopped_and_replaced_without_the_user_finding_a_process() {
    let dir = tempfile::tempdir().unwrap();

    // Isolate the endpoint into our tempdir and point the spawner at the freshly built binary; the
    // child inherits both, so both sides resolve the same socket path.
    // SAFETY: written before any spawn; `#[cfg(unix)]` + the single test this file deliberately has.
    std::env::set_var(DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", dir.path());
    std::env::set_var("MICOLD_LOG", "warn");

    let endpoint = micold_core::endpoint::resolve().expect("resolve isolated endpoint");

    // Nothing to stop yet, and asking is not an error. This is the state the flow lands in when
    // the user clicks the action twice, or when the stale daemon exited on its own between the
    // refusal and the click — the remedy has to be a no-op there, not a failure to report.
    assert!(
        !stop_running_daemon(&endpoint).expect("stopping nothing is not an error"),
        "with no daemon running there is nothing to stop, and that is an ordinary outcome"
    );

    // The daemon the previous version left running. A real one: what matters below is that it is a
    // separate process this client cannot talk its way out of.
    let stale = match connect_or_spawn(&endpoint, "old-client", Duration::from_secs(20))
        .await
        .expect("cold start should spawn a daemon")
    {
        Connected::Ready(conn, welcome) => {
            drop(conn);
            welcome
        }
        Connected::Refused(reason) => panic!("cold start was refused: {reason:?}"),
    };
    assert!(
        stale.daemon_build.starts_with("micold-daemon"),
        "the daemon must identify itself, got {:?}",
        stale.daemon_build
    );

    // Property 1: the handle exists without the protocol. This is the assertion that FR-023 turns
    // on — a client that cannot handshake can still read this file and act on it.
    let stale_pid = running_daemon_pid(&endpoint)
        .expect("the running daemon must record its pid; without it the remedy is `ps | grep`");

    // Property 2: stopping it is the whole remedy. This is precisely what the client's "restart
    // service" action runs, with no privilege, no service manager, and no name the user has to
    // know.
    assert!(
        stop_running_daemon(&endpoint).expect("stopping the stale daemon"),
        "a recorded pid means a stop was issued"
    );
    wait_until_nothing_listens(&endpoint).await;

    // And the next connection is served by a daemon matching the client that asked for it. No
    // second gesture: this is the reconnect the client would have made anyway.
    let fresh = match connect_or_spawn(&endpoint, "new-client", Duration::from_secs(20))
        .await
        .expect("the next connect must bring up a replacement")
    {
        Connected::Ready(conn, welcome) => {
            drop(conn);
            welcome
        }
        Connected::Refused(reason) => {
            panic!("the replacement refused the client that spawned it: {reason:?}")
        }
    };
    assert_eq!(
        fresh.daemon_build, stale.daemon_build,
        "both daemons are this build, so the replacement must agree with the client"
    );

    let fresh_pid = running_daemon_pid(&endpoint).expect("the replacement records its pid in turn");
    assert_ne!(
        fresh_pid, stale_pid,
        "the replacement must be a new process — an unchanged pid means nothing was restarted"
    );

    // Leave nothing behind: only the daemon this test started, in this test's own tempdir.
    let _ = stop_running_daemon(&endpoint);
    std::env::remove_var(DAEMON_BIN_ENV);
    std::env::remove_var("XDG_RUNTIME_DIR");
    std::env::remove_var("MICOLD_LOG");
}
