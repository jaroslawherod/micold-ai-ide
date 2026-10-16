//! Feature 491, US5 scenario 3 (FR-018, SC-001, SC-002, SC-006): a host daemon and a container
//! daemon run together with a session each; the container daemon is stopped; the host session
//! still accepts input, and the container's loss is reported within five seconds as that daemon's
//! state alone.
//!
//! The states are folded with the application's own `DaemonStates`, from what each connection
//! actually did, so a failure that leaked from one daemon to the other shows as the wrong state
//! and fails here.
//!
//! Behind `sandbox-real-runtime` (Principle VI). One test per target: the host daemon is spawned
//! from this process's environment.

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::{Duration, Instant};

use futures_util::StreamExt;
use micold_core::connect::{connect_at, Connected};
use micold_core::daemons::{DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime, DaemonState};
use micold_core::endpoint::DialAddress;
use micold_core::protocol::codec::Frame;

use sandbox_real_support::{
    credentials, fake_ai_cli, open_session, start_container_daemons, start_host_daemon, Terminal,
    FAKE_ECHO, FAKE_READY,
};

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

#[tokio::test]
async fn sandbox_real_multi_daemon_stopping_one_leaves_the_other_working() {
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_ai_cli(&dir.path().join("fake-bin"));
    let mut boxes = start_container_daemons(dir.path(), "md-multi", 17770, 1, &fake);
    let (host, mut host_conn, host_serial) =
        start_host_daemon(&dir.path().join("host"), DAEMON_BIN, &fake, "md-multi-host").await;

    // The application's state machine, one entry per daemon.
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host_id: DaemonId = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let box_id: DaemonId = registry
        .add(
            "Box",
            DaemonRuntime::Container(micold_core::daemons::ContainerSettings {
                profile: Default::default(),
                container_name: boxes[0].container.clone(),
                port: boxes[0].port,
            }),
            true,
        )
        .unwrap();
    let mut states = micold_core::daemons::DaemonStates::at_launch(&registry);

    let (mut box_conn, box_serial) = boxes[0].connect().await;
    states.apply(host_id, DaemonEvent::Connected);
    states.apply(box_id, DaemonEvent::Connected);

    // Both sessions run, each with its own answer (SC-001: input reaches the right session).
    let host_screen = open_session(&mut host_conn, &host.project, host.session, &host.log).await;
    let box_screen = open_session(
        &mut box_conn,
        &boxes[0].project,
        boxes[0].session,
        &boxes[0].log,
    )
    .await;
    let mut host_term = Terminal::new(
        &mut host_conn,
        host.session,
        host_screen,
        "host",
        &host.log,
        host_serial,
    );
    host_term
        .wait_for(FAKE_READY, Duration::from_secs(20))
        .await;
    host_term
        .converse(
            "one-host",
            &format!("{FAKE_ECHO}one-host"),
            Duration::from_secs(20),
        )
        .await;
    assert!(
        !host_term.screen.all().contains("one-box"),
        "the host session saw the container's input"
    );
    let mut box_term = Terminal::new(
        &mut box_conn,
        boxes[0].session,
        box_screen,
        &boxes[0].container,
        &boxes[0].log,
        box_serial,
    );
    box_term.wait_for(FAKE_READY, Duration::from_secs(20)).await;
    box_term
        .converse(
            "one-box",
            &format!("{FAKE_ECHO}one-box"),
            Duration::from_secs(20),
        )
        .await;
    assert!(
        !box_term.screen.all().contains("one-host"),
        "the container session saw the host's input"
    );

    // Stop the container daemon, and time how long its loss takes to be seen.
    let stopped_at = Instant::now();
    boxes[0].stop();
    let lost = loop {
        let remaining = Duration::from_secs(5).saturating_sub(stopped_at.elapsed());
        match tokio::time::timeout(remaining, box_term.conn.next()).await {
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => break e.to_string(),
            Ok(None) => break "connection closed".to_string(),
            Err(_) => panic!("the container daemon's loss was not seen within 5s (SC-002)"),
        }
    };
    states.apply(box_id, DaemonEvent::Lost(lost));

    // A redial fails too, which is what the application's retry loop would see.
    let redial = connect_at(
        &DialAddress::Loopback {
            port: boxes[0].port,
        },
        "multi-daemon-redial",
        &credentials(&boxes[0].token),
    )
    .await;
    assert!(
        !matches!(redial, Ok(Some(Connected::Ready(..)))),
        "a stopped container daemon accepted a connection"
    );

    // The host connection's own outcome goes through the same fold: it is lost only if its stream
    // errored or closed, which is what a leak from the container's failure would look like.
    match tokio::time::timeout(Duration::from_millis(500), host_term.conn.next()).await {
        Ok(Some(Err(e))) => {
            states.apply(host_id, DaemonEvent::Lost(e.to_string()));
        }
        Ok(Some(Ok(Frame::Grid(frame)))) if frame.session == host.session => {
            host_term.screen.apply(&frame)
        }
        Ok(None) => {
            states.apply(host_id, DaemonEvent::Lost("connection closed".into()));
        }
        _ => {}
    }

    assert!(
        matches!(states.state(box_id), Some(DaemonState::Reconnecting { .. })),
        "the stopped daemon is reconnecting, got {:?}",
        states.state(box_id)
    );
    assert_eq!(
        states.state(host_id),
        Some(&DaemonState::Connected),
        "the host daemon's state leaked from the container's failure (SC-002)"
    );

    // The host session is uninterrupted and still accepts input (SC-001, SC-002).
    host_term
        .converse(
            "after-stop",
            &format!("{FAKE_ECHO}after-stop"),
            Duration::from_secs(20),
        )
        .await;
}
