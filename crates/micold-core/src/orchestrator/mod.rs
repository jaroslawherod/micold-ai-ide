//! The orchestrator's pure rules (#1724): the daemon registry, the link announcement and what a
//! listing carries. No socket, no clock and no runtime here: every time is passed in, so the rules
//! are tested exactly. The daemon crate (`micold-daemon::orchestrator`) wires them to a socket.

pub mod announce;
pub mod audit;
pub mod auth;
pub mod dial;
pub mod liveness;
pub mod record;
pub mod registry;
pub mod store;

pub use announce::{
    decode_first_frame, encode_announcement, refusal_message, FirstFrame, LinkAnnouncement,
    LinkQuery, LinkVerdict, RefuseReason, MAX_PREAMBLE_FRAME,
};
pub use audit::{AuditEntry, AuditStore, MemoryAuditStore, NewAuditEntry, MAX_DETAIL_BYTES};
pub use auth::{
    AuthStore, Bootstrap, CertSerial, Identity, Invite, MemoryAuthStore, NewEnrollment, NewInvite,
    NewUser, Provider, RefreshToken, Revocation, Role, TokenHash, User, UserId,
};
pub use dial::{jittered, LinkStatus, RetryPolicy};
pub use liveness::Heartbeat;
pub use record::{DaemonRecord, Health, TenantId, UNOWNED};
pub use registry::{
    Claim, ClaimFailure, ConfirmRefused, Confirmed, LinkAction, LinkToken, Registry, Snapshot,
};
pub use store::{
    JsonRegistryStore, MemoryRegistryStore, RegistrySaver, RegistryStore, SchemaError, StoreError,
};

/// The tenant a daemon that announces none belongs to.
pub const DEFAULT_TENANT: &str = "default";
