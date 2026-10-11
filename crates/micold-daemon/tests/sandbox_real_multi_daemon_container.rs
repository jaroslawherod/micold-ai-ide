//! Feature 491, US5 scenario 1 (FR-018, SC-006): a container daemon, driven by the client, starts
//! a session that produces output and accepts input. The AI CLI is a script on `PATH`, not the
//! shipped one: no sign-in, no network, no cost.
//!
//! Behind `sandbox-real-runtime` (Principle VI). `mise run image && mise run test-sandbox`.

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::Duration;

use sandbox_real_support::{
    fake_ai_cli, open_session, start_container_daemons, Terminal, FAKE_ECHO, FAKE_READY,
};

#[tokio::test]
async fn sandbox_real_multi_daemon_container_session_produces_output_and_accepts_input() {
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_ai_cli(&dir.path().join("fake-bin"));
    let daemons = start_container_daemons(dir.path(), "md-container", 17760, 1, &fake);
    let d = &daemons[0];

    let (mut conn, serial) = d.connect().await;
    let screen = open_session(&mut conn, &d.project, d.session, &d.log).await;
    let mut term = Terminal::new(&mut conn, d.session, screen, &d.container, &d.log, serial);

    term.wait_for(FAKE_READY, Duration::from_secs(20)).await;
    term.converse(
        "hello-container",
        &format!("{FAKE_ECHO}hello-container"),
        Duration::from_secs(20),
    )
    .await;
}
