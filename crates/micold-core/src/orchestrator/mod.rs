//! The orchestrator's pure rules (#1724): the daemon registry, the link announcement and what a
//! listing carries. No socket, no clock and no runtime here: every time is passed in, so the rules
//! are tested exactly. The daemon crate (`micold-daemon::orchestrator`) wires them to a socket.

pub mod announce;
pub mod record;
pub mod registry;

pub use announce::{
    decode_first_frame, FirstFrame, LinkAnnouncement, LinkQuery, LinkVerdict, RefuseReason,
    MAX_PREAMBLE_FRAME,
};
pub use record::{DaemonRecord, Health, TenantId};
pub use registry::{Claim, ConfirmRefused, Confirmed, LinkAction, LinkToken, Registry, Snapshot};

/// The tenant a daemon that announces none belongs to.
pub const DEFAULT_TENANT: &str = "default";
