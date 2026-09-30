//! The tool server's policy table (feature 034, data-model TM4/TM7; FR-014, FR-015, FR-015a,
//! FR-016).
//!
//! [`decide`] is evaluated after the call's input is validated and its target resolved, and before
//! anything changes (contracts/mcp-tools.md *Order of checks*). It is pure, so every rule is pinned
//! by a table test rather than by a daemon round trip. Later milestones add rows; a row this table
//! does not have means "proceed".

use std::path::PathBuf;

use super::errors::{ErrorCategory, OpError};
use super::tools::Operation;
use crate::session::{AiCli, SessionId, SessionLocation};

/// The session a call comes from, resolved per request from its credential and the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    pub session: SessionId,
    /// The project that holds the caller; every target is resolved inside it (FR-010).
    pub project: PathBuf,
    /// Where the caller runs: `Default` (the project root) is what FR-015a restricts.
    pub location: SessionLocation,
    pub provider: AiCli,
}

impl Caller {
    /// Whether the caller runs in the project root rather than a worktree.
    pub fn is_default(&self) -> bool {
        matches!(self.location, SessionLocation::Default)
    }
}

/// What the service knows about a call's target when policy is decided. Empty until a row needs a
/// fact (the destructive and cross-session rows of later milestones).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetFacts {}

/// The user's option for reading and typing into sibling sessions (FR-016). `Auto` is the default
/// (decision D6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CrossSessionAccess {
    #[default]
    Auto,
    ConfirmEachSend,
    Off,
}

/// What policy says about one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Proceed,
    Refuse(OpError),
}

/// The refusal a Default session gets for a worktree mutation (FR-015a).
const PRINCIPLE_III: &str = "a session running in the project root (Default) may not create, \
    rename or delete worktrees (Constitution Principle III); ask from a session that runs in a \
    worktree";

/// Decide `operation` for `caller`.
pub fn decide(
    caller: &Caller,
    operation: &Operation,
    _facts: &TargetFacts,
    _access: CrossSessionAccess,
) -> PolicyDecision {
    match operation {
        Operation::CreateWorktree { .. } if caller.is_default() => PolicyDecision::Refuse(
            OpError::new(ErrorCategory::RefusedByPolicy, PRINCIPLE_III),
        ),
        _ => PolicyDecision::Proceed,
    }
}
