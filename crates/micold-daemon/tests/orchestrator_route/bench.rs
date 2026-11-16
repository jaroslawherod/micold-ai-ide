//! SC-003, US5: routing through the orchestrator is not noticeably slower. One daemon, one client
//! talking to it directly (its own connection) and one through the orchestrator; 1,000 requests
//! (`Ping` → `Pong`) and 1,000 4 KiB input chunks at 100/s (each followed by a `Ping` on the same
//! conn, so its `Pong` arrives once the chunk went through). The p95 of the routed path may be at
//! most 5 ms (requests) and 16 ms (chunks) above the direct one. `#[ignore]`: about 15 s of wall
//! time; run with `-- --ignored bench --nocapture` for the numbers.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{hello_message, Credentials};
use micold_core::protocol::codec::raw_frame_codec;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::routing::{control_body, control_payload};
use micold_core::session::SessionId;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::support::Orchestrator;
use crate::{Client, Daemon};

const COUNT: usize = 1_000;
const CHUNK: usize = 4 * 1024;
const RATE: Duration = Duration::from_millis(10);

/// A client on the daemon's own connection.
struct Direct(Framed<tokio::io::DuplexStream, LengthDelimitedCodec>);

impl Direct {
    async fn connect(state: Arc<DaemonState>) -> Self {
        let (ours, theirs) = tokio::io::duplex(1 << 20);
        tokio::spawn(micold_daemon::server::serve_connection(state, theirs));
        let mut direct = Self(Framed::new(ours, raw_frame_codec()));
        direct
            .send(&hello_message("bench-direct", &Credentials::default()))
            .await;
        assert!(matches!(direct.next().await, DaemonMsg::Welcome { .. }));
        direct
    }

    async fn send(&mut self, msg: &ClientMsg) {
        let payload = control_payload(msg).unwrap();
        self.0.send(Bytes::from(payload)).await.unwrap();
    }

    async fn next(&mut self) -> DaemonMsg {
        loop {
            let frame = self.0.next().await.unwrap().unwrap();
            if let Some(body) = control_body(&frame) {
                return serde_json::from_slice(body).unwrap();
            }
        }
    }

    async fn pong(&mut self, nonce: u64) {
        while !matches!(self.next().await, DaemonMsg::Pong { nonce: n } if n == nonce) {}
    }
}

async fn routed_pong(client: &mut Client, nonce: u64) {
    while !matches!(client.next_msg("alpha", 1).await, DaemonMsg::Pong { nonce: n } if n == nonce) {
    }
}

fn chunk(session: SessionId, serial: u64) -> ClientMsg {
    serde_json::from_value(serde_json::json!({"SessionInput": {
        "session": session,
        "serial": serial,
        "bytes": vec![b'x'; CHUNK],
    }}))
    .unwrap()
}

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() * 95 / 100]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "SC-003 benchmark, ~15 s"]
async fn bench_routed_p95_is_within_sc003_of_direct() {
    let orchestrator = Orchestrator::start().await;
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let daemon = Daemon::link_state(&orchestrator, "alpha", Arc::clone(&state));
    daemon.listed_online(&orchestrator).await;
    let mut direct = Direct::connect(Arc::clone(&state)).await;
    let mut routed = Client::connect(&orchestrator).await;
    routed.open("alpha", 1, "bench").await;
    let session = SessionId::new();

    let (mut d_req, mut r_req) = (Vec::new(), Vec::new());
    for n in 0..COUNT as u64 {
        let t = Instant::now();
        direct.send(&ClientMsg::Ping { nonce: n }).await;
        direct.pong(n).await;
        d_req.push(t.elapsed());
        let t = Instant::now();
        routed.send("alpha", 1, &ClientMsg::Ping { nonce: n }).await;
        routed_pong(&mut routed, n).await;
        r_req.push(t.elapsed());
    }

    let (mut d_chunk, mut r_chunk) = (Vec::new(), Vec::new());
    let mut tick = tokio::time::interval(RATE);
    for n in 0..COUNT as u64 {
        tick.tick().await;
        let nonce = 1_000_000 + n;
        let t = Instant::now();
        direct.send(&chunk(session, n)).await;
        direct.send(&ClientMsg::Ping { nonce }).await;
        direct.pong(nonce).await;
        d_chunk.push(t.elapsed());
        let t = Instant::now();
        routed.send("alpha", 1, &chunk(session, n)).await;
        routed.send("alpha", 1, &ClientMsg::Ping { nonce }).await;
        routed_pong(&mut routed, nonce).await;
        r_chunk.push(t.elapsed());
    }

    let (d_req, r_req, d_chunk, r_chunk) = (p95(d_req), p95(r_req), p95(d_chunk), p95(r_chunk));
    println!(
        "SC-003 p95: requests direct {d_req:?} routed {r_req:?} (+{:?}); \
         4 KiB chunks at 100/s direct {d_chunk:?} routed {r_chunk:?} (+{:?})",
        r_req.saturating_sub(d_req),
        r_chunk.saturating_sub(d_chunk),
    );
    assert!(
        r_req.saturating_sub(d_req) <= Duration::from_millis(5),
        "requests: routed p95 {r_req:?} vs direct {d_req:?}"
    );
    assert!(
        r_chunk.saturating_sub(d_chunk) <= Duration::from_millis(16),
        "chunks: routed p95 {r_chunk:?} vs direct {d_chunk:?}"
    );
}
