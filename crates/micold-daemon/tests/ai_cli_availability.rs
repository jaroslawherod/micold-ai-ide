//! The service answers "which AI CLIs are there?" about **itself** (feature 027, FR-023c).
//!
//! This is a one-message RPC, and the interesting part is not the message — it is *which process
//! runs the `PATH` walk*. Before 027 the client walked its own, which under sandboxed placement
//! describes the host while the sessions run in a container. The answer was plausible and wrong,
//! which is the worst combination and the reason FR-023c exists.
//!
//! So the assertions here are about the answer tracking **this process's** environment, not about
//! the wire shape: install a stub on `PATH` and the reply grows a CLI; hide every CLI and the
//! reply is empty. A test that only checked "a reply arrives" would have passed against the old,
//! wrong arrangement too.
//!
//! The client half of the same claim is
//! `micold-client/tests/cli_availability_comes_from_the_service.rs`, which asserts the client
//! *cannot* answer it locally any more.

use std::sync::{Arc, Mutex, OnceLock};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::AiCli;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// `PATH` is process-global; the two tests below move it in opposite directions.
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

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

async fn ask(client: &mut Client, req: u64) -> Vec<AiCli> {
    client
        .send(Frame::Control(ClientMsg::AiCliAvailabilityRequest { req }))
        .await
        .unwrap();
    loop {
        match client.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::AiCliAvailability {
                req: got,
                available,
                ..
            }) => {
                assert_eq!(
                    got, req,
                    "the reply must carry the request's correlation id"
                );
                return available;
            }
            Frame::Control(DaemonMsg::CatalogChanged { .. }) => continue,
            Frame::Grid(_) => continue,
            other => panic!("expected AiCliAvailability, got {other:?}"),
        }
    }
}

/// A `PATH` holding exactly the stubs named, and nothing else that is an AI CLI.
///
/// Both directions in one guard, because the two tests are the same claim read forwards and
/// backwards: the answer is *this process's* environment, so putting a CLI there must add it and
/// taking every CLI away must empty it.
struct ScratchPath {
    previous: Option<std::ffi::OsString>,
    _dir: tempfile::TempDir,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl ScratchPath {
    fn with(commands: &[&str]) -> Self {
        let guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var_os("PATH");
        let dir = tempfile::tempdir().unwrap();
        for command in commands {
            let path = dir.path().join(command);
            std::fs::write(&path, b"#!/bin/sh\n").unwrap();
        }
        // The scratch directory *replaces* `PATH` rather than prefixing it: a developer machine
        // with `claude` installed would otherwise make the "nothing available" case pass for the
        // wrong reason, or rather fail to be the case at all.
        std::env::set_var("PATH", dir.path());
        Self {
            previous,
            _dir: dir,
            _guard: guard,
        }
    }
}

impl Drop for ScratchPath {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

#[tokio::test]
async fn the_service_reports_the_clis_on_its_own_path() {
    let claude = AiCli::ClaudeCode.provider().command();
    let _path = ScratchPath::with(&[claude]);

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let mut client = connect(&state).await;

    assert_eq!(
        ask(&mut client, 7).await,
        vec![AiCli::ClaudeCode],
        "the service reported something other than what is on its own PATH — which is the whole \
         content of FR-023c, since under sandboxed placement its PATH is the image's and the \
         client's is the host's"
    );
}

/// Empty is an answer, not a failure.
///
/// A substituted image that ships no AI CLI is FR-023b's whole scenario, and it has to reach the
/// user as "this image provides none" rather than as a missing reply the client renders as
/// "not asked yet".
#[tokio::test]
async fn an_environment_with_no_cli_reports_an_empty_set_rather_than_failing() {
    let _path = ScratchPath::with(&[]);

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let mut client = connect(&state).await;

    assert!(
        ask(&mut client, 1).await.is_empty(),
        "an environment with no AI CLI must answer with an empty set, not with silence"
    );
}
