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
use micold_core::orchestrator::route::{ClientId, ClientKey, RouteTable};
use micold_core::protocol::messages::DaemonMsg;
use micold_core::protocol::routing::{
    control_body, control_payload, encode_json_payload, encode_opaque_payload, ConnectionId,
    RoutedHeader, Sequence, StreamEndReason, WireDaemonId,
};
use tokio::sync::mpsc;
use tokio::task::AbortHandle;

/// Frames that may wait toward one client on one route.
pub const ROUTE_QUEUE: usize = 64;

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
    queue: mpsc::Sender<Bytes>,
    pump: AbortHandle,
    client_tx: mpsc::Sender<Bytes>,
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
    /// The route exists; send the frame on this link conn through `link`.
    Route {
        /// The link's queue.
        link: LinkOut,
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
            let mut tail = Vec::new();
            let mut seq = out.seq;
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
            seq += 1;
            tail.push(frame_for(
                &dropped.key,
                out.json,
                seq,
                None,
                &DaemonMsg::StreamEnded {
                    reason: StreamEndReason::DaemonOffline,
                },
            ));
            finish(out, tail);
        }
    }

    /// A client session starts, writing through `writer`.
    pub fn register_client(&self, writer: mpsc::Sender<Bytes>) -> ClientId {
        let id = self.id();
        self.lock().clients.insert(id, writer);
        id
    }

    /// A client left: close its routes. Each daemon's link is told, with an empty frame, to end the
    /// conn. Requests in flight get no answer (nobody is left to read it).
    pub fn client_gone(&self, client: ClientId) {
        let (closed, links) = {
            let mut inner = self.lock();
            inner.clients.remove(&client);
            inner.ended.retain(|k| k.client != client);
            let closed = inner.routes.close_client(client);
            let links: Vec<_> = closed
                .iter()
                .map(|d| inner.links.get(&d.key.daemon).cloned())
                .collect();
            for d in &closed {
                if let Some(out) = inner.out.remove(&(d.key.daemon.clone(), d.link_conn)) {
                    out.pump.abort();
                }
            }
            (closed, links)
        };
        for (dropped, link) in closed.into_iter().zip(links) {
            if let Some(link) = link {
                close_on_link(&link, dropped.link_conn);
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
            let pump = tokio::spawn(pump(rx, client_tx.clone())).abort_handle();
            inner.out.insert(
                (key.daemon.clone(), link_conn),
                RouteOut {
                    queue,
                    pump,
                    client_tx,
                    json,
                    seq: 0,
                },
            );
        }
        Some(Opened::Route { link, link_conn })
    }

    /// Note a request forwarded on a route.
    pub fn forwarded(&self, daemon: &WireDaemonId, link_conn: u64, req: u64) {
        self.lock().routes.forwarded(daemon, link_conn, req);
    }

    /// The client closed its conn: end the route; returns the link to tell.
    pub fn client_closed(&self, key: &ClientKey) -> Option<(LinkOut, u64)> {
        let mut inner = self.lock();
        inner.ended.remove(key);
        let dropped = inner.routes.close(key)?;
        if let Some(out) = inner.out.remove(&(key.daemon.clone(), dropped.link_conn)) {
            out.pump.abort();
        }
        inner
            .links
            .get(&key.daemon)
            .cloned()
            .map(|l| (l, dropped.link_conn))
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
            Ended(RouteOut, Bytes),
            Slow(RouteOut, Bytes, Option<LinkOut>),
        }
        let next = {
            let mut guard = self.lock();
            let state = &mut *guard;
            match state.routes.client_of(daemon, link_conn).cloned() {
                None => Next::Nothing,
                Some(key) => {
                    if let Some(req) = req {
                        state.routes.answered(daemon, link_conn, req);
                    }
                    let slot = (daemon.clone(), link_conn);
                    if inner.is_empty() {
                        let _ = state.routes.close(&key);
                        match state.out.remove(&slot) {
                            Some(out) => {
                                let end = ended(&key, &out, StreamEndReason::Closed);
                                Next::Ended(out, end)
                            }
                            None => Next::Nothing,
                        }
                    } else if let Some(out) = state.out.get_mut(&slot) {
                        out.seq += 1;
                        let header = RoutedHeader {
                            target: Some(daemon.clone()),
                            conn: key.conn,
                            seq: Sequence(out.seq),
                            req,
                        };
                        let payload = Bytes::from(client_form(&header, inner, out.json));
                        if out.queue.try_send(payload).is_ok() {
                            Next::Nothing
                        } else {
                            // Full (or the pump ended): this route alone is too slow.
                            let out = state.out.remove(&slot).expect("just found");
                            let _ = state.routes.close(&key);
                            state.ended.insert(key.clone());
                            let end = ended(&key, &out, StreamEndReason::SlowConsumer);
                            Next::Slow(out, end, state.links.get(daemon).cloned())
                        }
                    } else {
                        Next::Nothing
                    }
                }
            }
        };
        match next {
            Next::Nothing => {}
            Next::Ended(out, end) => finish(out, vec![end]),
            Next::Slow(out, end, link) => {
                out.pump.abort();
                finish_after_abort(&out, vec![end]);
                if let Some(link) = link {
                    close_on_link(&link, link_conn);
                }
            }
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

/// The `StreamEnded` frame that closes `out`'s route.
fn ended(key: &ClientKey, out: &RouteOut, reason: StreamEndReason) -> Bytes {
    frame_for(
        key,
        out.json,
        out.seq + 1,
        None,
        &DaemonMsg::StreamEnded { reason },
    )
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

/// End a route in order: the `tail` frames queue behind what is already waiting; the pump then
/// ends by itself when the queue closes. When the queue cannot take them the route is slow, and
/// they go straight to the client's writer instead.
fn finish(out: RouteOut, tail: Vec<Bytes>) {
    let mut rest = tail.into_iter();
    let mut undelivered = Vec::new();
    for frame in rest.by_ref() {
        if out.queue.try_send(frame.clone()).is_err() {
            undelivered.push(frame);
            break;
        }
    }
    undelivered.extend(rest);
    if !undelivered.is_empty() {
        out.pump.abort();
        finish_after_abort(&out, undelivered);
    }
}

/// Deliver `tail` to the client's writer from a task (the route's queue is gone).
fn finish_after_abort(out: &RouteOut, tail: Vec<Bytes>) {
    let tx = out.client_tx.clone();
    tokio::spawn(async move {
        for frame in tail {
            if tx.send(frame).await.is_err() {
                break;
            }
        }
    });
}

/// Tell a daemon's link to end `link_conn`: an opaque frame with an empty inner part.
pub fn close_on_link(link: &LinkOut, link_conn: u64) {
    let header = RoutedHeader {
        target: None,
        conn: ConnectionId(link_conn),
        seq: Sequence(0),
        req: None,
    };
    let payload = Bytes::from(encode_opaque_payload(&header, &[]));
    if let Err(mpsc::error::TrySendError::Full(payload)) = link.tx.try_send(payload) {
        let tx = link.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(payload).await;
        });
    }
}
