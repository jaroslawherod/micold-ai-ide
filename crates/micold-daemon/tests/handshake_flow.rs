//! T020 (handshake path) — the daemon accept path speaks the strict handshake end to end.
//!
//! Drives `server::serve_connection` over an in-memory duplex with a real `ClientCodec`: a matching
//! Hello gets a Welcome (then Ping→Pong); a mismatched Hello gets a Refused and a closed connection.

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use tokio_util::codec::Framed;

#[tokio::test]
async fn matching_handshake_gets_welcome_and_pong() {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let state = std::sync::Arc::new(micold_daemon::state::DaemonState::new(
        micold_daemon::catalog::Catalog::ephemeral(),
    ));
    let server = tokio::spawn(micold_daemon::server::serve_connection(state, server_io));
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

    // Post-handshake Ping is answered with Pong.
    client
        .send(Frame::Control(ClientMsg::Ping { nonce: 99 }))
        .await
        .unwrap();
    assert_eq!(
        client.next().await.unwrap().unwrap(),
        Frame::Control(DaemonMsg::Pong { nonce: 99 })
    );

    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn mismatched_handshake_is_refused_naming_both_sides() {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let state = std::sync::Arc::new(micold_daemon::state::DaemonState::new(
        micold_daemon::catalog::Catalog::ephemeral(),
    ));
    let server = tokio::spawn(micold_daemon::server::serve_connection(state, server_io));
    let mut client = Framed::new(client_io, ClientCodec::new());

    // A different protocol version must be refused.
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION + 1,
            schema_hash: SCHEMA_HASH,
            client_build: "stale-client".into(),
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
        Frame::Control(DaemonMsg::Refused {
            reason:
                RefusalReason::VersionMismatch {
                    client: c,
                    daemon: d,
                    ..
                },
        }) => {
            assert_eq!(c, PROTOCOL_VERSION + 1);
            assert_eq!(d, PROTOCOL_VERSION);
        }
        other => panic!("expected Refused::VersionMismatch, got {other:?}"),
    }

    // The daemon closes the connection after refusing.
    assert!(
        client.next().await.is_none(),
        "connection must close after refusal"
    );
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn build_mismatch_is_refused_distinctly_when_contract_still_matches() {
    // T093 (convergence F2, FR-022a/SC-009a) — a same-contract `.deb` upgrade must be caught end to
    // end through the real accept path, not only at the pure `handshake::evaluate` unit level
    // (`crates/micold-core/tests/handshake.rs`).
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let state = std::sync::Arc::new(micold_daemon::state::DaemonState::new(
        micold_daemon::catalog::Catalog::ephemeral(),
    ));
    let server = tokio::spawn(micold_daemon::server::serve_connection(state, server_io));
    let mut client = Framed::new(client_io, ClientCodec::new());

    // Matching contract, but a package version that differs from this build's own.
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "micold-ai-ide/0.0.0-stale".into(),
            client_package_version: "0.0.0-stale".into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();

    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Refused {
            reason:
                RefusalReason::BuildMismatch {
                    client_build,
                    daemon_build,
                },
        }) => {
            assert_eq!(client_build, "micold-ai-ide/0.0.0-stale");
            assert_eq!(daemon_build, micold_daemon::server::daemon_build());
        }
        other => panic!("expected Refused::BuildMismatch, got {other:?}"),
    }

    // Same as a contract mismatch: the daemon closes the connection after refusing.
    assert!(
        client.next().await.is_none(),
        "connection must close after refusal"
    );
    server.await.unwrap().unwrap();
}
