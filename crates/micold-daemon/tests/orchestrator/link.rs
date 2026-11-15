//! The link task's own timers (contracts/link.md): a connection that sends nothing is closed at the
//! deadline, a reservation without a `Hello` is released then, and a link whose transport closed
//! after the deadline ends its task. All run on tokio's paused clock, so the nine seconds cost
//! nothing; the waits are bounded in real time, and a stretched timer fails the deadline bound.

use std::time::Duration;

use micold_core::protocol::keepalive::LIVENESS_DEADLINE;

use crate::support::{announcement, record_when, FakeDaemon, Orchestrator};

/// Tasks alive on this runtime: the test's orchestrator, one per open connection.
fn alive_tasks() -> usize {
    tokio::runtime::Handle::current()
        .metrics()
        .num_alive_tasks()
}

/// Closed at the deadline, neither before nor long after: a stretched timer would still close
/// on the paused clock, only later in virtual time.
fn assert_closed_at_the_deadline(started: tokio::time::Instant) {
    let elapsed = started.elapsed();
    assert!(
        elapsed >= LIVENESS_DEADLINE,
        "closed too early: {elapsed:?}"
    );
    assert!(
        elapsed <= LIVENESS_DEADLINE + Duration::from_secs(1),
        "closed too late: {elapsed:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_connection_that_sends_no_preamble_is_closed_at_the_deadline() {
    let orchestrator = Orchestrator::start().await;
    let mut silent = FakeDaemon::dial(&orchestrator).await;
    let started = tokio::time::Instant::now();

    silent.expect_closed_without_verdict().await;

    assert_closed_at_the_deadline(started);
}

#[tokio::test(start_paused = true)]
async fn an_accepted_link_that_never_sends_hello_is_closed_and_its_reservation_released() {
    let orchestrator = Orchestrator::start().await;
    let mut stalled = FakeDaemon::dial(&orchestrator)
        .await
        .accepted(&announcement("a"))
        .await;
    let started = tokio::time::Instant::now();

    stalled.expect_closed().await;

    assert_closed_at_the_deadline(started);
    // The test's own clock never moved, so only the link task can have released the id.
    assert!(orchestrator.daemons().await.is_empty());
    let _linked = FakeDaemon::link(&orchestrator, &announcement("a")).await;
}

#[tokio::test(start_paused = true)]
async fn a_closed_link_past_the_deadline_ends_its_task_with_no_relink() {
    let orchestrator = Orchestrator::start().await;
    tokio::time::advance(Duration::from_secs(1)).await;
    let idle = alive_tasks();
    let linked = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    assert!(alive_tasks() > idle, "the link has a task");

    drop(linked);
    orchestrator.clock.advance(LIVENESS_DEADLINE);
    record_when(&orchestrator, "a", |a| a["health"] == "offline").await;

    for _ in 0..10 {
        if alive_tasks() == idle {
            return;
        }
        tokio::time::advance(Duration::from_secs(1)).await;
    }
    panic!(
        "the link task still runs: {} alive, {idle} idle",
        alive_tasks()
    );
}

#[tokio::test]
async fn a_daemon_gone_before_it_reads_welcome_frees_its_id_at_once() {
    let orchestrator = Orchestrator::start().await;
    let mut linked = FakeDaemon::dial(&orchestrator)
        .await
        .accepted(&announcement("a"))
        .await;
    linked.hello().await;
    drop(linked);
    record_when(&orchestrator, "a", |a| a["health"] == "online").await;

    // The test clock does not move: the id is free because the link task released it.
    let _again = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    record_when(&orchestrator, "a", |a| a["health"] == "online").await;
}
