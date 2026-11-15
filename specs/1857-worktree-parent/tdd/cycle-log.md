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

## Milestone M1b (T030-T032, #1858): red against origin/main, green on the branch

Honest account: the M1b tests and the filter landed in one commit, so git history shows no
red-before-green. This section records the red run after the fact. Method: copy `origin/main`'s
`crates/micold-daemon/src/mcp/tools.rs` over the branch's, run each test of
`tests/mcp/mcp_worktree_parent_visibility.rs` (`scripts/build-lock.sh cargo test -p micold-daemon
--test mcp <name>`), restore the branch's file, run again.

| Test | On origin/main's `tools.rs` | On the branch |
|---|---|---|
| `a_worktree_with_a_parent_lists_itself_its_parent_siblings_and_children` | FAILED: left `[A,B,C,P,X,Y,default]`, right `[A,B,C,P]` | passed |
| `an_unparented_worktree_sees_itself_and_its_children` | FAILED: left `[A,B,C,P,X,Y,default]`, right `[X,Y]` | passed |
| `a_default_caller_sees_everything` | passed (guard: the unrestricted caller is unchanged, so it cannot be red) | passed |
| `every_tool_reports_an_out_of_scope_target_as_not_found` | FAILED: `get_session succeeded` for an out-of-scope session | passed |
| `a_message_to_an_out_of_scope_session_is_refused_while_the_parent_is_reachable` | FAILED: left `conflict`, right `not_found` | passed |
| `an_explicit_parent_outside_the_callers_set_is_invalid_input` | FAILED: `create_worktree succeeded` with `parent: default` | passed |
| `list_branches_does_not_name_a_worktree_the_caller_cannot_see` | FAILED: `checked_out_in` left `"X"`, right `null` | passed |

### FR-015, review B F4: the reason text names no hidden worktree (test first)

Two tests added before the fix, red on the branch as it was:

- `an_unavailable_reason_does_not_name_a_worktree_the_caller_cannot_see`: FAILED, `unavailable_reason`
  for branch X seen by A was `'X' is already checked out in the worktree 'X'.`, expected
  `'X' is already checked out in another worktree.`
- `a_create_worktree_conflict_does_not_name_a_worktree_the_caller_cannot_see`: FAILED, the
  `create_worktree` conflict message named worktree 'X' for A.

Fix: `Scope::explain` (tools.rs) replaces the holder with generic wording when it is outside the
scope; `describe_situation`, `describe_derived_collision`, `create_failure` and `list_branches` use
it. After the fix: `-p micold-daemon --test mcp worktree_parent` 22 passed (both new tests green,
the sibling case still named, `default` still sees the name).

## M3 (T023-T025, T033)

Honest account: `307e04e11` holds the test and the helper together, so git history shows no
red-before-green. Red runs recorded after the fact; the first review round found the fixture used
an invented `name` field, so the test was rebuilt on the real `list_worktrees` row shape
(`ref`, `display_name`, `branch`, `path`, `status`, `session_count`, `parent`, `parent_state`;
`tools.rs` `worktree_rows`) and the fixes below went test first.

### The original T023 cases against origin/main

`scripts/tests/supervisor-ownership.test.sh` as of `307e04e11`, run in a detached worktree of
`origin/main` (no `runs-of-parent.sh`, no `Parent` column, no ownership text): 25 cases, 24 FAILED
(helper missing: `No such file`; skill sections absent; the one pass is the usage-error case, which a
missing script also exits 2 on). On the branch: 25 passed.

### Review A F1-F6 and review B F2: test first

Rebuilt test (39 cases) run against the branch's old script, skill and `supervisor.sh`: 20 FAILED.

| Finding | Red on the old code | Green after |
|---|---|---|
| A F1: rows keyed by `ref`, not `name` | every list, check and orphans case: list printed `null`, `--check run-1` printed `refused: run-1 is not listed` | list, check, orphans print `ref` |
| A F3: `--check` given the ledger's `Worktree` path | `--check /work/proj/run-1` refused as not listed | accepted by ref or path; a display name is still not an id |
| A F2: orphan scan on a daemon-filtered list | skill lacked `worktrees-all.json` and `--orphans --rows`; filtered-list case documents that it shows no orphans | skill scans the unfiltered list |
| A F4: empty input and `{}` | exit 0 (want 2); `[1, 2]` gave raw jq exit | all exit 2 `unreadable` |
| A F5: `ledger set 12 Parent x` on an 8-column ledger | exit 0, wrote past the closing pipe | exit 2 naming the Parent column |
| A F6 / B F2: stray `,.` in the relay text | relay-text case failed | plain sentences; `parent and siblings` kept |

After: `bash scripts/tests/supervisor-ownership.test.sh` 39 cases, 0 failures; `supervisor.test.sh`
269 cases, 0 failures.
