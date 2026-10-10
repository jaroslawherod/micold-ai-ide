//! Shared by the daemon tests that drive `serve_connection` over an in-memory duplex: open a
//! connection, do the `Hello`/`Welcome` handshake and, optionally, attach to a project. A test that
//! varies the `Hello` itself (fingerprint, version mismatch) builds its own.
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg, DaemonSettings};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

/// The client end of a connection to the service.
pub type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// How long a connection waits for the `Welcome` the service owes it.
const WELCOME_WITHIN: Duration = Duration::from_secs(10);

async fn handshake(
    state: &Arc<DaemonState>,
    build: &str,
    instance: ClientInstance,
) -> (Client, DaemonMsg) {
    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: build.into(),
            client_instance: instance,
            client_package_version: PACKAGE_VERSION.into(),
            // The host-process placement presents no token, and a fingerprint mismatch is not a
            // refusal there. `BUILD_FINGERPRINT` because these tests compile against the same core
            // as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .expect("the hello is sent");
    let frame = tokio::time::timeout(WELCOME_WITHIN, client.next())
        .await
        .expect("the service answers the hello in time");
    match frame {
        Some(Ok(Frame::Control(welcome @ DaemonMsg::Welcome { .. }))) => (client, welcome),
        other => panic!("expected Welcome, got {other:?}"),
    }
}

/// Connect as build `"test"` and take the `Welcome`.
pub async fn connect(state: &Arc<DaemonState>) -> Client {
    connect_as(state, "test").await
}

/// As [`connect`], reporting `build` as the client build.
pub async fn connect_as(state: &Arc<DaemonState>, build: &str) -> Client {
    handshake(state, build, ClientInstance::current()).await.0
}

/// As [`connect`], also returning the settings the `Welcome` reported.
pub async fn connect_with_settings(state: &Arc<DaemonState>) -> (Client, DaemonSettings) {
    match handshake(state, "test", ClientInstance::current()).await {
        (client, DaemonMsg::Welcome { settings, .. }) => (client, settings),
        (_, other) => unreachable!("handshake returns a Welcome, not {other:?}"),
    }
}

/// Connect a window of build `build` that is its own client instance (pid 0, nonce `build`), so
/// several windows of one test are told apart. Returns it with the `Welcome` itself.
pub async fn connect_with_welcome(state: &Arc<DaemonState>, build: &str) -> (Client, DaemonMsg) {
    let instance = ClientInstance {
        pid: 0,
        nonce: build.into(),
    };
    handshake(state, build, instance).await
}

/// Handshake, then attach `project`, draining the `Attached` and `CatalogChanged` it produces.
pub async fn connect_and_attach(state: &Arc<DaemonState>, project: &Path) -> Client {
    let mut client = connect(state).await;
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }))
        .await
        .expect("the attach is sent");
    let mut attached = false;
    loop {
        match client.next().await.expect("stream open").expect("a frame") {
            Frame::Control(DaemonMsg::Attached { .. }) => attached = true,
            Frame::Control(DaemonMsg::CatalogChanged { .. }) if attached => return client,
            Frame::Control(_) | Frame::Grid(_) => {}
        }
    }
}
