//! User Story 2 (#1724, FR-005, SC-002): the orchestrator pings each link; a silent daemon lists
//! `offline` with its last answer's time within the deadline, and `online` again on its next frame.
//! The clock is the test's own, so no test waits nine real seconds.

use std::time::{Duration, Instant};

use micold_core::protocol::keepalive::{LIVENESS_DEADLINE, PING_INTERVAL};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};

use crate::support::{announcement, FakeDaemon, Orchestrator, WITHIN};

/// The listed record of `id`, once `ready` holds for it.
async fn record_when(
    orchestrator: &Orchestrator,
    id: &str,
    ready: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let started = Instant::now();
    loop {
        let daemons = orchestrator.daemons().await;
        let record = daemons.iter().find(|d| d["id"] == id).expect("listed");
        if ready(record) {
            return record.clone();
        }
        assert!(started.elapsed() < WITHIN, "{id} never became ready: {record}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn an_idle_link_gets_a_link_ping_every_ping_interval() {
    let orchestrator = Orchestrator::start().await;
    let mut daemon = FakeDaemon::link(&orchestrator, &announcement("a")).await;

    orchestrator.clock.advance(PING_INTERVAL);
    let DaemonMsg::LinkPing { nonce: first } = daemon.next().await else {
        panic!("expected a LinkPing")
    };
    daemon.send(ClientMsg::LinkPong { nonce: first }).await;

    orchestrator.clock.advance(PING_INTERVAL);
    let DaemonMsg::LinkPing { nonce: second } = daemon.next().await else {
        panic!("expected a second LinkPing")
    };
    assert_ne!(first, second);
}

#[tokio::test]
async fn a_silent_daemon_lists_offline_and_its_next_frame_brings_it_back() {
    let orchestrator = Orchestrator::start().await;
    let start = orchestrator.clock.wall();
    let mut silent = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    let mut chatty = FakeDaemon::link(&orchestrator, &announcement("b")).await;

    // Halfway to the deadline "b" answers; "a" does not.
    orchestrator.clock.advance(Duration::from_secs(5));
    chatty.send(ClientMsg::LinkPong { nonce: 1 }).await;
    record_when(&orchestrator, "b", |b| b["last_seen"] == start + 5).await;
    let before = record_when(&orchestrator, "b", |_| true).await;

    // Past the deadline for "a" only.
    orchestrator
        .clock
        .advance(LIVENESS_DEADLINE - Duration::from_secs(4));
    let offline = record_when(&orchestrator, "a", |a| a["health"] == "offline").await;
    assert_eq!(offline["last_seen"], start, "the last answer's time");
    assert_eq!(record_when(&orchestrator, "b", |_| true).await, before);

    // Any frame is an answer.
    let now = orchestrator.clock.wall();
    silent.send(ClientMsg::LinkPong { nonce: 2 }).await;
    let online = record_when(&orchestrator, "a", |a| a["health"] == "online").await;
    assert_eq!(online["last_seen"], now);
    assert!(online["last_seen"].as_u64() > offline["last_seen"].as_u64());
    assert_eq!(record_when(&orchestrator, "b", |_| true).await, before);
}

#[tokio::test]
async fn a_dropped_link_turns_offline_once_the_deadline_passes() {
    let orchestrator = Orchestrator::start().await;
    let start = orchestrator.clock.wall();
    let linked = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    drop(linked);

    // A drop is silence: the record keeps its health until the deadline runs out.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        record_when(&orchestrator, "a", |_| true).await["health"],
        "online"
    );
    orchestrator.clock.advance(LIVENESS_DEADLINE);
    let offline = record_when(&orchestrator, "a", |a| a["health"] == "offline").await;
    assert_eq!(offline["last_seen"], start);
}
