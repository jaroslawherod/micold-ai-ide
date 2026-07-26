//! T014 — strict exact-match handshake (contracts/protocol.md §4, FR-021/022).
//!
//! A version mismatch OR a schema-hash mismatch both refuse, and the refusal names **both** sides'
//! version and hash plus the daemon build, so the client can render an actionable diagnostic.

use micold_core::protocol::handshake::evaluate;
use micold_core::protocol::messages::RefusalReason;
use micold_core::protocol::version::{PROTOCOL_VERSION, SCHEMA_HASH};

#[test]
fn matching_version_and_hash_is_accepted() {
    assert!(evaluate(PROTOCOL_VERSION, SCHEMA_HASH, "daemon-build").is_ok());
}

#[test]
fn version_mismatch_is_refused_naming_both_sides() {
    let client_version = PROTOCOL_VERSION + 1;
    let err = evaluate(client_version, SCHEMA_HASH, "daemon-build")
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

    let err = evaluate(PROTOCOL_VERSION, client_hash, "daemon-build")
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
    assert!(evaluate(PROTOCOL_VERSION + 2, client_hash, "d").is_err());
}
