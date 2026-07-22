//! T019 — framing + hybrid encoding (contracts/protocol.md §3, messages.md §Ordering).
//!
//! A frame exceeding the cap is rejected loudly; JSON control frames and postcard grid frames
//! interleave on one stream in total order.

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, DaemonCodec, Frame, WireFormat};
use micold_core::protocol::envelope::MAX_FRAME_LENGTH;
use micold_core::protocol::grid::{
    GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine, WireStyle,
};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::SessionId;
use tokio_util::codec::{Decoder, Encoder, Framed};
use uuid::Uuid;

fn sid() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0xabcd))
}

fn grid_frame(seq: u64) -> GridFrame {
    GridFrame {
        session: sid(),
        seq,
        generation: 0,
        full: true,
        viewport_top: LineId(0),
        oldest_available: LineId(0),
        cols: 4,
        rows: 1,
        cursor: WireCursor {
            line: LineId(0),
            col: 0,
            shape: WireCursorShape::Block,
            visible: true,
            blinking: false,
        },
        styles: vec![WireStyle {
            fg: WireColor::Named(0),
            bg: WireColor::Named(0),
            flags: 0,
            underline_color: None,
        }],
        hyperlinks: vec![],
        lines: vec![WireLine {
            id: LineId(0),
            text: "abcd".into(),
            runs: vec![StyleRun { len: 4, style: 0 }],
            extras: vec![],
            wrapped: false,
        }],
        mode: 0,
        input_serial: None,
    }
}

#[tokio::test]
async fn json_control_and_postcard_grid_interleave_in_total_order() {
    // A duplex pipe: the daemon end writes DaemonMsg + GridFrame, the client end reads them back in
    // the exact order they were sent (one stream, total order — messages.md §Ordering 1).
    let (daemon_io, client_io) = tokio::io::duplex(64 * 1024);
    let mut daemon = Framed::new(daemon_io, DaemonCodec::with_format(WireFormat::Postcard));
    let mut client = Framed::new(client_io, ClientCodec::with_format(WireFormat::Postcard));

    // Sender: control, grid, control, grid — interleaved.
    daemon
        .send(Frame::Control(DaemonMsg::Pong { nonce: 1 }))
        .await
        .unwrap();
    daemon.send(Frame::Grid(grid_frame(10))).await.unwrap();
    daemon
        .send(Frame::Control(DaemonMsg::SessionBell { session: sid() }))
        .await
        .unwrap();
    daemon.send(Frame::Grid(grid_frame(11))).await.unwrap();
    daemon.flush().await.unwrap();

    // Receiver observes the same order.
    assert_eq!(
        client.next().await.unwrap().unwrap(),
        Frame::Control(DaemonMsg::Pong { nonce: 1 })
    );
    assert_eq!(
        client.next().await.unwrap().unwrap(),
        Frame::Grid(grid_frame(10))
    );
    assert_eq!(
        client.next().await.unwrap().unwrap(),
        Frame::Control(DaemonMsg::SessionBell { session: sid() })
    );
    assert_eq!(
        client.next().await.unwrap().unwrap(),
        Frame::Grid(grid_frame(11))
    );
}

#[tokio::test]
async fn json_wire_mode_still_round_trips_grid_frames() {
    // MICOLD_WIRE=json forces grid frames to JSON too; the stream must still decode.
    let (a, b) = tokio::io::duplex(64 * 1024);
    let mut tx = Framed::new(a, DaemonCodec::with_format(WireFormat::Json));
    let mut rx = Framed::new(b, ClientCodec::with_format(WireFormat::Json));
    tx.send(Frame::Grid(grid_frame(7))).await.unwrap();
    tx.flush().await.unwrap();
    assert_eq!(
        rx.next().await.unwrap().unwrap(),
        Frame::Grid(grid_frame(7))
    );
}

#[test]
fn a_frame_exceeding_the_cap_is_rejected_on_encode() {
    // A control message whose serialized body exceeds MAX_FRAME_LENGTH must fail loudly, not
    // silently truncate (protocol.md §3, Settled Decision 8).
    let mut codec = ClientCodec::with_format(WireFormat::Postcard);
    let huge = ClientMsg::SessionInput {
        session: sid(),
        serial: 0,
        bytes: vec![0u8; MAX_FRAME_LENGTH + 1],
    };
    let mut dst = BytesMut::new();
    let err = codec.encode(Frame::Control(huge), &mut dst);
    assert!(err.is_err(), "an over-cap frame must be rejected on encode");
}

#[test]
fn a_length_prefix_claiming_more_than_the_cap_is_rejected_on_decode() {
    // Craft a length prefix well over the cap; the decoder must reject rather than allocate.
    let mut codec = DaemonCodec::with_format(WireFormat::Postcard);
    let mut src = BytesMut::new();
    let bogus_len = (MAX_FRAME_LENGTH as u32) + 1;
    src.extend_from_slice(&bogus_len.to_le_bytes());
    src.extend_from_slice(&[0u8; 16]); // a little payload; decoder should error before waiting
    let result = codec.decode(&mut src);
    assert!(
        result.is_err(),
        "a length prefix over the cap must be rejected, not honoured"
    );
}
