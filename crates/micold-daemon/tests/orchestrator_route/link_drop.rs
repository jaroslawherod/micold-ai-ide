//! FR-006, FR-012, SC-006: when a daemon's link drops, a request in flight is answered
//! `OutcomeUnknown` (every time), the conversation ends `daemon-offline`, nothing is retried, and
//! the client re-attaches when the daemon is back.

use futures_util::StreamExt;
use micold_core::protocol::codec::raw_frame_codec;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::routing::{decode_routed_payload, StreamEndReason};
use micold_core::session::SessionId;

use crate::support::{announcement, record_when, within, FakeDaemon, Orchestrator};
use crate::{Client, Daemon};

fn delete(req: u64) -> ClientMsg {
    ClientMsg::SessionDelete {
        req,
        session: SessionId::new(),
    }
}

/// SC-006: 100 of 100 requests in flight across a drop are answered `unknown`, then the stream ends.
#[tokio::test]
async fn a_request_in_flight_across_a_link_drop_is_answered_unknown_every_time() {
    let orchestrator = Orchestrator::start().await;
    let mut client = Client::connect(&orchestrator).await;
    for round in 1..=100_u64 {
        let linked = FakeDaemon::link(&orchestrator, &announcement("box")).await;
        let mut daemon = linked.framed.map_codec(|_| raw_frame_codec());
        record_when(&orchestrator, "box", |r| r["health"] == "online").await;
        // The orchestrator must have forwarded the request before the link drops; the fake daemon
        // never answers it.
        client.send("box", round, &delete(round)).await;
        within("the request reaches the daemon", async {
            while let Some(Ok(payload)) = daemon.next().await {
                if decode_routed_payload(&payload).is_ok() {
                    return;
                }
            }
            panic!("the link ended before the request arrived");
        })
        .await;
        drop(daemon);

        let first = client.recv_on("box", round).await;
        assert_eq!(
            first.msg,
            Some(DaemonMsg::OutcomeUnknown { req: round }),
            "round {round}"
        );
        assert_eq!(first.header.req, Some(round));
        let second = client.recv_on("box", round).await;
        assert_eq!(
            second.msg,
            Some(DaemonMsg::StreamEnded {
                reason: StreamEndReason::DaemonOffline
            }),
            "round {round}"
        );
    }
}

/// A request answered before the drop is not reported unknown.
#[tokio::test]
async fn an_answered_request_is_not_reported_unknown() {
    let orchestrator = Orchestrator::start().await;
    let daemon = Daemon::start(&orchestrator, "alpha").await;
    let mut client = Client::connect(&orchestrator).await;
    client.open("alpha", 1, "c").await;
    client.send("alpha", 1, &delete(8)).await;
    let answer = client.recv_on("alpha", 1).await;
    assert_eq!(answer.header.req, Some(8));
    daemon.drop_link();
    assert_eq!(
        client.expect_ended("alpha", 1).await,
        StreamEndReason::DaemonOffline
    );
}

/// US2 scenario 5: after the daemon links again the client re-attaches and the stream resumes from
/// the daemon's current state.
#[tokio::test]
async fn the_client_reattaches_after_the_daemon_links_again() {
    let orchestrator = Orchestrator::start().await;
    let daemon = Daemon::start(&orchestrator, "alpha").await;
    let state = std::sync::Arc::clone(&daemon.state);
    let mut client = Client::connect(&orchestrator).await;
    client.open("alpha", 1, "c").await;
    let attach = ClientMsg::Attach {
        project: "/proj".into(),
        force: false,
    };
    client.send("alpha", 1, &attach).await;
    assert!(matches!(
        client.next_msg("alpha", 1).await,
        DaemonMsg::Attached { .. }
    ));

    daemon.drop_link();
    assert_eq!(
        client.expect_ended("alpha", 1).await,
        StreamEndReason::DaemonOffline
    );

    // The abort only drops the virtual conns: their serve tasks see end-of-stream and release the
    // attachment a moment later, so wait for the daemon to hold no client before relinking.
    within("the daemon releases the dropped conn", async {
        while state.client_count() != 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;

    let relinked = Daemon::link_state(&orchestrator, "alpha", state);
    relinked.listed_online(&orchestrator).await;
    client.open("alpha", 2, "c").await;
    client.send("alpha", 2, &attach).await;
    // The first conn's attachment went with its connection, so the project is free again.
    match client.next_msg("alpha", 2).await {
        DaemonMsg::Attached { .. } => {}
        other => panic!("expected Attached, got {other:?}"),
    }
}
