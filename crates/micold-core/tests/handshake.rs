//! T014 — strict exact-match handshake (contracts/protocol.md §4, FR-021/022).
//!
//! A version mismatch OR a schema-hash mismatch both refuse, and the refusal names **both** sides'
//! version and hash plus the daemon build, so the client can render an actionable diagnostic.
//!
//! T088 — a same-contract package-version difference refuses too, but distinctly (FR-022a, BUG-002):
//! most releases don't touch the wire schema, so `PROTOCOL_VERSION`/`SCHEMA_HASH` alone never catch
//! a `.deb` upgrade over an already-running daemon.

use micold_core::protocol::handshake::evaluate;
use micold_core::protocol::messages::RefusalReason;
use micold_core::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};

#[test]
fn matching_version_hash_and_package_is_accepted() {
    assert!(evaluate(
        PROTOCOL_VERSION,
        SCHEMA_HASH,
        PACKAGE_VERSION,
        "client-build",
        "daemon-build"
    )
    .is_ok());
}

#[test]
fn version_mismatch_is_refused_naming_both_sides() {
    let client_version = PROTOCOL_VERSION + 1;
    let err = evaluate(
        client_version,
        SCHEMA_HASH,
        PACKAGE_VERSION,
        "client-build",
        "daemon-build",
    )
    .expect_err("a version mismatch must refuse");
    match err {
        RefusalReason::VersionMismatch {
            client,
            daemon,
            client_hash,
            daemon_hash,
            daemon_build,
        } => {
            assert_eq!(client, client_version);
            assert_eq!(daemon, PROTOCOL_VERSION);
            assert_eq!(client_hash, SCHEMA_HASH);
            assert_eq!(daemon_hash, SCHEMA_HASH);
            assert_eq!(daemon_build, "daemon-build");
        }
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
}

#[test]
fn schema_hash_mismatch_is_refused_even_when_the_version_matches() {
    let mut client_hash = SCHEMA_HASH;
    client_hash[0] ^= 0xff; // a single flipped byte = a different wire

    let err = evaluate(
        PROTOCOL_VERSION,
        client_hash,
        PACKAGE_VERSION,
        "client-build",
        "daemon-build",
    )
    .expect_err("a schema-hash mismatch must refuse even at the same version");
    match err {
        RefusalReason::VersionMismatch {
            client,
            daemon,
            client_hash: reported_client,
            daemon_hash,
            ..
        } => {
            assert_eq!(client, PROTOCOL_VERSION);
            assert_eq!(daemon, PROTOCOL_VERSION);
            assert_eq!(
                reported_client, client_hash,
                "must echo the client's actual hash"
            );
            assert_eq!(daemon_hash, SCHEMA_HASH);
            assert_ne!(
                reported_client, daemon_hash,
                "the two hashes must be reported as different"
            );
        }
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
}

#[test]
fn both_mismatched_still_refuses() {
    let mut client_hash = SCHEMA_HASH;
    client_hash[31] ^= 0x01;
    assert!(evaluate(PROTOCOL_VERSION + 2, client_hash, PACKAGE_VERSION, "c", "d").is_err());
}

#[test]
fn build_mismatch_is_refused_distinctly_when_contract_still_matches() {
    // Same protocol_version/schema_hash as this build compiles, but a different package version —
    // the shape of a same-contract `.deb` upgrade over an already-running daemon (FR-022a, BUG-002).
    let err = evaluate(
        PROTOCOL_VERSION,
        SCHEMA_HASH,
        "0.0.0-stale",
        "micold-ai-ide/0.0.0-stale",
        "micold-daemon 0.0.0-stale",
    )
    .expect_err("a package-version mismatch must refuse even with a matching contract");
    match err {
        RefusalReason::BuildMismatch {
            client_build,
            daemon_build,
        } => {
            assert_eq!(client_build, "micold-ai-ide/0.0.0-stale");
            assert_eq!(daemon_build, "micold-daemon 0.0.0-stale");
        }
        other => panic!("expected BuildMismatch, got {other:?}"),
    }
}

#[test]
fn matching_package_version_is_accepted_even_with_differing_build_strings() {
    // `client_build`/`daemon_build` are free-form diagnostic strings (different program-name
    // prefixes even on a matching release) — only `client_package_version` decides this check.
    assert!(evaluate(
        PROTOCOL_VERSION,
        SCHEMA_HASH,
        PACKAGE_VERSION,
        "micold-ai-ide/differs-from-daemon-build-string",
        "micold-daemon differs-from-client-build-string",
    )
    .is_ok());
}
