# Contract: `Git` I/O Boundary — Submodule Extension

**Feature**: 010-submodule-worktree-support | `src/git.rs`, extending
[005's `git-trait.md`](../../005-worktree-session-terminal/contracts/git-trait.md).

Adds two methods to the existing `Git` trait so the worktree create/rollback orchestration in
`src/worktree.rs` stays pure over the trait (no direct subprocess/fs), unit-testable against
`FakeGit` (Constitution Principle I) — same pattern as every existing `Git` method.

## Trait additions

```rust
pub trait Git {
    // ... existing methods (is_repo_root, branch_exists, worktree_list_porcelain,
    // worktree_add_new_branch, worktree_remove, worktree_prune, branch_delete) unchanged ...

    /// Whether the checked-out tree at `worktree_path` declares any submodules — a
    /// `.gitmodules` file is present at its root. Checked against the *worktree's own*
    /// checkout (not the source repo), since that's the tree that was actually checked out
    /// and is what needs its submodules populated (research R1).
    fn has_submodules(&self, worktree_path: &Path) -> bool;

    /// Initialize, fetch, and check out every submodule declared under `worktree_path`,
    /// recursively (submodules of submodules included):
    /// `git -C <worktree_path> submodule update --init --recursive` (research R2).
    /// Returns `Err` with git's own stderr text on any failure (network, auth, unreachable
    /// commit, etc.) — the caller does not re-classify the reason (research R3).
    fn submodule_update_init_recursive(&self, worktree_path: &Path) -> io::Result<()>;
}
```

## Updated create orchestration (pure over `Git` + `fs`)

`create_worktree(git, repo, target_path, names, target_exists) -> Result<Worktree, CreateError>`:

1. **Pre-flight (fail fast, no mutation)** — unchanged: `branch_exists` → `DuplicateBranch`;
   dir already registered/non-empty → `DuplicateDir`.
2. `worktree_add_new_branch(repo, branch, path)`.
3. **On failure in 2**: run the rollback plan (unchanged) → `CreateError::RolledBack`.
4. **On success in 2**: `has_submodules(target_path)`.
   - `false` → skip straight to step 6 (no git invocation, no delay — FR-003).
   - `true` → `submodule_update_init_recursive(target_path)`.
5. **On failure in 4 (submodule step)**: run the **same** rollback plan as step 3 —
   `worktree_remove(force) → worktree_prune → branch_delete → remove target dir` — and return
   `CreateError::RolledBack(<git stderr from the submodule step>)`. No new `CleanupStep`
   variant; no partial/kept worktree is ever left behind (spec FR-005).
6. On success (with or without submodules) return `Worktree { status: Valid, .. }`.

`CreateError` is unchanged in shape: `DuplicateDir | DuplicateBranch | RolledBack(String)`. A
submodule-step failure and a worktree-add failure are indistinguishable at the type level by
design (research R3) — both are "the git step failed, everything was rolled back," and the
wrapped string already carries which step and why.

## `FakeGit` (tests)

Extends the existing in-memory `FakeGit`:

- A per-repo-path `submodules: bool` flag (default `false`), settable via a builder method
  (e.g. `.with_submodules(worktree_path)`) so tests can construct a "repo whose worktrees have
  submodules" scenario without a real `.gitmodules` file.
- `has_submodules` reads that flag for the given path.
- A primeable failure switch for `submodule_update_init_recursive` (mirrors the existing
  `failing_next_add` pattern used to exercise `worktree_add_new_branch` rollback), so the
  submodule-failure → rollback path is exercised without a real network or a real failing
  submodule.
- No subprocess, deterministic, matches the existing `FakeGit` contract for every other method.

## Constitution mapping

Principle III (submodule population is automatic — no manual `git submodule` step required of
the user), I (trait + fake → TDD, rollback path unit-tested without a real repo or network),
VI (git CLI is the same cross-platform mechanism already used for every other `Git` method),
V (submodule failure stays representable only via the existing typed `CreateError`, not a new
ad hoc error path).
