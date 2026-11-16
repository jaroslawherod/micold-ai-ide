//! A virtual client connection (#1726, research R1): one routed `conn` of the orchestrator link,
//! served by the daemon's own [`crate::server::serve_connection`] over an in-memory duplex. What
//! the orchestrator sends for the conn is written to the duplex as the frames a socket client
//! would send; what the server writes is read back frame by frame and wrapped into opaque routed
//! frames for the link. Every behaviour a socket client has (handshake, takeover, streams) is the
//! one implementation.

use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt;
use micold_core::protocol::codec::raw_frame_codec;
use micold_core::protocol::envelope::{Encoding, EnvelopeHeader, Kind};
use micold_core::protocol::routing::{
    encode_opaque_payload, message_req, ConnectionId, RoutedHeader, Sequence,
};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::codec::FramedRead;

use crate::state::DaemonState;

/// How many inbound frames may wait for the server of one conn before the conn is ended.
const INBOUND_QUEUE: usize = 1024;

/// The in-memory pipe's buffer.
const DUPLEX_BYTES: usize = 64 * 1024;

/// The tag of the one answer that carries a `req` and is not final.
const NON_FINAL: &str = "OperationProgress";

/// The `req` an outbound control frame answers, if it is a final answer. Grid frames and every
/// message with no `req` give `None`.
pub fn answer_req(payload: &[u8]) -> Option<u64> {
    let (header, body) = EnvelopeHeader::parse(payload).ok()?;
    if header.kind != Kind::Control || header.encoding != Encoding::Json {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    if value.as_object()?.contains_key(NON_FINAL) {
        return None;
    }
    message_req(&value)
}

/// One virtual connection. Dropping it ends it: the server sees the client leave.
pub struct VirtualConn {
    inbound: mpsc::Sender<Bytes>,
    writer: JoinHandle<()>,
}

impl VirtualConn {
    /// Start serving `conn`; frames it writes go to `outbound` as opaque routed payloads, ending
    /// with one empty-inner payload when the conn ends.
    pub fn open(state: Arc<DaemonState>, conn: u64, outbound: mpsc::Sender<Bytes>) -> Self {
        let (server_io, link_io) = tokio::io::duplex(DUPLEX_BYTES);
        let (read_half, mut write_half) = tokio::io::split(link_io);
        let (inbound, mut inbound_rx) = mpsc::channel::<Bytes>(INBOUND_QUEUE);

        // Not aborted when the conn ends: the server must see the pipe close and release the
        // client (attachments, presence) as it does for a socket that closes.
        tokio::spawn(async move {
            if let Err(e) = crate::server::serve_connection(state, server_io).await {
                tracing::debug!(conn, error = %e, "virtual connection ended with an error");
            }
        });
        // Inner frame payloads in, length-prefixed as a socket client would write them.
        let writer = tokio::spawn(async move {
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
        // Frames out, wrapped.
        tokio::spawn(async move {
            let mut frames = FramedRead::new(read_half, raw_frame_codec());
            let mut seq = 0_u64;
            while let Some(Ok(frame)) = frames.next().await {
                seq += 1;
                let header = RoutedHeader {
                    target: None,
                    conn: ConnectionId(conn),
                    seq: Sequence(seq),
                    req: answer_req(&frame),
                };
                let payload = Bytes::from(encode_opaque_payload(&header, &frame));
                if outbound.send(payload).await.is_err() {
                    return;
                }
            }
            let header = RoutedHeader {
                target: None,
                conn: ConnectionId(conn),
                seq: Sequence(seq + 1),
                req: None,
            };
            let _ = outbound
                .send(Bytes::from(encode_opaque_payload(&header, &[])))
                .await;
        });
        Self { inbound, writer }
    }

    /// Hand the server one inner frame payload, waiting while its queue is full: the link stops
    /// reading until the server catches up, which is the back-pressure a socket gives a client.
    /// `false` when the conn ended: the caller drops it.
    pub async fn deliver(&self, frame: Bytes) -> bool {
        self.inbound.send(frame).await.is_ok()
    }

    /// Whether the server side has gone.
    pub fn is_closed(&self) -> bool {
        self.inbound.is_closed()
    }
}

impl Drop for VirtualConn {
    fn drop(&mut self) {
        // Dropping the pipe's write half is the client leaving: the server reads end-of-stream,
        // finishes and releases the client; its output then ends and the reader task follows.
        self.writer.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control(json: &str) -> Vec<u8> {
        let mut out = vec![0, 0, 0, 0];
        out.extend_from_slice(json.as_bytes());
        out
    }

    #[test]
    fn a_final_answer_names_its_request() {
        assert_eq!(
            answer_req(&control(r#"{"OperationOk":{"req":7,"result":"Ack"}}"#)),
            Some(7)
        );
        assert_eq!(
            answer_req(&control(r#"{"OperationError":{"req":3,"kind":"Busy"}}"#)),
            Some(3)
        );
    }

    #[test]
    fn progress_and_pushes_name_none() {
        assert_eq!(
            answer_req(&control(r#"{"OperationProgress":{"req":7,"stage":"x"}}"#)),
            None
        );
        assert_eq!(answer_req(&control(r#"{"Pong":{"nonce":7}}"#)), None);
        assert_eq!(answer_req(&control(r#""Goodbye""#)), None);
        // A grid frame is never decoded.
        assert_eq!(answer_req(&[0, 1, 0, 0, 1, 2, 3]), None);
    }
}
