//! A refused mutation is determinable from the daemon's own log (`010` BUG-020, S13/FR-046).
//!
//! S13 asks that "for each failure in S7, S8 and S11, the cause is determinable from logs reachable
//! through the UI". It was not. The read-only-parent worktree create in the BUG-020 session produced
//! a perfect message in the client and **no log line at all** on the daemon side, so a user who had
//! dismissed the dialog had nothing left to read — and the diagnostics panel, which shows the recent
//! WARN/ERROR ring, had nothing to show either.
//!
//! There are twenty-five `OperationError` sites in `server.rs`. Logging at each of them is twenty-
//! five chances to forget, so the line is emitted where they all converge — `DaemonState::send` —
//! and this asserts it from both ends: a real refusal driven over a real connection, and the choke
//! point itself carrying a `detail`.

use std::io;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind};
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

#[tokio::test]
async fn a_refused_mutation_says_so_in_the_log() {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(BufWriter(buf.clone()))
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("set global subscriber once");

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(&state),
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
    let _welcome = client.next().await.unwrap().unwrap();

    // A mutation this daemon refuses, decided on the connection loop so the answer is deterministic:
    // the project is not one it knows, so no git command runs and no worktree is touched.
    client
        .send(Frame::Control(ClientMsg::WorktreeInclude {
            req: 7,
            project: "/no/such/project".into(),
            path: "/no/such/project/wt".into(),
        }))
        .await
        .unwrap();
    let reply = client.next().await.unwrap().unwrap();
    assert!(
        matches!(reply, Frame::Control(DaemonMsg::OperationError { .. })),
        "the fixture must actually be refused for this to be about anything, got {reply:?}"
    );

    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();
    // Awaiting the server task guarantees every log line for this connection has been written.
    server.await.unwrap().unwrap();

    let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(
        logs.contains("operation refused"),
        "a refusal the user saw in the UI left nothing in the log, so the cause is determinable \
         only from a dialog they have already dismissed (S13):\n{logs}"
    );
    assert!(
        logs.contains("unknown project"),
        "the line has to name the cause, not merely that something was refused:\n{logs}"
    );

    // The other half: `detail` is usually the only part that names *why* — git's own stderr for a
    // failed worktree create — so dropping it would log the generic half of every message and lose
    // the informative one. Driven at the choke point, because reaching a `detail`-carrying refusal
    // over the wire needs a real repository and this claim is about the logging, not about git.
    let before = buf.lock().unwrap().len();
    state.send(
        1,
        DaemonMsg::OperationError {
            req: 8,
            kind: ErrorKind::GitFailed,
            message: "git failed to create the worktree".into(),
            detail: Some("fatal: could not create leading directories".into()),
        },
    );
    let logs = String::from_utf8(buf.lock().unwrap()[before..].to_vec()).unwrap();
    assert!(
        logs.contains("could not create leading directories"),
        "the detail carries the cause and must reach the log with the message:\n{logs}"
    );
}
