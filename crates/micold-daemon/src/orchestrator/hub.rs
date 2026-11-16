//! What the links and the client sessions share (#1726): the route table, each live link's queue
//! and each client's writer. One lock, never held across an `.await`.
//!
//! Frames toward a client go through one bounded queue per route, drained by a pump task into the
//! client's writer queue, so a client that stops reading stalls only its own routes: a full route
//! queue ends that route with `slow-consumer` and nothing else (FR-004).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use bytes::Bytes;
use micold_core::orchestrator::route::{ClientId, ClientKey, DroppedRoute, RouteTable};
use micold_core::protocol::messages::DaemonMsg;
use micold_core::protocol::routing::{
    control_body, control_payload, encode_json_payload, encode_opaque_payload, ConnectionId,
    RoutedHeader, Sequence, StreamEndReason, WireDaemonId,
};
use tokio::sync::mpsc;
use tokio::task::AbortHandle;

/// Frames that may wait toward one client on one route.
pub const ROUTE_QUEUE: usize = 64;

/// Frames that may wait toward one daemon on one route: a client may burst this many requests
/// before its daemon has read any of them.
pub const TO_LINK_QUEUE: usize = 1024;

/// Frames that may wait in a client's writer, and toward a link.
pub const WRITER_QUEUE: usize = 256;

/// The queue toward a daemon's link.
#[derive(Clone)]
pub struct LinkOut {
    generation: u64,
    /// Whole link-frame payloads, written by the link task.
    pub tx: mpsc::Sender<Bytes>,
}

struct RouteOut {
    /// Toward the client, drained by `pump`.
    queue: mpsc::Sender<Bytes>,
    pump: AbortHandle,
    /// Toward the daemon's link, drained by `forwarder`: a daemon that stops reading fills this
    /// route's queue, not the client's read loop.
    to_link: mpsc::Sender<Bytes>,
    forwarder: AbortHandle,
    json: bool,
    seq: u64,
}

#[derive(Default)]
struct Inner {
    routes: RouteTable,
    links: HashMap<WireDaemonId, LinkOut>,
    clients: HashMap<ClientId, mpsc::Sender<Bytes>>,
    out: HashMap<(WireDaemonId, u64), RouteOut>,
    /// Conns ended as `slow-consumer`: frames on them are dropped until the client closes them,
    /// so a client that keeps writing does not reopen a route it cannot keep up with.
    ended: HashSet<ClientKey>,
}

/// The shared routing state.
#[derive(Default)]
pub struct Hub {
    inner: Mutex<Inner>,
    next_id: AtomicU64,
}

/// What opening or finding a route for a client frame came to.
pub enum Opened {
    /// The route exists; send the frame on this link conn with [`Hub::to_daemon`].
    Route {
        /// The conn on the link.
        link_conn: u64,
    },
    /// The daemon has no registered link.
    NoLink,
    /// The conn was ended as too slow and the client has not closed it: drop the frame.
    Ended,
}

impl Hub {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// A daemon's link is up and takes frames through `tx`. Returns its generation, which
    /// [`Self::unregister_link`] needs, so a replaced link's end does not unhook its successor.
    pub fn register_link(&self, daemon: &WireDaemonId, tx: mpsc::Sender<Bytes>) -> u64 {
        let generation = self.id();
        self.lock()
            .links
            .insert(daemon.clone(), LinkOut { generation, tx });
        generation
    }

    /// The link of `daemon` is gone (or silent): end every route on it, telling each client
    /// `OutcomeUnknown` for every request not yet answered, then `StreamEnded`. `generation` is
    /// `Some` to also unhook the link, only when it is still the registered one.
    pub fn link_lost(&self, daemon: &WireDaemonId, unhook: Option<u64>) {
        let ended = {
            let mut inner = self.lock();
            if let Some(generation) = unhook {
                if inner
                    .links
                    .get(daemon)
                    .is_some_and(|l| l.generation == generation)
                {
                    inner.links.remove(daemon);
                } else {
                    return;
                }
            }
            let dropped = inner.routes.drop_link(daemon);
            dropped
                .into_iter()
                .filter_map(|d| {
                    let out = inner.out.remove(&(daemon.clone(), d.link_conn))?;
                    Some((d, out))
                })
                .collect::<Vec<_>>()
        };
        for (dropped, out) in ended {
            let tail = end_tail(&dropped, &out, StreamEndReason::DaemonOffline);
            finish(out, tail);
        }
    }

    /// A client session starts, writing through `writer`.
    pub fn register_client(&self, writer: mpsc::Sender<Bytes>) -> ClientId {
        let id = self.id();
        self.lock().clients.insert(id, writer);
        id
    }

    /// A client left: close its routes. Each daemon's link is told, with an empty frame queued
    /// behind what the route already sent, to end the conn. Requests in flight get no answer
    /// (nobody is left to read it).
    pub fn client_gone(&self, client: ClientId) {
        let mut inner = self.lock();
        inner.clients.remove(&client);
        inner.ended.retain(|k| k.client != client);
        for dropped in inner.routes.close_client(client) {
            if let Some(out) = inner.out.remove(&(dropped.key.daemon.clone(), dropped.link_conn)) {
                out.pump.abort();
                close_in_order(&out.to_link, dropped.link_conn);
            }
        }
    }

    /// Find or open the route for `key`, with `json` the form the client speaks on it. `None`
    /// when `client` is unknown (it left meanwhile).
    pub fn open_route(&self, key: &ClientKey, json: bool) -> Option<Opened> {
        let mut inner = self.lock();
        let client_tx = inner.clients.get(&key.client)?.clone();
        if inner.ended.contains(key) {
            return Some(Opened::Ended);
        }
        let Some(link) = inner.links.get(&key.daemon).cloned() else {
            return Some(Opened::NoLink);
        };
        let (link_conn, fresh) = inner.routes.open(key);
        if fresh {
            let (queue, rx) = mpsc::channel(ROUTE_QUEUE);
            let pump = tokio::spawn(pump(rx, client_tx)).abort_handle();
            let (to_link, link_rx) = mpsc::channel(TO_LINK_QUEUE);
            let forwarder = tokio::spawn(forward(link_rx, link.tx.clone())).abort_handle();
            inner.out.insert(
                (key.daemon.clone(), link_conn),
                RouteOut {
                    queue,
                    pump,
                    to_link,
                    forwarder,
                    json,
                    seq: 0,
                },
            );
        }
        Some(Opened::Route { link_conn })
    }

    /// Note a request forwarded on a route.
    pub fn forwarded(&self, daemon: &WireDaemonId, link_conn: u64, req: u64) {
        self.lock().routes.forwarded(daemon, link_conn, req);
    }

    /// The client closed its conn: end the route, telling the daemon's link after the frames the
    /// route already sent.
    pub fn client_closed(&self, key: &ClientKey) {
        let mut inner = self.lock();
        inner.ended.remove(key);
        let Some(dropped) = inner.routes.close(key) else {
            return;
        };
        if let Some(out) = inner.out.remove(&(key.daemon.clone(), dropped.link_conn)) {
            out.pump.abort();
            close_in_order(&out.to_link, dropped.link_conn);
        }
    }

    /// Queue a client frame toward the daemon on its route. A full queue means the daemon is not
    /// keeping up with this route: the route ends (`slow-consumer`, its requests `unknown`) and
    /// nothing else waits (FR-004).
    pub fn to_daemon(&self, key: &ClientKey, link_conn: u64, frame: Bytes) {
        let slot = (key.daemon.clone(), link_conn);
        let slow = {
            let mut guard = self.lock();
            let state = &mut *guard;
            match state.out.get(&slot) {
                None => return,
                Some(out) if out.to_link.try_send(frame).is_ok() => return,
                Some(_) => {}
            }
            let out = state.out.remove(&slot).expect("just found");
            let dropped = state.routes.close(key);
            state.ended.insert(key.clone());
            let tail = dropped.map_or_else(Vec::new, |d| {
                end_tail(&d, &out, StreamEndReason::SlowConsumer)
            });
            (out, tail, state.links.get(&key.daemon).cloned())
        };
        end_slow(slow.0, slow.1, slow.2, link_conn);
    }

    /// A frame from `daemon`'s link on `link_conn`, carrying `req` if it is a final answer: hand
    /// `inner` (one inner frame payload) to the client. An empty `inner` is the daemon ending the
    /// conn. A full route queue ends the route as `slow-consumer`, telling the daemon to close it.
    pub fn from_daemon(
        &self,
        daemon: &WireDaemonId,
        link_conn: u64,
        req: Option<u64>,
        inner: &[u8],
    ) {
        enum Next {
            Nothing,
            Ended(RouteOut, Vec<Bytes>),
            Slow(RouteOut, Vec<Bytes>, Option<LinkOut>),
        }
        let next = {
            let mut guard = self.lock();
            let state = &mut *guard;
            match state.routes.client_of(daemon, link_conn).cloned() {
                None => Next::Nothing,
                Some(key) => {
                    let slot = (daemon.clone(), link_conn);
                    if inner.is_empty() {
                        let dropped = state.routes.close(&key);
                        match (dropped, state.out.remove(&slot)) {
                            (Some(dropped), Some(out)) => {
                                let tail = end_tail(&dropped, &out, StreamEndReason::Closed);
                                Next::Ended(out, tail)
                            }
                            _ => Next::Nothing,
                        }
                    } else if let Some(out) = state.out.get_mut(&slot) {
                        let seq = out.seq + 1;
                        let header = RoutedHeader {
                            target: Some(daemon.clone()),
                            conn: key.conn,
                            seq: Sequence(seq),
                            req,
                        };
                        let payload = Bytes::from(client_form(&header, inner, out.json));
                        if out.queue.try_send(payload).is_ok() {
                            // Answered only once the frame is queued: a frame that never was
                            // leaves its request pending, so it is reported `unknown`.
                            out.seq = seq;
                            if let Some(req) = req {
                                state.routes.answered(daemon, link_conn, req);
                            }
                            Next::Nothing
                        } else {
                            // Full (or the pump ended): this route alone is too slow.
                            let out = state.out.remove(&slot).expect("just found");
                            let dropped = state.routes.close(&key);
                            state.ended.insert(key.clone());
                            let tail = dropped.map_or_else(Vec::new, |d| {
                                end_tail(&d, &out, StreamEndReason::SlowConsumer)
                            });
                            Next::Slow(out, tail, state.links.get(daemon).cloned())
                        }
                    } else {
                        Next::Nothing
                    }
                }
            }
        };
        match next {
            Next::Nothing => {}
            Next::Ended(out, tail) => finish(out, tail),
            Next::Slow(out, tail, link) => end_slow(out, tail, link, link_conn),
        }
    }

    /// Send the client a frame that is not on a route (a refusal, an answer of the orchestrator's
    /// own). Delivered in the background when the writer is full.
    pub fn to_client(&self, client: ClientId, payload: Bytes) {
        let tx = self.lock().clients.get(&client).cloned();
        if let Some(tx) = tx {
            if let Err(mpsc::error::TrySendError::Full(payload)) = tx.try_send(payload) {
                tokio::spawn(async move {
                    let _ = tx.send(payload).await;
                });
            }
        }
    }

    /// How many routes are open (tests).
    pub fn route_count(&self) -> usize {
        self.lock().routes.len()
    }
}

/// The frames that close a dropped route: `OutcomeUnknown` for each request still pending, then
/// `StreamEnded` with `reason`. Numbered on from what the route already sent.
fn end_tail(dropped: &DroppedRoute, out: &RouteOut, reason: StreamEndReason) -> Vec<Bytes> {
    let mut seq = out.seq;
    let mut tail = Vec::new();
    for req in &dropped.pending {
        seq += 1;
        tail.push(frame_for(
            &dropped.key,
            out.json,
            seq,
            Some(*req),
            &DaemonMsg::OutcomeUnknown { req: *req },
        ));
    }
    tail.push(frame_for(
        &dropped.key,
        out.json,
        seq + 1,
        None,
        &DaemonMsg::StreamEnded { reason },
    ));
    tail
}

/// The payload a client gets for `inner` from the daemon, in the form it speaks.
fn client_form(header: &RoutedHeader, inner: &[u8], json: bool) -> Vec<u8> {
    if json {
        if let Some(body) = control_body(inner) {
            if let Ok(msg) = serde_json::from_slice::<serde_json::Value>(body) {
                if let Ok(payload) = encode_json_payload(header, &msg) {
                    return payload;
                }
            }
        }
    }
    encode_opaque_payload(header, inner)
}

/// A frame of the orchestrator's own for a route, in the form its client speaks.
pub fn frame_for(
    key: &ClientKey,
    json: bool,
    seq: u64,
    req: Option<u64>,
    msg: &DaemonMsg,
) -> Bytes {
    let header = RoutedHeader {
        target: Some(key.daemon.clone()),
        conn: key.conn,
        seq: Sequence(seq),
        req,
    };
    let payload = if json {
        encode_json_payload(&header, msg).expect("a message encodes")
    } else {
        encode_opaque_payload(&header, &control_payload(msg).expect("a message encodes"))
    };
    Bytes::from(payload)
}

/// A frame for the orchestrator's own conversation with a client: no target.
pub fn orchestrator_frame(
    conn: ConnectionId,
    seq: u64,
    req: Option<u64>,
    json: bool,
    msg: &DaemonMsg,
) -> Bytes {
    let header = RoutedHeader {
        target: None,
        conn,
        seq: Sequence(seq),
        req,
    };
    let payload = if json {
        encode_json_payload(&header, msg).expect("a message encodes")
    } else {
        encode_opaque_payload(&header, &control_payload(msg).expect("a message encodes"))
    };
    Bytes::from(payload)
}

/// Pump a route's queue into its client's writer.
async fn pump(mut rx: mpsc::Receiver<Bytes>, client: mpsc::Sender<Bytes>) {
    while let Some(frame) = rx.recv().await {
        if client.send(frame).await.is_err() {
            break;
        }
    }
}

/// End a route in order: the `tail` frames queue behind what is already waiting, and the pump
/// ends by itself when the queue closes. Nothing already queued is dropped: when the queue has no
/// room the rest of the tail waits for it from a task (the client's writer drains it, or ends).
fn finish(out: RouteOut, tail: Vec<Bytes>) {
    out.forwarder.abort();
    let mut rest = tail.into_iter();
    let mut left = Vec::new();
    for frame in rest.by_ref() {
        if let Err(e) = out.queue.try_send(frame) {
            left.push(e.into_inner());
            break;
        }
    }
    left.extend(rest);
    if !left.is_empty() {
        tokio::spawn(async move {
            for frame in left {
                if out.queue.send(frame).await.is_err() {
                    break;
                }
            }
        });
    }
}

/// End a route that is too slow: tell the client, and tell the daemon's link to end the conn.
fn end_slow(out: RouteOut, tail: Vec<Bytes>, link: Option<LinkOut>, link_conn: u64) {
    finish(out, tail);
    if let Some(link) = link {
        let payload = close_frame(link_conn);
        if let Err(mpsc::error::TrySendError::Full(payload)) = link.tx.try_send(payload) {
            tokio::spawn(async move {
                let _ = link.tx.send(payload).await;
            });
        }
    }
}

/// Drain a route's queue toward the daemon into its link, in order.
async fn forward(mut rx: mpsc::Receiver<Bytes>, link: mpsc::Sender<Bytes>) {
    while let Some(frame) = rx.recv().await {
        if link.send(frame).await.is_err() {
            break;
        }
    }
}

/// The frame that tells a daemon's link to end `link_conn`: an opaque frame with an empty inner
/// part.
fn close_frame(link_conn: u64) -> Bytes {
    let header = RoutedHeader {
        target: None,
        conn: ConnectionId(link_conn),
        seq: Sequence(0),
        req: None,
    };
    Bytes::from(encode_opaque_payload(&header, &[]))
}

/// Queue the end of `link_conn` behind what the route already sent toward its daemon. The
/// forwarder ends once the queue, now without its sender, is drained.
fn close_in_order(to_link: &mpsc::Sender<Bytes>, link_conn: u64) {
    let frame = close_frame(link_conn);
    if let Err(mpsc::error::TrySendError::Full(frame)) = to_link.try_send(frame) {
        let to_link = to_link.clone();
        tokio::spawn(async move {
            let _ = to_link.send(frame).await;
        });
    }
}
