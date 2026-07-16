# Contract: `Git` I/O Boundary

**Feature**: 005-worktree-session-terminal | `src/git.rs`.

Mirrors the existing `FolderScanner`/`ProjectStore` pattern: a trait for all git side effects,
with a thin production impl (`GitCli`, `std::process::Command` over the user's `git` binary,
research R7) and an in-memory `FakeGit` for tests. Orchestration logic is generic over `Git`
so create + rollback + discovery are unit-tested with no real repo.

## Trait

```rust
pub trait Git {
    /// True if `dir` is the ROOT of a git repository (git rev-parse --show-toplevel == dir).
    /// Used as the open-project gate (FR-001a) — stricter than a `.git` existence check.
    fn is_repo_root(&self, dir: &Path) -> bool;

    /// Does a local branch exist? (show-ref --verify --quiet refs/heads/<branch>) — FR-009.
    fn branch_exists(&self, repo: &Path, branch: &str) -> io::Result<bool>;

    /// Raw `git worktree list --porcelain -z` output for the pure parser (FR-018/018a).
    fn worktree_list_porcelain(&self, repo: &Path) -> io::Result<String>;

    /// Create branch @HEAD + add a worktree bound to it, in one step (FR-006):
    /// `git -C <repo> worktree add -b <branch> <path> HEAD`.
    fn worktree_add_new_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()>;

    // Rollback primitives — each idempotent; ignore "not found" (FR-006b).
    fn worktree_remove(&self, repo: &Path, path: &Path, force: bool) -> io::Result<()>;
    fn worktree_prune(&self, repo: &Path) -> io::Result<()>;
    fn branch_delete(&self, repo: &Path, branch: &str) -> io::Result<()>;
}
```

## Create orchestration (pure over `Git` + `fs`)

`create_worktree(git, repo, DerivedNames) -> Result<Worktree, CreateError>`:

1. **Pre-flight (fail fast, no mutation)**: `branch_exists` → `DuplicateBranch`; parse
   `worktree_list_porcelain` → dir already registered → `DuplicateDir`; target dir already
   non-empty → `DuplicateDir`.
2. `create_dir_all(.claude/worktrees/)` (record if we created the parent).
3. `worktree_add_new_branch(repo, branch, path)`.
4. **On any failure in 2–3**, run the rollback plan (ordered, testable as data):
   `worktree_remove(force) → worktree_prune → branch_delete → remove target dir if present`.
   Registration is removed BEFORE branch deletion (git refuses to delete a checked-out branch).
5. On success return the `Worktree { status: Valid, .. }`.

`CreateError`: `DuplicateDir | DuplicateBranch | Git(io::Error) | RolledBack(io::Error)`.

## Discovery + classification (pure)

`parse_worktrees(porcelain: &str) -> Vec<WorktreeRecord>` — pure parser over the stable
porcelain format (`worktree <path>` / `branch refs/heads/<name>` / `prunable <reason>` blocks).

`classify(record, dir_exists: bool) -> WorktreeStatus`:
- listed + dir exists → `Valid`
- listed + dir missing (or `prunable`) → `Missing`
- dir under `.claude/worktrees/` but not listed → `Invalid` (orphan)

The binary supplies `dir_exists` via `fs`; classification stays pure and unit-tested against
captured porcelain fixtures.

## FakeGit (tests)

In-memory maps repo → branches, repo → worktrees. `worktree_add_new_branch` mutates the maps
and can be primed to fail at a chosen step to exercise the rollback ordering.
`worktree_list_porcelain` returns a canned string. No subprocess, deterministic.

## Constitution mapping

Principle III (app owns create/switch, no manual git), I (trait + fake → TDD),
VI (git CLI is cross-platform), V (typed errors/states).
