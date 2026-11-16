//! FR-004, US2 scenarios 2 and 4: streams reach every attached client in order, none lost, and a
//! takeover is seen by both clients as it is with a direct connection.

use std::sync::Arc;

use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_daemon::state::DaemonState;

use crate::support::{within, Orchestrator};
use crate::{shell_catalog, Client, Daemon};

#[tokio::test]
async fn frames_arrive_in_the_order_they_were_sent_none_lost() {
    let orchestrator = Orchestrator::start().await;
    let _daemon = Daemon::start(&orchestrator, "alpha").await;
    let mut client = Client::connect(&orchestrator).await;
    client.open("alpha", 1, "ordered").await;
    for nonce in 1..=300 {
        client.send("alpha", 1, &ClientMsg::Ping { nonce }).await;
    }
    for nonce in 1..=300 {
        match client.next_msg("alpha", 1).await {
            DaemonMsg::Pong { nonce: got } => assert_eq!(got, nonce),
            other => panic!("expected Pong {nonce}, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_takeover_is_seen_by_both_clients() {
    let orchestrator = Orchestrator::start().await;
    let _daemon = Daemon::start(&orchestrator, "alpha").await;
    let mut one = Client::connect(&orchestrator).await;
    let mut two = Client::connect(&orchestrator).await;
    one.open("alpha", 1, "window-one").await;
    two.open("alpha", 1, "window-two").await;
    let attach = |force| ClientMsg::Attach {
        project: "/proj".into(),
        force,
    };

    one.send("alpha", 1, &attach(false)).await;
    assert!(matches!(
        one.next_msg("alpha", 1).await,
        DaemonMsg::Attached { .. }
    ));

    two.send("alpha", 1, &attach(false)).await;
    match two.next_msg("alpha", 1).await {
        DaemonMsg::Refused {
            reason: RefusalReason::ProjectBusy { holder, .. },
        } => assert_eq!(holder.build, "window-one"),
        other => panic!("expected ProjectBusy, got {other:?}"),
    }

    two.send("alpha", 1, &attach(true)).await;
    assert!(matches!(
        two.next_msg("alpha", 1).await,
        DaemonMsg::Attached { .. }
    ));
    match one.next_msg("alpha", 1).await {
        DaemonMsg::Displaced { by, .. } => assert_eq!(by.build, "window-two"),
        other => panic!("expected Displaced, got {other:?}"),
    }
}

#[tokio::test]
async fn a_terminal_stream_arrives_through_the_relay() {
    let orchestrator = Orchestrator::start().await;
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (catalog, session) = shell_catalog(project.path(), store.path());
    let state = Arc::new(DaemonState::new(catalog));
    let daemon = Daemon::link_state(&orchestrator, "alpha", Arc::clone(&state));
    daemon.listed_online(&orchestrator).await;
    let mut client = Client::connect(&orchestrator).await;
    client.open("alpha", 1, "viewer").await;

    let project_path = project.path().to_path_buf();
    client
        .send(
            "alpha",
            1,
            &ClientMsg::Attach {
                project: project_path.clone(),
                force: false,
            },
        )
        .await;
    client
        .send(
            "alpha",
            1,
            &ClientMsg::SessionStart {
                session,
                allow_additional_session: false,
            },
        )
        .await;
    client
        .send(
            "alpha",
            1,
            &ClientMsg::SetViewedSession {
                project: project_path,
                session: Some(session),
            },
        )
        .await;
    // A grid frame: opaque, copied through the orchestrator without being decoded.
    within("a grid frame", async {
        loop {
            let got = client.recv().await;
            if got.header.conn.0 == 1 && got.msg.is_none() {
                return;
            }
        }
    })
    .await;
    for pty in state.remove_session(session) {
        let _ = pty.kill();
    }
}
