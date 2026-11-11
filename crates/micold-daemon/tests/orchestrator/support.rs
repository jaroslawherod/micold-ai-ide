//! The harness: an orchestrator on a real local socket in a temp dir, a controllable clock, a fake
//! daemon that speaks the link and a listing client.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use interprocess::local_socket::tokio::{prelude::*, Stream};
use interprocess::local_socket::{GenericFilePath, ToFsName};
use micold_core::daemon_config::Kind;
use micold_core::orchestrator::{LinkAnnouncement, LinkVerdict};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg};
use micold_core::protocol::routing::WireDaemonId;
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::orchestrator::Clock;
use tokio::task::JoinHandle;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// Long enough for a loaded machine, short enough that a hang fails the test.
pub const WITHIN: Duration = Duration::from_secs(20);

/// A clock the test moves by hand.
pub struct TestClock {
    base: Instant,
    offset_ms: AtomicU64,
    wall: AtomicU64,
}

impl TestClock {
    pub fn new(wall: u64) -> Arc<Self> {
        Arc::new(Self {
            base: Instant::now(),
            offset_ms: AtomicU64::new(0),
            wall: AtomicU64::new(wall),
        })
    }

    /// The wall clock, in unix seconds.
    pub fn wall(&self) -> u64 {
        self.wall.load(Ordering::SeqCst)
    }

    /// Move both clocks on.
    pub fn advance(&self, by: Duration) {
        self.offset_ms
            .fetch_add(by.as_millis() as u64, Ordering::SeqCst);
        self.wall.fetch_add(by.as_secs(), Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        self.base + Duration::from_millis(self.offset_ms.load(Ordering::SeqCst))
    }
    fn wall_secs(&self) -> u64 {
        self.wall.load(Ordering::SeqCst)
    }
}

/// A running orchestrator on a socket in its own temp dir.
pub struct Orchestrator {
    pub dir: tempfile::TempDir,
    pub socket: PathBuf,
    pub clock: Arc<TestClock>,
    task: Mutex<Option<JoinHandle<std::io::Result<()>>>>,
}

impl Orchestrator {
    pub async fn start() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let socket = dir.path().join("orchestrator.sock");
        let clock = TestClock::new(1_700_000_000);
        let task = tokio::spawn(micold_daemon::orchestrator::run_with(
            socket.clone(),
            Arc::clone(&clock) as Arc<dyn Clock>,
        ));
        let started = Instant::now();
        loop {
            if connect(&socket).await.is_some() {
                break;
            }
            assert!(
                started.elapsed() < WITHIN,
                "the orchestrator never listened"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Self {
            dir,
            socket,
            clock,
            task: Mutex::new(Some(task)),
        }
    }

    /// Dial the orchestrator.
    pub async fn dial(&self) -> Stream {
        connect(&self.socket)
            .await
            .expect("the orchestrator accepts")
    }

    /// The listing, as the JSON frame the orchestrator answers a `list` query with.
    pub async fn list(&self) -> serde_json::Value {
        let mut framed = Framed::new(self.dial().await, preamble_codec());
        framed
            .send(br#"{"query":"list"}"#.as_slice().into())
            .await
            .expect("the query is sent");
        let frame = tokio::time::timeout(WITHIN, framed.next())
            .await
            .expect("the listing arrives")
            .expect("the connection answers")
            .expect("a frame");
        serde_json::from_slice(&frame).expect("the listing is JSON")
    }

    /// The `daemons` array of the listing.
    pub async fn daemons(&self) -> Vec<serde_json::Value> {
        self.list().await["daemons"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }
}

impl Drop for Orchestrator {
    fn drop(&mut self) {
        if let Some(task) = self.task.lock().unwrap().take() {
            task.abort();
        }
    }
}

async fn connect(socket: &std::path::Path) -> Option<Stream> {
    let name = socket.as_os_str().to_fs_name::<GenericFilePath>().ok()?;
    Stream::connect(name).await.ok()
}

/// `u32` little-endian length then body, at most 64 KiB: the preamble's framing.
pub fn preamble_codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .little_endian()
        .max_frame_length(64 * 1024)
        .new_codec()
}

/// The announcement of a current-version host daemon named `id`.
pub fn announcement(id: &str) -> LinkAnnouncement {
    LinkAnnouncement {
        format: 1,
        id: WireDaemonId::new(id).expect("a valid id"),
        kind: Kind::Container,
        name: format!("Daemon {id}"),
        protocol_version: PROTOCOL_VERSION,
        capabilities: BTreeSet::from(["sessions".to_owned(), "git".to_owned()]),
        tenant: None,
    }
}

/// A daemon dialling the orchestrator.
pub struct FakeDaemon {
    framed: Framed<Stream, LengthDelimitedCodec>,
}

/// A daemon after `Accepted`: now speaking `WireCodec` frames.
pub struct LinkedDaemon {
    pub framed: Framed<Stream, ClientCodec>,
}

impl FakeDaemon {
    pub async fn dial(orchestrator: &Orchestrator) -> Self {
        Self {
            framed: Framed::new(orchestrator.dial().await, preamble_codec()),
        }
    }

    /// Send the announcement and read the verdict.
    pub async fn announce(&mut self, announcement: &LinkAnnouncement) -> LinkVerdict {
        let body = serde_json::to_vec(announcement).expect("encodes");
        self.framed.send(body.into()).await.expect("sent");
        let frame = tokio::time::timeout(WITHIN, self.framed.next())
            .await
            .expect("a verdict arrives")
            .expect("the connection answers")
            .expect("a frame");
        serde_json::from_slice(&frame).expect("a verdict")
    }

    /// Announce, expect `Accepted`, and switch to the wire codec.
    pub async fn accepted(mut self, announcement: &LinkAnnouncement) -> LinkedDaemon {
        let verdict = self.announce(announcement).await;
        assert_eq!(verdict, LinkVerdict::Accepted);
        LinkedDaemon {
            framed: self.framed.map_codec(|_| ClientCodec::new()),
        }
    }

    /// The full link: announce, `Hello`, `Welcome`.
    pub async fn link(
        orchestrator: &Orchestrator,
        announcement: &LinkAnnouncement,
    ) -> LinkedDaemon {
        let mut linked = Self::dial(orchestrator).await.accepted(announcement).await;
        linked.hello().await;
        linked.expect_welcome().await;
        linked
    }
}

impl LinkedDaemon {
    pub async fn hello(&mut self) {
        self.send(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "fake-daemon".into(),
            client_instance: ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        })
        .await;
    }

    pub async fn send(&mut self, msg: ClientMsg) {
        self.framed.send(Frame::Control(msg)).await.expect("sent");
    }

    pub async fn next(&mut self) -> DaemonMsg {
        let frame = tokio::time::timeout(WITHIN, self.framed.next())
            .await
            .expect("a frame arrives")
            .expect("the connection stays open")
            .expect("a decodable frame");
        match frame {
            Frame::Control(msg) => msg,
            other => panic!("expected a control frame, got {other:?}"),
        }
    }

    pub async fn expect_welcome(&mut self) {
        match self.next().await {
            DaemonMsg::Welcome { catalog, .. } => {
                assert!(
                    catalog.projects.is_empty(),
                    "the orchestrator hosts nothing"
                )
            }
            other => panic!("expected Welcome, got {other:?}"),
        }
    }
}
