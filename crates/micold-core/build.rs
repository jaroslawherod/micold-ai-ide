//! Generates `SCHEMA_HASH` for the wire protocol (contracts/protocol.md §4, Decision 4).
//!
//! Hashes the canonical text of `src/protocol/{messages,grid,envelope}.rs` and emits the digest as
//! `pub const SCHEMA_HASH: [u8; 32]` into `$OUT_DIR/schema_hash.rs`, which `protocol/version.rs`
//! `include!`s. The hashing code itself is `include!`d from `src/protocol/hashing.rs` so the
//! generator and the crate/tests use one identical implementation — no chance of drift.

use std::env;
use std::fs;
use std::path::Path;

// Pulls in `sha256`, `canonicalize`, `schema_hash` — the exact functions the crate exposes.
include!("src/protocol/hashing.rs");

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let protocol = Path::new(&manifest_dir).join("src").join("protocol");

    let messages_path = protocol.join("messages.rs");
    let grid_path = protocol.join("grid.rs");
    let envelope_path = protocol.join("envelope.rs");

    // Rebuild the hash whenever any protocol source (or the hashing code) changes.
    for path in [&messages_path, &grid_path, &envelope_path] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        protocol.join("hashing.rs").display()
    );

    let messages = fs::read_to_string(&messages_path).expect("read messages.rs");
    let grid = fs::read_to_string(&grid_path).expect("read grid.rs");
    let envelope = fs::read_to_string(&envelope_path).expect("read envelope.rs");

    let hash = schema_hash(&messages, &grid, &envelope);

    let mut literal = String::from("pub const SCHEMA_HASH: [u8; 32] = [");
    for (i, byte) in hash.iter().enumerate() {
        if i > 0 {
            literal.push_str(", ");
        }
        literal.push_str(&format!("0x{byte:02x}"));
    }
    literal.push_str("];\n");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    fs::write(Path::new(&out_dir).join("schema_hash.rs"), literal).expect("write schema_hash.rs");
}
