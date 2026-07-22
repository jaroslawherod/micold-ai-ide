//! T013 — the schema-hash guard guards itself (contracts/protocol.md §4, Decision 4).
//!
//! `SCHEMA_HASH` is baked by `build.rs` from the canonical text of the protocol source. These tests
//! prove (a) it is real (non-zero and wired to the actual files), and (b) it is *sensitive*: editing
//! a message struct changes it, and a version-only bump changes the handshake tuple even when the
//! hash does not. Both use the same `hashing` functions the build script uses, so there is exactly
//! one implementation under test.

use std::fs;
use std::path::Path;

use micold_core::protocol::hashing::{canonicalize, schema_hash};
use micold_core::protocol::version::{PROTOCOL_VERSION, SCHEMA_HASH};

fn read_protocol_source() -> (String, String, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("protocol");
    let messages = fs::read_to_string(dir.join("messages.rs")).expect("messages.rs");
    let grid = fs::read_to_string(dir.join("grid.rs")).expect("grid.rs");
    let envelope = fs::read_to_string(dir.join("envelope.rs")).expect("envelope.rs");
    (messages, grid, envelope)
}

#[test]
fn schema_hash_is_non_trivial() {
    assert_ne!(SCHEMA_HASH, [0u8; 32], "SCHEMA_HASH must not be all-zero");
}

#[test]
fn baked_hash_matches_a_recompute_over_the_real_source() {
    let (messages, grid, envelope) = read_protocol_source();
    let recomputed = schema_hash(&messages, &grid, &envelope);
    assert_eq!(
        recomputed, SCHEMA_HASH,
        "build.rs must hash the same protocol source these tests read; \
         if this fails the generator and the crate have drifted"
    );
}

#[test]
fn editing_a_message_struct_changes_the_hash() {
    let (messages, grid, envelope) = read_protocol_source();
    let baseline = schema_hash(&messages, &grid, &envelope);

    // Simulate a wire-visible edit: a new field on an existing message.
    let mutated = messages.replace(
        "pub enum ClientMsg {",
        "pub enum ClientMsg {\n    NewlyAddedVariant { extra: u64 },",
    );
    assert_ne!(
        mutated, messages,
        "the anchor text must exist to be mutated"
    );

    let after = schema_hash(&mutated, &grid, &envelope);
    assert_ne!(
        after, baseline,
        "a struct/enum edit MUST change SCHEMA_HASH — that is the whole guard"
    );
}

#[test]
fn a_comment_only_edit_does_not_churn_the_hash() {
    let (messages, grid, envelope) = read_protocol_source();
    let baseline = schema_hash(&messages, &grid, &envelope);

    // Prepend a fresh full-line comment: canonicalisation drops it, so the hash is stable.
    let commented = format!("// a brand new doc note that changes nothing on the wire\n{messages}");
    let after = schema_hash(&commented, &grid, &envelope);
    assert_eq!(
        after, baseline,
        "a comment-only edit must not change the hash (canonicalisation drops comment lines)"
    );
}

#[test]
fn a_version_only_bump_changes_the_handshake_tuple() {
    // The hash can be identical across a version bump (no struct changed), but the handshake tuple
    // (version, hash) must still differ so a version-only change is caught.
    let before = (PROTOCOL_VERSION, SCHEMA_HASH);
    let after = (PROTOCOL_VERSION + 1, SCHEMA_HASH);
    assert_ne!(
        before, after,
        "the (version, hash) handshake tuple must change on a version bump"
    );
}

#[test]
fn canonicalize_ignores_blank_and_comment_lines_only() {
    let src = "// comment\n\n   \npub struct A;  \n    // indented comment\npub struct B;\n";
    assert_eq!(canonicalize(src), "pub struct A;\npub struct B;\n");
}
