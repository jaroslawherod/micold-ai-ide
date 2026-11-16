//! FR-005, US2 scenario 3: a request naming an unknown, `needs-update` or offline daemon is
//! refused with that reason, and nothing is relayed.

use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind};
use micold_core::protocol::routing::StreamEndReason;
use micold_core::session::SessionId;

use crate::support::{announcement, record_when, FakeDaemon, Orchestrator};
use crate::{Client, Daemon};

fn delete(req: u64) -> ClientMsg {
    ClientMsg::SessionDelete {
        req,
        session: SessionId::new(),
    }
}

#[tokio::test]
async fn an_unknown_daemon_is_not_found() {
    let orchestrator = Orchestrator::start().await;
    let mut client = Client::connect(&orchestrator).await;
    client.send("nowhere", 1, &delete(3)).await;
    let got = client.recv_on("nowhere", 1).await;
    assert_eq!(got.header.req, Some(3));
    assert!(matches!(
        got.msg,
        Some(DaemonMsg::OperationError {
            req: 3,
            kind: ErrorKind::NotFound,
            ..
        })
    ));
    // No request to answer: the conversation ends.
    client
        .send("nowhere", 2, &ClientMsg::Ping { nonce: 1 })
        .await;
    let got = client.recv_on("nowhere", 2).await;
    assert_eq!(
        got.msg,
        Some(DaemonMsg::StreamEnded {
            reason: StreamEndReason::NotFound
        })
    );
}

#[tokio::test]
async fn a_daemon_needing_an_update_is_refused_with_that_reason() {
    let orchestrator = Orchestrator::start().await;
    let mut old = announcement("stale");
    old.protocol_version = 70;
    let mut link = FakeDaemon::dial(&orchestrator).await;
    link.announce(&old).await;
    record_when(&orchestrator, "stale", |r| r["health"] == "needs-update").await;

    let mut client = Client::connect(&orchestrator).await;
    client.send("stale", 1, &delete(4)).await;
    let got = client.recv_on("stale", 1).await;
    match got.msg {
        Some(DaemonMsg::OperationError {
            req, kind, message, ..
        }) => {
            assert_eq!(
                (req, kind, message.as_str()),
                (4, ErrorKind::Refused, "needs-update")
            );
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn an_offline_daemon_is_refused_with_that_reason() {
    let orchestrator = Orchestrator::start().await;
    let daemon = Daemon::start(&orchestrator, "alpha").await;
    daemon.drop_link();
    // A restart lists every remembered daemon `offline` until its link is back.
    let orchestrator = orchestrator.restart().await;
    record_when(&orchestrator, "alpha", |r| r["health"] == "offline").await;

    let mut client = Client::connect(&orchestrator).await;
    client.send("alpha", 1, &delete(5)).await;
    let got = client.recv_on("alpha", 1).await;
    match got.msg {
        Some(DaemonMsg::OperationError {
            req, kind, message, ..
        }) => {
            assert_eq!(
                (req, kind, message.as_str()),
                (5, ErrorKind::Refused, "offline")
            );
        }
        other => panic!("{other:?}"),
    }
}
