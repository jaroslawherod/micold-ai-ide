//! Feature 491, US5 scenario 1 (FR-018, SC-006): a container daemon, driven by the client, starts
//! a session that produces output and accepts input. The AI CLI is the stand-in package the
//! install step puts in the container, not the shipped one: no sign-in, no network, no cost.
//!
//! Behind `sandbox-real-runtime` (Principle VI). `mise run image && mise run test-sandbox`.

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::Duration;

use micold_core::provision::{FailureCause, Step, StepBounds};

use sandbox_real_support::{
    failed, open_session, start_container_daemons, ProvisionRun, Terminal, FAKE_ECHO, FAKE_READY,
};

#[tokio::test]
async fn sandbox_real_multi_daemon_container_session_produces_output_and_accepts_input() {
    let dir = tempfile::tempdir().unwrap();
    let daemons = start_container_daemons(dir.path(), "md-container", 17760, 1);
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

/// FR-022: one daemon whose provisioning fails (its install outlives a short bound) does not
/// disturb another that stays connected: the second keeps answering input while the first fails.
#[tokio::test]
async fn sandbox_real_multi_daemon_container_a_failing_provision_leaves_the_other_responsive() {
    let dir = tempfile::tempdir().unwrap();
    let daemons = start_container_daemons(dir.path(), "md-provision", 17780, 2);
    let (steady, failing) = (&daemons[0], &daemons[1]);

    let (mut conn, serial) = steady.connect().await;
    let screen = open_session(&mut conn, &steady.project, steady.session, &steady.log).await;
    let mut term = Terminal::new(
        &mut conn,
        steady.session,
        screen,
        &steady.container,
        &steady.log,
        serial,
    );
    term.wait_for(FAKE_READY, Duration::from_secs(20)).await;

    let bounds = StepBounds {
        install: Duration::from_secs(5),
        ..StepBounds::container()
    };
    let failing_run = ProvisionRun {
        slow_install: true,
        ..ProvisionRun::new(bounds)
    }
    .run(failing.port);
    let expect = format!("{FAKE_ECHO}still-here");
    let responsive = term.converse("still-here", &expect, Duration::from_secs(20));
    let ((_, outcome, _), ()) = tokio::join!(failing_run, responsive);

    let failure = failed(outcome, "the slow install outlives its bound");
    assert_eq!(failure.step, Step::InstallAiClis);
    assert!(
        matches!(failure.cause, FailureCause::TimedOut { .. }),
        "{:?}",
        failure.cause
    );
    term.converse(
        "after-failure",
        &format!("{FAKE_ECHO}after-failure"),
        Duration::from_secs(20),
    )
    .await;
}
