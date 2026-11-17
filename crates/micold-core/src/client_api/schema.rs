//! The client API's JSON Schema (#1732, FR-009, research R3), derived from the wire types in
//! [`super::messages`]. `micold-api-schema` writes it to
//! `packages/protocol/schema/client-api.schema.json`, and `scripts/api-types.sh` turns that file into
//! the web client's TypeScript types; the drift check fails when either checked-in file is stale.

use schemars::generate::SchemaSettings;
use serde_json::{json, Value};

use super::messages::{
    Change, ClientRequest, ContentEntry, Entity, EntityRef, ServerMessage, Snapshot,
};

/// The schema as a JSON value: draft-07, every wire type under `definitions`, and the root a choice
/// of the two frame types, a [`ClientRequest`] (browser to orchestrator) or a [`ServerMessage`].
pub fn value() -> Value {
    let mut generator = SchemaSettings::draft07().into_generator();
    let roots = [
        generator.subschema_for::<ClientRequest>(),
        generator.subschema_for::<ServerMessage>(),
    ];
    // Named so the generated TypeScript exports them even where no frame refers to them directly.
    generator.subschema_for::<Snapshot>();
    generator.subschema_for::<Change>();
    generator.subschema_for::<Entity>();
    generator.subschema_for::<EntityRef>();
    generator.subschema_for::<ContentEntry>();
    let definitions = generator.take_definitions(true);
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "ClientApi",
        "description": "A text frame of the orchestrator's /api/ws client API (specs/1732-client-ws-api/contracts/ws-protocol.md).",
        "anyOf": roots,
        "definitions": definitions,
    })
}

/// The schema file's exact text: pretty-printed with a trailing newline, so that a regeneration of
/// unchanged types is byte-identical.
pub fn document() -> String {
    let mut text = serde_json::to_string_pretty(&value()).expect("a JSON value always serializes");
    text.push('\n');
    text
}
