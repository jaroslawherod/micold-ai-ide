# Phase 1 Data Model: Git Submodule Support for Worktree Creation

**Date**: 2026-07-18 | **Feature**: 010-submodule-worktree-support

No new persisted entities. This feature extends the existing `Worktree` create/rollback
orchestration (`src/worktree.rs`) and its `Git` I/O boundary (`src/git.rs`), and adds a transient
UI status so creation can be async (research R4). Enums keep new failure/progress states
unrepresentable-if-invalid (Constitution Principle V). Trait-level detail lives in
[contracts/git-trait-submodules.md](./contracts/git-trait-submodules.md).

## Entity: `CreateError` (existing — extended)

The typed outcome of `create_worktree`. No new variant is added — a submodule fetch failure is
represented with the **same** `RolledBack` variant a `worktree_add_new_branch` failure already
uses, since both now go through the identical rollback path (research R3).

```rust
pub enum CreateError {
    DuplicateDir,
    DuplicateBranch,
    /// A git step failed and the rollback plan ran to completion. Carries the failing
    /// operation's error text (git stderr) verbatim, whether the failure came from
    /// `worktree_add_new_branch` or, newly, `submodule_update_init_recursive`.
    RolledBack(String),
}
```

**Validation / rules**:
- `RolledBack` is only ever constructed after `rollback_plan()` has run (existing invariant,
  unchanged); a submodule failure does not bypass it.
- The wrapped `String` is git's own stderr text, not a re-classified reason — the UI presents it
  as-is (FR-006), and it already names the failing submodule and the underlying cause.

## Entity: `Worktree` creation flow (existing — extended step)

`create_worktree(git, repo, target_path, names, target_exists) -> Result<Worktree, CreateError>`
gains one step, inserted after the existing `worktree_add_new_branch` success and before
returning `Ok`:

```
pre-flight checks (unchanged: DuplicateBranch, DuplicateDir)
        │
        ▼
worktree_add_new_branch  ──failure──► rollback_plan() ──► Err(RolledBack)
        │ success
        ▼
git.has_submodules(target_path)?
        │
   ┌────┴────┐
  false      true
   │          │
   │          ▼
   │   submodule_update_init_recursive(target_path)
   │          │
   │     ┌────┴────┐
   │   success   failure
   │     │          │
   │     │          ▼
   │     │   rollback_plan() ──► Err(RolledBack)
   │     │
   └─────┴──► Ok(Worktree { status: Valid, .. })
```

**Rules**:
- The submodule step is skipped entirely (no git invocation, no delay) when `has_submodules`
  is `false` — a non-submodule repository's creation path is byte-for-byte what it is today
  (FR-003).
- A submodule failure runs the *same* `rollback_plan()` steps, in the *same* order, as a
  `worktree_add_new_branch` failure (`WorktreeRemove → WorktreePrune → BranchDelete`; the caller
  removes the directory) — no new `CleanupStep` variant is introduced.

## Entity: `Git` trait (existing — two new methods)

See [contracts/git-trait-submodules.md](./contracts/git-trait-submodules.md) for the full
contract. Summary:

| Method | Purpose |
|--------|---------|
| `has_submodules(&self, worktree_path: &Path) -> bool` | Whether the checked-out tree at `worktree_path` declares any submodules (`.gitmodules` present). |
| `submodule_update_init_recursive(&self, worktree_path: &Path) -> io::Result<()>` | Initialize + fetch + check out every submodule (recursively) inside `worktree_path`. |

Both are implemented by `GitCli` (real git/fs) and `FakeGit` (in-memory, primeable to simulate a
repo with/without submodules and to fail the fetch step) — same pattern as every existing `Git`
method.

## Entity: `WorktreeForm` status (new — transient UI state, not persisted)

Today, `AddWorktreeSubmitted` creates the worktree synchronously within one `update()` call, so
the form has no notion of "in progress." Making creation asynchronous (research R4) requires a
status the form can render:

```rust
/// Transient creation status for the add-worktree form (research R4). Not persisted —
/// reset to `Editing` whenever the form is (re)opened.
pub enum WorktreeFormStatus {
    /// The user is filling in the form; no create is in flight.
    Editing,
    /// `WorktreeCreateStarted` was dispatched; the async create (incl. any submodule fetch)
    /// is running. The form shows a "Creating worktree…" state and disables submission.
    Creating,
}
```

`WorktreeForm` gains a `status: WorktreeFormStatus` field (default `Editing`). Two new
`Message` variants drive it:

- `WorktreeCreateStarted` — dispatched immediately on submit (before the async `Task` resolves);
  sets `status = Creating`.
- Existing `WorktreeCreated` / `WorktreeCreateFailed` — unchanged in meaning, but now arrive
  asynchronously (via `Task::perform`) instead of synchronously; `WorktreeCreateFailed` resets
  `status = Editing` so the user can retry, matching today's "keep the form open" behavior.

**Validation / rules**:
- Submission (`AddWorktreeSubmitted`) is only actionable in `Editing`; while `Creating`, the
  submit action is a no-op (prevents a double-create from a repeated Enter/click).
- `status` always resets to `Editing` when the form is (re)opened (`AddWorktreeOpened`) or
  cancelled (`AddWorktreeCancelled`), same as `error` today.

## Relationship summary

```
WorktreeForm { ..., status: Editing|Creating }
        │ submit (Editing only)
        ▼
create_worktree(git, ...)              # unchanged signature, one new internal step
        │
        ├─ no submodules  → Ok(Worktree)                      (unchanged timing)
        ├─ submodules ok  → Ok(Worktree)                      (new: fetch happened)
        └─ any git failure → Err(CreateError::RolledBack(_))  (same rollback plan as before)
```
