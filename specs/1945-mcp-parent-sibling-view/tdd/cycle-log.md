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

## Cycle 13: review A round 2 fixes (F1, F3, F4)
- Tests: `mcp_worktree_parent_neighbourhood::a_directory_named_default_never_grants_the_project_roots_sessions` (F1); `mcp_cross_session::a_parent_running_only_a_shell_is_refused_as_having_no_ai_cli_session` (F3). F4 adds a check the parent always passes today, so no test can make it fail; the existing delivery tests cover its line.
- Red (tests in place, `src/mcp/tools.rs` unchanged): `--test mcp` 1 failed (`list_sessions` from the directory named `default` listed the root's session `...0001`); `--test mcp_cross_session` 1 failed (message "has no running session", not "no running AI CLI session (only shell sessions)").
- Green: `--test mcp` 271, `--test mcp_audit_log` 8, `--test mcp_cross_session` 28 passed. First run 27/28: `a_parent_directory_named_default_is_not_the_project_root` started the root's session as the linked caller, which the old scope allowed only through the bug; it now starts it with the full view before linking.

## Cycle 14: the reserved name `default` (D2)
- Tests: `branch_conflict::{a_worktree_named_default_is_refused_before_anything_is_touched, the_reserved_name_is_explained_in_one_sentence_naming_the_reservation}`; `mcp_create_session::a_worktree_named_default_is_refused_naming_the_reservation` (by branch and by `name`, with and without a session).
- Red: `--test main branch_conflict`: `E0599 no variant ReservedName`, `E0425 explain_reserved_name` (compile errors).
- Green: 33 passed in `branch_conflict`; the MCP test passed once the core refusal existed (it went through the same funnel), after a first run whose third case wrongly used `Default`, which `checked_dir_name` already refused with its own message.

## Cycle 15: a create with no parent retires the stale id (review A round 2 MINOR)
- Test: `mcp_create_session::a_create_without_a_parent_retires_the_stale_id_of_a_reused_name`.
- Red (ops.rs unchanged): `assertion left != right failed` (the stale `WorktreeId` was kept).
- Green: `--test mcp_create_session` 36 passed.

## Cycle 16: the link exists before the CLI starts (T015, SC-004)
- Test: `mcp_create_session::the_link_is_recorded_before_the_new_sessions_cli_starts`. The store is slow (400 ms) to save a catalog with links and then writes the link count to `links.saved`; the `copilot` stand-in copies that file into `copilot.seen` as its first act; the test asserts the marker equals the final count.
- Red (review A round 1 F1): `try_record_worktree_parent` moved below `create_session_with_prompt` in `ops::create_worktree_with_session`: `assertion left == right failed: the CLI started before its link was saved (left "5", right "6")`. Restored; green: `--test mcp_create_session` passes. (The first version, a 1 ms watcher on the launch line, passed with the record moved too: it had no red.)

## Cycle 17: wording for a parent with a stopped AI CLI session (review A round 3 a)
- Test: `mcp_cross_session::a_parent_with_a_stopped_ai_cli_session_is_not_described_as_only_shells`.
- Red: message said "no running AI CLI session (only shell sessions)".
- Green: `--test mcp_cross_session` 29 passed.

## Cycle 18: every create path retires the stale id (review A round 1 F2)
- Tests: `mcp_create_session::{an_unparented_create_retires_the_stale_id_and_survives_a_failed_write, a_no_parent_create_with_a_session_survives_a_failed_link_write}`.
- Red (before the fix): the second failed with `the worktree's parent was not recorded` on the unwritable store; the first did not compile (`create_unparented_worktree` missing).
- Green: `--test mcp_create_session` 38 passed.
