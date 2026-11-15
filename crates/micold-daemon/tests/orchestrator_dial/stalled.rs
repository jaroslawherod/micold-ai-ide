//! FR-011 (#1725, US4): the orchestrator's handshake sends are bounded by the liveness deadline,
//! like its steady-state sends. A dialer that keeps the connection open and stops reading cannot
//! hold a link task or an id reservation: at each of the four sends (`refuse`, `Accepted`,
//! `Welcome`, `Refused`) the link ends within `LIVENESS_DEADLINE` and the id is free again.
//!
//! A socket buffers far more than one verdict, so the stall needs a pipe smaller than a frame: the
//! orchestrator is driven through `Detached` over an in-memory duplex. The clock is paused, so the
//! deadline costs nothing; every wait is still bounded in real time by `within`.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{hello_message, Credentials};
use micold_core::orchestrator::{LinkAnnouncement, LinkVerdict, MemoryRegistryStore};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::keepalive::LIVENESS_DEADLINE;
use micold_core::protocol::messages::ClientMsg;
use micold_core::protocol::version::PROTOCOL_VERSION;
use micold_daemon::orchestrator::{Clock, Detached};
use tokio::io::DuplexStream;
use tokio::task::JoinHandle;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::support::{announcement, preamble_codec, within, TestClock};

/// What the pipe holds: less than any verdict or wire frame, so an unread one stalls its send.
const PIPE: usize = 8;

const ID: &str = "stalled";

fn orchestrator() -> Detached {
    Detached::new(
        TestClock::new(1_700_000_000) as Arc<dyn Clock>,
        Arc::new(MemoryRegistryStore::default()),
    )
    .expect("an orchestrator")
}

/// A dialer on a pipe of [`PIPE`] bytes, and the link task serving its other end.
fn dial(orchestrator: &Detached) -> (Framed<DuplexStream, LengthDelimitedCodec>, Link) {
    let (near, far) = tokio::io::duplex(PIPE);
    let task = tokio::spawn(orchestrator.serve(far));
    (Framed::new(near, preamble_codec()), Link(task))
}

struct Link(JoinHandle<io::Result<()>>);

impl Link {
    /// The link task ends with a timed-out send within the deadline of `since`.
    async fn ends_timed_out(self, since: tokio::time::Instant) {
        let ended = within("the link task ends", self.0)
            .await
            .expect("the link task did not panic");
        let elapsed = since.elapsed();
        assert!(
            elapsed <= LIVENESS_DEADLINE + Duration::from_secs(1),
            "the link task ended late: {elapsed:?}"
        );
        let error = ended.expect_err("a stalled send is an error");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut, "{error}");
    }
}

async fn announce(
    framed: &mut Framed<DuplexStream, LengthDelimitedCodec>,
    announcement: &LinkAnnouncement,
) {
    let body = serde_json::to_vec(announcement).expect("encodes");
    framed.send(Bytes::from(body)).await.expect("announced");
}

/// Announce, read `Accepted` and send `Hello` with `protocol_version`; then stop reading.
async fn hello_then_stall(
    orchestrator: &Detached,
    protocol_version: u32,
) -> (Framed<DuplexStream, ClientCodec>, Link) {
    let (mut framed, link) = dial(orchestrator);
    announce(&mut framed, &announcement(ID)).await;
    let verdict = within("a verdict arrives", framed.next())
        .await
        .expect("a verdict")
        .expect("a readable verdict");
    assert_eq!(
        serde_json::from_slice::<LinkVerdict>(&verdict).expect("a verdict"),
        LinkVerdict::Accepted
    );
    let mut framed = framed.map_codec(|_| ClientCodec::new());
    let mut hello = hello_message("stalled-dialer", &Credentials::default());
    let ClientMsg::Hello {
        protocol_version: announced,
        ..
    } = &mut hello
    else {
        unreachable!("hello_message builds a Hello")
    };
    *announced = protocol_version;
    framed
        .send(Frame::Control(hello))
        .await
        .expect("Hello sent");
    (framed, link)
}

/// The id is free: a new dial for it is accepted, not refused as already linked.
async fn id_is_free(orchestrator: &Detached) {
    let (mut framed, _link) = dial(orchestrator);
    announce(&mut framed, &announcement(ID)).await;
    let verdict = within("a verdict arrives", framed.next())
        .await
        .expect("a verdict")
        .expect("a readable verdict");
    assert_eq!(
        serde_json::from_slice::<LinkVerdict>(&verdict).expect("a verdict"),
        LinkVerdict::Accepted
    );
}

#[tokio::test(start_paused = true)]
async fn a_dialer_that_never_reads_its_refusal_does_not_hold_the_link_task() {
    let orchestrator = orchestrator();
    let (mut framed, link) = dial(&orchestrator);
    let mismatched = LinkAnnouncement {
        protocol_version: PROTOCOL_VERSION + 1,
        ..announcement(ID)
    };
    announce(&mut framed, &mismatched).await;
    let since = tokio::time::Instant::now();

    link.ends_timed_out(since).await;
    drop(framed);
}

#[tokio::test(start_paused = true)]
async fn a_dialer_that_never_reads_accepted_releases_its_reservation() {
    let orchestrator = orchestrator();
    let (mut framed, link) = dial(&orchestrator);
    announce(&mut framed, &announcement(ID)).await;
    let since = tokio::time::Instant::now();

    link.ends_timed_out(since).await;
    id_is_free(&orchestrator).await;
    drop(framed);
}

#[tokio::test(start_paused = true)]
async fn a_dialer_that_never_reads_welcome_frees_its_id() {
    let orchestrator = orchestrator();
    let (framed, link) = hello_then_stall(&orchestrator, PROTOCOL_VERSION).await;
    let since = tokio::time::Instant::now();

    link.ends_timed_out(since).await;
    id_is_free(&orchestrator).await;
    drop(framed);
}

#[tokio::test(start_paused = true)]
async fn a_dialer_that_never_reads_its_hello_refusal_does_not_hold_the_link_task() {
    let orchestrator = orchestrator();
    let (framed, link) = hello_then_stall(&orchestrator, PROTOCOL_VERSION + 1).await;
    let since = tokio::time::Instant::now();

    link.ends_timed_out(since).await;
    id_is_free(&orchestrator).await;
    drop(framed);
}
