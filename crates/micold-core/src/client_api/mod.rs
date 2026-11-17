//! The browser-facing client API of the orchestrator (#1732): JSON text messages over `/api/ws`
//! (`specs/1732-client-ws-api/contracts/ws-protocol.md`). Dedicated wire types, not the daemon
//! protocol's `ClientMsg` (research R2), with a JSON Schema derived from them (R3); the one
//! read-or-change table (R4); the snapshot diff and the change log that make resume exact (R5, R8);
//! and the `Origin` allow-list (FR-004). Pure: the orchestrator's `ws`, `feed` and `directory`
//! modules wire it to sockets and daemons.

pub mod classify;
pub mod content;
pub mod diff;
pub mod log;
pub mod messages;
pub mod origin;
