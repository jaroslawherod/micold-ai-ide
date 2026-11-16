//! FR-004: a daemon that stops reading its link ends only the routes toward it. The client's read
//! loop never waits on that link, so its routes to other daemons carry on.

use std::time::Duration;

use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::routing::StreamEndReason;

use crate::support::{announcement, record_when, FakeDaemon, Orchestrator};
use crate::{Client, Daemon};

#[tokio::test]
async fn a_daemon_that_stops_reading_does_not_hold_the_clients_other_routes() {
    let orchestrator = Orchestrator::start().await;
    let _alpha = Daemon::start(&orchestrator, "alpha").await;
    // The fake daemon links, then reads nothing more.
    let _stuck = FakeDaemon::link(&orchestrator, &announcement("stuck")).await;
    record_when(&orchestrator, "stuck", |r| r["health"] == "online").await;
    let mut client = Client::connect(&orchestrator).await;
    client.open("alpha", 1, "c").await;

    // Far more than the link's queue and the socket's buffers hold.
    for req in 1..=2_000_u64 {
        client
            .send_frame("stuck", 7, Some(req), vec![0; 60_000])
            .await;
    }

    // The client's other route answers at once, while the stuck link is still full.
    let started = std::time::Instant::now();
    client.send("alpha", 1, &ClientMsg::Ping { nonce: 5 }).await;
    assert!(matches!(
        client.next_msg("alpha", 1).await,
        DaemonMsg::Pong { nonce: 5 }
    ));
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the other route was held back: {:?}",
        started.elapsed()
    );
    // The route toward the stuck daemon ended, its requests reported unknown first (the daemon is
    // the slow consumer of that route).
    assert_eq!(
        client.expect_ended("stuck", 7).await,
        StreamEndReason::SlowConsumer
    );
}
