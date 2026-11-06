//! #1723 M2 — the routed envelope through the frame codec (FR-004 to FR-006, SC-001, SC-002).

use bytes::BytesMut;
use micold_core::protocol::codec::{ClientCodec, CodecError, DaemonCodec, Frame, Routing};
use micold_core::protocol::envelope::Kind;
use micold_core::protocol::messages::ClientMsg;
use micold_core::protocol::routing::{ConnectionId, RoutedEnvelope, RoutingError, Sequence};
use tokio_util::codec::{Decoder, Encoder};

use crate::protocol_roundtrip::{sample_client_msgs, sample_grid_frame};

fn routed(msg: ClientMsg) -> Frame<ClientMsg> {
    Frame::Routed(RoutedEnvelope {
        target: "dev-box".parse().unwrap(),
        conn: ConnectionId(u64::MAX),
        seq: Sequence(7),
        msg,
    })
}

fn encode_routed(frame: Frame<ClientMsg>) -> BytesMut {
    let mut buf = BytesMut::new();
    ClientCodec::new().encode(frame, &mut buf).unwrap();
    buf
}

#[test]
fn an_envelope_around_each_client_message_round_trips() {
    for msg in sample_client_msgs() {
        let mut buf = encode_routed(routed(msg.clone()));
        let mut codec = DaemonCodec::new().with_routing(Routing::Routed);
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(routed(msg)));
    }
}

#[test]
fn a_routed_frame_on_a_plain_codec_is_wrong_routing() {
    let mut buf = encode_routed(routed(ClientMsg::Ping { nonce: 1 }));
    let err = DaemonCodec::new().decode(&mut buf).unwrap_err();
    assert!(
        matches!(
            err,
            CodecError::WrongRouting {
                expected: Routing::Plain,
                got: Kind::Routed
            }
        ),
        "{err}"
    );
}

#[test]
fn a_control_frame_on_a_routed_codec_is_wrong_routing() {
    let mut buf = BytesMut::new();
    ClientCodec::new()
        .encode(Frame::Control(ClientMsg::Ping { nonce: 1 }), &mut buf)
        .unwrap();
    let err = DaemonCodec::new()
        .with_routing(Routing::Routed)
        .decode(&mut buf)
        .unwrap_err();
    assert!(
        matches!(
            err,
            CodecError::WrongRouting {
                expected: Routing::Routed,
                got: Kind::Control
            }
        ),
        "{err}"
    );
}

#[test]
fn the_handshake_is_plain_then_the_codec_switches_to_routed() {
    let mut daemon = DaemonCodec::new();
    let mut buf = BytesMut::new();
    ClientCodec::new()
        .encode(Frame::Control(ClientMsg::Ping { nonce: 1 }), &mut buf)
        .unwrap();
    assert_eq!(
        daemon.decode(&mut buf).unwrap(),
        Some(Frame::Control(ClientMsg::Ping { nonce: 1 }))
    );
    daemon.set_routing(Routing::Routed);
    let mut buf = BytesMut::new();
    ClientCodec::new()
        .encode(Frame::Control(ClientMsg::Ping { nonce: 1 }), &mut buf)
        .unwrap();
    assert!(matches!(
        daemon.decode(&mut buf),
        Err(CodecError::WrongRouting { .. })
    ));
    let mut buf = encode_routed(routed(ClientMsg::Ping { nonce: 1 }));
    assert_eq!(
        daemon.decode(&mut buf).unwrap(),
        Some(routed(ClientMsg::Ping { nonce: 1 }))
    );
}

#[test]
fn a_client_codec_receiving_a_routed_frame_is_wrong_routing() {
    let mut buf = encode_routed(routed(ClientMsg::Ping { nonce: 1 }));
    // A `ClientCodec` reads `DaemonMsg`; the plain default refuses the routed kind first.
    let err = ClientCodec::new().decode(&mut buf).unwrap_err();
    assert!(matches!(err, CodecError::WrongRouting { .. }), "{err}");
}

#[test]
fn a_routed_frame_without_a_target_is_a_routing_error() {
    let body = br#"{"conn":1,"seq":1,"msg":"Ping"}"#;
    let mut payload = vec![0u8, Kind::Routed as u8, 0, 0];
    payload.extend_from_slice(body);
    let mut buf = BytesMut::new();
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(&payload);
    let err = DaemonCodec::new()
        .with_routing(Routing::Routed)
        .decode(&mut buf)
        .unwrap_err();
    assert!(
        matches!(err, CodecError::Routing(RoutingError::MissingTarget)),
        "{err}"
    );
}

#[test]
fn grid_frames_pass_in_both_routings() {
    for routing in [Routing::Plain, Routing::Routed] {
        let mut buf = BytesMut::new();
        ClientCodec::new()
            .encode(Frame::Grid(sample_grid_frame()), &mut buf)
            .unwrap();
        let got = DaemonCodec::new()
            .with_routing(routing)
            .decode(&mut buf)
            .unwrap();
        assert_eq!(got, Some(Frame::Grid(sample_grid_frame())));
    }
}
