//! Feature 491, US5 scenario 2 (FR-018): the same scenario as the container one, against the host
//! daemon with the same scripted AI CLI, so the existing host path stays covered.
//!
//! Behind `sandbox-real-runtime` (Principle VI). One test per target: the host daemon is spawned
//! from this process's environment.

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::Duration;

use sandbox_real_support::{
    fake_ai_cli, open_session, start_host_daemon, Terminal, FAKE_ECHO, FAKE_READY,
};

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

#[tokio::test]
async fn sandbox_real_multi_daemon_host_session_produces_output_and_accepts_input() {
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_ai_cli(&dir.path().join("fake-bin"));
    let (host, mut conn, serial) =
        start_host_daemon(&dir.path().join("host"), DAEMON_BIN, &fake, "md-host").await;

    let screen = open_session(&mut conn, &host.project, host.session, &host.log).await;
    let mut term = Terminal::new(&mut conn, host.session, screen, "host", &host.log, serial);

    term.wait_for(FAKE_READY, Duration::from_secs(20)).await;
    term.converse(
        "hello-host",
        &format!("{FAKE_ECHO}hello-host"),
        Duration::from_secs(20),
    )
    .await;
}
