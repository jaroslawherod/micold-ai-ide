//! T026b — cold-start auto-spawn (FR-003, SC-003).
//!
//! From a state with **no daemon running**, a client reaches the attached state with no manual
//! command: it spawns a detached daemon itself and completes the handshake. The spawned daemon then
//! *outlives the spawning client* — which is the whole feature.
//!
//! This is a genuine two-process test: it runs the real `micold-daemon` binary that Cargo built.

#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use micold_core::connect::{connect, connect_or_spawn, Connected};
use micold_core::spawn::DAEMON_BIN_ENV;

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// Terminate a process we spawned ourselves, so the test leaves nothing behind.
fn terminate(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg(pid.to_string())
        .status();
}

/// Find the daemon holding `socket_path`, so the test can clean up after itself.
fn daemon_pid_holding(socket: &PathBuf) -> Option<u32> {
    let out = std::process::Command::new("fuser")
        .arg(socket)
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .find_map(|tok| tok.trim().parse::<u32>().ok())
}

#[tokio::test]
async fn a_client_cold_starts_a_daemon_and_it_outlives_the_client() {
    let dir = tempfile::tempdir().unwrap();

    // Point the spawner at the freshly-built daemon binary and isolate the endpoint into our
    // tempdir. The child inherits this environment, so both sides resolve the *same* path — which
    // is exactly why endpoint resolution lives in micold-core: the client and the spawned daemon
    // cannot disagree about where the socket is.
    // SAFETY: this test binary runs these env writes before any spawn; `#[cfg(unix)]` + single test.
    std::env::set_var(DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", dir.path());
    // Keep the spawned daemon's logs out of the user's real data dir.
    std::env::set_var("MICOLD_LOG", "warn");

    // Derive the endpoint through the shared resolver *after* setting XDG_RUNTIME_DIR, so it matches
    // what the spawned daemon computes ($XDG_RUNTIME_DIR/micold/daemon.sock on Linux).
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

    // Clean up the process this test created.
    if let Some(pid) = daemon_pid_holding(&endpoint.socket_path) {
        terminate(pid);
    }
    std::env::remove_var(DAEMON_BIN_ENV);
    std::env::remove_var("XDG_RUNTIME_DIR");
    std::env::remove_var("MICOLD_LOG");
}
