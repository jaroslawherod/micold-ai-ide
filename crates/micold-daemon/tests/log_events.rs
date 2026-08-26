//! Phase 10 (T080a) — the mandated FR-045 log events are actually emitted.
//!
//! Installs a global in-memory `tracing` subscriber and drives the connection-level operations,
//! asserting each mandated event appears with its reason: client connect/disconnect, project
//! attach/detach, an attach **refusal** (project busy), and a **takeover** (forced attach). The
//! remaining FR-045 sites — startup/shutdown, endpoint bind + bind failure, and session
//! start/exit/restart-attempt/give-up — are placed at their call sites (`server::run`,
//! `state::start_session`, `state::supervise_exited_sessions`) and exercised by the daemon-lifecycle,
//! autospawn, and supervision test suites; asserting them here too would need a second daemon process
//! or a live PTY, so this test owns the connection subset it can drive deterministically.

use std::io;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<'a> MakeWriter<'a> for BufWriter {
    type Writer = BufWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &Arc<DaemonState>, build: &str) -> Client {
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
            client_build: build.into(),
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

/// Read the next control message, skipping catalog pushes.
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
async fn connection_lifecycle_events_are_logged_with_reason() {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(BufWriter(buf.clone()))
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("set global subscriber once");

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));

    // A attaches the project.
    let mut a = connect(&state, "window-A").await;
    a.send(Frame::Control(ClientMsg::Attach {
        project: "/proj".into(),
        force: false,
    }))
    .await
    .unwrap();
    match next_control(&mut a).await {
        DaemonMsg::Attached { .. } => {}
        other => panic!("A expected Attached, got {other:?}"),
    }

    // B is refused (busy), then forces a takeover.
    let mut b = connect(&state, "window-B").await;
    b.send(Frame::Control(ClientMsg::Attach {
        project: "/proj".into(),
        force: false,
    }))
    .await
    .unwrap();
    match next_control(&mut b).await {
        DaemonMsg::Refused { .. } => {}
        other => panic!("B expected Refused, got {other:?}"),
    }
    b.send(Frame::Control(ClientMsg::Attach {
        project: "/proj".into(),
        force: true,
    }))
    .await
    .unwrap();
    match next_control(&mut b).await {
        DaemonMsg::Attached { .. } => {}
        other => panic!("B expected Attached, got {other:?}"),
    }

    // A detaches and disconnects cleanly.
    a.send(Frame::Control(ClientMsg::Detach {
        project: "/proj".into(),
    }))
    .await
    .unwrap();
    a.send(Frame::Control(ClientMsg::Goodbye)).await.unwrap();
    b.send(Frame::Control(ClientMsg::Goodbye)).await.unwrap();
    // Let the server tasks run their disconnect path.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    for needle in [
        "client attached to daemon", // client connect
        "project attached",          // attach (and, with force=true, the takeover)
        "attach refused",            // refusal, with reason
        "force=true",                // the takeover is distinguishable in the log
        "project detached",          // detach
        "client disconnected",       // client disconnect
    ] {
        assert!(
            logs.contains(needle),
            "missing mandated log event {needle:?} (FR-045) in:\n{logs}"
        );
    }
}
