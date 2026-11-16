//! #1726: every `client-example` fence in `specs/orchestrator/contracts/client-routing.md` decodes
//! to the type it documents, so the contract and `routing.rs`/`messages.rs` cannot drift apart.

use std::fs;
use std::path::Path;

use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind, ListRows};
use micold_core::protocol::routing::{
    decode_routed_payload, encode_json_payload, encode_opaque_payload, ConnectionId, JsonRouted,
    ListKind, MissingReason, RoutedBody, RoutedHeader, Sequence, StreamEndReason,
};

/// The fenced bodies labelled `client-example <name>`, in file order.
fn examples() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text =
        fs::read_to_string(root.join("specs/orchestrator/contracts/client-routing.md")).unwrap();
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if let Some(name) = line.trim().strip_prefix("```json client-example ") {
            let body: Vec<_> = lines
                .by_ref()
                .take_while(|l| !l.trim_start().starts_with("```"))
                .collect();
            out.push((name.trim().to_owned(), body.join("\n")));
        }
    }
    out
}

fn example(name: &str) -> String {
    examples()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no client-example {name}"))
        .1
}

#[test]
fn the_contract_has_an_example_per_frame() {
    let names: Vec<_> = examples().into_iter().map(|(n, _)| n).collect();
    for want in [
        "request",
        "opaque-header",
        "refusal",
        "stream-ended",
        "outcome-unknown",
        "orch-list",
        "orch-listed",
    ] {
        assert!(names.iter().any(|n| n == want), "missing {want}: {names:?}");
    }
}

#[test]
fn the_client_frames_decode() {
    let list: JsonRouted<ClientMsg> = serde_json::from_str(&example("orch-list")).unwrap();
    assert_eq!(list.header.target, None);
    assert_eq!(list.header.req, Some(5));
    assert_eq!(
        list.msg,
        ClientMsg::OrchList {
            req: 5,
            kind: ListKind::Sessions
        }
    );
    let request: JsonRouted<ClientMsg> = serde_json::from_str(&example("request")).unwrap();
    assert_eq!(request.header.conn, ConnectionId(7));
    assert_eq!(request.header.seq, Sequence(2));
    let header: RoutedHeader = serde_json::from_str(&example("opaque-header")).unwrap();
    assert_eq!(header.req, Some(12));
}

#[test]
fn the_daemon_frames_decode() {
    let listed: JsonRouted<DaemonMsg> = serde_json::from_str(&example("orch-listed")).unwrap();
    match listed.msg {
        DaemonMsg::OrchListed {
            req,
            rows,
            missing,
            no_daemons,
        } => {
            assert_eq!(req, 5);
            assert_eq!(rows, ListRows::Sessions(vec![]));
            assert_eq!(missing[0].reason, MissingReason::Offline);
            assert!(!no_daemons);
        }
        other => panic!("{other:?}"),
    }
    let ended: JsonRouted<DaemonMsg> = serde_json::from_str(&example("stream-ended")).unwrap();
    assert_eq!(
        ended.msg,
        DaemonMsg::StreamEnded {
            reason: StreamEndReason::DaemonOffline
        }
    );
    let unknown: JsonRouted<DaemonMsg> = serde_json::from_str(&example("outcome-unknown")).unwrap();
    assert_eq!(unknown.msg, DaemonMsg::OutcomeUnknown { req: 11 });
    let refusal: JsonRouted<DaemonMsg> = serde_json::from_str(&example("refusal")).unwrap();
    assert!(matches!(
        refusal.msg,
        DaemonMsg::OperationError {
            req: 3,
            kind: ErrorKind::NotFound,
            ..
        }
    ));
}

#[test]
fn every_example_frame_passes_the_header_check() {
    for name in [
        "request",
        "orch-list",
        "orch-listed",
        "stream-ended",
        "outcome-unknown",
        "refusal",
    ] {
        let value: serde_json::Value = serde_json::from_str(&example(name)).unwrap();
        let header: RoutedHeader = serde_json::from_value(value.clone()).unwrap();
        let mut payload = vec![0, 2, 0, 0];
        payload.extend(serde_json::to_vec(&value).unwrap());
        let decoded = decode_routed_payload(&payload).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(decoded.header, header, "{name}");
        assert!(matches!(decoded.body, RoutedBody::Json(_)), "{name}");
    }
}

#[test]
fn json_encoding_matches_the_example() {
    let header: RoutedHeader = serde_json::from_str(&example("outcome-unknown")).unwrap();
    let payload = encode_json_payload(&header, &DaemonMsg::OutcomeUnknown { req: 11 }).unwrap();
    let from_file: serde_json::Value = serde_json::from_str(&example("outcome-unknown")).unwrap();
    let encoded: serde_json::Value = serde_json::from_slice(&payload[4..]).unwrap();
    assert_eq!(encoded, from_file);
    let opaque = encode_opaque_payload(&header, b"\0\0\0\0{}");
    assert!(matches!(
        decode_routed_payload(&opaque).unwrap().body,
        RoutedBody::Opaque(b"\0\0\0\0{}")
    ));
}
