# Contract: `Git` trait — existing-branch operations

Extends `specs/005-worktree-session-terminal/contracts/git-trait.md`. Every method below is
implemented by `GitCli` (thin `std::process::Command`) and by `FakeGit` (in-memory, deterministic,
no subprocess), so the whole create/resolve/rollback flow is unit-testable without a repository
(Constitution Principle I).

**Offline invariant (FR-020, Principle IV)**: no method here may contact a remote. No `fetch`, no
`ls-remote`, no `--guess-remote`. `refs/remotes` is read from local ref storage only. Any future
method that would violate this belongs behind an explicit, user-initiated action, not here.

---

## `list_branch_refs(&self, repo: &Path) -> io::Result<String>`

Raw ref listing for the pure parser.

- **Command**: `git -C <repo> for-each-ref --format=%(refname) refs/heads refs/remotes`
- **Returns**: newline-separated full refnames, e.g.
  ```
  refs/heads/main
  refs/heads/feat/login
  refs/remotes/origin/HEAD
  refs/remotes/origin/feat/reporting
  ```
- **Parsing** is *not* this method's job — `parse_branch_refs()` in `src/worktree.rs` owns it
  (research R6), so the mapping is testable from canned strings.
- **`FakeGit`**: primed by `.with_branch(repo, name)` (existing, → `refs/heads/…`) and a new
  `.with_remote_branch(repo, remote, name)` (→ `refs/remotes/<remote>/<name>`). Renders them in
  the same format as git.

### Parser rules (`parse_branch_refs`)

| Input line | Result |
|---|---|
| `refs/heads/<name>` | `BranchCandidate { name, origin: Local, .. }` |
| `refs/remotes/<remote>/<name>` | `BranchCandidate { name, origin: Remote { remote }, .. }` |
| `refs/remotes/<remote>/HEAD` | **dropped** — symbolic alias, not a branch |
| anything else / blank | dropped |

`<name>` may contain `/` (e.g. `origin/feat/a/b` → remote `origin`, name `feat/a/b`): split once
on the first `/` after the `refs/remotes/` prefix.

A name yielding both a `Local` and a `Remote` candidate collapses to the `Local` one (FR-019).
`blocked_by` is filled in by the caller from worktree records — the parser never sets it.

---

## `worktree_add_existing_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()>`

Check an existing local branch out into a new worktree (`CreateMode::ReuseLocal`).

- **Command**: `git -C <repo> worktree add <path> <branch>`
- **Must not** create, move, reset, or delete the branch. The branch's tip is identical before and
  after, on both success and failure (FR-004, SC-003).
- **Errors** when the branch is checked out elsewhere or missing — git's own refusal is the
  backstop behind pre-flight (research R2).
- **`FakeGit`**: registers `(path, branch)` as a worktree; leaves `branches` untouched. Honors
  `failing_next_add`. Errors if `branch` is not a known local branch, or is already bound to
  another registered worktree in the same repo.

## `worktree_add_reset_branch(&self, repo: &Path, branch: &str, path: &Path) -> io::Result<()>`

Replace a branch and check it out (`CreateMode::Overwrite`).

- **Command**: `git -C <repo> worktree add -B <branch> <path> HEAD`
- Creates the branch if absent, resets it to `HEAD` if present, then checks it out — one command,
  so there is no window where the old branch is gone and no worktree exists (research R3).
- **Destructive**: only ever called after the explicit confirmation required by FR-005.
- **`FakeGit`**: inserts `branch` (idempotent) and registers `(path, branch)`. Honors
  `failing_next_add`, leaving the branch present — mirroring git, where `-B` resets before the
  checkout can fail.

## `worktree_add_tracking_branch(&self, repo: &Path, branch: &str, remote: &str, path: &Path) -> io::Result<()>`

Start a local branch from a remote one and check it out (`CreateMode::TrackRemote`).

- **Command**: `git -C <repo> worktree add --track -b <branch> <path> <remote>/<branch>`
- **Must not** modify the remote-tracking ref or contact the remote (FR-017, FR-020).
- The remote is passed explicitly — never inferred — so a name present on several remotes has no
  ambiguity to resolve (research R4).
- **`FakeGit`**: requires `refs/remotes/<remote>/<branch>` to be primed; inserts the local branch
  and registers `(path, branch)`. Records the upstream so tests can assert it. Honors
  `failing_next_add`, leaving the local branch behind for rollback to clean up.

---

## Behavior shared by the three `worktree_add_*` methods

| Property | Requirement |
|---|---|
| Atomic-ish | One git invocation; partial failure leaves at most the branch behind, never a half-registered worktree that `worktree prune` cannot clear. |
| Rollback interaction | On error the caller runs `rollback_plan(mode)` — which **omits** `BranchDelete` for `ReuseLocal` (FR-008). See `branch-conflict.md`. |
| Path handling | Absolute target path, passed through `to_string_lossy()` like the existing `worktree_add_new_branch`. |
| Submodules | Unchanged: the caller runs `has_submodules` / `submodule_update_init_recursive` against the new worktree afterward, identically for all modes. |

## Unchanged methods

`is_repo_root`, `branch_exists`, `worktree_list_porcelain`, `worktree_add_new_branch`,
`worktree_remove`, `worktree_prune`, `branch_delete`, `has_submodules`,
`submodule_update_init_recursive` keep their existing contracts verbatim.

`branch_delete` in particular is **not** relaxed: it still reports a genuine refusal (feature 013,
FR-015). This feature's protection for reused branches comes from not *calling* it, not from
weakening it.

---

## Test obligations (`tests/git_fake.rs`)

1. `list_branch_refs` renders local and remote refs in git's format; `origin/HEAD` is emitted by
   the fake and dropped by the parser.
2. `parse_branch_refs` maps each line shape correctly, drops `HEAD` aliases and junk, splits
   multi-segment remote names on the first component, and collapses local+remote duplicates to
   `Local`.
3. `worktree_add_existing_branch` leaves the branch set unchanged; fails for an unknown branch and
   for a branch already bound to another worktree.
4. `worktree_add_reset_branch` creates-or-keeps the branch and registers the worktree; on primed
   failure the branch survives.
5. `worktree_add_tracking_branch` requires the remote ref, creates the local branch, records the
   upstream, and never mutates the remote ref.
6. Every `worktree_add_*` honors `failing_next_add` exactly once.
