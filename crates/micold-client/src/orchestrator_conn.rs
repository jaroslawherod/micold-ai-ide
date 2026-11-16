//! The app's one client connection, to the local orchestrator (#1726, research R6, FR-007).
//!
//! Every daemon actor gets a [`Transport::Routed`] in place of its socket: an in-memory pipe that
//! carries the frames a socket would, which this module wraps into routed frames on the one
//! orchestrator connection (`specs/orchestrator/contracts/client-routing.md`). Each actor's pipe
//! is its own `(target, conn)`, so the actor's handshake, pump and keepalive are unchanged.
//!
//! The connection is opened on first use with [`connect_or_spawn_with`]: no orchestrator answering
//! starts one (FR-008), and the orchestrator's singleton makes concurrent starts converge on one.
//! When the connection drops every route ends, so each actor sees its link close and its existing
//! reconnect loop opens a new route, which reopens the connection (starting the orchestrator again
//! if it is gone) and re-handshakes: a fresh `Welcome` is the re-read (FR-009). A route the
//! orchestrator ends (`StreamEnded`) or answers `OutcomeUnknown` on is closed the same way, so the
//! actor re-requests its target's snapshot instead of guessing an outcome.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{connect_or_spawn_with, dial, hello_message, Credentials, Transport};
use micold_core::endpoint::Endpoint;
use micold_core::protocol::codec::raw_frame_codec;
use micold_core::protocol::messages::DaemonMsg;
use micold_core::protocol::routing::{
    control_body, control_payload, decode_routed_payload, encode_opaque_payload, message_req,
    ConnectionId, RoutedBody, RoutedHeader, Sequence, WireDaemonId,
};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

/// How long a started orchestrator is given to accept.
pub const START_TIMEOUT: Duration = Duration::from_secs(10);

/// Frames waiting for one route's pipe, and for the connection.
const QUEUE: usize = 1024;

/// One route's in-memory pipe.
const PIPE_BYTES: usize = 64 * 1024;

/// How the orchestrator is reached and started.
#[derive(Clone)]
pub struct Starter {
    /// Its socket.
    pub endpoint: Endpoint,
    /// Start one, detached. Called again while nothing accepts (a start that lost the singleton
    /// race exits).
    pub spawn: Arc<dyn Fn() -> io::Result<()> + Send + Sync>,
}

impl Starter {
    /// The user's orchestrator: its endpoint beside the daemon's, started as
    /// `micold-daemon orchestrator`.
    pub fn local() -> io::Result<Self> {
        Ok(Self {
            endpoint: micold_core::endpoint::orchestrator_endpoint()?,
            spawn: Arc::new(|| {
                micold_core::spawn::spawn_detached(micold_core::spawn::orchestrator_command())
                    .map(|_| ())
                    .map_err(|e| {
                        io::Error::new(e.kind(), format!("the orchestrator did not start: {e}"))
                    })
            }),
        })
    }
}

/// The routes of one connection, by `(target, conn)`.
type Routes = Arc<Mutex<HashMap<(WireDaemonId, u64), mpsc::Sender<Bytes>>>>;

/// What a route hands the connection: an inner frame, or empty for "this conn ends".
struct Outbound {
    target: WireDaemonId,
    conn: u64,
    inner: Bytes,
}

/// A live connection.
struct Link {
    outbound: mpsc::Sender<Outbound>,
    routes: Routes,
}

/// The app's orchestrator connection, shared by every actor.
pub struct OrchestratorConn {
    starter: Starter,
    link: tokio::sync::Mutex<Option<Link>>,
    next_conn: AtomicU64,
    connections: AtomicU64,
}

impl OrchestratorConn {
    pub fn new(starter: Starter) -> Self {
        Self {
            starter,
            link: tokio::sync::Mutex::new(None),
            next_conn: AtomicU64::new(1),
            connections: AtomicU64::new(0),
        }
    }

    /// How many connections to the orchestrator this has opened, over its life.
    pub fn connections_opened(&self) -> u64 {
        self.connections.load(Ordering::SeqCst)
    }

    /// Whether the connection is up now.
    pub async fn is_connected(&self) -> bool {
        self.link
            .lock()
            .await
            .as_ref()
            .is_some_and(|l| !l.outbound.is_closed())
    }

    /// A new route to `target`, opening (and if need be starting) the orchestrator connection.
    /// The returned transport is what a daemon socket would be: the caller sends `Hello` on it.
    pub async fn open(&self, target: &WireDaemonId) -> io::Result<Transport> {
        let mut guard = self.link.lock().await;
        if guard.as_ref().is_none_or(|l| l.outbound.is_closed()) {
            *guard = Some(self.connect().await?);
        }
        let link = guard.as_ref().expect("connected above");
        let conn = self.next_conn.fetch_add(1, Ordering::SeqCst);
        Ok(Transport::Routed(open_route(link, target.clone(), conn)))
    }

    async fn connect(&self) -> io::Result<Link> {
        let endpoint = self.starter.endpoint.clone();
        let spawn = Arc::clone(&self.starter.spawn);
        let framed = connect_or_spawn_with(
            &endpoint,
            START_TIMEOUT,
            || introduce(&endpoint),
            || spawn(),
            "orchestrator",
        )
        .await?;
        self.connections.fetch_add(1, Ordering::SeqCst);
        let (outbound, outbound_rx) = mpsc::channel(QUEUE);
        let routes: Routes = Arc::default();
        tokio::spawn(run_link(framed, outbound_rx, Arc::clone(&routes)));
        Ok(Link { outbound, routes })
    }
}

type Framed = tokio_util::codec::Framed<Transport, LengthDelimitedCodec>;

/// Dial and send the plain `Hello`; `None` while nothing accepts.
async fn introduce(endpoint: &Endpoint) -> io::Result<Option<Framed>> {
    let Some(stream) = dial(endpoint).await? else {
        return Ok(None);
    };
    let mut framed = Framed::new(stream, raw_frame_codec());
    let hello = hello_message(crate::daemon::CLIENT_BUILD, &Credentials::default());
    let payload = control_payload(&hello).map_err(io::Error::other)?;
    if framed.send(Bytes::from(payload)).await.is_err() {
        return Ok(None);
    }
    let Some(Ok(answer)) = framed.next().await else {
        // A departing orchestrator: absence, as for a daemon.
        return Ok(None);
    };
    let msg: DaemonMsg = control_body(&answer)
        .and_then(|body| serde_json::from_slice(body).ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "the orchestrator's answer"))?;
    match msg {
        DaemonMsg::Welcome { .. } => Ok(Some(framed)),
        DaemonMsg::Refused { reason } => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("the orchestrator refused this app: {reason:?}"),
        )),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected Welcome from the orchestrator, got {other:?}"),
        )),
    }
}

/// Register `(target, conn)` and return the actor's end of its pipe.
fn open_route(link: &Link, target: WireDaemonId, conn: u64) -> tokio::io::DuplexStream {
    let (actor_io, route_io) = tokio::io::duplex(PIPE_BYTES);
    let (read_half, mut write_half) = tokio::io::split(route_io);
    let (inbound, mut inbound_rx) = mpsc::channel::<Bytes>(QUEUE);
    link.routes
        .lock()
        .expect("routes lock")
        .insert((target.clone(), conn), inbound);

    // Frames from the orchestrator into the pipe. Ends when the route is removed, then shuts the
    // pipe so the actor reads the end.
    tokio::spawn(async move {
        while let Some(frame) = inbound_rx.recv().await {
            let len = u32::try_from(frame.len()).unwrap_or(u32::MAX).to_le_bytes();
            if write_half.write_all(&len).await.is_err()
                || write_half.write_all(&frame).await.is_err()
            {
                break;
            }
        }
        let _ = write_half.shutdown().await;
    });
    // Frames the actor writes, out to the connection; empty when the actor closes its end.
    let outbound = link.outbound.clone();
    let routes = Arc::clone(&link.routes);
    tokio::spawn(async move {
        let mut frames = FramedRead::new(read_half, raw_frame_codec());
        while let Some(Ok(frame)) = frames.next().await {
            let out = Outbound {
                target: target.clone(),
                conn,
                inner: frame.freeze(),
            };
            if outbound.send(out).await.is_err() {
                return;
            }
        }
        if routes
            .lock()
            .expect("routes lock")
            .remove(&(target.clone(), conn))
            .is_some()
        {
            let _ = outbound
                .send(Outbound {
                    target,
                    conn,
                    inner: Bytes::new(),
                })
                .await;
        }
    });
    actor_io
}

/// The `req` header of an inner frame: set for a control message that carries one.
fn request_req(inner: &[u8]) -> Option<u64> {
    let value: serde_json::Value = serde_json::from_slice(control_body(inner)?).ok()?;
    message_req(&value)
}

/// What one frame from the orchestrator means for its route.
#[derive(Debug, PartialEq, Eq)]
enum Delivery {
    /// Hand this inner frame to the route.
    Frame(Bytes),
    /// The route is over: close it, so its actor re-handshakes (and so re-reads).
    End,
    /// Not for a route.
    Drop,
}

fn delivery(payload: &[u8]) -> (Option<(WireDaemonId, u64)>, Delivery) {
    let Ok(routed) = decode_routed_payload(payload) else {
        return (None, Delivery::Drop);
    };
    let key = routed
        .header
        .target
        .clone()
        .map(|t| (t, routed.header.conn.0));
    let what = match routed.body {
        RoutedBody::Opaque([]) => Delivery::End,
        RoutedBody::Opaque(inner) => Delivery::Frame(Bytes::copy_from_slice(inner)),
        RoutedBody::Json(body) => json_delivery(body),
    };
    (key, what)
}

/// A JSON-form frame is the orchestrator's own: a refusal is handed to the actor as its daemon
/// would have answered; `StreamEnded` and `OutcomeUnknown` end the route (FR-009).
fn json_delivery(body: &[u8]) -> Delivery {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
        return Delivery::Drop;
    };
    let msg = &value["msg"];
    let ends = msg
        .as_object()
        .is_some_and(|m| m.contains_key("StreamEnded") || m.contains_key("OutcomeUnknown"));
    if ends {
        return Delivery::End;
    }
    match control_payload(msg) {
        Ok(inner) => Delivery::Frame(Bytes::from(inner)),
        Err(_) => Delivery::Drop,
    }
}

/// Pump one connection until it ends; then every route ends with it.
async fn run_link(framed: Framed, mut outbound: mpsc::Receiver<Outbound>, routes: Routes) {
    let (mut sink, mut stream) = framed.split();
    let mut seq = 0_u64;
    loop {
        tokio::select! {
            out = outbound.recv() => {
                let Some(out) = out else { break };
                seq += 1;
                let header = RoutedHeader {
                    req: request_req(&out.inner),
                    target: Some(out.target),
                    conn: ConnectionId(out.conn),
                    seq: Sequence(seq),
                };
                if sink.send(Bytes::from(encode_opaque_payload(&header, &out.inner))).await.is_err() {
                    break;
                }
            }
            frame = stream.next() => {
                let Some(Ok(frame)) = frame else { break };
                let (Some(key), what) = delivery(&frame) else { continue };
                let route = routes.lock().expect("routes lock").get(&key).cloned();
                match (what, route) {
                    (Delivery::Frame(inner), Some(route)) => {
                        if route.send(inner).await.is_err() {
                            routes.lock().expect("routes lock").remove(&key);
                        }
                    }
                    (Delivery::End, _) => {
                        routes.lock().expect("routes lock").remove(&key);
                    }
                    _ => {}
                }
            }
        }
    }
    // Dropping every route's sender shuts its pipe: each actor reads the end and reconnects.
    routes.lock().expect("routes lock").clear();
    outbound.close();
}

static SHARED: OnceLock<OrchestratorConn> = OnceLock::new();

/// The app's one orchestrator connection.
pub fn shared() -> io::Result<&'static OrchestratorConn> {
    if let Some(conn) = SHARED.get() {
        return Ok(conn);
    }
    let starter = Starter::local()?;
    Ok(SHARED.get_or_init(|| OrchestratorConn::new(starter)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::protocol::routing::encode_json_payload;

    fn header(req: Option<u64>) -> RoutedHeader {
        RoutedHeader {
            target: Some(WireDaemonId::local()),
            conn: ConnectionId(3),
            seq: Sequence(1),
            req,
        }
    }

    #[test]
    fn a_request_names_its_req_and_a_push_none() {
        let ping = control_payload(&serde_json::json!({"Ping": {"nonce": 1}})).unwrap();
        assert_eq!(request_req(&ping), None);
        let op = control_payload(&serde_json::json!({"Op": {"req": 7, "op": "x"}})).unwrap();
        assert_eq!(request_req(&op), Some(7));
        assert_eq!(request_req(&[0, 1, 0, 0, 9]), None, "a grid frame");
    }

    #[test]
    fn stream_ended_and_outcome_unknown_end_the_route() {
        let ended = encode_json_payload(
            &header(None),
            &serde_json::json!({"StreamEnded": {"reason": "daemon-offline"}}),
        )
        .unwrap();
        assert_eq!(delivery(&ended).1, Delivery::End);
        let unknown = encode_json_payload(
            &header(Some(11)),
            &serde_json::json!({"OutcomeUnknown": {"req": 11}}),
        )
        .unwrap();
        let (key, what) = delivery(&unknown);
        assert_eq!(key, Some((WireDaemonId::local(), 3)));
        assert_eq!(what, Delivery::End);
        assert_eq!(
            delivery(&encode_opaque_payload(&header(None), &[])).1,
            Delivery::End
        );
    }

    #[test]
    fn a_refusal_reaches_the_actor_as_a_control_frame() {
        let msg = serde_json::json!({"OperationError": {"req": 3, "kind": "NotFound", "message": "unknown daemon", "detail": null}});
        let refused = encode_json_payload(&header(Some(3)), &msg).unwrap();
        assert_eq!(
            delivery(&refused).1,
            Delivery::Frame(Bytes::from(control_payload(&msg).unwrap()))
        );
    }
}
