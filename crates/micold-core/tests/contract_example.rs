//! #1723 M4 — the routed-envelope contract's worked example decodes (FR-009, SC-004).
//!
//! `specs/orchestrator/contracts/routed-envelope.md` is where the orchestrator's developer reads
//! the envelope from. Its example frame and its lists of fields and errors are test inputs, so an
//! edit to the contract or to `routing.rs` that disagrees with the other fails here.

use std::fs;
use std::path::Path;

use bytes::BytesMut;
use micold_core::protocol::codec::{ClientCodec, DaemonCodec, Frame, Routing};
use micold_core::protocol::messages::ClientMsg;
use micold_core::protocol::routing::{ConnectionId, RoutedEnvelope, Sequence};
use tokio_util::codec::{Decoder, Encoder};

fn read(relative: &str) -> String {
    // tests/ -> micold-core/ -> crates/ -> repo root
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    fs::read_to_string(root.join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

fn contract() -> String {
    read("specs/orchestrator/contracts/routed-envelope.md")
}

/// The body of the fence whose info string is `routed-example`.
fn example(markdown: &str) -> String {
    let mut lines = markdown.lines();
    lines
        .by_ref()
        .find(|l| l.trim_start().starts_with("```json routed-example"))
        .expect("the contract has a ```json routed-example fence");
    lines
        .take_while(|l| !l.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_worked_example_decodes_to_the_stated_envelope() {
    let body = example(&contract());
    let envelope = RoutedEnvelope::<ClientMsg>::from_json_slice(body.as_bytes())
        .unwrap_or_else(|e| panic!("{e}: {body}"));
    assert_eq!(envelope.target.as_str(), "build-box");
    assert_eq!(envelope.conn, ConnectionId(7));
    assert_eq!(envelope.seq, Sequence(42));
    assert_eq!(envelope.msg, ClientMsg::Ping { nonce: 9 });

    // The same envelope crosses a routed link as a frame.
    let mut buf = BytesMut::new();
    ClientCodec::new()
        .encode(Frame::Routed(envelope.clone()), &mut buf)
        .unwrap();
    let frame = DaemonCodec::new()
        .with_routing(Routing::Routed)
        .decode(&mut buf)
        .unwrap();
    assert!(matches!(frame, Some(Frame::Routed(e)) if e == envelope));
}

#[test]
fn every_envelope_field_and_error_variant_is_listed() {
    let contract = contract();
    for field in ["target", "conn", "seq", "msg"] {
        assert!(
            contract.contains(&format!("| `{field}` |")),
            "field `{field}` is missing from the contract's table"
        );
    }
    // Only the paragraph that lists the errors counts, not a mention elsewhere.
    let errors = contract
        .split("Errors (`RoutingError`)")
        .nth(1)
        .and_then(|rest| rest.split("\n\n").next())
        .expect("the contract has an Errors (`RoutingError`) paragraph");
    let source = read("crates/micold-core/src/protocol/routing.rs");
    let body = source
        .split("pub enum RoutingError {")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("RoutingError is in routing.rs");
    // A variant line is indented four spaces and starts with a capital.
    let variants: Vec<&str> = body
        .lines()
        .filter(|l| l.starts_with("    ") && !l.starts_with("     "))
        .filter_map(|l| l.trim().split(['(', ' ', ',', '{']).next())
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .collect();
    assert_eq!(variants.len(), 5, "found {variants:?}");
    for variant in variants {
        assert!(
            errors.contains(&format!("`{variant}`")),
            "`RoutingError::{variant}` is missing from the contract"
        );
    }
    assert!(errors.contains("`WrongRouting`"));
}
