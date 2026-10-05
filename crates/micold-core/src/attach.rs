//! Attaching a provider's existing worktrees and sessions (feature 582).
//!
//! Render-free and pure over the facts the daemon supplies: nothing here stores anything. The one
//! durable effect of attaching a worktree is the existing per-project provenance record
//! (`store.rs`), written by the daemon's `Catalog::attach_worktrees`.
//!
//! The types are enums rather than flags (Principle V): a session never has "no location"
//! ([`AttachTarget`]) and "resumable without a worktree" is not representable
//! ([`ResumableStatus`]).

use crate::provider::AiCliProvider;
use crate::session::{AiCli, Session, SessionLocation};
use crate::worktree::{classify_owner, ProvenanceView, Worktree, WorktreeOwner, WorktreeStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
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
    /// The session is starting, running or restarting here: resumed in at most one place (FR-016).
    AlreadyRunning,
}

impl RefuseReason {
    /// The sentence shown to the user for this refusal.
    pub fn text(self) -> &'static str {
        match self {
            Self::NotAWorktreeOfProject => "not a worktree of this project",
            Self::Unavailable => "its worktree or session is not available",
            Self::IoFailed => "the catalog could not be written",
            Self::AlreadyRunning => "that session is already running",
        }
    }
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

/// A path as a comparison key: separators unified, no trailing separator, lower case. Windows and
/// macOS file systems fold case and Windows writes `\`, so two spellings of one project path must
/// meet; a case-only collision between two real Linux projects is vanishingly rare and would only
/// ever cause a *skipped* listing, never a wrong attach.
pub fn path_key(path: &Path) -> String {
    let unified = path.to_string_lossy().replace('\\', "/");
    unified.trim_end_matches('/').to_lowercase()
}

/// Whether the key `inner` is strictly below the key `outer`.
pub fn is_within(inner: &str, outer: &str) -> bool {
    inner
        .strip_prefix(outer)
        .is_some_and(|rest| rest.starts_with('/') && rest.len() > 1)
}

/// One provider's store, with the config directory it is read from (`None` when the provider
/// cannot tell where it is).
pub struct StoreView<'a> {
    /// The provider.
    pub provider: &'a dyn AiCliProvider,
    /// Its base directory.
    pub config_dir: Option<PathBuf>,
}

/// What [`discover_resumable`] works from.
pub struct DiscoverInput<'a> {
    /// The project root.
    pub root: &'a Path,
    /// The worktrees git lists (from [`crate::worktree::discover`]).
    pub worktrees: &'a [Worktree],
    /// The provenance records, deciding which listed worktrees are hidden.
    pub provenance: &'a ProvenanceView<'a>,
    /// Session ids the catalog already holds; never listed (FR-007).
    pub known_ids: &'a BTreeSet<Uuid>,
    /// The most sessions to return, newest first (research R4).
    pub page: usize,
    /// When set, only these ids are considered: how an apply finds the one session it was asked
    /// to resume without reading the title of every other.
    pub only: Option<&'a BTreeSet<Uuid>>,
}

/// What discovery found in the provider stores.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResumableDiscovery {
    /// Sessions the catalog does not hold, newest first, at most one page.
    pub sessions: Vec<ResumableSession>,
    /// Why something was skipped.
    pub notes: Vec<DiscoveryNote>,
}

struct Found {
    id: Uuid,
    provider: AiCli,
    cwd: PathBuf,
    config_dir: PathBuf,
    target: AttachTarget,
    status: ResumableStatus,
    last_activity: SystemTime,
}

/// The sessions the provider stores hold for the project that the catalog does not (feature 582,
/// FR-006 to FR-010, FR-014; research R3, R4).
///
/// Read-only: only the provider's read methods are called. Cost: one directory listing per store
/// location and one `stat` per candidate; the title and label are read for the returned page only.
/// Known catalog ids are subtracted before any archive check. A store that is missing, unreadable
/// or partly corrupt yields a [`DiscoveryNote`] and never stops the pass.
pub fn discover_resumable(
    stores: &[StoreView<'_>],
    input: &DiscoverInput<'_>,
) -> ResumableDiscovery {
    let mut notes = Vec::new();
    let mut found: Vec<Found> = Vec::new();
    let mut seen: BTreeSet<Uuid> = input.known_ids.clone();
    let listed: Vec<PathBuf> = input.worktrees.iter().map(|w| w.path.clone()).collect();
    for store in stores {
        let provider = store.provider.id();
        let Some(config_dir) = store.config_dir.as_deref().filter(|d| d.is_dir()) else {
            notes.push(DiscoveryNote {
                provider: Some(provider),
                path: store.config_dir.clone(),
                reason: SkipReason::StoreMissing,
            });
            continue;
        };
        let dirs = store.provider.store_dirs(config_dir, input.root, &listed);
        notes.extend(dirs.notes);
        for dir in dirs.dirs {
            let Some((target, status)) = locate(&dir.cwd, input) else {
                continue;
            };
            for id in store.provider.recorded_session_ids(config_dir, &dir.cwd) {
                // Subtract first: a known id costs no stat.
                if input.only.is_some_and(|only| !only.contains(&id)) || !seen.insert(id) {
                    continue;
                }
                if store.provider.is_archived(config_dir, &dir.cwd, id) {
                    continue;
                }
                found.push(Found {
                    id,
                    provider,
                    cwd: dir.cwd.clone(),
                    config_dir: config_dir.to_path_buf(),
                    target: target.clone(),
                    status,
                    last_activity: store
                        .provider
                        .last_activity(config_dir, &dir.cwd, id)
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                });
            }
        }
    }
    found.sort_by(|a, b| b.last_activity.cmp(&a.last_activity).then(a.id.cmp(&b.id)));
    found.truncate(input.page);
    let sessions = found
        .into_iter()
        .map(|f| {
            let provider = stores
                .iter()
                .map(|s| s.provider)
                .find(|p| p.id() == f.provider)
                .expect("a found session came from one of the stores");
            // A title outranks a label, as everywhere else.
            let title = provider
                .read_title(&f.config_dir, &f.cwd, f.id)
                .or_else(|| provider.read_label(&f.config_dir, &f.cwd, f.id));
            ResumableSession {
                id: f.id,
                provider: f.provider,
                title,
                last_activity: f.last_activity,
                target: f.target,
                status: f.status,
            }
        })
        .collect();
    ResumableDiscovery { sessions, notes }
}

/// Where a store directory's sessions ran, and what resuming them needs; `None` when `cwd` is not
/// this project's at all.
fn locate(cwd: &Path, input: &DiscoverInput<'_>) -> Option<(AttachTarget, ResumableStatus)> {
    let key = path_key(cwd);
    if key == path_key(input.root) {
        return Some((AttachTarget::Default, ResumableStatus::Resumable));
    }
    if let Some(w) = input.worktrees.iter().find(|w| path_key(&w.path) == key) {
        let target = AttachTarget::Worktree {
            dir_name: w.dir_name.clone(),
        };
        let status = match w.status {
            WorktreeStatus::Missing => {
                ResumableStatus::Unresumable(UnresumableReason::WorktreeMissing)
            }
            WorktreeStatus::Invalid => {
                ResumableStatus::Unresumable(UnresumableReason::WorktreeInvalid)
            }
            WorktreeStatus::Valid => match classify_owner(w, input.provenance) {
                WorktreeOwner::Agent => ResumableStatus::NeedsWorktreeAttach,
                _ => ResumableStatus::Resumable,
            },
        };
        return Some((target, status));
    }
    // A worktree git does not list: its store directory outlived it.
    let managed = path_key(&input.root.join(".claude").join("worktrees"));
    let rest = key.strip_prefix(&managed)?.strip_prefix('/')?;
    // Split by hand: `Path::file_name` does not know `\` on a host that is not Windows.
    let dir_name = cwd
        .to_string_lossy()
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())?
        .to_string();
    (!rest.is_empty() && !rest.contains('/')).then_some({
        (
            AttachTarget::Worktree { dir_name },
            ResumableStatus::Unresumable(UnresumableReason::NoLocation),
        )
    })
}
