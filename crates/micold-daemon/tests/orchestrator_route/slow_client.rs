//! FR-004: one client that stops reading ends only its own route (`slow-consumer`); another
//! client on the same daemon keeps its rate and the link goes on.

use std::time::Duration;

use futures_util::SinkExt;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::routing::StreamEndReason;

use crate::support::Orchestrator;
use crate::{Client, Daemon};

#[tokio::test]
async fn a_client_that_stops_reading_loses_its_own_route_and_nobody_elses() {
    let orchestrator = Orchestrator::start().await;
    let _daemon = Daemon::start(&orchestrator, "alpha").await;
    let mut slow = Client::connect(&orchestrator).await;
    let mut fast = Client::connect(&orchestrator).await;
    slow.open("alpha", 1, "slow").await;
    fast.open("alpha", 1, "fast").await;

    // The slow client asks for far more answers than its socket and queues can hold, and reads none.
    for nonce in 0..20_000 {
        let frame = slow.frame("alpha", 1, &ClientMsg::Ping { nonce });
        slow.sink().feed(frame).await.expect("written");
    }
    SinkExt::<bytes::Bytes>::flush(slow.sink())
        .await
        .expect("flushed");

    // Meanwhile the other client round-trips at full speed.
    let started = std::time::Instant::now();
    for nonce in 1..=200 {
        fast.send("alpha", 1, &ClientMsg::Ping { nonce }).await;
        match fast.next_msg("alpha", 1).await {
            DaemonMsg::Pong { nonce: got } => assert_eq!(got, nonce),
            other => panic!("{other:?}"),
        }
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the fast client was held back: {:?}",
        started.elapsed()
    );

    // The slow client reads at last: its route was ended, with the reason.
    assert_eq!(
        slow.expect_ended("alpha", 1).await,
        StreamEndReason::SlowConsumer
    );
    // The link still works for everyone.
    fast.send("alpha", 1, &ClientMsg::Ping { nonce: 999 }).await;
    assert!(matches!(
        fast.next_msg("alpha", 1).await,
        DaemonMsg::Pong { nonce: 999 }
    ));
}
