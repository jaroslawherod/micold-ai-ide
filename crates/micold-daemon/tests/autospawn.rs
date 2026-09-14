//! T026b — cold-start auto-spawn (FR-003, SC-003).
//!
//! From a state with **no daemon running**, a client reaches the attached state with no manual
//! command: it spawns a detached daemon itself and completes the handshake. The spawned daemon then
//! *outlives the spawning client* — which is the whole feature.
//!
//! This is a genuine two-process test: it runs the real `micold-daemon` binary that Cargo built.

use std::time::Duration;

use micold_core::connect::{connect, connect_or_spawn, Connected};
use micold_core::spawn::DAEMON_BIN_ENV;

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

#[tokio::test]
async fn a_client_cold_starts_a_daemon_and_it_outlives_the_client() {
    let dir = tempfile::tempdir().unwrap();

    // Point the spawner at the freshly-built daemon binary and isolate the endpoint into our
    // tempdir. The child inherits this environment, so both sides resolve the *same* path — which
    // is exactly why endpoint resolution lives in micold-core: the client and the spawned daemon
    // cannot disagree about where the socket is.
    // SAFETY: this test binary runs these env writes before any spawn, and holds a single test.
    // The Windows endpoint takes no environment input, so there this is the user's real endpoint.
    std::env::set_var(DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", dir.path());
    // macOS keys the endpoint on `$HOME` and ignores `XDG_RUNTIME_DIR`; without this the test finds
    // whatever daemon the user (or an earlier CI step) left at `~/.micold/run/d.sock`.
    std::env::set_var("HOME", dir.path());
    // Keep the spawned daemon's logs out of the user's real data dir.
    std::env::set_var("MICOLD_LOG", "warn");

    // Derive the endpoint through the shared resolver *after* setting XDG_RUNTIME_DIR, so it matches
    // what the spawned daemon computes ($XDG_RUNTIME_DIR/micold/daemon.sock on Linux,
    // $HOME/.micold/run/d.sock on macOS).
    let endpoint = micold_core::endpoint::resolve().expect("resolve isolated endpoint");

    // Precondition: nothing is listening. This is a true cold start.
    assert!(
        connect(&endpoint, "test-client").await.unwrap().is_none(),
        "precondition: no daemon should be running yet"
    );

    // The client reaches an attached (handshaked) state with no manual command (FR-003, SC-003).
    let connected = connect_or_spawn(&endpoint, "test-client", Duration::from_secs(20))
        .await
        .expect("cold start should spawn a daemon and hand back a connection");

    let welcome = match connected {
        Connected::Ready(conn, welcome) => {
            // Drop the connection immediately — this simulates the client going away.
            drop(conn);
            welcome
        }
        Connected::Refused(reason) => panic!("cold start was refused: {reason:?}"),
    };
    assert!(
        welcome.daemon_build.starts_with("micold-daemon"),
        "the daemon must identify itself, got {:?}",
        welcome.daemon_build
    );

    // The spawning client is gone, but the daemon it started is still there and still accepting —
    // sessions would keep running (FR-003). A fresh connect proves it.
    let again = connect(&endpoint, "second-client")
        .await
        .expect("connect after the first client left");
    assert!(
        matches!(again, Some(Connected::Ready(_, _))),
        "the spawned daemon must outlive the client that spawned it"
    );

    // Clean up the process this test created, through the pid record it wrote.
    let _ = micold_core::spawn::stop_running_daemon(&endpoint);
    std::env::remove_var(DAEMON_BIN_ENV);
    std::env::remove_var("XDG_RUNTIME_DIR");
    std::env::remove_var("MICOLD_LOG");
}
