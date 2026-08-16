//! Worktree domain model, git-porcelain discovery, and create + rollback orchestration.
//!
//! Pure over the [`crate::git::Git`] boundary (no direct subprocess/`fs`), so the whole
//! create-then-rollback flow and the discovery/classification logic are unit-testable with
//! [`crate::git::FakeGit`] (Constitution Principle I). Contracts: `git-trait.md`, `naming.md`.
//! The one exception is [`discover`], the I/O convenience that pairs the git query with a directory
//! listing before delegating to the pure [`reconcile`].

use crate::git::Git;
use crate::naming::DerivedNames;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Health of a discovered worktree on disk (FR-018a). Enum, not bools, so the sidebar can
/// render an explicit state and block session-start at the type level (Principle V).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeStatus {
    /// Registered with git and its directory exists — fully usable.
    Valid,
    /// Registered with git but the directory is gone (deleted externally) / prunable.
    Missing,
    /// A directory exists under `.claude/worktrees/` that git does not know as a worktree.
    Invalid,
}

/// Directory-name prefix reserved for an AI assistant's own worktrees (feature 014, FR-005).
const AGENT_DIR_PREFIX: &str = "agent-";
/// Branch-name prefix reserved for an AI assistant's own worktrees (feature 014, FR-005).
const AGENT_BRANCH_PREFIX: &str = "worktree-agent-";
/// Shortest machine-generated identifier accepted after a reserved prefix (feature 014, FR-005).
///
/// The real generator emits 17 hex characters; a floor of 16 tolerates it changing width while
/// staying long enough that FR-006's false positives are not merely unlikely but essentially
/// impossible — an ordinary word can only reach it by being 16+ characters drawn solely from
/// `[0-9a-fA-F]`.
const AGENT_ID_MIN_LEN: usize = 16;

/// Whether `s` is a machine-generated agent identifier: long enough, and hex all the way through
/// (feature 014, FR-005/FR-006).
///
/// Requiring the *whole* string to be hex — not merely a hex prefix — is what keeps a real branch
/// like `agent-deadbeefdeadbeef-parser` visible: it is long enough, but the `-parser` tail is not
/// hex, so it is the user's own work.
fn is_agent_id(s: &str) -> bool {
    s.len() >= AGENT_ID_MIN_LEN && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Who created a worktree (feature 014, FR-001): the user, or an AI assistant for its own
/// background sub-task. An enum rather than a `bool` (Principle V) so a future third owner is an
/// added variant instead of a boolean-blindness refactor of every call site.
///
/// Derived from names only — never stored on [`Worktree`], never persisted — so it cannot drift
/// out of sync with the names it is derived from (FR-009, contracts/agent-worktree-classification.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeOwner {
    /// Created by the user, through the app or by hand. Always listed.
    User,
    /// Created by an AI assistant for a sub-task. Hidden unless the reveal control is on (FR-002).
    Agent,
}

/// Classify a worktree's owner from its names alone (feature 014, FR-005).
///
/// The single implementation of the naming half of FR-005, shared by [`Worktree::owner`] and by
/// [`BlockReason`]'s classification — a second copy would let "this worktree is hidden" and "the
/// branch is held by a hidden worktree" disagree about the same directory (BUG-001).
///
/// Either identifier suffices (OR, not AND): the directory survives when git no longer registers
/// the worktree, the branch survives when the directory was renamed, and a detached worktree has no
/// branch at all — all three must still classify (FR-007).
fn classify_owner(dir_name: &str, branch: Option<&str>) -> WorktreeOwner {
    let dir_match = dir_name
        .strip_prefix(AGENT_DIR_PREFIX)
        .is_some_and(is_agent_id);
    let branch_match = branch
        .and_then(|b| b.strip_prefix(AGENT_BRANCH_PREFIX))
        .is_some_and(is_agent_id);
    if dir_match || branch_match {
        WorktreeOwner::Agent
    } else {
        WorktreeOwner::User
    }
}

/// The directory this project's worktrees live in — the one location the app manages.
///
/// A function rather than a repeated `join` so [`discover`]'s "what the sidebar lists" and
/// [`checked_out_branches`]'s "whose worktree is this" ask the same question of the same path
/// (BUG-001, contract `branch-conflict.md` §1 rule 2).
pub fn worktrees_root(repo: &Path) -> PathBuf {
    repo.join(".claude/worktrees")
}

/// A path's last component, for naming a holder the user can see in the sidebar. Falls back to the
/// whole path when there is no final component (a root, or a path ending in `..`).
pub fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// An isolated worktree under `.claude/worktrees/`, bound to a dedicated branch (FR-006).
///
/// Sessions are associated by `dir_name` at the application layer (they are persisted
/// separately), so this type stays focused on git-derived facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    /// Directory component under `.claude/worktrees/` (`${type}-${ticket}-${name}`). Identity.
    ///
    /// For an [included](Self::included) worktree this is its folder name, which is *not* under that
    /// root and could therefore collide with one that is. [`reconcile`] disambiguates when it does —
    /// this is a key (sessions are addressed by it), so it has to be unique per project even though
    /// nothing on disk is renamed to make it so.
    pub dir_name: String,
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// Bound git branch (`${type}/${ticket}-${name}`). `None` for an orphan/invalid dir.
    pub branch: Option<String>,
    /// Health on disk (FR-018a).
    pub status: WorktreeStatus,
    /// Shown because the user asked for it rather than because of where it lives — a worktree the
    /// repository knows about, outside the directory this app creates its own in (BUG-002, FR-027).
    ///
    /// The list shows these by location as well as by name (FR-029): a folder name alone says
    /// nothing about where a worktree the app did not create actually is.
    pub included: bool,
}

impl Worktree {
    /// Whether a new session may be started on this worktree (FR-018a).
    pub fn can_start_session(&self) -> bool {
        self.status == WorktreeStatus::Valid
    }

    /// Classify this worktree from its names alone (feature 014, FR-005).
    ///
    /// PRECONDITION: `self` came from [`reconcile`], which already guarantees the worktree lives
    /// directly under the project's `.claude/worktrees/` root — the *location* half of FR-005.
    /// This method decides only the *naming* half, so do not call it on a `Worktree` obtained by
    /// any other route (contracts/agent-worktree-classification.md).
    ///
    /// Pure, total, health-blind, and stateless: no I/O, defined for `branch: None` and every
    /// [`WorktreeStatus`], and nothing is cached (FR-007, FR-009).
    pub fn owner(&self) -> WorktreeOwner {
        classify_owner(&self.dir_name, self.branch.as_deref())
    }

    /// `true` iff [`Self::owner`] is [`WorktreeOwner::Agent`] — the predicate the sidebar's
    /// visible-set filtering reads (FR-002).
    pub fn is_agent_owned(&self) -> bool {
        matches!(self.owner(), WorktreeOwner::Agent)
    }
}

/// One worktree entry parsed from `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRecord {
    /// Absolute path git reports for the worktree.
    pub path: PathBuf,
    /// Bound branch short name (`refs/heads/<name>` → `<name>`), or `None` if detached.
    pub branch: Option<String>,
    /// Git already considers this worktree stale/prunable.
    pub prunable: bool,
}

/// Parse `git worktree list --porcelain` output (FR-018). Pure — records are blank-line
/// separated `worktree <path>` / `branch refs/heads/<name>` / `prunable <reason>` blocks.
pub fn parse_worktrees(porcelain: &str) -> Vec<WorktreeRecord> {
    let mut records = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut prunable = false;

    let mut flush =
        |path: &mut Option<PathBuf>, branch: &mut Option<String>, prunable: &mut bool| {
            if let Some(p) = path.take() {
                records.push(WorktreeRecord {
                    path: p,
                    branch: branch.take(),
                    prunable: std::mem::take(prunable),
                });
            } else {
                *branch = None;
                *prunable = false;
            }
        };

    for line in porcelain.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            flush(&mut path, &mut branch, &mut prunable);
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            // A new record begins; flush any in-progress one first (defensive).
            flush(&mut path, &mut branch, &mut prunable);
            path = Some(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix("branch ") {
            branch = Some(
                rest.trim()
                    .strip_prefix("refs/heads/")
                    .unwrap_or(rest.trim())
                    .to_string(),
            );
        } else if line == "prunable" || line.starts_with("prunable ") {
            prunable = true;
        }
    }
    flush(&mut path, &mut branch, &mut prunable);
    records
}

/// Classify a parsed record given whether its directory currently exists on disk (FR-018a).
pub fn classify(record: &WorktreeRecord, dir_exists: bool) -> WorktreeStatus {
    if record.prunable || !dir_exists {
        WorktreeStatus::Missing
    } else {
        WorktreeStatus::Valid
    }
}

/// Build the `Worktree` list for the worktrees under `worktrees_root`, combining git records
/// with on-disk facts (FR-018/018a). `exists` reports whether a path currently exists;
/// `on_disk_dir_names` are the directory names actually present under `worktrees_root` (used
/// to surface orphan/invalid dirs git does not know about). Pure — the binary supplies the
/// `fs` facts.
pub fn reconcile(
    records: &[WorktreeRecord],
    worktrees_root: &Path,
    included: &[PathBuf],
    on_disk_dir_names: &[String],
    exists: &dyn Fn(&Path) -> bool,
) -> Vec<Worktree> {
    let mut out = Vec::new();
    let mut registered_dirs = Vec::new();

    // The app's own first, so their names are the ones an included worktree has to work around
    // rather than the other way about: a folder the app created is addressed by its own name in
    // sessions that already exist, and inclusion must not move that key out from under them.
    for rec in records {
        if rec.path.parent() != Some(worktrees_root) {
            continue;
        }
        let dir_name = folder_name(&rec.path);
        registered_dirs.push(dir_name.clone());
        out.push(Worktree {
            dir_name,
            path: rec.path.clone(),
            branch: rec.branch.clone(),
            status: classify(rec, exists(&rec.path)),
            included: false,
        });
    }

    // Directories present on disk but not registered with git → Invalid (orphan).
    for name in on_disk_dir_names {
        if !registered_dirs.contains(name) {
            registered_dirs.push(name.clone());
            out.push(Worktree {
                dir_name: name.clone(),
                path: worktrees_root.join(name),
                branch: None,
                status: WorktreeStatus::Invalid,
                included: false,
            });
        }
    }

    // …then the ones the user asked for, wherever they live (BUG-002, FR-027/FR-029).
    for path in included {
        // Only what git actually reports. A recorded location the repository no longer registers is
        // not a worktree, and inventing a row for it would be exactly the stale entry FR-031 asks
        // to be told about instead.
        let Some(rec) = records.iter().find(|r| &r.path == path) else {
            continue;
        };
        // An app-managed record is already above; including one is a no-op rather than a duplicate.
        if rec.path.parent() == Some(worktrees_root) {
            continue;
        }
        let dir_name = unique_dir_name(&rec.path, &registered_dirs);
        registered_dirs.push(dir_name.clone());
        out.push(Worktree {
            dir_name,
            path: rec.path.clone(),
            branch: rec.branch.clone(),
            status: classify(rec, exists(&rec.path)),
            included: true,
        });
    }

    out.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    out
}

/// A `dir_name` for an included worktree that no other entry is already using.
///
/// Its folder name if that is free — which is the ordinary case and the one the user recognises.
/// Otherwise the folder qualified by its parent, and failing that the whole path, which cannot
/// collide with anything. Nothing on disk is renamed by any of it (FR-028): this is the key the
/// app addresses the worktree by, and sessions are stored against it, so two worktrees sharing one
/// would hand each other's sessions out.
fn unique_dir_name(path: &Path, taken: &[String]) -> String {
    let folder = folder_name(path);
    if !taken.contains(&folder) {
        return folder;
    }
    if let Some(parent) = path.parent().map(folder_name) {
        let qualified = format!("{folder} ({parent})");
        if !taken.contains(&qualified) {
            return qualified;
        }
    }
    path.display().to_string()
}

// ---------------------------------------------------------------------------------------
// Feature 016 — existing-branch situations, candidates, and creation modes.
// Contracts: `contracts/branch-conflict.md`, `contracts/branch-picker.md`.
// ---------------------------------------------------------------------------------------

/// Where a candidate branch lives (feature 016, FR-011).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BranchOrigin {
    /// `refs/heads/<name>` exists.
    Local,
    /// Only `refs/remotes/<remote>/<name>` exists.
    Remote { remote: String },
}

/// Why a branch cannot back a new worktree (feature 016, FR-021/FR-021a/FR-021b).
///
/// Three variants rather than one path-carrying variant so each holder is described in terms the
/// user can act on, instead of every holder being rendered as a folder name. That mattered more
/// than it looked: git refuses a second checkout no matter who holds the branch, so this enum also
/// has to cover holders the app does not manage and never lists (BUG-001, research R1a).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReason {
    /// Checked out in another worktree **this app manages** — a direct child of
    /// `.claude/worktrees/`, i.e. one [`discover`] returns and the sidebar can show. `owner`
    /// separates a user's worktree from an agent's, which is hidden unless the reveal control is
    /// on (feature 014, FR-002): the holder is still the app's, so it is named as such, but the
    /// message has to say the list is currently omitting it.
    CheckedOutAt { path: PathBuf, owner: WorktreeOwner },
    /// Checked out in a worktree git knows about that this app does **not** manage — another
    /// tool's worktree directory, or a folder anywhere else on disk. Never appears in the sidebar,
    /// so the message must give the path rather than the folder name (FR-021a).
    CheckedOutOutsideApp { path: PathBuf },
    /// Checked out as the repository's own current branch.
    CheckedOutInProjectRoot,
}

impl BlockReason {
    /// The user-facing sentence explaining who holds `branch` (FR-021/FR-021a/FR-021b, SC-006).
    ///
    /// Lives in core, not in the two renderers, for two reasons. It is the part of the block a
    /// user actually reads, so it needs tests, and Principle I keeps logic out of the `iced`
    /// layer. And the client's pre-flight refusal and the daemon's create-time refusal describe
    /// the same fact — before BUG-001 they described it in two hand-written wordings that could
    /// drift apart, and did.
    ///
    /// Each holder is phrased for what the user can do about it: a listed worktree by the folder
    /// name that is its sidebar row; a hidden one by that name plus how to bring it into view; an
    /// unmanaged one by its full path, since no amount of looking in the sidebar will find it.
    pub fn explain(&self, branch: &str) -> String {
        match self {
            BlockReason::CheckedOutInProjectRoot => {
                format!("'{branch}' is currently checked out in the project itself.")
            }
            BlockReason::CheckedOutAt {
                path,
                owner: WorktreeOwner::User,
            } => format!(
                "'{branch}' is already checked out in the worktree '{}'.",
                folder_name(path)
            ),
            BlockReason::CheckedOutAt {
                path,
                owner: WorktreeOwner::Agent,
            } => format!(
                "'{branch}' is already checked out in the agent worktree '{}', which the sidebar \
                 hides until you turn on \"Show agent worktrees\".",
                folder_name(path)
            ),
            BlockReason::CheckedOutOutsideApp { path } => format!(
                "'{branch}' is already checked out in a worktree outside this app: {}.",
                path.display()
            ),
        }
    }
}

/// One row of the existing-branch picker (feature 016, FR-010–FR-012).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchCandidate {
    /// Short branch name, with no `refs/…` prefix (may itself contain `/`).
    pub name: String,
    /// Local, or the named remote it came from.
    pub origin: BranchOrigin,
    /// `Some` when the branch is visible but not creatable (FR-012).
    pub blocked_by: Option<BlockReason>,
}

impl BranchCandidate {
    /// Whether this candidate can back a new worktree.
    pub fn is_available(&self) -> bool {
        self.blocked_by.is_none()
    }
}

impl fmt::Display for BranchCandidate {
    /// The picker row label (contract `branch-picker.md` §2). `Select` requires `ToString`, so
    /// this IS the rendered row — keeping it here means no widget change is needed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        if let BranchOrigin::Remote { remote } = &self.origin {
            write!(f, " · {remote}")?;
        }
        // A row is one line, so it names the KIND of holder. The location that makes an unmanaged
        // holder findable belongs in the inline explanation, which has room for it — and a row
        // must never print a folder name for a holder the sidebar cannot show (BUG-001).
        match &self.blocked_by {
            Some(BlockReason::CheckedOutInProjectRoot) => {
                f.write_str(" · in use by the project checkout")
            }
            Some(BlockReason::CheckedOutAt {
                owner: WorktreeOwner::Agent,
                ..
            }) => f.write_str(" · in use by a hidden agent worktree"),
            Some(BlockReason::CheckedOutAt { path, .. }) => {
                write!(f, " · in use by {}", folder_name(path))
            }
            Some(BlockReason::CheckedOutOutsideApp { .. }) => {
                f.write_str(" · in use outside this app")
            }
            None => Ok(()),
        }
    }
}

/// What pre-flight found for one derived or selected branch name (feature 016).
///
/// The ONLY producer of a [`CreateMode`]: "reuse a branch that is checked out elsewhere" and
/// "overwrite a remote branch" are unrepresentable rather than merely unreachable
/// (Constitution Principle V).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchSituation {
    /// The name is unused — create exactly as before, with no prompt (FR-025).
    Free,
    /// A local branch of this name exists and is not checked out: reuse or overwrite (FR-002).
    LocalAvailable { branch: String },
    /// The name exists only on a remote: continue from it, or start fresh (FR-016/FR-018).
    /// `remotes` lists EVERY remote carrying the name, sorted — not just one. When a name exists
    /// on several remotes the user picks which to continue from; the app must never choose
    /// silently (spec Edge Cases).
    RemoteOnly {
        branch: String,
        remotes: Vec<String>,
    },
    /// The branch is checked out somewhere — explain only, no branch action (FR-021).
    Blocked { branch: String, reason: BlockReason },
    /// The target directory is taken — explain only; outranks every branch case (FR-022).
    DirectoryTaken { dir: PathBuf },
}

/// The user's resolved decision, handed back into [`create_worktree`] (feature 016).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CreateMode {
    /// Create a fresh branch at HEAD — today's behavior, and the default.
    #[default]
    NewBranch,
    /// Check out an existing local branch, leaving its history untouched (FR-004).
    ReuseLocal,
    /// Discard the existing branch and recreate it at HEAD (FR-006). Destructive.
    Overwrite,
    /// Start a local branch at `<remote>/<branch>` and track it (FR-017).
    TrackRemote { remote: String },
}

impl CreateMode {
    /// Whether this mode brings the branch into existence — and therefore whether rollback owns
    /// it (feature 016, FR-008).
    ///
    /// This single predicate is what stops a failed *reuse* from deleting the user's
    /// pre-existing commits. See [`rollback_plan`].
    pub fn creates_branch(&self) -> bool {
        !matches!(self, CreateMode::ReuseLocal)
    }

    /// Whether `situation`, as re-observed at the moment of action, still supports this mode
    /// (feature 016, FR-009; contract `branch-conflict.md` §4).
    pub fn is_compatible_with(&self, situation: &BranchSituation) -> bool {
        match (self, situation) {
            (CreateMode::NewBranch, BranchSituation::Free) => true,
            // The deliberate "start fresh at HEAD" answer to a remote-only name (FR-018).
            (CreateMode::NewBranch, BranchSituation::RemoteOnly { .. }) => true,
            (CreateMode::ReuseLocal, BranchSituation::LocalAvailable { .. }) => true,
            (CreateMode::Overwrite, BranchSituation::LocalAvailable { .. }) => true,
            // Any remote that actually carries the ref is a valid answer — which one is the
            // user's explicit choice, not ours.
            (CreateMode::TrackRemote { remote }, BranchSituation::RemoteOnly { remotes, .. }) => {
                remotes.contains(remote)
            }
            _ => false,
        }
    }
}

/// Parse `git for-each-ref --format=%(refname) refs/heads refs/remotes` output (FR-011).
///
/// Pure. Drops `refs/remotes/<remote>/HEAD` (a symbolic alias, not a branch) and any line that
/// is neither a head nor a remote. A name present both locally and on a remote collapses to the
/// LOCAL candidate — reuse and overwrite act on the local branch (FR-019).
pub fn parse_branch_refs(refs: &str) -> Vec<BranchCandidate> {
    let mut local: Vec<String> = Vec::new();
    let mut remote: Vec<(String, String)> = Vec::new();

    for line in refs.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("refs/heads/") {
            if !name.is_empty() {
                local.push(name.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("refs/remotes/") {
            // The remote is the FIRST path component; everything after it is the branch name,
            // which may itself contain `/`.
            let Some((remote_name, branch)) = rest.split_once('/') else {
                continue;
            };
            if remote_name.is_empty() || branch.is_empty() || branch == "HEAD" {
                continue;
            }
            remote.push((remote_name.to_string(), branch.to_string()));
        }
    }

    let mut out: Vec<BranchCandidate> = local
        .iter()
        .map(|name| BranchCandidate {
            name: name.clone(),
            origin: BranchOrigin::Local,
            blocked_by: None,
        })
        .collect();

    for (remote_name, branch) in remote {
        if local.contains(&branch) {
            continue; // FR-019: the local branch wins.
        }
        out.push(BranchCandidate {
            name: branch,
            origin: BranchOrigin::Remote {
                remote: remote_name,
            },
            blocked_by: None,
        });
    }

    sort_candidates(&mut out);
    out
}

/// Order candidates for stable rendering and assertions (contract `branch-picker.md` §2):
/// available before blocked, local before remote, then by remote name, then by branch name.
fn sort_candidates(candidates: &mut [BranchCandidate]) {
    candidates.sort_by(|a, b| {
        a.is_available()
            .cmp(&b.is_available())
            .reverse()
            .then_with(|| a.origin.cmp(&b.origin))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// Which branch, if any, each registered worktree is holding, paired with why that blocks.
///
/// Reads the RAW porcelain records rather than [`reconcile`]: `reconcile` keeps only worktrees
/// under `.claude/worktrees/`, which discards the repository's own main checkout — the very
/// record FR-021's project-root case needs (research R1).
///
/// But raw means *raw*, and that is the half research R1 missed: the records also carry worktrees
/// this app does not manage, which `reconcile` drops and the sidebar therefore never shows. They
/// must still block — git refuses the second checkout wherever the branch is held — so the fix is
/// to describe them honestly rather than to stop seeing them (BUG-001, research R1a).
///
/// The app-managed test is [`reconcile`]'s own, applied to the same [`worktrees_root`], so the set
/// of holders called "one of your worktrees" is exactly the set [`discover`] returns.
fn checked_out_branches(
    records: &[WorktreeRecord],
    repo: &Path,
    included: &[PathBuf],
) -> Vec<(String, BlockReason)> {
    let root = worktrees_root(repo);
    records
        .iter()
        .filter_map(|rec| {
            let branch = rec.branch.clone()?;
            let reason = if rec.path == repo {
                BlockReason::CheckedOutInProjectRoot
            } else if rec.path.parent() == Some(root.as_path()) || included.contains(&rec.path) {
                // The included half is what keeps FR-032 true by construction rather than by
                // agreement: this is `reconcile`'s test, so a holder described as one of the app's
                // worktrees is one the list is showing — before inclusion and after it (BUG-002).
                BlockReason::CheckedOutAt {
                    path: rec.path.clone(),
                    owner: classify_owner(&folder_name(&rec.path), rec.branch.as_deref()),
                }
            } else {
                BlockReason::CheckedOutOutsideApp {
                    path: rec.path.clone(),
                }
            };
            Some((branch, reason))
        })
        .collect()
}

/// Classify what stands between the user and a new worktree on `branch` (feature 016, FR-001).
///
/// Pure over the [`Git`] boundary and **never mutates**. Called twice per creation: once to
/// raise the prompt, and again inside [`create_worktree`] to re-verify the answer against
/// reality before acting (FR-009).
///
/// Precedence — first match wins (contract `branch-conflict.md` §1):
/// directory clash → checked out → local branch → remote-only branch → free.
pub fn preflight(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    branch: &str,
    target_exists: bool,
    included: &[PathBuf],
) -> io::Result<BranchSituation> {
    let porcelain = git.worktree_list_porcelain(repo)?;
    let records = parse_worktrees(&porcelain);

    // 1. Directory first: no branch choice could resolve it (FR-022).
    if target_exists || records.iter().any(|r| r.path == target_path) {
        return Ok(BranchSituation::DirectoryTaken {
            dir: target_path.to_path_buf(),
        });
    }

    // 2. Checked out somewhere: neither reusable nor overwritable (FR-021).
    if let Some((_, reason)) = checked_out_branches(&records, repo, included)
        .into_iter()
        .find(|(b, _)| b == branch)
    {
        return Ok(BranchSituation::Blocked {
            branch: branch.to_string(),
            reason,
        });
    }

    // 3. An ordinary local branch — the case this feature exists for.
    if git.branch_exists(repo, branch)? {
        return Ok(BranchSituation::LocalAvailable {
            branch: branch.to_string(),
        });
    }

    // 4. Remote-only. Ask for the ref listing only now — a free name costs no extra git call.
    let refs = git.list_branch_refs(repo)?;
    let mut remotes: Vec<String> = parse_branch_refs(&refs)
        .into_iter()
        .filter_map(|c| match c {
            BranchCandidate {
                name,
                origin: BranchOrigin::Remote { remote },
                ..
            } if name == branch => Some(remote),
            _ => None,
        })
        .collect();
    remotes.sort();
    if !remotes.is_empty() {
        return Ok(BranchSituation::RemoteOnly {
            branch: branch.to_string(),
            remotes,
        });
    }

    Ok(BranchSituation::Free)
}

/// Every branch in the repository, annotated with why it cannot be used where that applies
/// (feature 016, FR-010–FR-012).
///
/// One ref listing plus the worktree records already needed elsewhere — no second git call per
/// candidate.
pub fn branch_candidates(
    git: &dyn Git,
    repo: &Path,
    included: &[PathBuf],
) -> io::Result<Vec<BranchCandidate>> {
    let refs = git.list_branch_refs(repo)?;
    let porcelain = git.worktree_list_porcelain(repo)?;
    let held = checked_out_branches(&parse_worktrees(&porcelain), repo, included);

    let mut candidates = parse_branch_refs(&refs);
    for candidate in &mut candidates {
        candidate.blocked_by = held
            .iter()
            .find(|(b, _)| *b == candidate.name)
            .map(|(_, reason)| reason.clone());
    }
    sort_candidates(&mut candidates);
    Ok(candidates)
}

/// Discover a project's worktrees from git + the filesystem in one call (FR-018/018a).
///
/// The one I/O convenience in this otherwise-pure module: it performs the git query and the
/// `.claude/worktrees` directory listing, then hands both to the pure [`reconcile`]. Shared by the
/// client (its sidebar) and the daemon (its catalog snapshot) so the two never drift in *how* a
/// worktree is discovered. A git failure degrades to "no registrations" — an unavailable repo simply
/// surfaces its on-disk orphan dirs (as `Invalid`), never a panic.
/// `included` is the project's own set of worktrees to show from elsewhere (BUG-002, FR-030) — the
/// one input here that is not derived, because nothing git reports distinguishes a worktree the user
/// asked for from one they have never heard of.
pub fn discover(git: &dyn Git, repo: &Path, included: &[PathBuf]) -> Vec<Worktree> {
    let porcelain = git.worktree_list_porcelain(repo).unwrap_or_default();
    let records = parse_worktrees(&porcelain);
    let root = worktrees_root(repo);
    let on_disk = list_dir_names(&root);
    reconcile(&records, &root, included, &on_disk, &|p| p.exists())
}

/// The immediate sub-directory names of `dir` (non-recursive), used to surface on-disk worktree
/// dirs git does not know about. An unreadable/absent directory yields an empty list.
fn list_dir_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Why creating a worktree failed (FR-006b, FR-009). Carries `String` messages (not
/// `io::Error`) so it is `Clone`/`Eq` and can live in application state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateError {
    /// A worktree with the derived directory name already exists (FR-009).
    DuplicateDir,
    /// The branch is checked out elsewhere and cannot back another worktree (feature 016,
    /// FR-021). Carries enough to name the holder rather than just failing.
    BranchInUse { branch: String, reason: BlockReason },
    /// The branch's situation changed between the user's answer and the act, so the operation
    /// was abandoned before touching anything (feature 016, FR-009).
    SituationChanged,
    /// Creation failed and was rolled back cleanly (FR-006b); carries the git error text.
    RolledBack(String),
}

/// A single rollback action, in the order it must run (FR-006b). Modeled as data so the
/// unwind sequence is unit-testable. `RemoveDir` is executed by the binary (fs); the git
/// steps run through the [`Git`] boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupStep {
    /// `git worktree remove --force <path>` (remove registration BEFORE deleting the branch).
    WorktreeRemove,
    /// `git worktree prune` (clear any half-written admin entry).
    WorktreePrune,
    /// `git branch -D <branch>`.
    BranchDelete,
    /// Remove the target directory if it still exists (fs).
    RemoveDir,
}

/// The ordered rollback plan for a failed create (FR-006b), for `mode` (feature 016, FR-008).
///
/// Order matters: the worktree registration is removed before the branch is deleted (git refuses
/// to delete a checked-out branch).
///
/// [`CleanupStep::BranchDelete`] is omitted when the mode did not create the branch — i.e. for
/// [`CreateMode::ReuseLocal`]. Without that, recovering from a failure the user did not cause
/// would destroy the pre-existing commits they were trying to continue (SC-003). Overwrite and
/// remote-tracking DO delete: the branch present at failure is the one this attempt put there.
pub fn rollback_plan(mode: &CreateMode) -> Vec<CleanupStep> {
    let mut plan = vec![CleanupStep::WorktreeRemove, CleanupStep::WorktreePrune];
    if mode.creates_branch() {
        plan.push(CleanupStep::BranchDelete);
    }
    plan.push(CleanupStep::RemoveDir);
    plan
}

/// The result of [`remove_worktree`] (feature 013, FR-011–FR-015). Returned only on `Ok` — the
/// worktree directory/registration removal itself succeeded; `branch_delete_failed` reports the
/// separate, non-fatal outcome of the (optional) branch-deletion step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemoveOutcome {
    /// `true` when the caller asked to delete the branch (`branch: Some(_)`) and the branch
    /// genuinely could not be deleted (FR-015). Always `false` when `branch` was `None` (the
    /// user opted to keep it) — that path never calls `branch_delete` at all.
    pub branch_delete_failed: bool,
}

/// Remove a worktree and, optionally, its branch, app-owned (feature 008, FR-020; feature 013,
/// FR-011–FR-015). Runs the git steps in [`CleanupStep`] order — `worktree_remove(force)` →
/// `worktree_prune` → conditional `branch_delete` — so a checked-out branch can be deleted after
/// its registration is gone. `worktree_remove`/`worktree_prune` failures still propagate via `?`
/// unchanged (an already-missing worktree is not an error, so a partially-removed worktree still
/// resolves to a consistent state, FR-023); only a `branch_delete` failure (when `branch` is
/// `Some`) is captured into the returned [`RemoveOutcome`] instead of aborting the function — the
/// worktree/session cleanup already succeeded independent of whether its branch could be deleted.
/// `branch: None` (the user opted to keep it) skips `branch_delete` entirely, leaving the branch
/// untouched. The caller removes the directory (`fs`) and terminates the worktree's session
/// processes.
pub fn remove_worktree(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    branch: Option<&str>,
) -> io::Result<RemoveOutcome> {
    git.worktree_remove(repo, target_path, true)?;
    git.worktree_prune(repo)?;
    let branch_delete_failed = match branch {
        Some(branch) => git.branch_delete(repo, branch).is_err(),
        None => false,
    };
    Ok(RemoveOutcome {
        branch_delete_failed,
    })
}

/// How many surviving paths [`remove_worktree_dir`] names before the list stops informing anyone.
/// A blocked `build/` tree can hold tens of thousands of entries; the first few blockers say
/// everything the rest would.
pub const LEFTOVER_REPORT_CAP: usize = 10;

/// One filesystem entry that survived [`remove_worktree_dir`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leftover {
    /// The surviving path.
    pub path: PathBuf,
    /// The uid owning it, when that is **not** the removing process's own — the ordinary cause of
    /// an unremovable worktree, and the one thing that makes the failure actionable (a container
    /// wrote build output through a bind mount as root, so the daemon's own uid cannot unlink it).
    /// `None` when the owner matches, or on platforms without uids.
    pub foreign_uid: Option<u32>,
}

/// Remove the worktree's working directory once git has released it (feature 008, FR-023a),
/// reporting what survived.
///
/// `git worktree remove` deletes the working directory itself, so this normally finds nothing
/// left — that is the ordinary success path, not a failure. Reporting `NotFound` made every
/// successful delete raise "its folder could not be removed: No such file or directory"
/// naming a path that no longer existed (BUG-001).
///
/// An empty return means the directory is gone. A non-empty one means it is not, and names the
/// entries that blocked it — `std::fs::remove_dir_all` reports only the first errno and never the
/// path, which left the user with a bare "Permission denied (os error 13)" for a tree they had no
/// way to identify. Removal is **not** retried here: the walk only observes what is still there.
///
/// This is deliberately not an `Err`. Reaching this point means git already deregistered the
/// worktree, so the operation did partly succeed; treating leftovers as total failure skipped the
/// session cleanup and left the directory to reappear as an unregistered orphan.
pub fn remove_worktree_dir(path: &Path) -> Vec<Leftover> {
    if let Err(err) = std::fs::remove_dir_all(path) {
        if err.kind() != io::ErrorKind::NotFound {
            let mut leftovers = Vec::new();
            collect_leftovers(path, &mut leftovers);
            return leftovers;
        }
    }
    Vec::new()
}

/// Name the shallowest entries still present under `dir`, up to [`LEFTOVER_REPORT_CAP`].
///
/// A foreign-owned entry is not descended into: its whole subtree shares that one cause, so
/// listing it would crowd out the *other* blockers, which is exactly the information the user
/// needs. Directory recursion uses the entry's own file type (`lstat`, no symlink following), so a
/// symlink inside the worktree can never walk the reporter out of the tree.
fn collect_leftovers(dir: &Path, out: &mut Vec<Leftover>) {
    if out.len() >= LEFTOVER_REPORT_CAP {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        // Not listable (or not a directory at all) — `dir` itself is what survived.
        out.push(leftover(dir));
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= LEFTOVER_REPORT_CAP {
            return;
        }
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let found = leftover(&path);
        if is_dir && found.foreign_uid.is_none() {
            collect_leftovers(&path, out);
        } else {
            out.push(found);
        }
    }
}

/// Describe one surviving path, recording its owner only when it differs from this process's.
fn leftover(path: &Path) -> Leftover {
    Leftover {
        path: path.to_path_buf(),
        foreign_uid: foreign_uid(path),
    }
}

#[cfg(unix)]
fn foreign_uid(path: &Path) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    // `symlink_metadata`: unlinking is governed by the link's own owner, not its target's.
    let uid = std::fs::symlink_metadata(path).ok()?.uid();
    // SAFETY: `geteuid` is always safe — it takes no arguments and cannot fail.
    let euid = unsafe { libc::geteuid() };
    (uid != euid).then_some(uid)
}

#[cfg(not(unix))]
fn foreign_uid(_path: &Path) -> Option<u32> {
    None
}

/// The named stage a worktree creation is (or was, on failure) currently in (feature 013,
/// FR-006/FR-007/FR-009). A closed enum (Principle V) so the progress display can never show a
/// stage that isn't a real, reachable step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreateStage {
    /// Duplicate-branch / duplicate-directory pre-flight checks (no mutation yet).
    PreflightCheck,
    /// `git worktree add -b <branch> <path> HEAD`.
    CreatingWorktree,
    /// `git submodule update --init --recursive` — only reached when the new worktree's own
    /// checkout declares submodules (FR-008).
    SettingUpSubmodules,
    /// Unwinding a failed create (`worktree remove` → `prune` → `branch delete`).
    RollingBack,
}

impl CreateStage {
    /// Plain-language description of the current stage, for the progress display (FR-007).
    ///
    /// The stage SET is closed and mode-independent; only the worktree-creating stage's wording
    /// varies, so the user is told which of the four things is actually happening (feature 016,
    /// FR-024).
    pub fn label(&self, mode: &CreateMode) -> &'static str {
        match self {
            Self::PreflightCheck => "Checking for naming conflicts",
            Self::CreatingWorktree => match mode {
                CreateMode::NewBranch => "Creating branch and worktree",
                CreateMode::ReuseLocal => "Checking out existing branch",
                CreateMode::Overwrite => "Replacing branch and creating worktree",
                CreateMode::TrackRemote { .. } => "Creating tracking branch and worktree",
            },
            Self::SettingUpSubmodules => "Setting up submodules",
            Self::RollingBack => "Rolling back",
        }
    }
}

/// One stage-tagged progress line from an in-flight [`create_worktree`] call (feature 013,
/// replaces the earlier bare-`String` progress channel — the `line` content is unchanged, only
/// the stage tag is new).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProgressEvent {
    /// Which stage produced this line.
    pub stage: CreateStage,
    /// The human-readable line itself (an executed command, or live submodule-fetch output).
    pub line: String,
}

/// Create a branch + worktree from derived names, rolling back on failure (FR-006/006b/009).
///
/// Pre-flight duplicate checks run before any mutation. `target_exists` is the binary's `fs`
/// check that the target directory is already present/non-empty. On a git failure the
/// [`rollback_plan`] git steps run (the caller removes the directory — [`CleanupStep::RemoveDir`]).
///
/// `on_progress` is called with a stage-tagged [`CreateProgressEvent`] for each executed command
/// and, for the (potentially slow) submodule fetch, its live output — so the caller can surface
/// progress instead of the operation appearing to hang (feature 010 follow-up; stage tags added
/// feature 013). Callers that don't care about progress can pass `&mut |_| {}`.
///
/// The parameter list is at the lint's limit plus one, and deliberately so: every argument here is
/// an independent input the caller genuinely has, and bundling them into a struct would only move
/// the same eight names one indirection away. `included` is the newest of them (016 BUG-002) and
/// exists for one reason — the re-verification below classifies the holder, and it must classify it
/// exactly as the list does (FR-032).
#[allow(clippy::too_many_arguments)]
pub fn create_worktree(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    names: &DerivedNames,
    target_exists: bool,
    mode: &CreateMode,
    included: &[PathBuf],
    on_progress: &mut dyn FnMut(CreateProgressEvent),
) -> Result<Worktree, CreateError> {
    // Pre-flight (fail fast, no mutation). Re-run here rather than trusting whatever the caller
    // observed before prompting: the user's answer is separated from the act by think-time, in
    // which another terminal can create, delete, or check out the branch (feature 016, FR-009).
    on_progress(CreateProgressEvent {
        stage: CreateStage::PreflightCheck,
        line: "Checking for naming conflicts…".to_string(),
    });
    let situation = preflight(
        git,
        repo,
        target_path,
        &names.branch,
        target_exists,
        included,
    )
    .map_err(|e| CreateError::RolledBack(e.to_string()))?;
    match &situation {
        BranchSituation::DirectoryTaken { .. } => return Err(CreateError::DuplicateDir),
        BranchSituation::Blocked { branch, reason } => {
            return Err(CreateError::BranchInUse {
                branch: branch.clone(),
                reason: reason.clone(),
            })
        }
        _ if !mode.is_compatible_with(&situation) => return Err(CreateError::SituationChanged),
        _ => {}
    }

    on_progress(CreateProgressEvent {
        stage: CreateStage::CreatingWorktree,
        line: create_command_line(mode, &names.branch, target_path),
    });
    let added = match mode {
        CreateMode::NewBranch => git.worktree_add_new_branch(repo, &names.branch, target_path),
        CreateMode::ReuseLocal => {
            git.worktree_add_existing_branch(repo, &names.branch, target_path)
        }
        CreateMode::Overwrite => git.worktree_add_reset_branch(repo, &names.branch, target_path),
        CreateMode::TrackRemote { remote } => {
            git.worktree_add_tracking_branch(repo, &names.branch, remote, target_path)
        }
    };
    if let Err(e) = added {
        on_progress(CreateProgressEvent {
            stage: CreateStage::CreatingWorktree,
            line: format!("worktree add failed: {e}"),
        });
        on_progress(CreateProgressEvent {
            stage: CreateStage::RollingBack,
            line: "Rolling back…".to_string(),
        });
        run_rollback(git, repo, target_path, &names.branch, mode);
        return Err(CreateError::RolledBack(e.to_string()));
    }

    // Submodules, if any, are fetched from the worktree's own checkout (feature 010,
    // research R1) — a failure here rolls back the whole creation exactly like a failed
    // `worktree_add_new_branch` above (spec FR-005), via the same rollback plan.
    if git.has_submodules(target_path) {
        on_progress(CreateProgressEvent {
            stage: CreateStage::SettingUpSubmodules,
            line: "$ git submodule update --init --recursive".to_string(),
        });
        let mut on_line = |line: String| {
            on_progress(CreateProgressEvent {
                stage: CreateStage::SettingUpSubmodules,
                line,
            });
        };
        if let Err(e) = git.submodule_update_init_recursive(target_path, &mut on_line) {
            on_progress(CreateProgressEvent {
                stage: CreateStage::SettingUpSubmodules,
                line: format!("submodule update failed: {e}"),
            });
            on_progress(CreateProgressEvent {
                stage: CreateStage::RollingBack,
                line: "Rolling back…".to_string(),
            });
            run_rollback(git, repo, target_path, &names.branch, mode);
            return Err(CreateError::RolledBack(e.to_string()));
        }
    }

    Ok(Worktree {
        dir_name: names.dir_name.clone(),
        path: target_path.to_path_buf(),
        branch: Some(names.branch.clone()),
        status: WorktreeStatus::Valid,
        // Created where the app creates its own, so it is listed for that reason and not because
        // anyone asked for it by location (FR-027).
        included: false,
    })
}

/// The git command a mode will run, echoed into the progress log (feature 016, FR-024).
fn create_command_line(mode: &CreateMode, branch: &str, target_path: &Path) -> String {
    let path = target_path.display();
    match mode {
        CreateMode::NewBranch => format!("$ git worktree add -b {branch} {path} HEAD"),
        CreateMode::ReuseLocal => format!("$ git worktree add {path} {branch}"),
        CreateMode::Overwrite => format!("$ git worktree add -B {branch} {path} HEAD"),
        CreateMode::TrackRemote { remote } => {
            format!("$ git worktree add --track -b {branch} {path} {remote}/{branch}")
        }
    }
}

/// Run the git steps of the rollback plan in order (RemoveDir is the caller's — [`CleanupStep::RemoveDir`]).
///
/// The plan is a function of `mode`, so a failed reuse never reaches [`CleanupStep::BranchDelete`]
/// at all (feature 016, FR-008) — the guard lives in [`rollback_plan`], not in a special case
/// here.
fn run_rollback(git: &dyn Git, repo: &Path, target_path: &Path, branch: &str, mode: &CreateMode) {
    for step in rollback_plan(mode) {
        match step {
            CleanupStep::WorktreeRemove => {
                let _ = git.worktree_remove(repo, target_path, true);
            }
            CleanupStep::WorktreePrune => {
                let _ = git.worktree_prune(repo);
            }
            CleanupStep::BranchDelete => {
                let _ = git.branch_delete(repo, branch);
            }
            CleanupStep::RemoveDir => {}
        }
    }
}
