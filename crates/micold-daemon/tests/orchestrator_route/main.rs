//! #1726: clients through the orchestrator. A real orchestrator on a temp socket, real daemons
//! linked to it (each serving the orchestrator's clients through `serve_connection`), and a client
//! that speaks the routed frames of `specs/orchestrator/contracts/client-routing.md`. Where a case
//! needs a daemon that misbehaves (never answers, drops the link at a chosen moment) a fake one
//! plays that side by hand. One binary, one module per behaviour.

#[path = "../orchestrator/support.rs"]
mod support;

mod bench;
mod hub_queues;
mod link_drop;
mod link_full;
mod lists;
mod outage;
mod refusals;
mod requests;
mod slow_client;
mod streams;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use interprocess::local_socket::tokio::Stream;
use micold_core::connect::{hello_message, Credentials};
use micold_core::daemon_config;
use micold_core::orchestrator::LinkStatus;
use micold_core::protocol::codec::raw_frame_codec;
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg};
use micold_core::protocol::routing::{
    control_body, control_payload, decode_routed_payload, encode_opaque_payload, message_req,
    ConnectionId, RoutedBody, RoutedHeader, Sequence, StreamEndReason,
};
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::{announcement, spawn_with, LinkHandle};
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

use support::{record_when, within, Orchestrator};

/// A real daemon linked to the orchestrator: its state and its link task.
pub struct Daemon {
    pub id: String,
    pub state: Arc<DaemonState>,
    pub link: LinkHandle,
}

/// The configuration of a host daemon named `id` that dials `socket`.
pub fn config(id: &str, socket: &Path) -> Arc<daemon_config::DaemonConfig> {
    let text = format!(
        "schema = 1\n[daemon]\nname = \"Daemon {id}\"\nid = \"{id}\"\nkind = \"host\"\n\
         [orchestrator_link]\nenabled = true\nsocket = \"{}\"\n",
        socket.display()
    );
    Arc::new(
        daemon_config::parse(&text, "daemon.toml")
            .unwrap_or_else(|errors| panic!("a valid test config: {errors:#?}")),
    )
}

impl Daemon {
    /// Start daemon `id` and wait until the orchestrator lists it online.
    pub async fn start(orchestrator: &Orchestrator, id: &str) -> Self {
        let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
        let daemon = Self::link_state(orchestrator, id, state);
        daemon.listed_online(orchestrator).await;
        daemon
    }

    /// Link `state` as daemon `id`, without waiting.
    pub fn link_state(orchestrator: &Orchestrator, id: &str, state: Arc<DaemonState>) -> Self {
        let config = config(id, &orchestrator.socket);
        let link = spawn_with(&config, announcement(&config), Some(Arc::clone(&state)))
            .expect("the link is enabled");
        Self {
            id: id.to_owned(),
            state,
            link,
        }
    }

    pub async fn listed_online(&self, orchestrator: &Orchestrator) {
        record_when(orchestrator, &self.id, |r| r["health"] == "online").await;
        within("the link settles", async {
            while self.link.status() != LinkStatus::Linked {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
    }

    /// Drop the link as a crash would: the task and its socket go, the state stays.
    pub fn drop_link(&self) {
        self.link.abort();
    }
}

/// A catalog with one restored shell (Regular-mode) session at `project`, kept in `store`, and its
/// id: starting it spawns the platform shell, so no AI CLI is needed.
pub fn shell_catalog(project: &Path, store: &Path) -> (Catalog, micold_core::session::SessionId) {
    use std::collections::BTreeMap;

    use micold_core::project::{Availability, Project};
    use micold_core::session::{
        AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
    };
    use micold_core::settings::JsonFileSettingsStore;
    use micold_core::store::{JsonFileStore, ProjectStore};
    use micold_core::workspace::Workspace;

    let id = SessionId::new();
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), vec![session])]),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects = store.join("projects.json");
    JsonFileStore::at(projects.clone())
        .save(&workspace)
        .expect("the workspace is saved");
    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    );
    (catalog, id)
}

/// What a client read: the routed header and the inner message, when it is a control one.
#[derive(Debug)]
pub struct Received {
    pub header: RoutedHeader,
    pub msg: Option<DaemonMsg>,
}

/// A client of the orchestrator, speaking routed frames.
pub struct Client {
    framed: Framed<Stream, tokio_util::codec::LengthDelimitedCodec>,
    seq: u64,
}

/// A `Hello` of the window `name` (its own client instance).
fn hello(name: &str) -> ClientMsg {
    let ClientMsg::Hello {
        protocol_version,
        schema_hash,
        client_package_version,
        auth_token,
        client_fingerprint,
        require_fingerprint_match,
        ..
    } = hello_message("test", &Credentials::default())
    else {
        unreachable!("hello_message builds a Hello")
    };
    ClientMsg::Hello {
        protocol_version,
        schema_hash,
        client_build: name.into(),
        client_instance: ClientInstance {
            pid: 0,
            nonce: name.into(),
        },
        client_package_version,
        auth_token,
        client_fingerprint,
        require_fingerprint_match,
    }
}

impl Client {
    /// Connect, send a plain `Hello` and read `Welcome`.
    pub async fn connect(orchestrator: &Orchestrator) -> Self {
        let mut framed = Framed::new(orchestrator.dial().await, raw_frame_codec());
        let payload = control_payload(&hello("orchestrator-client")).unwrap();
        framed.send(Bytes::from(payload)).await.expect("hello sent");
        let welcome = within("Welcome", framed.next())
            .await
            .expect("an answer")
            .expect("a frame");
        let body = control_body(&welcome).expect("a plain control frame");
        let msg: DaemonMsg = serde_json::from_slice(body).unwrap();
        assert!(matches!(msg, DaemonMsg::Welcome { .. }), "{msg:?}");
        Self { framed, seq: 0 }
    }

    /// Send `msg` to `daemon` on `conn`, header `req` set as the contract requires.
    pub async fn send(&mut self, daemon: &str, conn: u64, msg: &ClientMsg) {
        let value = serde_json::to_value(msg).unwrap();
        self.send_frame(
            daemon,
            conn,
            message_req(&value),
            control_payload(msg).unwrap(),
        )
        .await;
    }

    /// The bytes [`Self::send`] would write, without writing them.
    pub fn frame(&mut self, daemon: &str, conn: u64, msg: &ClientMsg) -> Bytes {
        let value = serde_json::to_value(msg).unwrap();
        self.seq += 1;
        let header = RoutedHeader {
            target: Some(daemon.parse().unwrap()),
            conn: ConnectionId(conn),
            seq: Sequence(self.seq),
            req: message_req(&value),
        };
        Bytes::from(encode_opaque_payload(
            &header,
            &control_payload(msg).unwrap(),
        ))
    }

    pub async fn send_frame(&mut self, daemon: &str, conn: u64, req: Option<u64>, inner: Vec<u8>) {
        self.seq += 1;
        let header = RoutedHeader {
            target: Some(daemon.parse().unwrap()),
            conn: ConnectionId(conn),
            seq: Sequence(self.seq),
            req,
        };
        let payload = encode_opaque_payload(&header, &inner);
        self.framed
            .send(Bytes::from(payload))
            .await
            .expect("frame sent");
    }

    /// Open conn `conn` to `daemon`: `Hello`, then its `Welcome`.
    pub async fn open(&mut self, daemon: &str, conn: u64, name: &str) {
        self.send(daemon, conn, &hello(name)).await;
        let got = self.recv_on(daemon, conn).await;
        assert!(
            matches!(got.msg, Some(DaemonMsg::Welcome { .. })),
            "expected Welcome from {daemon}, got {got:?}"
        );
    }

    /// The next frame, decoded.
    pub async fn recv(&mut self) -> Received {
        let payload = within("a frame arrives", self.framed.next())
            .await
            .expect("the orchestrator keeps the client")
            .expect("a frame");
        let routed = decode_routed_payload(&payload).expect("a routed frame");
        let msg = match routed.body {
            RoutedBody::Opaque(inner) => control_body(inner)
                .map(|body| serde_json::from_slice(body).expect("a daemon message")),
            RoutedBody::Json(body) => {
                let value: serde_json::Value = serde_json::from_slice(body).unwrap();
                Some(serde_json::from_value(value["msg"].clone()).expect("a daemon message"))
            }
        };
        Received {
            header: routed.header,
            msg,
        }
    }

    /// The next frame on `(daemon, conn)`, skipping what other conns and grid frames carry.
    pub async fn recv_on(&mut self, daemon: &str, conn: u64) -> Received {
        loop {
            let got = self.recv().await;
            if got.header.conn.0 == conn
                && got
                    .header
                    .target
                    .as_ref()
                    .is_some_and(|t| t.as_str() == daemon)
                && got.msg.is_some()
            {
                return got;
            }
        }
    }

    /// The next control message on `(daemon, conn)` that is not one of the pushes an attach brings.
    pub async fn next_msg(&mut self, daemon: &str, conn: u64) -> DaemonMsg {
        loop {
            match self.recv_on(daemon, conn).await.msg.unwrap() {
                DaemonMsg::CatalogChanged { .. } | DaemonMsg::RunGroupsChanged { .. } => {}
                other => return other,
            }
        }
    }

    /// Read until `(daemon, conn)` ends, returning why; other frames on it are skipped.
    pub async fn expect_ended(&mut self, daemon: &str, conn: u64) -> StreamEndReason {
        loop {
            if let Some(DaemonMsg::StreamEnded { reason }) = self.recv_on(daemon, conn).await.msg {
                return reason;
            }
        }
    }

    /// Nothing arrives within `wait`.
    pub async fn expect_quiet(&mut self, wait: Duration) {
        if let Ok(Some(Ok(payload))) = tokio::time::timeout(wait, self.framed.next()).await {
            panic!("an unexpected frame: {:?}", decode_routed_payload(&payload));
        }
    }

    /// Ask the orchestrator itself for a `kind` list on conn `conn` and read its answer.
    pub async fn list(
        &mut self,
        conn: u64,
        req: u64,
        kind: micold_core::protocol::routing::ListKind,
    ) -> DaemonMsg {
        self.seq += 1;
        let header = RoutedHeader {
            target: None,
            conn: ConnectionId(conn),
            seq: Sequence(self.seq),
            req: Some(req),
        };
        let payload = micold_core::protocol::routing::encode_json_payload(
            &header,
            &ClientMsg::OrchList { req, kind },
        )
        .unwrap();
        self.framed
            .send(Bytes::from(payload))
            .await
            .expect("list sent");
        loop {
            let got = self.recv().await;
            if got.header.target.is_none() && got.header.conn.0 == conn {
                return got.msg.expect("the orchestrator answers in JSON");
            }
        }
    }

    /// Write raw frames without reading (the slow client's side).
    pub fn sink(&mut self) -> &mut Framed<Stream, tokio_util::codec::LengthDelimitedCodec> {
        &mut self.framed
    }
}
