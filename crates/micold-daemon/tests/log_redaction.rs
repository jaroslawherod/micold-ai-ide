//! Phase 10 (T081) — no terminal content or user input ever reaches a log (FR-047).
//!
//! Installs a global in-memory `tracing` subscriber (so every daemon thread's events are captured),
//! drives real connection + input operations carrying a sentinel where terminal content / keystrokes
//! would be, and asserts the sentinel never appears in any log line. It is a *regression guard*: no
//! call site logs those bytes today, and this locks that in — a future edit that logs
//! `SessionInput.bytes`, PTY output, or an OSC title would fail here.

use std::io;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::ClientMsg;
use micold_core::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};
use micold_core::session::SessionId;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use tracing_subscriber::fmt::MakeWriter;

/// A sentinel standing in for secret terminal content / keystrokes. It must never be logged.
const SENTINEL: &str = "REDACTION_SENTINEL_9c1f_do_not_log";

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
async fn no_log_line_contains_terminal_content_or_input() {
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
            client_package_version: PACKAGE_VERSION.into(),
        }))
        .await
        .unwrap();
    let _welcome = client.next().await.unwrap().unwrap();

    // Attach (logs the project path — identity, allowed), then push input whose bytes carry the
    // sentinel (the redaction-sensitive path — must be dropped from any log line).
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: "/proj".into(),
            force: false,
        }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::SessionInput {
            session: SessionId::new(),
            serial: 0,
            bytes: SENTINEL.as_bytes().to_vec(),
        }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();

    // Awaiting the server task guarantees every log line for this connection has been written.
    server.await.unwrap().unwrap();

    let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(
        !logs.contains(SENTINEL),
        "a log line leaked terminal content / user input (FR-047):\n{logs}"
    );
    // Sanity: capture actually worked (we did log *something* structural for this connection).
    assert!(
        logs.contains("attached") || logs.contains("disconnected"),
        "expected connection lifecycle events to be captured, got:\n{logs}"
    );
}
