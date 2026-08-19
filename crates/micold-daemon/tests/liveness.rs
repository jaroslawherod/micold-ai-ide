//! Phase 7 (US5) — half-open-connection liveness (T068a, FR-026, SC-011).
//!
//! The keepalive deadline itself is a pure, time-injected [`Keepalive`] state machine (unit-tested
//! in `micold-core`). Here we bolt it to the *real* daemon `Pong` responder over an in-memory socket
//! and drive a synthetic clock, so the SC-011 guarantee is exercised end to end without waiting real
//! seconds: a daemon that keeps answering is never spuriously reaped, and a daemon gone silent (a
//! half-open link — no FIN ever arrives) is surfaced as expired inside the 10 s budget.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::keepalive::{Keepalive, KeepaliveAction, LIVENESS_DEADLINE};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

#[tokio::test]
async fn a_responsive_daemon_is_never_reaped() {
    // A healthy connection to the real daemon. We simulate 15 s of once-a-second keepalive ticks
    // against a synthetic clock, sending a real `Ping` whenever the state machine asks and folding
    // the real `Pong` back in. Across the whole span — well past the 9 s deadline — the connection is
    // never declared expired: a daemon that answers keeps the client alive (SC-011, no false reap).
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    tokio::spawn(micold_daemon::server::serve_connection(state, server_io));
    let mut client = Framed::new(client_io, ClientCodec::new());

    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }

    let t0 = Instant::now();
    let mut ka = Keepalive::new(t0);
    let mut nonce = 0u64;
    for sec in 1..=15 {
        let now = t0 + Duration::from_secs(sec);
        match ka.poll(now) {
            KeepaliveAction::Expired => panic!("a responsive daemon was reaped at t+{sec}s"),
            KeepaliveAction::SendPing => {
                nonce += 1;
                client
                    .send(Frame::Control(ClientMsg::Ping { nonce }))
                    .await
                    .unwrap();
                match client.next().await.unwrap().unwrap() {
                    Frame::Control(DaemonMsg::Pong { nonce: got }) => {
                        assert_eq!(got, nonce);
                        ka.on_daemon_frame(now); // real proof of life resets the deadline
                    }
                    other => panic!("expected Pong, got {other:?}"),
                }
            }
            KeepaliveAction::Idle => {}
        }
    }
    assert!(
        nonce >= 4,
        "expected several probes over 15 s, sent {nonce}"
    );
}

#[tokio::test]
async fn a_half_open_connection_is_surfaced_within_10s() {
    // The daemon connection exists but the peer has gone silent without a FIN (power loss, a severed
    // link) — the reader would block forever. The keepalive turns that silence into an explicit
    // expiry within the SC-011 budget. We drive the synthetic clock and never feed a frame back.
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    tokio::spawn(micold_daemon::server::serve_connection(state, server_io));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    let _welcome = client.next().await.unwrap().unwrap();

    let t0 = Instant::now();
    let mut ka = Keepalive::new(t0);

    // Tick every second; the daemon never answers (we don't read/feed). Find when it is surfaced.
    let mut surfaced_at = None;
    for sec in 1..=10 {
        if ka.poll(t0 + Duration::from_secs(sec)) == KeepaliveAction::Expired {
            surfaced_at = Some(sec);
            break;
        }
    }
    let at = surfaced_at.expect("a silent half-open connection was never surfaced");
    assert!(
        at <= 10,
        "half-open must be surfaced within 10 s (SC-011), was {at}s"
    );
    assert!(
        LIVENESS_DEADLINE < Duration::from_secs(10),
        "the deadline itself must sit inside the 10 s budget"
    );
}
