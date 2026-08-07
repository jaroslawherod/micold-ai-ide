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

    /// Every local and remote-tracking branch ref, one full refname per line, for
    /// [`crate::worktree::parse_branch_refs`] (feature 016, FR-011).
    ///
    /// Reads local ref storage only — this MUST NOT contact a remote (FR-020, Constitution
    /// Principle IV). Contract: `contracts/git-trait-branches.md`.
    fn list_branch_refs(&self, repo: &Path) -> io::Result<String>;

    /// Check an EXISTING local `branch` out into a new worktree at `path`
    /// (`git worktree add <path> <branch>`) (feature 016, FR-004).
    ///
    /// Must not create, move, reset, or delete the branch: its tip is identical before and
    /// after, on success and on failure alike.
    fn worktree_add_existing_branch(
        &self,
        repo: &Path,
        branch: &str,
        path: &Path,
    ) -> io::Result<()>;

    /// Create-or-reset `branch` to HEAD and check it out at `path`
    /// (`git worktree add -B <branch> <path> HEAD`) (feature 016, FR-006).
    ///
    /// **Destructive**: discards whatever `branch` pointed at. Only ever called after the
    /// explicit confirmation FR-005 requires.
    fn worktree_add_reset_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()>;

    /// Create local `branch` at `<remote>/<branch>`, set it to track that ref, and check it out
    /// at `path` (`git worktree add --track -b <branch> <path> <remote>/<branch>`) (feature 016,
    /// FR-017).
    ///
    /// The remote is named explicitly — never inferred via git's DWIM/`--guess-remote` — so a
    /// name present on several remotes has no ambiguity to resolve (research R4). Reads the
    /// remote-tracking ref already on disk; does not contact the remote (FR-020).
    fn worktree_add_tracking_branch(
        &self,
        repo: &Path,
        branch: &str,
        remote: &str,
        path: &Path,
    ) -> io::Result<()>;

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

/// Whether two paths name the same location, resolving symlinks when both sides exist.
///
/// `git worktree list --porcelain` reports the fully resolved real path, while the app builds
/// its paths from the project root as the user opened it. When that root traverses a symlink
/// (a symlinked checkout, a symlinked home, `/tmp` on macOS) a raw `==` never matches. Falls
/// back to a literal comparison when either side cannot be canonicalized — the "already gone"
/// case, where the path no longer exists on disk. Mirrors [`GitCli::is_repo_root`].
fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
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

    fn list_branch_refs(&self, repo: &Path) -> io::Result<String> {
        // `for-each-ref` rather than `branch -a`: stable machine-readable output, no `*` marker
        // and no `HEAD ->` decoration to strip (research R6). Local ref storage only — no
        // network (FR-020).
        run_git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/heads",
                "refs/remotes",
            ],
        )
    }

    fn worktree_add_existing_branch(
        &self,
        repo: &Path,
        branch: &str,
        path: &Path,
    ) -> io::Result<()> {
        let path = path.to_string_lossy();
        // Positional branch, no `-b`: git checks the existing branch out and refuses if it is
        // already checked out elsewhere — the backstop behind pre-flight (research R2).
        run_git(repo, &["worktree", "add", &path, branch]).map(|_| ())
    }

    fn worktree_add_reset_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()> {
        let path = path.to_string_lossy();
        // `-B` = create-or-reset then check out, in one command, so there is never a window in
        // which the old branch is gone and no worktree exists yet (research R3).
        run_git(repo, &["worktree", "add", "-B", branch, &path, "HEAD"]).map(|_| ())
    }

    fn worktree_add_tracking_branch(
        &self,
        repo: &Path,
        branch: &str,
        remote: &str,
        path: &Path,
    ) -> io::Result<()> {
        let path = path.to_string_lossy();
        let start_point = format!("{remote}/{branch}");
        run_git(
            repo,
            &[
                "worktree",
                "add",
                "--track",
                "-b",
                branch,
                &path,
                &start_point,
            ],
        )
        .map(|_| ())
    }

    fn worktree_remove(&self, repo: &Path, path: &Path, force: bool) -> io::Result<()> {
        let path_arg = path.to_string_lossy();
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(&path_arg);
        let Err(err) = run_git(repo, &args) else {
            return Ok(());
        };
        // Idempotent, but not silent (feature 008, FR-023b). This previously discarded every
        // result with `let _ = ...` so that rollback and the missing-worktree edge case stayed
        // quiet — which also swallowed real failures like a locked worktree, reporting success
        // while the worktree survived on disk (BUG-001).
        //
        // Distinguish the two by outcome rather than by parsing git's message: prune the stale
        // registrations git itself would drop, then ask whether this path is still registered.
        // Gone => the caller got what it asked for. Still there => a genuine refusal.
        //
        // The prune is only a diagnostic step, so its own failure must not replace the reason
        // the removal failed — fall through to reporting `err`.
        let still_registered = self
            .worktree_prune(repo)
            .and_then(|()| self.worktree_list_porcelain(repo))
            .map(|list| {
                crate::worktree::parse_worktrees(&list)
                    .iter()
                    .any(|r| same_path(&r.path, path))
            })
            // Can't tell (not a repo, git missing) — surface the original failure rather than
            // inventing a success.
            .unwrap_or(true);
        if still_registered {
            Err(err)
        } else {
            Ok(())
        }
    }

    fn worktree_prune(&self, repo: &Path) -> io::Result<()> {
        run_git(repo, &["worktree", "prune"]).map(|_| ())
    }

    fn branch_delete(&self, repo: &Path, branch: &str) -> io::Result<()> {
        // Outcome-based, like `worktree_remove` above (BUG-001): don't trust/parse git's own
        // exit status or message, ask git directly whether the branch is actually gone
        // afterward. Idempotent ("branch not found" is fine during rollback), but a branch that
        // demonstrably still exists (e.g. checked out elsewhere) is a genuine, reportable
        // refusal (feature 013, FR-015) — not silently swallowed.
        let attempt = run_git(repo, &["branch", "-D", branch]);
        match self.branch_exists(repo, branch) {
            Ok(false) => Ok(()),
            Ok(true) => Err(attempt.err().unwrap_or_else(|| {
                io::Error::other(format!("branch '{branch}' could not be deleted"))
            })),
            // Can't tell — surface rather than assume success.
            Err(e) => Err(e),
        }
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
        //
        // The lines are *also* retained here, not merely forwarded (FR-006/SC-003, found by
        // `010-submodule-worktree-support` T023's manual validation). Streaming them to `on_line`
        // makes them visible *while* the fetch runs; on failure they are the only place that says
        // which submodule failed and why — `git submodule update` reports both on its own output
        // and communicates nothing through its exit status but "something went wrong". Without
        // this, every failure reached the user as one fixed sentence: the progress had scrolled
        // past, and the error carried no words of git's own.
        let mut retained: Vec<String> = Vec::new();
        for line in rx {
            if retained.len() < FAILURE_CONTEXT_LINES {
                retained.push(line.clone());
            }
            on_line(line);
        }
        let _ = stdout_handle.join();
        let _ = stderr_handle.join();

        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(failure_message(&retained)))
        }
    }
}

/// How many of a failed fetch's output lines are retained for its error message. Generous, because
/// [`failure_message`] discards most of them again — this only has to be wide enough to still hold
/// the interesting ones after git's own repetition.
const FAILURE_CONTEXT_LINES: usize = 12;

/// How many *distinct* lines survive into the message. `git submodule update` retries a failed
/// clone once, so the same two `fatal:` lines arrive twice; after deduplication three or four lines
/// carry everything the user needs — the submodule path and the reason — and this is a message
/// rendered in a dialog, not a log.
const FAILURE_MESSAGE_LINES: usize = 4;

/// Build the failure message for a `git submodule update` that exited non-zero, from the output it
/// produced. Falls back to the bare summary when git said nothing at all — a message with a dangling
/// colon and no content would be worse than the summary alone.
fn failure_message(lines: &[String]) -> String {
    const SUMMARY: &str = "git submodule update --init --recursive failed";
    let mut context: Vec<&str> = Vec::new();
    for line in lines {
        let line = line.trim();
        // Deduplicated, not just capped: git's automatic retry repeats each `fatal:` verbatim, and a
        // message that says the same thing twice reads as noise rather than as detail.
        if line.is_empty() || context.contains(&line) {
            continue;
        }
        context.push(line);
        if context.len() == FAILURE_MESSAGE_LINES {
            break;
        }
    }
    if context.is_empty() {
        return SUMMARY.to_string();
    }
    format!("{SUMMARY}: {}", context.join("; "))
}

#[cfg(test)]
mod failure_message_tests {
    use super::*;

    #[test]
    fn git_s_own_words_are_carried_into_the_message() {
        let msg = failure_message(&[
            "Cloning into '/w/vendor/broken'...".into(),
            "fatal: repository '/nope' does not exist".into(),
            "fatal: clone of '/nope' into submodule path '/w/vendor/broken' failed".into(),
        ]);
        assert!(msg.contains("vendor/broken"), "{msg}");
        assert!(msg.contains("does not exist"), "{msg}");
    }

    #[test]
    fn git_s_retry_does_not_double_the_message() {
        // `submodule update` retries a failed clone once and repeats both `fatal:` lines verbatim.
        let fatal = "fatal: repository '/nope' does not exist";
        let clone = "fatal: clone of '/nope' into submodule path '/w/vendor/broken' failed";
        let msg = failure_message(&[
            fatal.into(),
            clone.into(),
            "Failed to clone 'vendor/broken'. Retry scheduled".into(),
            fatal.into(),
            clone.into(),
            "Failed to clone 'vendor/broken' a second time, aborting".into(),
        ]);
        assert_eq!(msg.matches("does not exist").count(), 1, "{msg}");
        assert!(msg.contains("vendor/broken"), "{msg}");
    }

    #[test]
    fn a_silent_failure_falls_back_to_the_summary_alone() {
        // No dangling colon, no empty tail — the summary has to stand on its own.
        assert_eq!(
            failure_message(&[]),
            "git submodule update --init --recursive failed"
        );
        assert_eq!(
            failure_message(&["   ".into(), "".into()]),
            "git submodule update --init --recursive failed"
        );
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
    /// repo -> remote-tracking branches, stored as `<remote>/<branch>` (feature 016).
    remote_branches: BTreeMap<PathBuf, BTreeSet<String>>,
    /// repo -> local branch -> its upstream, as `<remote>/<branch>` (feature 016).
    upstreams: BTreeMap<PathBuf, BTreeMap<String, String>>,
    /// repo -> list of (worktree path, branch).
    worktrees: BTreeMap<PathBuf, Vec<(PathBuf, String)>>,
    /// Branches passed to `worktree_add_existing_branch`, in call order (feature 016 assertions).
    add_existing_calls: BTreeMap<PathBuf, Vec<String>>,
    /// Branches passed to `worktree_add_reset_branch`, in call order (feature 016 assertions).
    add_reset_calls: BTreeMap<PathBuf, Vec<String>>,
    /// When true, the next `worktree_add_new_branch` fails (after creating the branch, to
    /// mimic a checkout failure) so rollback ordering can be asserted.
    fail_next_add: bool,
    /// When true, the next `worktree_remove` fails, standing in for a locked worktree or a
    /// branch checked out elsewhere (feature 008, FR-023).
    fail_next_remove: bool,
    /// When true, the next `branch_delete` fails to actually remove the branch — a genuine
    /// refusal (feature 013, FR-015), as opposed to the ordinary idempotent "already absent"
    /// case.
    fail_next_branch_delete: bool,
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

    /// Pre-existing remote-tracking branch, as if a previous `git fetch` had brought it down
    /// (feature 016). Stored as `refs/remotes/<remote>/<branch>`.
    pub fn with_remote_branch(self, repo: impl Into<PathBuf>, remote: &str, branch: &str) -> Self {
        self.inner
            .borrow_mut()
            .remote_branches
            .entry(repo.into())
            .or_default()
            .insert(format!("{remote}/{branch}"));
        self
    }

    /// Pre-register a worktree bound to `branch` at `path` without going through a create
    /// (feature 016). Needed to stage the "branch already checked out" cases — including the
    /// project's own checkout, registered by passing the repo root as `path`.
    pub fn with_worktree(
        self,
        repo: impl Into<PathBuf>,
        path: impl Into<PathBuf>,
        branch: &str,
    ) -> Self {
        self.inner
            .borrow_mut()
            .worktrees
            .entry(repo.into())
            .or_default()
            .push((path.into(), branch.to_string()));
        self
    }

    /// Snapshot the remote-tracking branches known for a repo, as `<remote>/<branch>` (test
    /// assertions, feature 016 — used to prove no code path ever writes a remote ref).
    pub fn remote_branches(&self, repo: &Path) -> Vec<String> {
        self.inner
            .borrow()
            .remote_branches
            .get(repo)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// The upstream recorded for a local branch, as `<remote>/<branch>` (feature 016).
    pub fn upstream(&self, repo: &Path, branch: &str) -> Option<String> {
        self.inner
            .borrow()
            .upstreams
            .get(repo)
            .and_then(|m| m.get(branch))
            .cloned()
    }

    /// Branches passed to `worktree_add_existing_branch`, in call order (feature 016).
    pub fn add_existing_calls(&self, repo: &Path) -> Vec<String> {
        self.inner
            .borrow()
            .add_existing_calls
            .get(repo)
            .cloned()
            .unwrap_or_default()
    }

    /// Branches passed to `worktree_add_reset_branch`, in call order (feature 016).
    pub fn add_reset_calls(&self, repo: &Path) -> Vec<String> {
        self.inner
            .borrow()
            .add_reset_calls
            .get(repo)
            .cloned()
            .unwrap_or_default()
    }

    /// Prime the next `worktree_add_new_branch` to fail (rollback tests).
    pub fn failing_next_add(self) -> Self {
        self.inner.borrow_mut().fail_next_add = true;
        self
    }

    /// Prime the next `worktree_remove` to fail — a locked worktree, a branch checked out
    /// elsewhere, or a permission error (feature 008, FR-023 delete-failure reporting).
    pub fn failing_next_remove(self) -> Self {
        self.inner.borrow_mut().fail_next_remove = true;
        self
    }

    /// Prime the next `branch_delete` to genuinely fail — the branch is left registered rather
    /// than removed (feature 013, FR-015 delete-failure reporting).
    pub fn failing_next_branch_delete(self) -> Self {
        self.inner.borrow_mut().fail_next_branch_delete = true;
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

    fn list_branch_refs(&self, repo: &Path) -> io::Result<String> {
        let state = self.inner.borrow();
        let mut out = String::new();
        if let Some(branches) = state.branches.get(repo) {
            for b in branches {
                out.push_str(&format!("refs/heads/{b}\n"));
            }
        }
        if let Some(remotes) = state.remote_branches.get(repo) {
            // Emit the symbolic alias real git emits, so the parser's filtering is exercised
            // rather than assumed.
            let names: BTreeSet<&str> =
                remotes.iter().filter_map(|r| r.split('/').next()).collect();
            for remote in names {
                out.push_str(&format!("refs/remotes/{remote}/HEAD\n"));
            }
            for r in remotes {
                out.push_str(&format!("refs/remotes/{r}\n"));
            }
        }
        Ok(out)
    }

    fn worktree_add_existing_branch(
        &self,
        repo: &Path,
        branch: &str,
        path: &Path,
    ) -> io::Result<()> {
        let mut state = self.inner.borrow_mut();
        state
            .add_existing_calls
            .entry(repo.to_path_buf())
            .or_default()
            .push(branch.to_string());
        // Mirror git's refusals: the branch must exist and must not be checked out elsewhere.
        if !state.branches.get(repo).is_some_and(|b| b.contains(branch)) {
            return Err(io::Error::other(format!("invalid reference: {branch}")));
        }
        if state
            .worktrees
            .get(repo)
            .is_some_and(|list| list.iter().any(|(_, b)| b == branch))
        {
            return Err(io::Error::other(format!(
                "'{branch}' is already checked out"
            )));
        }
        if state.fail_next_add {
            state.fail_next_add = false;
            // Deliberately leaves the branch untouched — reuse never creates it, so a failure
            // here has nothing of its own to clean up (FR-008).
            return Err(io::Error::other("simulated worktree add failure"));
        }
        state
            .worktrees
            .entry(repo.to_path_buf())
            .or_default()
            .push((path.to_path_buf(), branch.to_string()));
        Ok(())
    }

    fn worktree_add_reset_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()> {
        let mut state = self.inner.borrow_mut();
        state
            .add_reset_calls
            .entry(repo.to_path_buf())
            .or_default()
            .push(branch.to_string());
        // Mimic `-B`: the branch is created/reset first, so a later checkout failure leaves it
        // in place — exactly what rollback must then clean up.
        state
            .branches
            .entry(repo.to_path_buf())
            .or_default()
            .insert(branch.to_string());
        if state.fail_next_add {
            state.fail_next_add = false;
            return Err(io::Error::other("simulated worktree add failure"));
        }
        state
            .worktrees
            .entry(repo.to_path_buf())
            .or_default()
            .push((path.to_path_buf(), branch.to_string()));
        Ok(())
    }

    fn worktree_add_tracking_branch(
        &self,
        repo: &Path,
        branch: &str,
        remote: &str,
        path: &Path,
    ) -> io::Result<()> {
        let mut state = self.inner.borrow_mut();
        let start_point = format!("{remote}/{branch}");
        if !state
            .remote_branches
            .get(repo)
            .is_some_and(|r| r.contains(&start_point))
        {
            return Err(io::Error::other(format!(
                "invalid reference: {start_point}"
            )));
        }
        state
            .branches
            .entry(repo.to_path_buf())
            .or_default()
            .insert(branch.to_string());
        state
            .upstreams
            .entry(repo.to_path_buf())
            .or_default()
            .insert(branch.to_string(), start_point);
        if state.fail_next_add {
            state.fail_next_add = false;
            // Local branch created, checkout failed — rollback deletes it (it is ours).
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
        {
            let mut state = self.inner.borrow_mut();
            if state.fail_next_remove {
                state.fail_next_remove = false;
                // Nothing is unregistered — the worktree survives, as it would on a real
                // locked-worktree failure.
                return Err(io::Error::other("simulated worktree remove failure"));
            }
        }
        if let Some(list) = self.inner.borrow_mut().worktrees.get_mut(repo) {
            list.retain(|(p, _)| p != path);
        }
        Ok(())
    }

    fn worktree_prune(&self, _repo: &Path) -> io::Result<()> {
        Ok(())
    }

    fn branch_delete(&self, repo: &Path, branch: &str) -> io::Result<()> {
        let mut state = self.inner.borrow_mut();
        if state.fail_next_branch_delete {
            state.fail_next_branch_delete = false;
            // Left registered — the branch really does survive, as it would on a genuine
            // real-git refusal.
            return Err(io::Error::other("simulated branch delete failure"));
        }
        if let Some(b) = state.branches.get_mut(repo) {
            b.remove(branch);
        }
        // A deleted branch takes its upstream config with it; the remote-tracking ref itself is
        // untouched (feature 016).
        if let Some(u) = state.upstreams.get_mut(repo) {
            u.remove(branch);
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
