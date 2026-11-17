//! The client API harness (#1732): an orchestrator serving `/api/ws` on a loopback port, real
//! daemons linked to it, and a `tokio-tungstenite` client that connects with a chosen `Origin` and
//! speaks JSON frames. A feed over a scripted directory stands in where a test needs to make
//! changes at will.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::future::{BoxFuture, FutureExt};
use futures_util::{SinkExt, StreamExt};
use micold_core::client_api::messages::{ContentEntry, Daemon, DaemonStatus, Snapshot};
use micold_core::client_api::origin::OriginAllowList;
use micold_core::orchestrator::JsonRegistryStore;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator::directory::{ApiError, LinkDirectory};
use micold_daemon::orchestrator::ws::{self, ApiOptions};
use micold_daemon::orchestrator::{ApiListener, Clock};
use micold_daemon::state::DaemonState;
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::WebSocketStream;

use crate::linked_daemon::Daemon as LinkedDaemon;
use crate::support::{within, Orchestrator, TestClock};

/// The web client's local origin, allowed by every harness listener.
pub const ALLOWED: &str = "http://127.0.0.1:8731";

/// Fast timings for tests: the feed lists every 50 ms.
pub fn fast() -> ApiOptions {
    ApiOptions {
        poll: Duration::from_millis(50),
        ..ApiOptions::default()
    }
}

async fn loopback() -> TcpListener {
    TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port")
}

/// An orchestrator serving `/api/ws` too.
pub struct Api {
    pub orchestrator: Orchestrator,
    pub addr: SocketAddr,
}

impl Api {
    /// An orchestrator allowing [`ALLOWED`], with `options`.
    pub async fn start_with(options: ApiOptions) -> Self {
        let dir = Arc::new(tempfile::tempdir().expect("temp dir"));
        let socket = dir.path().join("orchestrator.sock");
        let clock = TestClock::new(1_700_000_000);
        let listener = loopback().await;
        let addr = listener.local_addr().unwrap();
        let api = ApiListener {
            listener,
            allow: OriginAllowList::new([ALLOWED]).unwrap(),
            options,
        };
        let task = tokio::spawn(micold_daemon::orchestrator::run_with_api(
            socket.clone(),
            Arc::clone(&clock) as Arc<dyn Clock>,
            Arc::new(JsonRegistryStore::in_data_dir(dir.path())),
            api,
        ));
        let orchestrator = Orchestrator::spawned(dir, socket, clock, task).await;
        Self { orchestrator, addr }
    }

    pub async fn start() -> Self {
        Self::start_with(fast()).await
    }

    /// A client from [`ALLOWED`].
    pub async fn client(&self) -> Ws {
        Ws::connect(self.addr, Some(ALLOWED))
            .await
            .expect("the upgrade is accepted")
    }

    /// Daemon `id` linked with `catalog`, stamped with its id, once it is listed online.
    pub async fn daemon(&self, id: &str, mut catalog: Catalog) -> LinkedDaemon {
        catalog.set_daemon_id(id.parse().unwrap());
        let daemon =
            LinkedDaemon::link_state(&self.orchestrator, id, Arc::new(DaemonState::new(catalog)));
        daemon.listed_online(&self.orchestrator).await;
        daemon
    }
}

/// A directory whose list the test sets.
#[derive(Default)]
pub struct Scripted {
    pub snapshot: Mutex<Snapshot>,
}

impl Scripted {
    /// Add daemon `id` at the end of the list.
    pub fn add_daemon(&self, id: &str) {
        self.snapshot.lock().unwrap().daemons.push(Daemon {
            id: id.to_owned(),
            name: id.to_owned(),
            status: DaemonStatus::Online,
        });
    }
}

impl LinkDirectory for Scripted {
    fn list(&self) -> BoxFuture<'_, Snapshot> {
        let snapshot = self.snapshot.lock().unwrap().clone();
        async move { snapshot }.boxed()
    }

    fn read_session<'a>(
        &'a self,
        _daemon: &'a str,
        _project: &'a str,
        _session: &'a str,
    ) -> BoxFuture<'a, Result<Vec<ContentEntry>, ApiError>> {
        async { Ok(Vec::new()) }.boxed()
    }

    fn changed(&self) -> BoxFuture<'_, ()> {
        std::future::pending().boxed()
    }
}

/// `/api/ws` alone over `directory`, allowing [`ALLOWED`]; the task ends with the test.
pub async fn serve(directory: Arc<dyn LinkDirectory>, options: ApiOptions) -> SocketAddr {
    let listener = loopback().await;
    let addr = listener.local_addr().unwrap();
    tokio::spawn(ws::serve(
        listener,
        OriginAllowList::new([ALLOWED]).unwrap(),
        directory,
        options,
    ));
    addr
}

/// A client of `/api/ws`.
pub struct Ws(pub WebSocketStream<TcpStream>);

impl Ws {
    /// Open `path` on `addr` with `origin`; the error is the refused upgrade.
    pub async fn connect_path(
        addr: SocketAddr,
        path: &str,
        origin: Option<&str>,
    ) -> Result<Self, tungstenite::Error> {
        let mut request = format!("ws://{addr}{path}").into_client_request().unwrap();
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert("origin", origin.parse().unwrap());
        }
        let stream = TcpStream::connect(addr)
            .await
            .expect("the listener accepts");
        let (socket, _) = tokio_tungstenite::client_async(request, stream).await?;
        Ok(Self(socket))
    }

    pub async fn connect(
        addr: SocketAddr,
        origin: Option<&str>,
    ) -> Result<Self, tungstenite::Error> {
        Self::connect_path(addr, "/api/ws", origin).await
    }

    pub async fn send(&mut self, value: Value) {
        self.send_text(value.to_string()).await;
    }

    pub async fn send_text(&mut self, text: String) {
        self.0.send(Message::text(text)).await.expect("sent");
    }

    pub async fn send_binary(&mut self, bytes: Vec<u8>) {
        self.0.send(Message::binary(bytes)).await.expect("sent");
    }

    /// The next message; panics on a binary frame (FR-006) or a close.
    pub async fn next(&mut self) -> Value {
        loop {
            match within("a server message arrives", self.0.next()).await {
                Some(Ok(Message::Text(text))) => {
                    return serde_json::from_str(text.as_str()).expect("a JSON message")
                }
                Some(Ok(Message::Binary(b))) => panic!("the server sent a binary frame: {b:?}"),
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                other => panic!("expected a message, got {other:?}"),
            }
        }
    }

    /// The reply to request `id`, skipping unsolicited messages.
    pub async fn reply(&mut self, id: u64) -> Value {
        loop {
            let message = self.next().await;
            if message["id"] == id {
                return message;
            }
        }
    }

    /// Send `request` and read its reply.
    pub async fn ask(&mut self, request: Value) -> Value {
        let id = request["id"].as_u64().expect("the request has an id");
        self.send(request).await;
        self.reply(id).await
    }

    /// The next `changes` message's changes.
    pub async fn changes(&mut self) -> Vec<Value> {
        loop {
            let message = self.next().await;
            if message["type"] == "changes" {
                return message["changes"].as_array().cloned().unwrap_or_default();
            }
        }
    }

    /// The close code the server ends the connection with (`None`: no close frame).
    pub async fn closed(&mut self) -> Option<CloseCode> {
        loop {
            match within("the server closes", self.0.next()).await {
                Some(Ok(Message::Close(frame))) => return frame.map(|f| f.code),
                Some(Ok(_)) => {}
                None | Some(Err(_)) => return None,
            }
        }
    }
}

/// The HTTP status of a refused upgrade.
pub fn refused_status(result: Result<Ws, tungstenite::Error>) -> u16 {
    match result {
        Err(tungstenite::Error::Http(response)) => response.status().as_u16(),
        Err(other) => panic!("expected an HTTP refusal, got {other:?}"),
        Ok(_) => panic!("the upgrade was accepted"),
    }
}
