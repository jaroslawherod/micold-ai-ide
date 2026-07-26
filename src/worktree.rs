//! Worktree domain model, git-porcelain discovery, and create + rollback orchestration.
//!
//! Pure over the [`crate::git::Git`] boundary (no direct subprocess/`fs`), so the whole
//! create-then-rollback flow and the discovery/classification logic are unit-testable with
//! [`crate::git::FakeGit`] (Constitution Principle I). Contracts: `git-trait.md`, `naming.md`.

use crate::git::Git;
use crate::naming::DerivedNames;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeOwner {
    /// Created by the user, through the app or by hand. Always listed.
    User,
    /// Created by an AI assistant for a sub-task. Hidden unless the reveal control is on (FR-002).
    Agent,
}

/// An isolated worktree under `.claude/worktrees/`, bound to a dedicated branch (FR-006).
///
/// Sessions are associated by `dir_name` at the application layer (they are persisted
/// separately), so this type stays focused on git-derived facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    /// Directory component under `.claude/worktrees/` (`${type}-${ticket}-${name}`). Identity.
    pub dir_name: String,
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// Bound git branch (`${type}/${ticket}-${name}`). `None` for an orphan/invalid dir.
    pub branch: Option<String>,
    /// Health on disk (FR-018a).
    pub status: WorktreeStatus,
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
        // Either identifier suffices (OR, not AND): the directory survives when git no longer
        // registers the worktree, the branch survives when the directory was renamed, and a
        // detached worktree has no branch at all — all three must still classify (FR-007).
        let dir_match = self
            .dir_name
            .strip_prefix(AGENT_DIR_PREFIX)
            .is_some_and(is_agent_id);
        let branch_match = self
            .branch
            .as_deref()
            .and_then(|b| b.strip_prefix(AGENT_BRANCH_PREFIX))
            .is_some_and(is_agent_id);
        if dir_match || branch_match {
            WorktreeOwner::Agent
        } else {
            WorktreeOwner::User
        }
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
    on_disk_dir_names: &[String],
    exists: &dyn Fn(&Path) -> bool,
) -> Vec<Worktree> {
    let mut out = Vec::new();
    let mut registered_dirs = Vec::new();

    for rec in records {
        // Only worktrees under this project's `.claude/worktrees/` are ours.
        if rec.path.parent() != Some(worktrees_root) {
            continue;
        }
        let dir_name = rec
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        registered_dirs.push(dir_name.clone());
        out.push(Worktree {
            dir_name,
            path: rec.path.clone(),
            branch: rec.branch.clone(),
            status: classify(rec, exists(&rec.path)),
        });
    }

    // Directories present on disk but not registered with git → Invalid (orphan).
    for name in on_disk_dir_names {
        if !registered_dirs.contains(name) {
            out.push(Worktree {
                dir_name: name.clone(),
                path: worktrees_root.join(name),
                branch: None,
                status: WorktreeStatus::Invalid,
            });
        }
    }

    out.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    out
}

// ---------------------------------------------------------------------------------------
// Feature 016 — existing-branch situations, candidates, and creation modes.
// Contracts: `contracts/branch-conflict.md`, `contracts/branch-picker.md`.
// ---------------------------------------------------------------------------------------

/// Where a candidate branch lives (feature 016, FR-011).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BranchOrigin {
    /// `refs/heads/<name>` exists.
    Local,
    /// Only `refs/remotes/<remote>/<name>` exists.
    Remote { remote: String },
}

/// Why a branch cannot back a new worktree (feature 016, FR-021).
///
/// Two variants rather than one path-carrying variant so the UI can phrase the project-root case
/// in the user's language instead of showing them the repository path as if it were a worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    /// Checked out in another worktree, at this path.
    CheckedOutAt { path: PathBuf },
    /// Checked out as the repository's own current branch.
    CheckedOutInProjectRoot,
}

/// One row of the existing-branch picker (feature 016, FR-010–FR-012).
#[derive(Debug, Clone, PartialEq, Eq)]
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
        match &self.blocked_by {
            Some(BlockReason::CheckedOutInProjectRoot) => {
                f.write_str(" · in use by the project checkout")
            }
            Some(BlockReason::CheckedOutAt { path }) => {
                let holder = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                write!(f, " · in use by {holder}")
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
fn checked_out_branches(records: &[WorktreeRecord], repo: &Path) -> Vec<(String, BlockReason)> {
    records
        .iter()
        .filter_map(|rec| {
            let branch = rec.branch.clone()?;
            let reason = if rec.path == repo {
                BlockReason::CheckedOutInProjectRoot
            } else {
                BlockReason::CheckedOutAt {
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
    if let Some((_, reason)) = checked_out_branches(&records, repo)
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
pub fn branch_candidates(git: &dyn Git, repo: &Path) -> io::Result<Vec<BranchCandidate>> {
    let refs = git.list_branch_refs(repo)?;
    let porcelain = git.worktree_list_porcelain(repo)?;
    let held = checked_out_branches(&parse_worktrees(&porcelain), repo);

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

/// Remove the worktree's working directory once git has released it (feature 008, FR-023a).
///
/// `git worktree remove` deletes the working directory itself, so this normally finds nothing
/// left — that is the ordinary success path, not a failure. Reporting `NotFound` made every
/// successful delete raise "its folder could not be removed: No such file or directory"
/// naming a path that no longer existed (BUG-001). Any other error kind means the directory
/// really did survive and is still worth telling the user about (FR-023).
pub fn remove_worktree_dir(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// The named stage a worktree creation is (or was, on failure) currently in (feature 013,
/// FR-006/FR-007/FR-009). A closed enum (Principle V) so the progress display can never show a
/// stage that isn't a real, reachable step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub fn create_worktree(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    names: &DerivedNames,
    target_exists: bool,
    mode: &CreateMode,
    on_progress: &mut dyn FnMut(CreateProgressEvent),
) -> Result<Worktree, CreateError> {
    // Pre-flight (fail fast, no mutation). Re-run here rather than trusting whatever the caller
    // observed before prompting: the user's answer is separated from the act by think-time, in
    // which another terminal can create, delete, or check out the branch (feature 016, FR-009).
    on_progress(CreateProgressEvent {
        stage: CreateStage::PreflightCheck,
        line: "Checking for naming conflicts…".to_string(),
    });
    let situation = preflight(git, repo, target_path, &names.branch, target_exists)
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
