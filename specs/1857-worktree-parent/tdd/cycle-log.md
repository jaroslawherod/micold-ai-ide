# Cycle Log: 1857 worktree parent, milestone M1 (#1858)

## Honest account

Milestone M1 was **not** driven test-first. The code, the tests and the docs were written in one
pass (WIP commit `5cfb009ca`), and the build lock was contended for over 20 minutes, so nothing was
compiled or run until after all of it existed. There is no per-behaviour red-before-green evidence
from the time of writing. The tests were run for the first time against finished code:

- first run: 12 of 13 daemon tests passed; one failed for a test bug, not a behaviour bug
  (`deleting_the_parent_leaves_children_removed_and_a_same_name_worktree_adopts_none` recreated the
  deleted worktree with the default `new_branch` mode, and the branch still existed: `conflict:
  branch 'w' already exists locally`). Fixed in the test with `mode: existing_local`.
- after the fix: `-p micold-core --lib worktree_parent` 9 passed; `--test main mcp_` 115 passed;
  `-p micold-daemon --test mcp worktree_parent` 13 passed.

## Substitute red-phase evidence: mutants

Each mutant edits `crates/micold-core/src/worktree_parent.rs`, runs the tests, and is reverted.

| Mutant | Daemon tests (`mcp worktree_parent`) | Core unit tests |
|---|---|---|
| `record` ignores the chosen parent (`Some(_) if true => return`) | 9 of 13 FAILED (every test that reads a recorded link) | not run |
| `retire` never sets `removed` (`link.removed = false`) | survived (13 passed) | see below |
| `retire` keeps the deleted child's link entry | survived (13 passed) | see below |

The first survivor (`removed` never set) was also invisible to the core unit tests: the flag is
persisted but every read path also goes through the missing id. Added an assertion on the persisted
flag to `forgetting_a_child_drops_its_entries_and_a_parent_tombstones` (red evidence: with `link.removed = false`, 8 passed and 1 FAILED, that test; reverted).
The second survivor is killed by the same core unit test (8 passed, 1 FAILED:
`forgetting_a_child_drops_its_entries_and_a_parent_tombstones`).

## Daemon-wiring mutants (review B F3)

Each edit reverted after the run; `-p micold-daemon --test mcp worktree_parent`:

| Mutant | Result |
|---|---|
| `refresh_worktrees` never calls `reconcile_worktree_parents` (state.rs) | 2 FAILED: `the_first_refresh_mints_an_id_for_every_live_worktree_and_keeps_them`, `a_worktree_removed_outside_the_app_tombstones_its_children_at_the_next_refresh` |
| session rows lose the parent (mcp/tools.rs `session_row`) | 1 FAILED: `session_rows_carry_the_worktrees_parent_independent_of_origin` |
| `ops::delete_worktree` does not call `forget_worktree_parent` | survived (13 passed): the refresh that follows every delete reconciles the same way, so the explicit forget is an equivalent mutant at this surface; it keeps the maps current when the refresh is skipped |
