# Cycle log: 1945-mcp-parent-sibling-view

Baseline: `micold-core --lib` 2149 passed, `micold-daemon --test mcp` 256 passed at the first M1 commit. Appended by hand: the first M1 pass wrote tests and code together (no red run); this log records the red runs taken afterwards (review B-F1).

## Cycle 1: core `neighbourhood()` (T001, T002)
- Behaviours: implicit `default` parent and unparented siblings; mutual siblings between recorded and implicit `default`; removed parent; `default` caller's children; unknown worktree.
- Red: `neighbourhood()` did not exist before the first commit; the tests were written with it, so there is no separate red. Gap, accepted: the daemon tests below fail on the pre-1945 `Scope::new` (origin/main `tools.rs`).
- Green: `cargo test -p micold-core --lib worktree_parent` 15 passed.

## Cycle 2: a live directory named `default` (A-F2)
- Test: `a_live_directory_named_default_is_not_the_project_root`.
- Red (mutation: the `DEFAULT_NAME` exclusions in `children_of`, the siblings filter and the own-check removed): `test worktree_parent::tests::a_live_directory_named_default_is_not_the_project_root ... FAILED`, 15 passed, 1 failed.
- Green: exclusions restored, 15 passed.

## Cycle 3: daemon visibility for an unparented caller (T003, T004)
- Tests: `an_unparented_worktree_sees_default_its_siblings_and_its_children`, `siblings_are_mutual_between_a_recorded_and_an_implicit_default_parent`.
- Red (origin/main `mcp/tools.rs`, `ops.rs`, `state.rs`, `server.rs` checked out over the working tree): both `FAILED`, 10 passed 3 failed.
- Green: working tree restored, `--test mcp worktree_parent` 27 passed.

## Cycle 4: caller whose worktree is not in the cached list (A-F3)
- Test: `a_caller_whose_worktree_is_missing_from_the_cached_list_sees_only_itself`.
- Red (origin/main sources): `FAILED` (the old scope showed the caller `default` and siblings). Fail-closed behaviour is kept; no per-call refresh (a git call on every tool call).
- Green: passes with the working tree.

## Cycle 5: removed caller worktree (A-F4)
- Test: `a_caller_whose_worktree_was_removed_sees_only_itself`: `gone` under `P` with sibling `sib` and child `kid`, removed with `git worktree remove`, then the daemon's `refresh_worktrees`; sees only itself, addressing `default`, `P`, `sib`, `kid` is `not_found`.
- Red: not observed. It passes on origin/main's stricter `Scope` as well; it guards the new derivation against failing open. Accepted gap.
- Green: passes.

## Cycle 6: record the link before the session starts (T016, T017; A-F1)
- Tests in `tests/mcp_create_session.rs`: `a_create_with_a_session_records_the_parent_link`, `a_create_that_fails_before_the_worktree_exists_records_no_link`.
- Red: not observed; both pass on the old post-call record too (the missing-CLI case never reached it). The ordering itself (link visible to the starting session) is T015, in M3. Accepted gap.
- Green: `--test mcp_create_session` 32 passed after one test fix (other worktrees' ids exist, so assert only the new directory has none).
