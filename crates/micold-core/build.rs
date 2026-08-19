//! Generates `SCHEMA_HASH` for the wire protocol (contracts/protocol.md §4, Decision 4) and
//! `BUILD_FINGERPRINT` for feature 027's stale-image check.
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

    emit_build_fingerprint(&Path::new(&manifest_dir).join("src"), Path::new(&out_dir));
}

/// Emit `BUILD_FINGERPRINT`: a hash over **this crate's whole source tree**.
///
/// Feature 027, research R8. The handshake already compares `PROTOCOL_VERSION`, `SCHEMA_HASH` and
/// `PACKAGE_VERSION`, and within one released version a daemon rebuilt yesterday and a client built
/// today present identical values for all three — so a stale `:dev` image connects and then
/// misbehaves in ways that look like bugs in the new code. FR-024c makes that rebuild loop a
/// supported path, which is exactly where the failure is most likely.
///
/// # Why this crate's `src/`, and not the workspace's
///
/// `micold-core` is what the client and the daemon *share*: the protocol, the session model, the
/// settings, the sandbox rules. A disagreement they could both detect is a disagreement about
/// something in here. Hashing the whole workspace would also work, but it would make `micold-core`
/// rebuild whenever `micold-client` changed — which it does not today, and which this repository
/// can least afford (see CLAUDE.md on the shared target directory). Hashing this crate's own
/// sources adds **no** rebuild the compiler was not already doing.
///
/// The limitation that buys: a change confined to `micold-daemon`'s own sources does not move the
/// fingerprint. Such a change also cannot create a disagreement the client is able to notice, so
/// the check loses nothing it could have caught.
fn emit_build_fingerprint(src: &Path, out_dir: &Path) {
    println!("cargo:rerun-if-changed={}", src.display());

    let mut files = Vec::new();
    collect_rs(src, &mut files);
    // Sorted, so the fingerprint is a property of the sources and not of directory iteration order.
    files.sort();

    let mut combined = String::new();
    for path in &files {
        // The path is part of the hash: moving a file without editing it still changes the tree.
        combined.push_str(&path.to_string_lossy());
        combined.push('\n');
        combined.push_str(&fs::read_to_string(path).unwrap_or_default());
        combined.push('\n');
    }

    let digest = sha256(canonicalize(&combined).as_bytes());
    // The first 8 bytes are plenty: this distinguishes two builds, it does not resist an adversary.
    let short: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    fs::write(
        out_dir.join("build_fingerprint.rs"),
        format!(
            "/// A value that changes on every build of `micold-core`'s sources, not every \
             release.\n\
             ///\n\
             /// Feature 027, research R8: the handshake's three existing constants cannot \
             detect a\n\
             /// stale development image, because a daemon rebuilt yesterday and a client built \
             today\n\
             /// present identical values for all of them. Compared asymmetrically — only a \
             locally\n\
             /// built image refuses on a mismatch.\n\
             pub const BUILD_FINGERPRINT: &str = \"{short}\";\n"
        ),
    )
    .expect("write build_fingerprint.rs");
}

fn collect_rs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
