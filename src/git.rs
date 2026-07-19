//! Git I/O boundary (FR-001a, FR-006, FR-009, FR-018).
//!
//! Mirrors the [`crate::fs_scan::FolderScanner`] pattern: a trait for all git side effects,
//! a thin production impl ([`GitCli`], shelling out to the user's `git` binary), and an
//! in-memory [`FakeGit`] so the pure create/rollback/discovery orchestration is unit-testable
//! without a real repository (Constitution Principle I). Research R7 explains why the CLI is
//! used rather than libgit2/gitoxide. Contract: `contracts/git-trait.md`.

use std::io;
use std::path::Path;
use std::process::Command;

/// All git side effects the feature needs. See `contracts/git-trait.md`.
pub trait Git {
    /// Whether `dir` is the ROOT of a git repository — the open-project gate (FR-001a).
    /// Stricter than a `.git` existence check: uses `git rev-parse --show-toplevel` and
    /// compares it to `dir`.
    fn is_repo_root(&self, dir: &Path) -> bool;

    /// Whether a local branch already exists (FR-009).
    fn branch_exists(&self, repo: &Path, branch: &str) -> io::Result<bool>;

    /// Raw `git worktree list --porcelain` output for the pure parser (FR-018).
    fn worktree_list_porcelain(&self, repo: &Path) -> io::Result<String>;

    /// Create `branch` at HEAD and add a worktree bound to it at `path`, in one step
    /// (`git worktree add -b <branch> <path> HEAD`) (FR-006).
    fn worktree_add_new_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()>;

    /// Remove a worktree registration (and its working dir when `force`) (FR-006b).
    fn worktree_remove(&self, repo: &Path, path: &Path, force: bool) -> io::Result<()>;

    /// Prune stale worktree admin records (FR-006b).
    fn worktree_prune(&self, repo: &Path) -> io::Result<()>;

    /// Delete a local branch (FR-006b). Idempotent: absence is not an error.
    fn branch_delete(&self, repo: &Path, branch: &str) -> io::Result<()>;

    /// Whether the checked-out tree at `worktree_path` declares any submodules (a
    /// `.gitmodules` file is present at its root) — checked against the worktree's own
    /// checkout, not the source repo (feature 010, contracts/git-trait-submodules.md).
    fn has_submodules(&self, worktree_path: &Path) -> bool;

    /// Initialize, fetch, and check out every submodule declared under `worktree_path`,
    /// recursively (submodules of submodules included):
    /// `git -C <worktree_path> submodule update --init --recursive` (feature 010).
    ///
    /// `on_line` is called with each line of git's own stdout/stderr as it is produced, so a
    /// slow fetch is visible to the user in real time instead of appearing to hang (feature
    /// 010 follow-up).
    fn submodule_update_init_recursive(
        &self,
        worktree_path: &Path,
        on_line: &mut dyn FnMut(String),
    ) -> io::Result<()>;
}

/// Production [`Git`] backed by the user's `git` binary (research R7). Cross-platform via the
/// git CLI (Constitution Principle VI); each method is a thin `std::process::Command`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GitCli;

impl GitCli {
    /// Create a git-CLI boundary.
    pub fn new() -> Self {
        Self
    }
}

/// Run `git -C <repo> <args...>`, returning stdout on success or an `io::Error` carrying
/// stderr on a non-zero exit.
fn run_git(repo: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )))
    }
}

impl Git for GitCli {
    fn is_repo_root(&self, dir: &Path) -> bool {
        let Ok(out) = run_git(dir, &["rev-parse", "--show-toplevel"]) else {
            return false;
        };
        let toplevel = out.trim();
        // Compare canonically so trailing slashes / symlinks don't cause false negatives.
        match (std::fs::canonicalize(toplevel), std::fs::canonicalize(dir)) {
            (Ok(a), Ok(b)) => a == b,
            _ => Path::new(toplevel) == dir,
        }
    }

    fn branch_exists(&self, repo: &Path, branch: &str) -> io::Result<bool> {
        let refname = format!("refs/heads/{branch}");
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["show-ref", "--verify", "--quiet", &refname])
            .output()?;
        Ok(output.status.success())
    }

    fn worktree_list_porcelain(&self, repo: &Path) -> io::Result<String> {
        run_git(repo, &["worktree", "list", "--porcelain"])
    }

    fn worktree_add_new_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()> {
        let path = path.to_string_lossy();
        run_git(repo, &["worktree", "add", "-b", branch, &path, "HEAD"]).map(|_| ())
    }

    fn worktree_remove(&self, repo: &Path, path: &Path, force: bool) -> io::Result<()> {
        let path = path.to_string_lossy();
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(&path);
        // Idempotent: a missing worktree is not a failure for rollback.
        let _ = run_git(repo, &args);
        Ok(())
    }

    fn worktree_prune(&self, repo: &Path) -> io::Result<()> {
        run_git(repo, &["worktree", "prune"]).map(|_| ())
    }

    fn branch_delete(&self, repo: &Path, branch: &str) -> io::Result<()> {
        // Idempotent: "branch not found" is fine during rollback.
        let _ = run_git(repo, &["branch", "-D", branch]);
        Ok(())
    }

    fn has_submodules(&self, worktree_path: &Path) -> bool {
        worktree_path.join(".gitmodules").is_file()
    }

    fn submodule_update_init_recursive(
        &self,
        worktree_path: &Path,
        on_line: &mut dyn FnMut(String),
    ) -> io::Result<()> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;
        use std::sync::mpsc;

        let mut child = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .args(["submodule", "update", "--init", "--recursive"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // git interleaves progress/status lines across stdout and stderr; stream both live
        // (rather than buffering with `Command::output()` until the whole fetch finishes) so
        // a slow clone is visible line-by-line as it happens.
        let stdout = child
            .stdout
            .take()
            .expect("child spawned with piped stdout");
        let stderr = child
            .stderr
            .take()
            .expect("child spawned with piped stderr");
        let (tx, rx) = mpsc::channel::<String>();

        let tx_stdout = tx.clone();
        let stdout_handle = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx_stdout.send(line).is_err() {
                    break;
                }
            }
        });
        let stderr_handle = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });

        // Both senders above were moved into the reader threads, so this loop ends once both
        // threads finish (all output drained) and their senders drop.
        for line in rx {
            on_line(line);
        }
        let _ = stdout_handle.join();
        let _ = stderr_handle.join();

        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(
                "git submodule update --init --recursive failed",
            ))
        }
    }
}

// ---------------------------------------------------------------------------------------
// In-memory fake for unit tests. Public (not `#[cfg(test)]`) so integration tests in
// `tests/` can share it. Pure — no subprocess, deterministic.
// ---------------------------------------------------------------------------------------

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// An in-memory [`Git`] for tests: tracks repos, branches, and worktrees, and can be primed
/// to fail `worktree_add_new_branch` to exercise rollback (contract git-trait.md).
#[derive(Debug, Default)]
pub struct FakeGit {
    inner: RefCell<FakeState>,
}

#[derive(Debug, Default)]
struct FakeState {
    repos: BTreeSet<PathBuf>,
    branches: BTreeMap<PathBuf, BTreeSet<String>>,
    /// repo -> list of (worktree path, branch).
    worktrees: BTreeMap<PathBuf, Vec<(PathBuf, String)>>,
    /// When true, the next `worktree_add_new_branch` fails (after creating the branch, to
    /// mimic a checkout failure) so rollback ordering can be asserted.
    fail_next_add: bool,
    /// Worktree paths primed to report `has_submodules == true` (feature 010).
    submodules: BTreeSet<PathBuf>,
    /// When true, the next `submodule_update_init_recursive` call fails (feature 010).
    fail_next_submodule_update: bool,
    /// Log of paths passed to `submodule_update_init_recursive`, in call order (test
    /// assertions, feature 010).
    submodule_update_calls: Vec<PathBuf>,
    /// Canned lines fed to `on_line` on the next `submodule_update_init_recursive` call, to
    /// exercise the progress-streaming plumbing without a real subprocess (feature 010
    /// follow-up).
    submodule_progress_lines: Vec<String>,
}

impl FakeGit {
    /// A fake with no repos.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `repo` as an existing git repository root.
    pub fn with_repo(self, repo: impl Into<PathBuf>) -> Self {
        self.inner.borrow_mut().repos.insert(repo.into());
        self
    }

    /// Pre-existing branch (for duplicate-detection tests).
    pub fn with_branch(self, repo: impl Into<PathBuf>, branch: &str) -> Self {
        self.inner
            .borrow_mut()
            .branches
            .entry(repo.into())
            .or_default()
            .insert(branch.to_string());
        self
    }

    /// Prime the next `worktree_add_new_branch` to fail (rollback tests).
    pub fn failing_next_add(self) -> Self {
        self.inner.borrow_mut().fail_next_add = true;
        self
    }

    /// Prime `worktree_path` to report `has_submodules == true` (feature 010).
    pub fn with_submodules(self, worktree_path: impl Into<PathBuf>) -> Self {
        self.inner
            .borrow_mut()
            .submodules
            .insert(worktree_path.into());
        self
    }

    /// Prime the next `submodule_update_init_recursive` call to fail (rollback tests,
    /// feature 010).
    pub fn failing_next_submodule_update(self) -> Self {
        self.inner.borrow_mut().fail_next_submodule_update = true;
        self
    }

    /// Prime the next `submodule_update_init_recursive` call to report `lines` via `on_line`,
    /// in order, before returning (feature 010 follow-up — exercises progress streaming).
    pub fn with_submodule_progress_lines(self, lines: Vec<String>) -> Self {
        self.inner.borrow_mut().submodule_progress_lines = lines;
        self
    }

    /// Snapshot of paths passed to `submodule_update_init_recursive`, in call order (test
    /// assertions, feature 010).
    pub fn submodule_update_calls(&self) -> Vec<PathBuf> {
        self.inner.borrow().submodule_update_calls.clone()
    }

    /// Snapshot the branches known for a repo (test assertions).
    pub fn branches(&self, repo: &Path) -> Vec<String> {
        self.inner
            .borrow()
            .branches
            .get(repo)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Snapshot the worktree (path, branch) pairs for a repo (test assertions).
    pub fn worktrees(&self, repo: &Path) -> Vec<(PathBuf, String)> {
        self.inner
            .borrow()
            .worktrees
            .get(repo)
            .cloned()
            .unwrap_or_default()
    }
}

impl Git for FakeGit {
    fn is_repo_root(&self, dir: &Path) -> bool {
        self.inner.borrow().repos.contains(dir)
    }

    fn branch_exists(&self, repo: &Path, branch: &str) -> io::Result<bool> {
        Ok(self
            .inner
            .borrow()
            .branches
            .get(repo)
            .is_some_and(|b| b.contains(branch)))
    }

    fn worktree_list_porcelain(&self, repo: &Path) -> io::Result<String> {
        let state = self.inner.borrow();
        let mut out = String::new();
        if let Some(list) = state.worktrees.get(repo) {
            for (path, branch) in list {
                out.push_str(&format!("worktree {}\n", path.display()));
                out.push_str("HEAD 0000000000000000000000000000000000000000\n");
                out.push_str(&format!("branch refs/heads/{branch}\n\n"));
            }
        }
        Ok(out)
    }

    fn worktree_add_new_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()> {
        let mut state = self.inner.borrow_mut();
        // Mimic git: the branch is created first.
        state
            .branches
            .entry(repo.to_path_buf())
            .or_default()
            .insert(branch.to_string());
        if state.fail_next_add {
            state.fail_next_add = false;
            // Branch created but worktree registration/checkout "failed" — leaves the branch
            // behind, which rollback must clean up.
            return Err(io::Error::other("simulated worktree add failure"));
        }
        state
            .worktrees
            .entry(repo.to_path_buf())
            .or_default()
            .push((path.to_path_buf(), branch.to_string()));
        Ok(())
    }

    fn worktree_remove(&self, repo: &Path, path: &Path, _force: bool) -> io::Result<()> {
        if let Some(list) = self.inner.borrow_mut().worktrees.get_mut(repo) {
            list.retain(|(p, _)| p != path);
        }
        Ok(())
    }

    fn worktree_prune(&self, _repo: &Path) -> io::Result<()> {
        Ok(())
    }

    fn branch_delete(&self, repo: &Path, branch: &str) -> io::Result<()> {
        if let Some(b) = self.inner.borrow_mut().branches.get_mut(repo) {
            b.remove(branch);
        }
        Ok(())
    }

    fn has_submodules(&self, worktree_path: &Path) -> bool {
        self.inner.borrow().submodules.contains(worktree_path)
    }

    fn submodule_update_init_recursive(
        &self,
        worktree_path: &Path,
        on_line: &mut dyn FnMut(String),
    ) -> io::Result<()> {
        let mut state = self.inner.borrow_mut();
        state
            .submodule_update_calls
            .push(worktree_path.to_path_buf());
        let lines = std::mem::take(&mut state.submodule_progress_lines);
        let should_fail = state.fail_next_submodule_update;
        state.fail_next_submodule_update = false;
        drop(state);

        for line in lines {
            on_line(line);
        }
        if should_fail {
            Err(io::Error::other("simulated submodule update failure"))
        } else {
            Ok(())
        }
    }
}
