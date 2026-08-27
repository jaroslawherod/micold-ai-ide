//! The sandbox transport carries the protocol identically (feature 027, US2 scenario 6, SC-001).
//!
//! A containerised daemon is reached over **loopback TCP** rather than a Unix socket, because a
//! bind-mounted socket does not survive Docker Desktop's file sharing (research R1). That is a
//! change to the one thing every session depends on, so it is exercised here over a real
//! `TcpListener` rather than the in-memory duplex the other daemon tests use — a duplex would prove
//! the codec works and say nothing about whether TCP does.
//!
//! What it establishes: the handshake, the authentication step, and a full request/response round
//! trip all behave over TCP exactly as they do over the socket. What it cannot establish is that
//! the container publishes the port correctly — that is `evidence/us1-isolation.md`'s job.

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::auth::Token;
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use std::sync::Arc;
use tokio_util::codec::Framed;

fn hello(token: Option<&Token>) -> ClientMsg {
    ClientMsg::Hello {
        protocol_version: PROTOCOL_VERSION,
        schema_hash: SCHEMA_HASH,
        client_build: "test-client".into(),
        client_instance: micold_core::protocol::messages::ClientInstance::current(),
        client_package_version: PACKAGE_VERSION.into(),
        auth_token: token.map(|t| micold_core::protocol::messages::PresentedToken::new(t.as_str())),
        client_fingerprint: BUILD_FINGERPRINT.into(),
        require_fingerprint_match: false,
    }
}

/// Bind a loopback listener, serve one connection from it, and hand back a framed client.
///
/// Port 0, so the OS picks a free one: a fixed port would make these tests fail against whatever
/// else happened to be listening, including a real sandbox on this machine.
async fn connected(
    state: Arc<micold_daemon::state::DaemonState>,
) -> Framed<tokio::net::TcpStream, ClientCodec> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (conn, _peer) = listener.accept().await.unwrap();
        let _ = micold_daemon::server::serve_connection(state, conn).await;
    });

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.set_nodelay(true).unwrap();
    Framed::new(stream, ClientCodec::new())
}

fn ephemeral() -> Arc<micold_daemon::state::DaemonState> {
    Arc::new(micold_daemon::state::DaemonState::new(
        micold_daemon::catalog::Catalog::ephemeral(),
    ))
}

#[tokio::test]
async fn the_handshake_completes_over_loopback_tcp() {
    let mut client = connected(ephemeral()).await;
    client.send(Frame::Control(hello(None))).await.unwrap();

    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
}

/// SC-001: what runs over the new transport is the same protocol, not a reduced one. A round trip
/// after the handshake is the cheapest proof that framing survives a real socket — TCP delivers a
/// byte stream with no message boundaries, so a codec that happened to work over a duplex (which
/// preserves write sizes) could still fail here.
#[tokio::test]
async fn a_full_round_trip_works_over_loopback_tcp() {
    let mut client = connected(ephemeral()).await;
    client.send(Frame::Control(hello(None))).await.unwrap();
    let _welcome = client.next().await.unwrap().unwrap();

    client
        .send(Frame::Control(ClientMsg::Ping { nonce: 42 }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        // The nonce comes back, so this is a reply to *this* ping and not a stray frame.
        Frame::Control(DaemonMsg::Pong { nonce }) => assert_eq!(nonce, 42),
        other => panic!("expected Pong, got {other:?}"),
    }
}

/// Research R1's whole reason for existing: this transport has no filesystem permission behind it,
/// so the token is what stands between a local process and the daemon.
#[tokio::test]
async fn a_daemon_expecting_a_token_refuses_a_client_without_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("token");
    let token = Token::generate();
    token.write_to(&path).unwrap();

    let state = ephemeral();
    state.set_auth_token(&path).unwrap();

    let mut client = connected(state).await;
    client.send(Frame::Control(hello(None))).await.unwrap();

    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Refused {
            reason: RefusalReason::AuthRejected,
        }) => {}
        other => panic!("expected AuthRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn the_right_token_is_accepted_over_the_same_transport() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("token");
    let token = Token::generate();
    token.write_to(&path).unwrap();

    let state = ephemeral();
    state.set_auth_token(&path).unwrap();

    let mut client = connected(state).await;
    client
        .send(Frame::Control(hello(Some(&token))))
        .await
        .unwrap();

    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
}

/// FR-014: a client going away does not take the daemon with it. Two connections in sequence
/// against one state, with the first dropped — which is what a client restart looks like from the
/// daemon's side, and the property the whole daemon exists to provide.
#[tokio::test]
async fn a_second_client_connects_after_the_first_disconnects() {
    let state = ephemeral();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let serving = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            let (conn, _peer) = listener.accept().await.unwrap();
            let state = Arc::clone(&serving);
            tokio::spawn(async move {
                let _ = micold_daemon::server::serve_connection(state, conn).await;
            });
        }
    });

    // First client: handshake, then hang up.
    {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut client = Framed::new(stream, ClientCodec::new());
        client.send(Frame::Control(hello(None))).await.unwrap();
        let _ = client.next().await.unwrap().unwrap();
    }

    // Second client against the same daemon state: the catalogue is still there to be sent.
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut client = Framed::new(stream, ClientCodec::new());
    client.send(Frame::Control(hello(None))).await.unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { catalog, .. }) => {
            // The point is not what is *in* it — an ephemeral catalog is empty — but that the
            // daemon served a second client at all, from the state the first one left behind.
            let _ = catalog;
        }
        other => panic!("expected Welcome, got {other:?}"),
    }
}

/// FR-024d through the **real accept path**, not just the pure evaluator.
///
/// The scenario is the one FR-024c makes supported and therefore likely: a maintainer rebuilds the
/// application, forgets to rebuild the `:dev` image, and reconnects. All three of the handshake's
/// existing identity constants match — the daemon in the image was built from the same release —
/// so without the fingerprint the connection succeeds and the daemon then misbehaves in ways that
/// look like bugs in the new code.
#[tokio::test]
async fn a_stale_development_image_is_refused_with_its_tag_named() {
    let mut client = connected(ephemeral()).await;
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: "0000000000000000".into(),
            // Set by the client because the client is what knows the image is a local build.
            require_fingerprint_match: true,
        }))
        .await
        .unwrap();

    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Refused {
            reason:
                RefusalReason::StaleDevImage {
                    client_fingerprint,
                    daemon_fingerprint,
                    ..
                },
        }) => {
            assert_eq!(client_fingerprint, "0000000000000000");
            assert_eq!(daemon_fingerprint, BUILD_FINGERPRINT);
        }
        other => panic!("expected StaleDevImage, got {other:?}"),
    }
}

/// The other half, and the one that would break every normal install if it were wrong: a released
/// image whose daemon was built separately carries a different fingerprint, legitimately, and must
/// still connect.
#[tokio::test]
async fn a_released_image_connects_despite_a_fingerprint_difference() {
    let mut client = connected(ephemeral()).await;
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: "0000000000000000".into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();

    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
}
