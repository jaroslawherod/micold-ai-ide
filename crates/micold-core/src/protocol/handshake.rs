//! Handshake evaluation (contracts/protocol.md §4).
//!
//! Strict exact-match, no negotiation, no compatibility range (FR-021). Both `protocol_version`
//! **and** `schema_hash` must match; on either mismatch the daemon refuses and names both sides'
//! version and hash plus its own build (FR-022), so the client can render an actionable diagnostic
//! and offer the restart action.
//!
//! A matching contract does not mean matching *builds*: most releases (e.g. a daemon-only bugfix)
//! don't touch the wire schema, so `protocol_version`/`schema_hash` alone never catch a `.deb`
//! upgrade over an already-running daemon. [`PACKAGE_VERSION`] changes on every release and closes
//! that gap as a second, independent check (FR-022a, BUG-002).

use crate::protocol::messages::RefusalReason;
use crate::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};

/// Evaluate a client's handshake against this daemon build.
///
/// Returns `Ok(())` only when the client's version, hash, **and** package version all match this
/// build's compiled constants. A contract mismatch (version or hash) refuses with
/// [`RefusalReason::VersionMismatch`] naming both sides (FR-021/022); a same-contract package-version
/// difference refuses with [`RefusalReason::BuildMismatch`] instead (FR-022a, BUG-002).
pub fn evaluate(
    client_version: u32,
    client_hash: [u8; 32],
    client_package_version: impl AsRef<str>,
    client_build: impl Into<String>,
    daemon_build: impl Into<String>,
) -> Result<(), RefusalReason> {
    if client_version != PROTOCOL_VERSION || client_hash != SCHEMA_HASH {
        return Err(RefusalReason::VersionMismatch {
            client: client_version,
            daemon: PROTOCOL_VERSION,
            client_hash,
            daemon_hash: SCHEMA_HASH,
            daemon_build: daemon_build.into(),
        });
    }
    if client_package_version.as_ref() != PACKAGE_VERSION {
        return Err(RefusalReason::BuildMismatch {
            client_build: client_build.into(),
            daemon_build: daemon_build.into(),
        });
    }
    Ok(())
}
