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

## Cycle 7: a failed parent-link write (diff-coverage fix, `WorktreeStartFailure::Link`)
- Tests: `a_link_that_cannot_be_written_names_the_worktree_and_starts_no_session` (`tests/mcp_create_session.rs`, a project store refusing any catalog that holds a link) and `link_failure_tests::a_failed_link_write_is_an_io_failure_naming_the_worktree` (`server.rs`).
- Red (mutation: `try_record_worktree_parent(..).ok()` in `ops::create_worktree_with_session`): the MCP test `FAILED` (no error reported).
- Green: mutation reverted, both pass. The app path (`server.rs`) names no parent today, so no real create reaches its `Link` arm; its reply is extracted to `link_failed` and tested directly.

## M2

## Cycle 8: catalog and policy for the two tools (T006, T007)
- Tests: `mcp_tools_catalog::neighbourhood_and_send_parent_input_parse_and_reject_extra_arguments`, the `SHIPPED`/`READ_TOOLS` lists, `mcp_policy::send_parent_input_is_decided_as_send_session_input`, `Neighbourhood` in the read-only proceed list.
- Red: `cargo test -p micold-core --test main mcp_`: `error[E0599]: no variant ... named Neighbourhood / SendParentInput found for enum Operation` (5 errors).
- Green: first run 116 passed, 1 failed (`report_review_comment_is_listed_after_send_session_input_as_the_contract_gives`: the new tool sat between them); moved after the report tool, 117 passed.

## Cycle 9: `neighbourhood` and the `send_parent_input` refusals (T008, T009, T011)
- Tests: `tests/mcp/mcp_worktree_parent_neighbourhood.rs` (recorded, unparented, removed, `default`, several sessions, unknown worktree, scope over every fixture caller; four refusals).
- Red (daemon arms stubbed: `Neighbourhood => Ok(json!({}))`, `SendParentInput => service_error`): `--test mcp worktree_parent` 27 passed, 11 failed (every new test).
- Green: 38 passed. Later a directory named `default` case was added for the `None` arm (diff coverage): 12 passed in the file.

## Cycle 10: `parent_target` (T010)
- Tests: `mcp::tools::parent_target_tests` (first running in catalog order, same twice, `default`; none running).
- Red (mutation: the `Running` filter replaced by "not failed"): both `FAILED`. Written together with the function, so the red is by mutation.
- Green: mutation reverted, 2 passed.

## Cycle 11: delivery through `send_parent_input` (T012, T013)
- Tests in `tests/mcp_cross_session.rs`: one parent session, two sessions reached twice at the first, implicit `default`, `Off`, `Confirm each send`, a running session outside the scope never reached.
- Red (dispatch arm stubbed to `service_error` with the tests in place): `--test mcp_cross_session parent` 5 failed, 0 passed.
- Green: `--test mcp_cross_session` 25 passed.

## Cycle 12: review A round 1 fixes (F1-F4)
- Tests: `mcp_cross_session::{a_shell_ahead_of_the_parents_ai_cli_session_is_never_reached, a_parent_directory_named_default_is_not_the_project_root}`; `mcp_worktree_parent_neighbourhood::{a_parent_directory_named_default_lists_its_own_sessions, a_worktree_named_default_is_its_own_worktree_and_a_send_from_it_is_refused}` (replaces `a_send_from_a_worktree_named_default_is_refused`); `mcp_audit_log::a_text_sent_to_the_parent_never_reaches_the_log_and_the_line_names_the_session` and the `send_parent_input` target in `every_successful_mutating_call_writes_one_info_line`; `worktree_parent::tests::a_worktree_recorded_under_a_directory_named_default_is_its_child_not_the_roots`.
- Red (tests in place, code unchanged): `--test mcp parent` 2 failed (the root's session listed as the parent's and as the caller's own worktree); `--test mcp_cross_session --test mcp_audit_log parent` 3 failed (the shell reached; the root session reached for a directory named `default`; the audit line said `target=parent`). The core unit test failed to compile (no `root` field).
- Green: core `worktree_parent` 16 passed; daemon lib `parent_target` 2 passed; `--test mcp` 270, `--test mcp_cross_session` 27, `--test mcp_audit_log` 8 passed (first run 7/8: the `parent` field was Debug-quoted, logged as Display).
