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

/// Remove a worktree and its branch, app-owned (feature 008, FR-020). Runs the git steps in
/// [`CleanupStep`] order — `worktree_remove(force)` → `worktree_prune` → `branch_delete` — so
/// the checked-out branch can be deleted after its registration is gone. Every step is
/// idempotent (an already-missing worktree/branch is not an error), so a partially-removed
/// worktree still resolves to a consistent state (FR-023). The caller removes the directory
/// (`fs`) and terminates the worktree's session processes.
pub fn remove_worktree(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    branch: Option<&str>,
) -> io::Result<()> {
    git.worktree_remove(repo, target_path, true)?;
    git.worktree_prune(repo)?;
    if let Some(branch) = branch {
        git.branch_delete(repo, branch)?;
    }
    Ok(())
}

/// Create a branch + worktree from derived names, rolling back on failure (FR-006/006b/009).
///
/// Pre-flight duplicate checks run before any mutation. `target_exists` is the binary's `fs`
/// check that the target directory is already present/non-empty. On a git failure the
/// [`rollback_plan`] git steps run (the caller removes the directory — [`CleanupStep::RemoveDir`]).
pub fn create_worktree(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    names: &DerivedNames,
    target_exists: bool,
) -> Result<Worktree, CreateError> {
    // Pre-flight (fail fast, no mutation).
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

    if let Err(e) = git.worktree_add_new_branch(repo, &names.branch, target_path) {
        run_rollback(git, repo, target_path, &names.branch);
        return Err(CreateError::RolledBack(e.to_string()));
    }

    // Submodules, if any, are fetched from the worktree's own checkout (feature 010,
    // research R1) — a failure here rolls back the whole creation exactly like a failed
    // `worktree_add_new_branch` above (spec FR-005), via the same rollback plan.
    if git.has_submodules(target_path) {
        if let Err(e) = git.submodule_update_init_recursive(target_path) {
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
