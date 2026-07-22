//! Handshake evaluation (contracts/protocol.md §4).
//!
//! Strict exact-match, no negotiation, no compatibility range (FR-021). Both `protocol_version`
//! **and** `schema_hash` must match; on either mismatch the daemon refuses and names both sides'
//! version and hash plus its own build (FR-022), so the client can render an actionable diagnostic
//! and offer the restart action.

use crate::protocol::messages::RefusalReason;
use crate::protocol::version::{PROTOCOL_VERSION, SCHEMA_HASH};

/// Evaluate a client's handshake against this daemon build.
///
/// Returns `Ok(())` only when the client's version **and** hash both match this build's compiled
/// constants; otherwise a [`RefusalReason::VersionMismatch`] naming both sides (FR-021/022).
pub fn evaluate(
    client_version: u32,
    client_hash: [u8; 32],
    daemon_build: impl Into<String>,
) -> Result<(), RefusalReason> {
    if client_version == PROTOCOL_VERSION && client_hash == SCHEMA_HASH {
        return Ok(());
    }
    Err(RefusalReason::VersionMismatch {
        client: client_version,
        daemon: PROTOCOL_VERSION,
        client_hash,
        daemon_hash: SCHEMA_HASH,
        daemon_build: daemon_build.into(),
    })
}
