//! FR-004, FR-006, contract §5: a route's queue toward its client never loses a final answer
//! silently. Whatever a full queue drops, or a link drop finds still queued, is either delivered
//! or reported `OutcomeUnknown`; the frames of a route are numbered without a gap. These drive the
//! hub directly so the queue is full at a known frame.

use std::time::Duration;

use bytes::Bytes;
use micold_core::orchestrator::route::ClientKey;
use micold_core::protocol::messages::DaemonMsg;
use micold_core::protocol::routing::{
    control_body, control_payload, decode_routed_payload, ConnectionId, RoutedBody,
    StreamEndReason, WireDaemonId,
};
use micold_daemon::orchestrator::hub::{Hub, Opened, ROUTE_QUEUE, WRITER_QUEUE};
use tokio::sync::mpsc;

struct Setup {
    hub: Hub,
    daemon: WireDaemonId,
    link_conn: u64,
    client_rx: mpsc::Receiver<Bytes>,
    _link_rx: mpsc::Receiver<Bytes>,
}

/// A route whose client's writer holds one frame and is read by nobody yet.
fn setup() -> Setup {
    let hub = Hub::default();
    let daemon: WireDaemonId = "box".parse().unwrap();
    let (link_tx, link_rx) = mpsc::channel(WRITER_QUEUE);
    hub.register_link(&daemon, link_tx);
    let (writer, client_rx) = mpsc::channel(1);
    let client = hub.register_client(writer);
    let key = ClientKey {
        client,
        daemon: daemon.clone(),
        conn: ConnectionId(1),
    };
    let Some(Opened::Route { link_conn, .. }) = hub.open_route(&key, false) else {
        panic!("the route opens");
    };
    Setup {
        hub,
        daemon,
        link_conn,
        client_rx,
        _link_rx: link_rx,
    }
}

fn answer(setup: &Setup, req: u64) {
    let inner = control_payload(&DaemonMsg::Pong { nonce: req }).unwrap();
    setup
        .hub
        .from_daemon(&setup.daemon, setup.link_conn, Some(req), &inner);
}

/// Everything the client's writer receives until the route's `StreamEnded`: `(seq, req, msg)`.
async fn drain(rx: &mut mpsc::Receiver<Bytes>) -> Vec<(u64, Option<u64>, DaemonMsg)> {
    let mut got = Vec::new();
    loop {
        let payload = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .expect("the route ends")
            .expect("the writer stays open");
        let routed = decode_routed_payload(&payload).expect("a routed frame");
        let RoutedBody::Opaque(inner) = routed.body else {
            panic!("opaque expected");
        };
        let msg: DaemonMsg = serde_json::from_slice(control_body(inner).unwrap()).unwrap();
        let ended = matches!(msg, DaemonMsg::StreamEnded { .. });
        got.push((routed.header.seq.0, routed.header.req, msg));
        if ended {
            return got;
        }
    }
}

/// Every request is answered once or reported unknown once, in numbered frames with no gap.
fn assert_accounted(got: &[(u64, Option<u64>, DaemonMsg)], reqs: u64, reason: StreamEndReason) {
    for (index, (seq, _, _)) in got.iter().enumerate() {
        assert_eq!(*seq, index as u64 + 1, "frame {index} is out of sequence");
    }
    let mut seen = std::collections::BTreeSet::new();
    for (_, req, msg) in got {
        if let (Some(req), DaemonMsg::Pong { .. } | DaemonMsg::OutcomeUnknown { .. }) = (req, msg) {
            assert!(seen.insert(*req), "request {req} reported twice");
        }
    }
    assert_eq!(
        seen,
        (1..=reqs).collect(),
        "a request was neither answered nor reported unknown"
    );
    assert_eq!(
        got.last().map(|(_, _, msg)| msg),
        Some(&DaemonMsg::StreamEnded { reason })
    );
}

/// F1: the link drops while the route's queue is full of final answers: they are delivered, and
/// the requests still pending are `unknown`.
#[tokio::test]
async fn a_link_drop_with_a_full_queue_keeps_the_queued_answers() {
    let mut s = setup();
    let reqs = ROUTE_QUEUE as u64 + 2;
    for req in 1..=reqs {
        s.hub.forwarded(&s.daemon, s.link_conn, req);
    }
    // The queue takes ROUTE_QUEUE answers (the pump has not run); the last two stay pending.
    for req in 1..=ROUTE_QUEUE as u64 {
        answer(&s, req);
    }
    s.hub.link_lost(&s.daemon, None);
    let got = drain(&mut s.client_rx).await;
    assert_accounted(&got, reqs, StreamEndReason::DaemonOffline);
}

/// F2 and F5: the answer that does not fit is not counted as answered, so it is `unknown`; the
/// route ends `slow-consumer` with the frames before it kept and no gap in the numbering.
#[tokio::test]
async fn a_slow_route_reports_the_dropped_answer_unknown() {
    let mut s = setup();
    let reqs = ROUTE_QUEUE as u64 + 3;
    for req in 1..=reqs {
        s.hub.forwarded(&s.daemon, s.link_conn, req);
    }
    // ROUTE_QUEUE answers fit; the next does not, and the route ends.
    for req in 1..=ROUTE_QUEUE as u64 + 1 {
        answer(&s, req);
    }
    let got = drain(&mut s.client_rx).await;
    assert_accounted(&got, reqs, StreamEndReason::SlowConsumer);
    assert!(
        got.iter().any(|(_, req, msg)| {
            *req == Some(ROUTE_QUEUE as u64 + 1) && matches!(msg, DaemonMsg::OutcomeUnknown { .. })
        }),
        "the dropped answer is reported unknown"
    );
}
