//! The daemon's side of the orchestrator link (#1725): the real `orchestrator::run_with` on a temp
//! socket, a real dialer, and a fake orchestrator for the cases the real one never produces. One
//! binary, one module per user story. The orchestrator harness is #1724's, reused as it is.

#[path = "../orchestrator/support.rs"]
mod support;

mod host;

use std::path::Path;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use interprocess::local_socket::tokio::{prelude::*, Listener, Stream};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName};
use micold_core::daemon_config::{self, DaemonConfig};
use micold_core::orchestrator::{decode_first_frame, FirstFrame, LinkAnnouncement, LinkVerdict};
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use support::preamble_codec;

/// A host daemon's configuration with an `[orchestrator_link]` table, or none.
pub fn config(link: Option<&str>) -> Arc<DaemonConfig> {
    let text = format!(
        "schema = 1\n[daemon]\nname = \"Test Host\"\nid = \"hostbox\"\nkind = \"host\"\n{}",
        link.unwrap_or_default()
    );
    Arc::new(
        daemon_config::parse(&text, "daemon.toml")
            .unwrap_or_else(|errors| panic!("a valid test config: {errors:#?}")),
    )
}

/// `[orchestrator_link]` enabled, at `socket`.
pub fn link_to(socket: &Path) -> Arc<DaemonConfig> {
    config(Some(&format!(
        "[orchestrator_link]\nenabled = true\nsocket = \"{}\"\n",
        socket.display()
    )))
}

/// An orchestrator stand-in that listens on `socket` and lets the test play its side by hand.
pub struct FakeOrchestrator {
    listener: Listener,
}

/// One accepted connection, before the verdict.
pub struct Incoming {
    framed: Framed<Stream, LengthDelimitedCodec>,
    pub announcement: LinkAnnouncement,
}

impl FakeOrchestrator {
    pub fn bind(socket: &Path) -> Self {
        let name = socket
            .as_os_str()
            .to_fs_name::<GenericFilePath>()
            .expect("a socket name");
        let listener = ListenerOptions::new()
            .name(name)
            .create_tokio()
            .expect("the fake orchestrator listens");
        Self { listener }
    }

    /// Accept one connection and read its announcement.
    pub async fn accept(&self) -> Incoming {
        let stream = support::within("a dial arrives", self.listener.accept())
            .await
            .expect("accepted");
        let mut framed = Framed::new(stream, preamble_codec());
        let body = support::within("the announcement arrives", framed.next())
            .await
            .expect("an announcement frame")
            .expect("a readable frame");
        match decode_first_frame(&body).expect("a valid announcement") {
            FirstFrame::Announcement(announcement) => Incoming {
                framed,
                announcement,
            },
            FirstFrame::Query(q) => panic!("a query: {q:?}"),
        }
    }
}

impl Incoming {
    /// Write a verdict frame.
    pub async fn verdict(&mut self, verdict: &LinkVerdict) {
        let body = serde_json::to_vec(verdict).expect("encodes");
        self.framed.send(Bytes::from(body)).await.expect("sent");
    }

    /// `Accepted`, switched to the wire codec: the daemon's `Hello` is next.
    pub async fn accepted(mut self) -> Wire {
        self.verdict(&LinkVerdict::Accepted).await;
        Wire {
            framed: self.framed.map_codec(|_| DaemonCodec::new()),
        }
    }

    /// Write half a verdict frame and close: the length prefix promises more than arrives.
    pub async fn half_verdict_then_close(mut self) {
        use tokio::io::AsyncWriteExt;
        let io = self.framed.get_mut();
        io.write_all(&100_u32.to_le_bytes()).await.expect("written");
        io.write_all(b"{\"verdict\":").await.expect("written");
        io.flush().await.expect("flushed");
    }

    /// The whole link: `Accepted`, read `Hello`, answer `Welcome`.
    pub async fn link(self) -> Wire {
        let mut wire = self.accepted().await;
        assert!(matches!(wire.next().await, ClientMsg::Hello { .. }));
        wire.send(DaemonMsg::Welcome {
            daemon_build: "fake-orchestrator".into(),
            catalog: Default::default(),
            settings: micold_daemon::catalog::wire_settings(&Default::default()),
            plan_usage: None,
        })
        .await;
        wire
    }
}

/// A connection speaking the wire protocol, from the orchestrator's side.
pub struct Wire {
    framed: Framed<Stream, DaemonCodec>,
}

impl Wire {
    pub async fn send(&mut self, msg: DaemonMsg) {
        self.framed.send(Frame::Control(msg)).await.expect("sent");
    }

    pub async fn next(&mut self) -> ClientMsg {
        match support::within("a frame arrives", self.framed.next())
            .await
            .expect("the daemon stays connected")
            .expect("a decodable frame")
        {
            Frame::Control(msg) => msg,
            other => panic!("expected a control frame, got {other:?}"),
        }
    }
}
