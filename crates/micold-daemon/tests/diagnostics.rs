//! Phase 10 (T080) — the diagnostics RPC surface end to end (FR-043–046).
//!
//! Drives `LogLocationRequest` / `RecentErrorsRequest` / `SetLogLevel` over an in-memory socket
//! against a `DaemonState` seeded with a test diagnostics handle, asserting each reply and that an
//! invalid log directive is refused specifically rather than silently accepted.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind, LogEntry, LogSink};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::catalog::Catalog;
use micold_daemon::logging::Logging;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
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
    client
}

async fn next_control(client: &mut Client) -> DaemonMsg {
    loop {
        match client.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { .. }) => continue,
            Frame::Control(msg) => return msg,
            Frame::Grid(_) => continue,
        }
    }
}

#[tokio::test]
async fn diagnostics_rpcs_serve_location_recent_errors_and_level() {
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let logging = Logging::in_memory();
    // Seed a couple of recent errors so RecentErrors returns real content.
    for i in 0..3 {
        logging.push_error_for_test(LogEntry {
            timestamp_secs: 100 + i,
            level: "ERROR".into(),
            target: "micold_daemon".into(),
            message: format!("boom {i}"),
        });
    }
    state.set_diagnostics(logging);
    let mut client = connect(&state).await;

    // LogLocation reports the active sink (Stderr for the in-memory handle) and no file path.
    client
        .send(Frame::Control(ClientMsg::LogLocationRequest { req: 1 }))
        .await
        .unwrap();
    match next_control(&mut client).await {
        DaemonMsg::LogLocation { req, path, sink } => {
            assert_eq!(req, 1);
            assert_eq!(path, None);
            assert_eq!(sink, LogSink::Stderr);
        }
        other => panic!("expected LogLocation, got {other:?}"),
    }

    // RecentErrors returns the seeded entries, newest last, capped at the requested limit.
    client
        .send(Frame::Control(ClientMsg::RecentErrorsRequest {
            req: 2,
            limit: 2,
        }))
        .await
        .unwrap();
    match next_control(&mut client).await {
        DaemonMsg::RecentErrors { req, entries } => {
            assert_eq!(req, 2);
            assert_eq!(entries.len(), 2, "limit is honoured");
            assert_eq!(entries.last().unwrap().message, "boom 2", "newest last");
        }
        other => panic!("expected RecentErrors, got {other:?}"),
    }

    // A valid level change is acknowledged.
    client
        .send(Frame::Control(ClientMsg::SetLogLevel {
            req: 3,
            directives: "debug".into(),
        }))
        .await
        .unwrap();
    match next_control(&mut client).await {
        DaemonMsg::OperationOk { req, .. } => assert_eq!(req, 3),
        other => panic!("expected OperationOk, got {other:?}"),
    }

    // An invalid directive is refused specifically (InvalidInput), never silently accepted.
    client
        .send(Frame::Control(ClientMsg::SetLogLevel {
            req: 4,
            directives: "not a=valid=filter===".into(),
        }))
        .await
        .unwrap();
    match next_control(&mut client).await {
        DaemonMsg::OperationError { req, kind, .. } => {
            assert_eq!(req, 4);
            assert_eq!(kind, ErrorKind::InvalidInput);
        }
        other => panic!("expected OperationError, got {other:?}"),
    }
}

#[tokio::test]
async fn recent_errors_is_empty_when_diagnostics_are_unset() {
    // With no diagnostics handle (the ephemeral/test daemon), the RPC still answers — with an empty
    // list and the default sink — rather than hanging or erroring.
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let mut client = connect(&state).await;

    client
        .send(Frame::Control(ClientMsg::RecentErrorsRequest {
            req: 1,
            limit: 10,
        }))
        .await
        .unwrap();
    match next_control(&mut client).await {
        DaemonMsg::RecentErrors { req, entries } => {
            assert_eq!(req, 1);
            assert!(entries.is_empty());
        }
        other => panic!("expected RecentErrors, got {other:?}"),
    }
}
