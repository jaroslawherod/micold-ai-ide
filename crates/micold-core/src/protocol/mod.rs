//! The client ↔ daemon wire protocol (feature 010).
//!
//! Render-free and shared: both `micold-daemon` and `micold-client` compile against this one
//! definition, which is what makes the [`version::SCHEMA_HASH`] guard sound (contracts/protocol.md,
//! messages.md). Nothing here touches iced, PTY, or alacritty — it lives in `micold-core` precisely
//! so a wire type cannot smuggle a render dependency across the boundary (FR-040).
//!
//! - [`envelope`] — the 4-byte framing header and frame cap.
//! - [`messages`] — the control-plane [`messages::ClientMsg`] / [`messages::DaemonMsg`] surface.
//! - [`grid`] — the postcard-encoded [`grid::GridFrame`] streaming types.
//! - [`version`] — [`version::PROTOCOL_VERSION`] and the generated [`version::SCHEMA_HASH`].
//! - [`handshake`] — strict exact-match handshake evaluation.
//! - [`hashing`] — the dependency-free hash shared by `build.rs` and the guard test.

pub mod envelope;
pub mod grid;
pub mod handshake;
pub mod hashing;
pub mod messages;
pub mod version;
