//! The checked-in client API schema is the one the Rust wire types generate (#1732, FR-009, FR-010,
//! SC-004). Regenerate it, and the TypeScript types made from it, with `scripts/api-types.sh`.

use micold_core::client_api::schema;

const CHECKED_IN: &str = "../../packages/protocol/schema/client-api.schema.json";

#[test]
fn the_checked_in_schema_equals_the_generated_one() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CHECKED_IN);
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run scripts/api-types.sh", path.display()));
    assert!(
        committed == schema::document(),
        "{} is stale; run scripts/api-types.sh",
        path.display()
    );
}

#[test]
fn the_schema_names_both_frame_types_and_the_feed_types() {
    let value = schema::value();
    let definitions = value["definitions"].as_object().expect("definitions");
    for name in [
        "ClientRequest",
        "ServerMessage",
        "Snapshot",
        "Change",
        "Entity",
        "EntityRef",
        "ContentEntry",
    ] {
        assert!(definitions.contains_key(name), "missing definition {name}");
    }
    assert_eq!(value["anyOf"].as_array().map(Vec::len), Some(2));
}
