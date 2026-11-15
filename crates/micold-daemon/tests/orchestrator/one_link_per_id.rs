//! User Story 4 (#1724, FR-007, SC-004): one live link per daemon id. A second link on a live or
//! reserved id is refused `AlreadyLinked`; a closed, silent or never-confirmed link gives way.

use std::time::{Duration, Instant};

use micold_core::orchestrator::{refusal_message, LinkVerdict, RefuseReason};
use micold_core::protocol::keepalive::{LIVENESS_DEADLINE, PING_INTERVAL};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::PROTOCOL_VERSION;

use crate::support::{announcement, record_when, FakeDaemon, LinkedDaemon, Orchestrator, WITHIN};

fn already_linked(verdict: &LinkVerdict) -> bool {
    matches!(
        verdict,
        LinkVerdict::Refused {
            reason: RefuseReason::AlreadyLinked,
            ..
        }
    )
}

/// Dial and announce `name` for id `a` until it is accepted, then `Hello`: the orchestrator may
/// still be noticing that the old transport ended.
async fn link_when_free(orchestrator: &Orchestrator, name: &str) -> LinkedDaemon {
    let mut announced = announcement("a");
    announced.name = name.to_owned();
    let started = Instant::now();
    loop {
        let mut daemon = FakeDaemon::dial(orchestrator).await;
        let verdict = daemon.announce(&announced).await;
        if verdict == LinkVerdict::Accepted {
            let mut linked = daemon.accepted_after_verdict();
            linked.hello().await;
            linked.expect_welcome().await;
            return linked;
        }
        assert!(already_linked(&verdict), "unexpected verdict {verdict:?}");
        assert!(started.elapsed() < WITHIN, "the id was never freed");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn a_second_link_is_refused_and_the_first_stays_online() {
    let orchestrator = Orchestrator::start().await;
    let mut first = FakeDaemon::link(&orchestrator, &announcement("a")).await;

    let mut rival = FakeDaemon::dial(&orchestrator).await;
    let verdict = rival.announce(&announcement("a")).await;
    assert!(already_linked(&verdict), "{verdict:?}");

    orchestrator.clock.advance(PING_INTERVAL);
    let DaemonMsg::LinkPing { nonce } = first.next().await else {
        panic!("the first link is still served")
    };
    first.send(ClientMsg::LinkPong { nonce }).await;
    // Only the pong, after the rival's claim, can move `last_seen` to the advanced wall time.
    let pinged_at = orchestrator.clock.wall();
    let record = record_when(&orchestrator, "a", |a| a["last_seen"] == pinged_at).await;
    assert_eq!(record["health"], "online");
}

#[tokio::test]
async fn a_closed_transport_is_replaced_without_waiting_for_the_deadline() {
    let orchestrator = Orchestrator::start().await;
    let first = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    drop(first);

    // The test clock does not move: the id is free because the transport ended.
    let _second = link_when_free(&orchestrator, "replacement").await;
    let record = record_when(&orchestrator, "a", |a| a["name"] == "replacement").await;
    assert_eq!(record["health"], "online");
}

#[tokio::test]
async fn a_silent_link_is_closed_when_a_new_link_takes_over() {
    let orchestrator = Orchestrator::start().await;
    let mut silent = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    orchestrator.clock.advance(LIVENESS_DEADLINE);
    record_when(&orchestrator, "a", |a| a["health"] == "offline").await;

    let mut announced = announcement("a");
    announced.name = "replacement".into();
    let _new = FakeDaemon::link(&orchestrator, &announced).await;
    silent.expect_closed().await;
    let record = record_when(&orchestrator, "a", |a| a["name"] == "replacement").await;
    assert_eq!(record["health"], "online");
}

#[tokio::test]
async fn a_hello_after_the_replaced_link_answered_is_refused_already_linked() {
    let orchestrator = Orchestrator::start().await;
    let mut silent = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    orchestrator.clock.advance(LIVENESS_DEADLINE);
    record_when(&orchestrator, "a", |a| a["health"] == "offline").await;

    // The new link reserves the id; before its `Hello`, the silent link answers and is live again.
    let mut announced = announcement("a");
    announced.name = "replacement".into();
    let mut late = FakeDaemon::dial(&orchestrator)
        .await
        .accepted(&announced)
        .await;
    silent.send(ClientMsg::LinkPong { nonce: 0 }).await;
    record_when(&orchestrator, "a", |a| a["health"] == "online").await;

    late.hello().await;
    match late.next().await {
        DaemonMsg::Refused {
            reason: RefusalReason::NotPermitted { detail },
        } => assert_eq!(
            detail,
            refusal_message(
                RefuseReason::AlreadyLinked,
                &announced.id,
                PROTOCOL_VERSION,
                PROTOCOL_VERSION
            )
        ),
        other => panic!("expected an already-linked refusal, got {other:?}"),
    }
    late.expect_closed().await;
    let record = record_when(&orchestrator, "a", |_| true).await;
    assert_ne!(record["name"], "replacement");
}

#[tokio::test]
async fn a_link_that_never_sends_hello_is_released_at_the_deadline() {
    let orchestrator = Orchestrator::start().await;
    let _stalled = FakeDaemon::dial(&orchestrator)
        .await
        .accepted(&announcement("a"))
        .await;

    let mut rival = FakeDaemon::dial(&orchestrator).await;
    assert!(already_linked(&rival.announce(&announcement("a")).await));

    orchestrator.clock.advance(LIVENESS_DEADLINE);
    let _linked = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    record_when(&orchestrator, "a", |a| a["health"] == "online").await;
}

#[tokio::test]
async fn of_two_daemons_dialling_one_id_at_once_exactly_one_is_accepted() {
    let orchestrator = Orchestrator::start().await;
    for run in 0..50 {
        let id = format!("race-{run}");
        let (mut left, mut right) = tokio::join!(
            FakeDaemon::dial(&orchestrator),
            FakeDaemon::dial(&orchestrator)
        );
        let announced = announcement(&id);
        let (l, r) = tokio::join!(left.announce(&announced), right.announce(&announced));
        let accepted = [&l, &r]
            .iter()
            .filter(|v| ***v == LinkVerdict::Accepted)
            .count();
        assert_eq!(accepted, 1, "run {run}: {l:?} / {r:?}");
        assert!(already_linked(&l) || already_linked(&r), "run {run}");
    }
}
