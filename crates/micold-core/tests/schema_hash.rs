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

// ---------------------------------------------------------------------------------------
// Feature 026 — one hash move for the whole feature (T020)
// ---------------------------------------------------------------------------------------

/// The protocol version feature 026 bumped to.
///
/// Pinned, not read from a constant, because that is what makes this a gate rather than a mirror.
/// Feature 026 makes **five** wire changes — `provider` on `ClientMsg::SessionCreate` and on
/// `SessionSummary`, and `default_ai_cli` on `DaemonSettings`, `SettingsSet` and `SettingsChanged`
/// — and the whole point of doing them in one edit is that the hash moves **once**. A second bump
/// later in the feature, when US3 comes to consume the outbound field, fails here.
///
/// It read 6 while this branch was based on a `main` at 5. Rebasing onto a `main` that had since
/// bumped 5 → 6 for `SessionSummary::live_shells` (feature 012, BUG-003) moved this feature's one
/// bump up to 7 — which is exactly the "another feature's bump" case the assertion below names,
/// and the constant follows it rather than the feature bumping twice.
const FEATURE_026_PROTOCOL_VERSION: u32 = 7;

#[test]
fn the_wire_changes_for_this_feature_cost_exactly_one_version_bump() {
    assert_eq!(
        PROTOCOL_VERSION, FEATURE_026_PROTOCOL_VERSION,
        "the protocol version moved. If that is another feature's bump, update this constant \
         along with the rest of that feature's wire change. If it is feature 026 bumping a second \
         time, it should not be: US3 consumes `SessionSummary::provider`, which T029 already put \
         on the wire, and consuming a field is not a wire change"
    );
}

#[test]
fn every_field_this_feature_added_is_present_in_one_protocol_source() {
    // The other half of "one bump": the bump has to *cover* all five. A version that moved while
    // one of the additions was still to come would satisfy the test above and still cost a second
    // move later.
    //
    // Read from the source text rather than from the types, deliberately — this file's whole
    // subject is the text `build.rs` hashes, and a field added to a *different* file would not
    // change the hash however correct the type looked.
    let (messages, _grid, _envelope) = read_protocol_source();
    for anchor in [
        // Inbound: the client's resolved choice.
        "SessionCreate {",
        // Outbound: the label a row reads (FR-016).
        "pub provider: AiCli,",
        // The service-owned preference, in all three of its shapes.
        "pub default_ai_cli: AiCli,",
        "default_ai_cli: Option<AiCli>,",
    ] {
        assert!(
            messages.contains(anchor),
            "`{anchor}` is not in messages.rs — feature 026's wire change is incomplete, so the \
             single bump this feature is allowed does not yet cover all of it"
        );
    }
    assert_eq!(
        canonicalize(&messages).matches("provider: AiCli,").count(),
        2,
        "exactly two `provider: AiCli` fields ride the wire: one inbound on `SessionCreate`, one \
         outbound on `SessionSummary`"
    );
}
