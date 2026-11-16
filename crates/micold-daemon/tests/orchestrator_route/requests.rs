//! FR-003, US2 scenario 1: a request reaches the daemon its ref names, and its answer reaches the
//! client that asked, and only that client.

use std::time::Duration;

use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::SessionId;

use crate::support::Orchestrator;
use crate::{Client, Daemon};

fn delete(req: u64) -> ClientMsg {
    ClientMsg::SessionDelete {
        req,
        session: SessionId::new(),
    }
}

#[tokio::test]
async fn a_request_is_answered_by_the_daemon_it_names_to_the_client_that_asked() {
    let orchestrator = Orchestrator::start().await;
    let _a = Daemon::start(&orchestrator, "alpha").await;
    let _b = Daemon::start(&orchestrator, "beta").await;
    let mut one = Client::connect(&orchestrator).await;
    let mut two = Client::connect(&orchestrator).await;
    one.open("alpha", 1, "one").await;
    one.open("beta", 1, "one").await;
    two.open("alpha", 1, "two").await;

    one.send("alpha", 1, &delete(10)).await;
    let got = one.recv_on("alpha", 1).await;
    assert_eq!(
        got.header.req,
        Some(10),
        "the daemon link stamps the answer's req"
    );
    assert!(
        matches!(
            got.msg,
            Some(
                DaemonMsg::OperationError { req: 10, .. } | DaemonMsg::OperationOk { req: 10, .. }
            )
        ),
        "{got:?}"
    );

    // The same conn number on another daemon is another conversation.
    one.send("beta", 1, &delete(11)).await;
    let got = one.recv_on("beta", 1).await;
    assert_eq!(got.header.req, Some(11));

    // The other client sees nothing of it.
    two.expect_quiet(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn two_conns_to_one_daemon_are_two_clients_of_it() {
    let orchestrator = Orchestrator::start().await;
    let daemon = Daemon::start(&orchestrator, "alpha").await;
    let mut client = Client::connect(&orchestrator).await;
    client.open("alpha", 1, "first").await;
    client.open("alpha", 2, "second").await;
    // Each conn is its own connection of the daemon, with its own attachment.
    assert_eq!(daemon.state.client_count(), 2);
}
