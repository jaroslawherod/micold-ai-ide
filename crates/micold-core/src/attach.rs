//! Attaching a provider's existing worktrees and sessions (feature 582).
//!
//! Render-free and pure over the facts the daemon supplies: nothing here stores anything. The one
//! durable effect of attaching a worktree is the existing per-project provenance record
//! (`store.rs`), written by the daemon's `Catalog::attach_worktrees`.
//!
//! The types are enums rather than flags (Principle V): a session never has "no location"
//! ([`AttachTarget`]) and "resumable without a worktree" is not representable
//! ([`ResumableStatus`]).

use crate::session::{AiCli, Session, SessionLocation};
use crate::worktree::{classify_owner, ProvenanceView, Worktree, WorktreeOwner, WorktreeStatus};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

/// Why a worktree git lists cannot be attached right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unavailable {
    /// Git lists it but its directory is gone (prunable).
    Missing,
    /// A directory under the worktrees root that git does not know as a worktree.
    Invalid,
}

/// Whether an [`AttachableWorktree`] can be attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Availability {
    /// Valid on disk and known to git.
    Attachable,
    /// Listed so the user sees why it cannot be attached.
    Unavailable(Unavailable),
}

/// A provider worktree the catalog has no provenance record for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachableWorktree {
    /// Key of the provenance record; also the MCP reference.
    pub dir_name: String,
    /// Absolute path.
    pub path: PathBuf,
    /// Checked-out branch; `None` when detached.
    pub branch: Option<String>,
    /// The provider whose sessions are hosted here, when known.
    pub provider: Option<AiCli>,
    /// Catalog sessions hosted at this worktree, which attaching reveals.
    pub session_count: usize,
    /// Whether it can be attached.
    pub availability: Availability,
}

/// Where a resumable session ran. Never "nowhere".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachTarget {
    /// The project root.
    Default,
    /// A worktree, by its `dir_name`.
    Worktree {
        /// The worktree's directory name.
        dir_name: String,
    },
}

/// Why a stored session cannot be resumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnresumableReason {
    /// Its worktree is listed by git but its directory is gone.
    WorktreeMissing,
    /// Its worktree directory is not a worktree git knows.
    WorktreeInvalid,
    /// Its store directory names a location git does not list.
    NoLocation,
}

/// What resuming a stored session needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResumableStatus {
    /// Its location is the root or a startable catalog worktree.
    Resumable,
    /// Its worktree is attachable but not attached; resuming attaches it first.
    NeedsWorktreeAttach,
    /// Never resumed elsewhere.
    Unresumable(UnresumableReason),
}

/// A provider session found in a store that the catalog does not hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumableSession {
    /// The CLI's own id.
    pub id: Uuid,
    /// The provider that recorded it.
    pub provider: AiCli,
    /// Title, else label, when readable.
    pub title: Option<String>,
    /// Transcript or index modification time.
    pub last_activity: SystemTime,
    /// Where it ran.
    pub target: AttachTarget,
    /// What resuming it needs.
    pub status: ResumableStatus,
}

/// Why discovery skipped something. Informational: a note never aborts the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    /// The provider has no store here (not shown as an error).
    StoreMissing,
    /// The store exists but could not be read.
    StoreUnreadable,
    /// One entry could not be parsed.
    EntryCorrupt,
    /// The project runs in a sandbox whose home is not the host store.
    SandboxStoreNotReadable,
}

/// One thing discovery skipped, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryNote {
    /// The provider concerned, when one is.
    pub provider: Option<AiCli>,
    /// The path concerned, when one is.
    pub path: Option<PathBuf>,
    /// Why it was skipped.
    pub reason: SkipReason,
}

/// What a project offers to attach.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    /// Provider worktrees without a provenance record.
    pub worktrees: Vec<AttachableWorktree>,
    /// Stored sessions the catalog does not hold.
    pub sessions: Vec<ResumableSession>,
    /// Why something was skipped.
    pub notes: Vec<DiscoveryNote>,
}

impl DiscoveryReport {
    /// Nothing to attach (notes alone do not make a report non-empty).
    pub fn is_empty(&self) -> bool {
        self.worktrees.is_empty() && self.sessions.is_empty()
    }
}

/// One thing the user or an agent asks to attach.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AttachItem {
    /// A worktree, by `dir_name`.
    Worktree {
        /// The worktree's directory name.
        dir_name: String,
    },
    /// A stored session, by id.
    Session {
        /// The CLI's own id.
        id: Uuid,
    },
}

/// Why an attach was refused. Attaching changes nothing on a refused target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefuseReason {
    /// Absent from this repository's `git worktree list`, including another project's checkout.
    NotAWorktreeOfProject,
    /// Listed but not usable (missing or invalid).
    Unavailable,
    /// The catalog could not be written.
    IoFailed,
}

/// The result of attaching one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachOutcome {
    /// Newly attached.
    Attached,
    /// Already attached: nothing was duplicated.
    AlreadyAttached,
    /// Nothing changed, for this reason.
    Refused(RefuseReason),
}

/// One target and what happened to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachResult {
    /// The target.
    pub item: AttachItem,
    /// What happened.
    pub outcome: AttachOutcome,
}

/// The worktrees `worktrees` (from [`crate::worktree::reconcile`]) offers to attach: those the
/// catalog classifies [`WorktreeOwner::Agent`], so a recorded worktree is never listed (FR-004) and
/// only worktrees directly under `.claude/worktrees/` can qualify (the rest are `included`, hence
/// `User`). Pure: `sessions` are the catalog's sessions, from which `session_count` and `provider`
/// are read.
pub fn attachable_worktrees(
    worktrees: &[Worktree],
    provenance: &ProvenanceView<'_>,
    sessions: &[Session],
) -> Vec<AttachableWorktree> {
    worktrees
        .iter()
        .filter(|w| classify_owner(w, provenance) == WorktreeOwner::Agent)
        .map(|w| {
            let here = || {
                sessions.iter().filter(move |s| {
                    matches!(&s.location, SessionLocation::Worktree(d) if *d == w.dir_name)
                })
            };
            AttachableWorktree {
                dir_name: w.dir_name.clone(),
                path: w.path.clone(),
                branch: w.branch.clone(),
                provider: here().next().map(|s| s.provider),
                session_count: here().count(),
                availability: match w.status {
                    WorktreeStatus::Valid => Availability::Attachable,
                    WorktreeStatus::Missing => Availability::Unavailable(Unavailable::Missing),
                    WorktreeStatus::Invalid => Availability::Unavailable(Unavailable::Invalid),
                },
            }
        })
        .collect()
}
