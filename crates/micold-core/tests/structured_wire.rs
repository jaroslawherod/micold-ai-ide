//! Feature 938, T012 — the structured-session wire delta round-trips (contracts/wire.md, FR-001,
//! FR-016, FR-020). Every addition is serde-defaulted, so a frame from an older peer decodes as a
//! terminal session.

use micold_core::protocol::messages::{ClientMsg, DaemonMsg, SessionSummary};
use micold_core::session::{AiCli, SessionId, SessionKind};
use micold_core::structured::{ContentBlock, Delta, SequencedEvent, StructuredEvent, Usage};
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
            raw: "{\"type\":\"system\"}".into(),
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
