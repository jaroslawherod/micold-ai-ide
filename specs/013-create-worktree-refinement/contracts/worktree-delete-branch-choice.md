# Contract: Worktree Delete — Branch-Deletion Choice

**Modules**: `src/app.rs` (`State`, `Message`), `src/main.rs` (`WorktreeDeleteConfirmed`
handler), `src/worktree.rs` (`remove_worktree` → `RemoveOutcome`), `src/git.rs`
(`GitCli`/`FakeGit` `branch_delete`), `src/ui/confirm_delete.rs`.

Extends `specs/008-worktree-sidebar-refinement/contracts/worktree-removal.md` (the existing
delete/confirm contract) rather than replacing it — every rule there not mentioned below is
unchanged (session termination still runs first, git steps still run in the same order, "no
confirmation ⇒ no removal" still holds).

## State & Messages (additions)

```rust
// State
pub worktree_delete_keep_branch: bool,   // default false = delete (today's behavior)

// Message
WorktreeDeleteKeepBranchToggled(bool),
```

## Reducer (pure) responsibilities (additions to the existing contract)

- `WorktreeDeleteRequested(dir)` — in addition to its existing effects, resets
  `worktree_delete_keep_branch = false` (never carries a choice over from a previous dialog).
- `WorktreeDeleteKeepBranchToggled(v)` → `worktree_delete_keep_branch = v`. No other state
  changes.
- `WorktreeDeleteConfirmed`/`WorktreeDeleteCancelled` — unchanged from the existing contract
  (the reducer's own drop-records/close-modal behavior does not need `worktree_delete_keep_branch`
  at all; only the boundary orchestration below reads it).

## Boundary orchestration (`src/main.rs`, on `WorktreeDeleteConfirmed`) — updated step 4

The existing 8-step sequence in `worktree-removal.md` is unchanged except step 4 (branch
deletion) is now conditional on the new choice, and step 6 gains a distinct failure notice:

4. `let branch = if app.core.worktree_delete_keep_branch { None } else { wt.branch.as_deref() };`
   then call `remove_worktree(&GitCli::new(), &repo, &wt.path, branch)` (steps 2–4 of the old
   contract — `worktree_remove` → `worktree_prune` → conditional `branch_delete` — are now all
   inside this one call, unchanged in order; see `data-model.md`'s `RemoveOutcome`).
5. On `Ok(RemoveOutcome { branch_delete_failed })`:
   - `branch_delete_failed == false` → proceed exactly as today (silent on full success,
     FR-023a unchanged).
   - `branch_delete_failed == true` → after the existing directory cleanup
     (`remove_worktree_dir`), surface a distinct notice, e.g. `format!("Deleted worktree
     \"{name}\", but its branch could not be deleted: {branch}")` — the worktree is still
     dropped from the sidebar (removal itself succeeded); this is reported as a warning about the
     branch specifically, not as the delete having failed.
   - `Err(e)` (a `worktree_remove`/`worktree_prune` failure) — unchanged from today: reported as
     "Could not delete worktree" and the row survives via the post-delete `discover_worktrees`
     reconcile (FR-023, unchanged).

## `GitCli`/`FakeGit::branch_delete` — outcome-based behavior (was: always `Ok(())`)

See `data-model.md`'s `Git::branch_delete contract` entity for the exact check. Summary: attempt
`git branch -D <branch>`, then treat the outcome as success iff `branch_exists` now reports
`false` — mirrors `GitCli::worktree_remove`'s existing BUG-001 idiom in the same file. `FakeGit`
gains a `.failing_next_branch_delete()` priming method (mirrors `.failing_next_remove()`) so the
refusal path is testable without a real repository.

## UI: `src/ui/confirm_delete.rs` (addition)

Adds a checkbox (reusing the existing `style::checkbox` already used by `settings_form.rs`) below
the existing warning text, e.g. "Also delete the branch `<branch>`", checked by default (mirrors
`worktree_delete_keep_branch`'s inverted default — the checkbox reads "delete," the state field
reads "keep," so `checked == !worktree_delete_keep_branch`), wired to
`Message::WorktreeDeleteKeepBranchToggled(!checked)`. The existing warning copy ("This
permanently removes the worktree directory … and its git branch.") is adjusted to reflect that
branch removal is now conditional rather than unconditional, e.g. splitting the directory/session
removal (always true) from the branch removal (conditional on the checkbox) in the sentence.

## Rules

- **Default preserves today's behavior** (FR-012): an unmodified confirm still deletes the
  branch — `worktree_delete_keep_branch` defaults to `false`, and the checkbox defaults to
  checked.
- **Keep-branch path cannot produce a branch-delete failure** (data-model.md): `branch = None` ⇒
  `branch_delete` is never called ⇒ `RemoveOutcome::branch_delete_failed` is always `false`.
- **Cancel discards the choice** (FR-016): `WorktreeDeleteCancelled` doesn't need to explicitly
  reset `worktree_delete_keep_branch` — `WorktreeDeleteRequested`'s reset on the *next* open
  already guarantees no stale carry-over, and nothing reads the field while no delete is pending.

## Tests

- `tests/worktree_delete.rs`: keep-branch path — `remove_worktree(..., None)` leaves the branch
  registered in `FakeGit`, directory/session removal still proceeds. Branch-delete-fails path —
  `FakeGit::failing_next_branch_delete()` primed, `remove_worktree(..., Some(branch))` returns
  `Ok(RemoveOutcome { branch_delete_failed: true })`, not `Err(_)`, and the worktree registration
  is still gone (`worktree_remove`/`prune` unaffected by the branch-delete outcome).
- `tests/app_state.rs`: `WorktreeDeleteRequested` resets `worktree_delete_keep_branch` to
  `false` even when a previous dialog had set it `true`; `WorktreeDeleteKeepBranchToggled` sets
  the field directly.
- `tests/git_fake.rs`: `FakeGit::branch_delete` returns `Ok(())` when the branch is genuinely
  absent/removed, `Err(_)` only when primed via `.failing_next_branch_delete()`.
