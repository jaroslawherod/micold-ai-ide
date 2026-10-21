//! Feature 938, T012 — the structured-session wire delta round-trips (contracts/wire.md, FR-001,
//! FR-016, FR-020). Every addition is serde-defaulted, so a frame from an older peer decodes as a
//! terminal session.

use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusedRequest, SessionSummary};
use micold_core::session::{AiCli, SessionId, SessionKind};
use micold_core::structured::{
    ContentBlock, ControlOutcome, Delta, SequencedEvent, StructuredEvent, Usage,
};
use uuid::Uuid;

fn sid() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x938))
}

fn json_roundtrip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let back: T = serde_json::from_slice(&serde_json::to_vec(value).expect("json encode"))
        .expect("json decode");
    assert_eq!(&back, value, "json round-trip mismatch");
    let back: T = postcard::from_bytes(&postcard::to_stdvec(value).expect("postcard encode"))
        .expect("postcard decode");
    assert_eq!(&back, value, "postcard round-trip mismatch");
    back
}

/// A JSON object with its `key` removed, as an older peer writes it.
fn without(mut value: serde_json::Value, path: &[&str], key: &str) -> serde_json::Value {
    let mut at = &mut value;
    for step in path {
        at = at.get_mut(*step).expect("path exists");
    }
    at.as_object_mut()
        .expect("object")
        .remove(key)
        .expect("key present");
    value
}

#[test]
fn a_session_create_without_a_kind_decodes_as_a_terminal_session() {
    let msg = ClientMsg::SessionCreate {
        req: 7,
        project: "/p".into(),
        worktree_dir: "wt".into(),
        provider: AiCli::ClaudeCode,
        allow_additional_session: false,
        kind: SessionKind::Structured,
        permission_mode: Default::default(),
    };
    json_roundtrip(&msg);
    let old = without(
        serde_json::to_value(&msg).unwrap(),
        &["SessionCreate"],
        "kind",
    );
    let back: ClientMsg = serde_json::from_value(old).expect("an older frame decodes");
    let ClientMsg::SessionCreate { kind, .. } = back else {
        panic!("not a SessionCreate: {back:?}");
    };
    assert_eq!(kind, SessionKind::Terminal);
}

fn summary(kind: SessionKind) -> SessionSummary {
    let snapshot: serde_json::Value = serde_json::json!({
        "id": sid(),
        "worktree_dir": "wt",
        "title": "Pending",
        "lifecycle": "Running",
        "activity": "Unknown",
        "provider": "ClaudeCode",
        "input_serial": 0,
        "live_shells": [],
        "attention_seq": 0,
        "unread": false,
        "kind": kind,
    });
    serde_json::from_value(snapshot).expect("a summary")
}

#[test]
fn a_session_summary_carries_its_kind_and_defaults_to_terminal() {
    for kind in [SessionKind::Terminal, SessionKind::Structured] {
        let summary = summary(kind);
        assert_eq!(json_roundtrip(&summary).kind, kind);
    }
    let old = without(
        serde_json::to_value(summary(SessionKind::Structured)).unwrap(),
        &[],
        "kind",
    );
    let back: SessionSummary = serde_json::from_value(old).expect("an older frame decodes");
    assert_eq!(back.kind, SessionKind::Terminal);
}

fn every_variant() -> Vec<StructuredEvent> {
    vec![
        StructuredEvent::Init {
            session_id: "s".into(),
            model: Some("m".into()),
            cwd: Some("/w".into()),
            tools: vec!["Bash".into()],
            permission_mode: Some("plan".into()),
            raw: "{\"type\":\"system\"}".into(),
        },
        StructuredEvent::Status {
            permission_mode: Some("acceptEdits".into()),
            raw: "{\"type\":\"system\",\"subtype\":\"status\"}".into(),
        },
        StructuredEvent::ControlResponse {
            request_id: "micold-1".into(),
            outcome: ControlOutcome::Success("{\"mode\":\"acceptEdits\"}".into()),
            raw: "{\"type\":\"control_response\"}".into(),
        },
        StructuredEvent::ControlResponse {
            request_id: "micold-2".into(),
            outcome: ControlOutcome::Error {
                message: "no".into(),
                code: Some("bypass_not_launched".into()),
            },
            raw: "{\"type\":\"control_response\"}".into(),
        },
        StructuredEvent::Assistant {
            message_id: Some("msg".into()),
            blocks: vec![
                ContentBlock::Text { text: "hi".into() },
                ContentBlock::Thinking {
                    thinking: "hm".into(),
                },
                ContentBlock::ToolUse {
                    id: "t1".into(),
                    name: "Bash".into(),
                    input: "{}".into(),
                },
                ContentBlock::Other { raw: "{}".into() },
            ],
            parent_tool_use_id: None,
            raw: "{\"type\":\"assistant\"}".into(),
        },
        StructuredEvent::User {
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "t1".into(),
                content: "ok".into(),
                is_error: false,
            }],
            parent_tool_use_id: Some("t0".into()),
            raw: "{\"type\":\"user\"}".into(),
        },
        StructuredEvent::Result {
            subtype: "success".into(),
            is_error: false,
            duration_ms: Some(12),
            num_turns: Some(1),
            total_cost_usd: Some(0.5),
            usage: Some(Usage {
                input_tokens: Some(1),
                output_tokens: Some(2),
                cache_creation_input_tokens: None,
                cache_read_input_tokens: Some(3),
            }),
            result: Some("done".into()),
            session_id: Some("s".into()),
            raw: "{\"type\":\"result\"}".into(),
        },
        StructuredEvent::StreamDelta {
            event_type: "content_block_delta".into(),
            index: Some(0),
            delta: Some(Delta::Text { text: "h".into() }),
            parent_tool_use_id: None,
            raw: "{\"type\":\"stream_event\"}".into(),
        },
        StructuredEvent::StreamDelta {
            event_type: "content_block_delta".into(),
            index: Some(1),
            delta: Some(Delta::InputJson {
                partial_json: "{\"a\"".into(),
            }),
            parent_tool_use_id: None,
            raw: "{}".into(),
        },
        StructuredEvent::Unrecognised {
            raw: "{\"type\":\"new\"}".into(),
        },
        StructuredEvent::Unreadable {
            text: "not json".into(),
        },
        StructuredEvent::TurnEndedWithoutResult,
        StructuredEvent::OversizedOnWire {
            original_bytes: 9_000_000,
        },
    ]
}

#[test]
fn structured_events_round_trip_with_every_event_variant() {
    for (seq, event) in every_variant().into_iter().enumerate() {
        for live in [false, true] {
            let msg = DaemonMsg::StructuredEvents {
                session: sid(),
                omitted: 3,
                events: vec![SequencedEvent {
                    seq: seq as u64,
                    event: event.clone(),
                }],
                live,
            };
            json_roundtrip(&msg);
        }
    }
}

#[test]
fn a_session_user_message_round_trips_with_its_text_unchanged() {
    for text in ["hello", "two\nlines \"quoted\"", "zażółć 🦀"] {
        let msg = ClientMsg::SessionUserMessage {
            req: 41,
            session: sid(),
            text: text.into(),
            images: vec![],
        };
        let ClientMsg::SessionUserMessage { text: back, .. } = json_roundtrip(&msg) else {
            panic!("not a SessionUserMessage");
        };
        assert_eq!(back, text);
    }
}

#[test]
fn a_session_request_refused_round_trips_for_every_refused_request() {
    for request in [
        RefusedRequest::Input,
        RefusedRequest::Resize,
        RefusedRequest::Scrollback,
        RefusedRequest::Interrupt,
    ] {
        let msg = DaemonMsg::SessionRequestRefused {
            session: sid(),
            request,
            reason: "a structured session has no terminal".into(),
        };
        json_roundtrip(&msg);
    }
}

// -- Feature 941 (T019): prompt input, interrupt and the queue ------------------------------------

mod prompt_input {
    use super::*;
    use bytes::BytesMut;
    use micold_core::protocol::codec::{ClientCodec, Frame, WireFormat};
    use micold_core::protocol::envelope::MAX_FRAME_LENGTH;
    use micold_core::protocol::messages::{OperationResult, TurnPhase};
    use micold_core::structured::attachment::{
        admit, AttachmentMeta, ImageAttachment, ImageMedia, MAX_IMAGE_BYTES,
        MAX_MESSAGE_IMAGE_BYTES,
    };
    use micold_core::structured::{InterruptAsk, QueueEntryId, QueuedSummary, TurnId};
    use tokio_util::codec::Encoder;

    fn picture(len: usize) -> ImageAttachment {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.resize(len, 0xA5);
        admit(&[], "shot.png", bytes).expect("a picture")
    }

    #[test]
    fn a_user_message_with_pictures_round_trips() {
        json_roundtrip(&ClientMsg::SessionUserMessage {
            req: 1,
            session: sid(),
            text: "look".into(),
            images: vec![picture(64), picture(9)],
        });
    }

    #[test]
    fn a_user_message_from_before_941_has_no_pictures() {
        let msg = ClientMsg::SessionUserMessage {
            req: 1,
            session: sid(),
            text: "hi".into(),
            images: vec![],
        };
        let old = without(
            serde_json::to_value(&msg).expect("json"),
            &["SessionUserMessage"],
            "images",
        );
        assert_eq!(
            serde_json::from_value::<ClientMsg>(old).expect("decodes"),
            msg
        );
    }

    #[test]
    fn structured_input_round_trips_in_every_phase() {
        let queue = vec![QueuedSummary {
            id: QueueEntryId(3),
            text: "next".into(),
            attachments: vec![AttachmentMeta {
                name: "shot.png".into(),
                media: ImageMedia::Png,
                size: 64,
            }],
        }];
        for phase in [
            TurnPhase::NotRunning,
            TurnPhase::Idle,
            TurnPhase::Busy {
                turn: TurnId(7),
                interrupt: InterruptAsk::None,
            },
            TurnPhase::Busy {
                turn: TurnId(7),
                interrupt: InterruptAsk::Asked,
            },
            TurnPhase::Busy {
                turn: TurnId(7),
                interrupt: InterruptAsk::Overdue,
            },
        ] {
            json_roundtrip(&DaemonMsg::StructuredInput {
                session: sid(),
                phase,
                hold: true,
                queue: queue.clone(),
            });
        }
    }

    #[test]
    fn the_interrupt_and_the_queue_edits_round_trip() {
        let entry = QueueEntryId(4);
        for msg in [
            ClientMsg::TurnInterrupt {
                session: sid(),
                turn: TurnId(2),
            },
            ClientMsg::QueueEdit {
                req: 1,
                session: sid(),
                entry,
                text: "edited".into(),
                keep: vec![0, 2],
            },
            ClientMsg::QueueRemove {
                req: 2,
                session: sid(),
                entry,
            },
            ClientMsg::QueueTake {
                req: 3,
                session: sid(),
                entry,
            },
            ClientMsg::QueueRelease {
                req: 4,
                session: sid(),
            },
        ] {
            json_roundtrip(&msg);
        }
        json_roundtrip(&DaemonMsg::OperationOk {
            req: 3,
            result: OperationResult::Taken {
                text: "back".into(),
                images: vec![picture(32)],
            },
        });
    }

    #[test]
    fn a_message_with_the_most_pictures_and_a_mebibyte_of_text_fits_one_frame() {
        let mut images = Vec::new();
        let mut held = 0;
        while held + MAX_IMAGE_BYTES <= MAX_MESSAGE_IMAGE_BYTES {
            images.push(picture(MAX_IMAGE_BYTES));
            held += MAX_IMAGE_BYTES;
        }
        images.push(picture(MAX_MESSAGE_IMAGE_BYTES - held));
        let total: usize = images.iter().map(|i| i.bytes().len()).sum();
        assert_eq!(total, MAX_MESSAGE_IMAGE_BYTES);
        let msg = ClientMsg::SessionUserMessage {
            req: 1,
            session: sid(),
            text: "ż".repeat(512 * 1024),
            images,
        };
        let mut frame = BytesMut::new();
        ClientCodec::with_format(WireFormat::Postcard)
            .encode(Frame::Control(msg), &mut frame)
            .expect("encodes within the frame limit");
        assert!(frame.len() < MAX_FRAME_LENGTH, "{} bytes", frame.len());
    }
}
