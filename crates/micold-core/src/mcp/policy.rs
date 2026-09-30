//! The tool server's policy table (feature 034, data-model TM4/TM7; FR-014, FR-015, FR-015a,
//! FR-016).
//!
//! [`decide`] is evaluated after the call's input is validated and its target resolved, and before
//! anything changes (contracts/mcp-tools.md *Order of checks*). It is pure, so every rule is pinned
//! by a table test rather than by a daemon round trip. A row this table does not have means
//! "proceed".

use std::path::PathBuf;

use super::errors::{ErrorCategory, OpError};
use super::tools::{Operation, SessionRef, WorktreeRef};
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

/// What policy says about one call. A refusal is always decided before a confirmation, so the user
/// is never asked about a request that would be refused anyway (FR-014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Proceed,
    Confirm(ConfirmedOp),
    Refuse(OpError),
}

/// A destructive operation that waits for the user's confirmation (FR-014, data-model TM5). Names
/// the operation and the options the user is asked to allow; the target travels beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmedOp {
    DeleteWorktree {
        stop_sessions: bool,
        delete_branch: bool,
    },
    DeleteSession,
    StopSession,
    InterruptSession,
}

/// The refusal a Default session gets for a worktree mutation (FR-015a).
const PRINCIPLE_III: &str = "a session running in the project root (Default) may not create, \
    rename or delete worktrees (Constitution Principle III); ask from a session that runs in a \
    worktree";

/// Decide `operation` for `caller`. The refusals come first: Principle III for a Default caller
/// (FR-015a), then the caller's own session or hosting worktree (FR-015); only then does a
/// destructive operation wait for the user (FR-014).
pub fn decide(
    caller: &Caller,
    operation: &Operation,
    _facts: &TargetFacts,
    _access: CrossSessionAccess,
) -> PolicyDecision {
    let refused = |message: &str| {
        PolicyDecision::Refuse(OpError::new(ErrorCategory::RefusedByPolicy, message))
    };
    let is_me = |session: &SessionRef| session.0 == caller.session.0;
    match operation {
        Operation::CreateWorktree { .. }
        | Operation::RenameWorktree { .. }
        | Operation::DeleteWorktree { .. }
            if caller.is_default() =>
        {
            refused(PRINCIPLE_III)
        }
        Operation::DeleteWorktree {
            worktree: WorktreeRef::Named(name),
            ..
        } if matches!(&caller.location, SessionLocation::Worktree(mine) if mine == name) => {
            refused("the calling session runs in this worktree, so it may not delete it")
        }
        Operation::StopSession { session } if is_me(session) => {
            refused("a session may not stop itself; the user can stop it from the sidebar")
        }
        Operation::DeleteSession { session } if is_me(session) => {
            refused("a session may not delete itself; the user can delete it from the sidebar")
        }
        Operation::InterruptSession { session } if is_me(session) => {
            PolicyDecision::Refuse(OpError::invalid_input(
                "a session may not interrupt itself: the interrupt would abort the turn waiting \
                 for this result",
            ))
        }
        Operation::DeleteWorktree {
            stop_sessions,
            delete_branch,
            ..
        } => PolicyDecision::Confirm(ConfirmedOp::DeleteWorktree {
            stop_sessions: *stop_sessions,
            delete_branch: *delete_branch,
        }),
        Operation::StopSession { .. } => PolicyDecision::Confirm(ConfirmedOp::StopSession),
        Operation::InterruptSession { .. } => {
            PolicyDecision::Confirm(ConfirmedOp::InterruptSession)
        }
        Operation::DeleteSession { .. } => PolicyDecision::Confirm(ConfirmedOp::DeleteSession),
        _ => PolicyDecision::Proceed,
    }
}
