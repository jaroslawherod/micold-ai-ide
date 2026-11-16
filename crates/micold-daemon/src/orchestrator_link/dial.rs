//! One dial attempt (contracts/dial.md, steps 2 to 5), over any stream: the preamble, the verdict,
//! `Hello`/`Welcome`, then the steady state. Every failure is an `io::Error` the task turns into
//! a retry; nothing here panics on a partial or hostile frame.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{hello_message, Credentials};
use micold_core::orchestrator::{
    encode_announcement, LinkAnnouncement, LinkVerdict, RefuseReason, MAX_PREAMBLE_FRAME,
};
use micold_core::protocol::codec::{raw_frame_codec, ClientCodec, Frame};
use micold_core::protocol::keepalive::LIVENESS_DEADLINE;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::routing::{
    control_body, control_payload, decode_routed_payload, RoutedBody,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use super::virtual_conn::VirtualConn;
use crate::server::daemon_build;
use crate::state::DaemonState;

/// How a dial attempt that reached the orchestrator ended before the steady state.
pub enum Handshake<S> {
    /// `Welcome` was read: the link is up.
    Linked(Framed<S, ClientCodec>),
    /// The orchestrator refused the announcement.
    Refused {
        /// Why.
        reason: RefuseReason,
        /// The orchestrator's sentence.
        message: String,
    },
    /// The orchestrator accepted the announcement and refused `Hello`.
    HelloRefused(String),
}

fn preamble_codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .little_endian()
        .max_frame_length(MAX_PREAMBLE_FRAME)
        .new_codec()
}

fn invalid(why: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_string())
}

fn timed_out(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!("{what} not within {LIVENESS_DEADLINE:?}"),
    )
}

/// Announce, read the verdict, send `Hello`, read the answer. Each step has the liveness deadline.
pub async fn attempt<S>(stream: S, announcement: &LinkAnnouncement) -> io::Result<Handshake<S>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let mut framed = Framed::new(stream, preamble_codec());
    let body = Bytes::from(encode_announcement(announcement));
    timeout(LIVENESS_DEADLINE, framed.send(body))
        .await
        .map_err(|_| timed_out("the announcement was not written"))??;
    let frame = timeout(LIVENESS_DEADLINE, framed.next())
        .await
        .map_err(|_| timed_out("no verdict"))?
        .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))??;
    match serde_json::from_slice::<LinkVerdict>(&frame).map_err(invalid)? {
        LinkVerdict::Accepted => {}
        LinkVerdict::Refused {
            reason, message, ..
        } => return Ok(Handshake::Refused { reason, message }),
    }

    // From here the frames are the wire protocol's.
    let mut framed = framed.map_codec(|_| ClientCodec::new());
    let hello = hello_message(&daemon_build(), &Credentials::default());
    timeout(LIVENESS_DEADLINE, framed.send(Frame::Control(hello)))
        .await
        .map_err(|_| timed_out("Hello was not written"))?
        .map_err(io::Error::from)?;
    let answer = timeout(LIVENESS_DEADLINE, framed.next())
        .await
        .map_err(|_| timed_out("no Welcome"))?
        .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?
        .map_err(io::Error::from)?;
    match answer {
        Frame::Control(DaemonMsg::Welcome { .. }) => Ok(Handshake::Linked(framed)),
        Frame::Control(DaemonMsg::Refused { reason }) => {
            Ok(Handshake::HelloRefused(format!("{reason:?}")))
        }
        other => Err(invalid(format!(
            "expected Welcome or Refused, got {other:?}"
        ))),
    }
}

/// How many frames from virtual connections may wait for the link's writer.
const OUTBOUND_QUEUE: usize = 256;

/// The steady state: answer each `LinkPing` with a `LinkPong` of its nonce; any frame resets the
/// deadline. Routed frames (#1726) are the orchestrator's clients: each `conn` is a virtual client
/// connection served by `state` ([`super::virtual_conn`]); with no `state` they are ignored.
/// Returns when the link ends: the end of the stream is `Ok`, silence past the deadline or a
/// transport error is an `Err`. Either way the link is lost, and every virtual connection with it.
///
/// Reading and writing are separate tasks: a loop that wrote while the orchestrator wrote back
/// would stop reading, and two full socket buffers would hold both sides for ever.
pub async fn steady<S>(
    framed: Framed<S, ClientCodec>,
    state: Option<Arc<DaemonState>>,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Raw frames: routed ones are relayed as bytes, never decoded as messages.
    let (mut sink, mut stream) = framed.map_codec(|_| raw_frame_codec()).split();
    let (out_tx, mut out_rx) = mpsc::channel::<Bytes>(OUTBOUND_QUEUE);
    let mut writing = AbortOnDrop(tokio::spawn(async move {
        while let Some(payload) = out_rx.recv().await {
            send_bounded(&mut sink, payload).await?;
        }
        Ok::<(), io::Error>(())
    }));
    let mut conns: HashMap<u64, VirtualConn> = HashMap::new();
    let mut deadline = Instant::now() + LIVENESS_DEADLINE;
    let result = loop {
        tokio::select! {
            frame = stream.next() => match frame {
                None => break Ok(()),
                Some(Err(e)) => break Err(e),
                Some(Ok(payload)) => {
                    deadline = Instant::now() + LIVENESS_DEADLINE;
                    match ping_nonce(&payload) {
                        Err(e) => break Err(e),
                        Ok(Some(nonce)) => {
                            // A full queue drops the pong: the frames filling it are proof of life.
                            let _ = out_tx.try_send(control_pong(nonce));
                        }
                        Ok(None) => {
                            if let Some(state) = &state {
                                route_inbound(state, &mut conns, &out_tx, &payload).await;
                            }
                        }
                    }
                }
            },
            written = &mut writing.0 => {
                break match written {
                    Ok(result) => result,
                    Err(e) => Err(io::Error::other(e)),
                };
            }
            () = tokio::time::sleep_until(deadline) => break Err(timed_out("no frame")),
        }
    };
    result
}

/// Aborts its task when dropped, including when the future owning it is cancelled (the link task
/// is aborted when it is stopped).
struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The nonce when `payload` is a `LinkPing`; `None` for any other frame. A control frame that is
/// not a daemon message is an error, as the typed codec made it.
fn ping_nonce(payload: &[u8]) -> io::Result<Option<u64>> {
    let Some(body) = control_body(payload) else {
        return Ok(None);
    };
    match serde_json::from_slice::<DaemonMsg>(body).map_err(invalid)? {
        DaemonMsg::LinkPing { nonce } => Ok(Some(nonce)),
        _ => Ok(None),
    }
}

fn control_pong(nonce: u64) -> Bytes {
    Bytes::from(control_payload(&ClientMsg::LinkPong { nonce }).expect("a pong encodes"))
}

/// Deliver one routed frame to its virtual connection, opening it on the first. A conn that has
/// ended is forgotten: the orchestrator is told by the empty frame its reader sends.
async fn route_inbound(
    state: &Arc<DaemonState>,
    conns: &mut HashMap<u64, VirtualConn>,
    out_tx: &mpsc::Sender<Bytes>,
    payload: &[u8],
) {
    let Ok(routed) = decode_routed_payload(payload) else {
        return;
    };
    let RoutedBody::Opaque(inner) = routed.body else {
        return;
    };
    let conn = routed.header.conn.0;
    if inner.is_empty() {
        conns.remove(&conn);
        return;
    }
    conns.retain(|_, c| !c.is_closed());
    let virtual_conn = conns
        .entry(conn)
        .or_insert_with(|| VirtualConn::open(Arc::clone(state), conn, out_tx.clone()));
    if !virtual_conn.deliver(Bytes::copy_from_slice(inner)).await {
        // Gone: forget it. Its reader has told the orchestrator.
        conns.remove(&conn);
    }
}

/// Send one frame, giving up after the deadline: a peer that stops reading must not stall the task.
async fn send_bounded<K>(sink: &mut K, payload: Bytes) -> io::Result<()>
where
    K: futures_util::Sink<Bytes, Error = io::Error> + Unpin,
{
    timeout(LIVENESS_DEADLINE, sink.send(payload))
        .await
        .map_err(|_| timed_out("a frame was not written"))?
}
