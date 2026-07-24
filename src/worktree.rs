//! Worktree domain model, git-porcelain discovery, and create + rollback orchestration.
//!
//! Pure over the [`crate::git::Git`] boundary (no direct subprocess/`fs`), so the whole
//! create-then-rollback flow and the discovery/classification logic are unit-testable with
//! [`crate::git::FakeGit`] (Constitution Principle I). Contracts: `git-trait.md`, `naming.md`.

use crate::git::Git;
use crate::naming::DerivedNames;
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

/// Why creating a worktree failed (FR-006b, FR-009). Carries `String` messages (not
/// `io::Error`) so it is `Clone`/`Eq` and can live in application state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateError {
    /// A worktree with the derived directory name already exists (FR-009).
    DuplicateDir,
    /// A branch with the derived name already exists (FR-009).
    DuplicateBranch,
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

/// The ordered rollback plan for a failed create (FR-006b). Order matters: the worktree
/// registration is removed before the branch is deleted (git refuses to delete a checked-out
/// branch).
pub fn rollback_plan() -> [CleanupStep; 4] {
    [
        CleanupStep::WorktreeRemove,
        CleanupStep::WorktreePrune,
        CleanupStep::BranchDelete,
        CleanupStep::RemoveDir,
    ]
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
    pub fn label(&self) -> &'static str {
        match self {
            Self::PreflightCheck => "Checking for naming conflicts",
            Self::CreatingWorktree => "Creating branch and worktree",
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
    on_progress: &mut dyn FnMut(CreateProgressEvent),
) -> Result<Worktree, CreateError> {
    // Pre-flight (fail fast, no mutation).
    on_progress(CreateProgressEvent {
        stage: CreateStage::PreflightCheck,
        line: "Checking for naming conflicts…".to_string(),
    });
    if git
        .branch_exists(repo, &names.branch)
        .map_err(|e| CreateError::RolledBack(e.to_string()))?
    {
        return Err(CreateError::DuplicateBranch);
    }
    let porcelain = git
        .worktree_list_porcelain(repo)
        .map_err(|e| CreateError::RolledBack(e.to_string()))?;
    let registered = parse_worktrees(&porcelain);
    if registered.iter().any(|r| r.path == target_path) || target_exists {
        return Err(CreateError::DuplicateDir);
    }

    on_progress(CreateProgressEvent {
        stage: CreateStage::CreatingWorktree,
        line: format!(
            "$ git worktree add -b {} {} HEAD",
            names.branch,
            target_path.display()
        ),
    });
    if let Err(e) = git.worktree_add_new_branch(repo, &names.branch, target_path) {
        on_progress(CreateProgressEvent {
            stage: CreateStage::CreatingWorktree,
            line: format!("worktree add failed: {e}"),
        });
        on_progress(CreateProgressEvent {
            stage: CreateStage::RollingBack,
            line: "Rolling back…".to_string(),
        });
        run_rollback(git, repo, target_path, &names.branch);
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
            run_rollback(git, repo, target_path, &names.branch);
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

/// Run the git steps of the rollback plan in order (RemoveDir is the caller's — [`CleanupStep::RemoveDir`]).
fn run_rollback(git: &dyn Git, repo: &Path, target_path: &Path, branch: &str) {
    for step in rollback_plan() {
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
