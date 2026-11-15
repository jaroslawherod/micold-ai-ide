//! User Story 1 (#1725, FR-001, FR-002, FR-003, FR-010, SC-001, SC-005): a host daemon with
//! `[orchestrator_link] enabled = true` dials the local orchestrator and is listed `online`; with
//! no link configured nothing is spawned, opened or logged.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{hello_message, Credentials};
use micold_core::orchestrator::LinkStatus;
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::keepalive::{CHECK_INTERVAL, LIVENESS_DEADLINE};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::PROTOCOL_VERSION;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::spawn;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use tracing_subscriber::fmt::MakeWriter;

use crate::support::{record_when, within, Orchestrator};
use crate::{config, link_to, FakeOrchestrator};

async fn status_is(handle: &micold_daemon::orchestrator_link::LinkHandle, want: LinkStatus) {
    within("the status settles", async {
        while handle.status() != want {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

#[tokio::test]
async fn a_host_daemon_is_listed_online_with_every_announced_field() {
    let orchestrator = Orchestrator::start().await;
    let handle = spawn(&link_to(&orchestrator.socket)).expect("enabled spawns a task");

    let record = record_when(&orchestrator, "hostbox", |r| r["health"] == "online").await;
    assert_eq!(record["kind"], "host");
    assert_eq!(record["name"], "Test Host");
    assert_eq!(record["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(record["tenant"], "default");
    assert!(
        !record["capabilities"].as_array().unwrap().is_empty(),
        "{record}"
    );
    status_is(&handle, LinkStatus::Linked).await;
    assert_eq!(orchestrator.daemons().await.len(), 1);
}

#[tokio::test]
async fn a_daemon_started_before_the_orchestrator_links_without_a_restart() {
    let dir = Arc::new(tempfile::tempdir().unwrap());
    let socket = dir.path().join("orchestrator.sock");
    let handle = spawn(&link_to(&socket)).expect("enabled spawns a task");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        handle.status(),
        LinkStatus::NotLinked,
        "nothing listens yet"
    );

    let orchestrator = Orchestrator::start_in(dir).await;
    record_when(&orchestrator, "hostbox", |r| r["health"] == "online").await;
    status_is(&handle, LinkStatus::Linked).await;
}

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Captured;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// FR-010, SC-005: no table, a disabled table or an empty `socket`: no task, no socket, no log.
#[tokio::test]
async fn a_daemon_with_no_link_configured_spawns_nothing_opens_nothing_and_logs_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let disabled = format!(
        "[orchestrator_link]\nenabled = false\nsocket = \"{}\"\n",
        socket.display()
    );
    let tasks_before = tokio::runtime::Handle::current()
        .metrics()
        .num_alive_tasks();
    tracing::subscriber::with_default(subscriber, || {
        for table in [None, Some(disabled.as_str()), Some("[orchestrator_link]\n")] {
            assert!(spawn(&config(table)).is_none(), "{table:?}");
        }
    });
    assert_eq!(
        tokio::runtime::Handle::current()
            .metrics()
            .num_alive_tasks(),
        tasks_before,
        "no task was spawned"
    );
    assert!(
        captured.0.lock().unwrap().is_empty(),
        "{}",
        String::from_utf8_lossy(&captured.0.lock().unwrap())
    );
    let dialled = tokio::time::timeout(Duration::from_millis(300), fake.accept()).await;
    assert!(dialled.is_err(), "a disabled daemon dialled");
}

/// An empty `socket` is refused by `config check`, but a daemon that starts with one still starts
/// and does not dial (edge case).
#[tokio::test]
async fn a_daemon_started_with_an_empty_socket_does_not_dial() {
    // `config check` refuses this file (rule L1), so build the value by hand.
    let mut value = (*config(Some("[orchestrator_link]\nenabled = true\n"))).clone();
    value.orchestrator_link.as_mut().unwrap().socket = Some(std::path::PathBuf::new());
    assert!(spawn(&Arc::new(value)).is_none());
}

async fn handshake_ms(state: Arc<DaemonState>) -> u128 {
    let started = std::time::Instant::now();
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(state, server_io));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(hello_message(
            "test",
            &Credentials::default(),
        )))
        .await
        .unwrap();
    let frame = within("Welcome", client.next()).await.unwrap().unwrap();
    assert!(matches!(frame, Frame::Control(DaemonMsg::Welcome { .. })));
    started.elapsed().as_millis()
}

/// Scenario 4, FR-001: an absent or a slow orchestrator never delays a client.
#[tokio::test]
async fn a_client_is_accepted_while_the_orchestrator_is_absent_or_slow() {
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("absent.sock");
    let slow = dir.path().join("slow.sock");
    let fake = FakeOrchestrator::bind(&slow);
    let _absent = spawn(&link_to(&absent)).unwrap();
    let _slow = spawn(&link_to(&slow)).unwrap();
    // The slow one accepts the dial and says nothing.
    let _held = fake.accept().await;

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let ms = handshake_ms(state).await;
    assert!(ms < 2_000, "the handshake took {ms} ms");
}

/// FR-002: one link, one pong per ping, any frame is life; past the liveness deadline the daemon
/// is still linked and does not dial a second time.
#[tokio::test(start_paused = true)]
async fn a_daemon_answers_every_ping_and_keeps_exactly_one_link() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    let handle = spawn(&link_to(&socket)).unwrap();
    let mut wire = fake.accept().await.link().await;

    let mut nonce = 0;
    let mut waited = Duration::ZERO;
    while waited <= LIVENESS_DEADLINE + CHECK_INTERVAL {
        nonce += 1;
        wire.send(DaemonMsg::LinkPing { nonce }).await;
        match wire.next().await {
            ClientMsg::LinkPong { nonce: echoed } => assert_eq!(echoed, nonce),
            other => panic!("expected a LinkPong, got {other:?}"),
        }
        // Silence, but under the deadline each time.
        tokio::time::sleep(Duration::from_secs(3)).await;
        waited += Duration::from_secs(3);
    }
    assert_eq!(handle.status(), LinkStatus::Linked);
    // Nothing else came and no second connection: the daemon dials once per link.
    let second = tokio::time::timeout(Duration::from_secs(1), fake.accept()).await;
    assert!(second.is_err(), "a second link was dialled");
}

/// A frame other than a ping also keeps the link alive.
#[tokio::test(start_paused = true)]
async fn any_frame_resets_the_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    let handle = spawn(&link_to(&socket)).unwrap();
    let mut wire = fake.accept().await.link().await;
    for _ in 0..3 {
        tokio::time::sleep(LIVENESS_DEADLINE - Duration::from_secs(1)).await;
        wire.send(DaemonMsg::ConfigChanged { status: None }).await;
    }
    assert_eq!(handle.status(), LinkStatus::Linked);
}

/// Edge case: a verdict cut off halfway is a dropped link that is retried, never a panic.
#[tokio::test(start_paused = true)]
async fn half_a_verdict_is_a_dropped_link_that_is_retried() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    let handle = spawn(&link_to(&socket)).unwrap();

    fake.accept().await.half_verdict_then_close().await;
    let again = fake.accept().await;
    assert_eq!(again.announcement.id.as_str(), "hostbox");
    let _wire = again.link().await;
    status_is(&handle, LinkStatus::Linked).await;
    // The task survived: it is still running.
    assert!(!handle.is_finished());
}
