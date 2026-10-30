---
feature: 1602-mcp-self-delete
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 93ce38cfb # short SHA audited
behaviors: 30 # no test list exists: the 30 tasks T001-T030 stand in for behaviors
proven: 0
likely: 0
test_after: 30 # no red recorded anywhere; ACCEPTED by the requester (tests written beside code), not counted toward the verdict
no_test: 0
high_smells: 1
criteria_total: 29 # US1 s1-6, US2 s1-5, US3 s1-2, US4 s1-3 (16 acceptance scenarios) + FR-001..FR-017 minus 3 non-behavioral (FR-012, FR-017 docs; FR-013 unchanged-behavior)
criteria_covered: 29
mutation_score: 75 # scope: 8 hand-made mutants in 4 changed source files, no tool (profile has none); not a tool score
mutants_survived: 2 # both inside DONE behaviors, not equivalent
suite: feature binaries 100% green (core 5, daemon 2, client 4 binaries, 232 + 7 daemon tests); full `cargo test --workspace` stopped at 1 unrelated flaky failure after 11735 passed, 11m10s
---

# TDD Verification: MCP self stop and own-worktree delete (#1602)

**Verdict: FAIL.** Two deliberate mutants in changed code survive (the branch-delete-failed notice and the unreadable-git prompt are asserted by no test), and one HIGH smell lets two failure-path tests pass without asserting anything when the suite runs as root.

Test-first ordering is not part of this verdict: the requester accepted that the tests were written beside the code, and no `tdd/test-list.md` or `tdd/cycle-log.md` exists. Independence: this audit was run by a separate cold-context agent that wrote none of the code or tests; every cited line was opened in this run.

Stage that failed (rubric question order): 1 (test-first) is waived; 2 (assert behavior) fails on finding 1; 3 (would they catch a bug) fails on findings 2 and 3; 4 (criteria covered) passes; 5 has MED/LOW findings only.

## Test-first evidence

No cycle log and no test list. History cannot show an order: every feature commit adds tests and source together.

| Milestone | Commits (test and source together) | Class |
| --------- | ---------------------------------- | ----- |
| M1 T001-T016 | `899d6eb85`, `11d249c42`, `e08436650`, `714b34c54` | TEST_AFTER (waived) |
| M2 T017-T025 | `3a4b2a086`, `3f908c5f6`, `0ba7d304a` | TEST_AFTER (waived) |
| M3 T026-T030 | `9cc340db8` | TEST_AFTER (waived) |

`tasks.md` states "every test task precedes the implementation task it covers and must fail first" (Constitution I). The history does not support that claim; the requester accepted it. All 30 tasks are ticked `[x]`; there is no `DONE` list to compare against, so the tick-versus-list check cannot be done.

### Tests that already existed

Checked every removed or changed assertion in the feature's range (`899d6eb85^..HEAD`, feature commits only).

- Removed, with a replacement for the new behavior: `mcp_policy.rs` (`deleting_the_callers_own_worktree_is_refused_by_policy`, `stop_session_on_self_is_refused...`, `delete_session_on_self_is_refused...`), `mcp_lifecycle_tools.rs` `self_targets_are_refused_without_a_prompt` (now only interrupt, read, send, which stay refused), `mcp_audit_log.rs:300` self-refusal cases, `mcp_tools_catalog.rs` omitted-`session` invalid-input cases. Each is a required behavior change (FR-001, FR-011) with a new test asserting the new decision. The `PROTOCOL_VERSION` pins moved 73 to 74 (`protocol_roundtrip.rs`, `schema_hash.rs`).
- No assertion was loosened to a truthiness check, no tolerance widened, no test renamed out of a filter, no `#[ignore]`, no config exclusion, no threshold lowered. No weakened existing test.

## Findings

| # | Severity | Finding | Evidence |
| - | -------- | ------- | -------- |
| 1 | HIGH | Conditional logic: the failed-deferred-removal tests return or skip their assertions when the process is not stopped by a 0o500 directory (root, some CI containers), with no skip marker, so they report green having asserted nothing. The one assertion that does run is vacuous: `!subject.is_empty() && !reason.is_empty()` where FR-009 requires the notice to name the worktree and the reason. Should assert `subject == "<worktree display name>"` and that `reason` names the failure, and fail (or be `#[ignore]` with a reason) when the fault cannot be injected. Or inject the fault through `FakeGit`, as T005 allowed. | `crates/micold-daemon/tests/mcp/mcp_lifecycle_tools.rs:1314-1316` (early `return`), `:1351-1354` (vacuous); `crates/micold-daemon/tests/mcp_audit_log.rs:532,545` (`if locked`) |
| 2 | HIGH | Surviving mutant M4: removing `&& !deleted.branch_delete_failed` from the deferred-delete success arm leaves the "worktree removed but its branch could not be deleted" notice unreachable and every test green. T004 and the spec edge case promise `branch_delete_failed` surfaces as a notice. No test makes branch deletion fail after an accepted self-delete. | `crates/micold-daemon/src/mcp/tools.rs:197,207` |
| 3 | HIGH | Surviving mutant M5: replacing `LossSummary::unreadable()` with an empty `LossSummary::bounded(...)` when `worktree_loss` fails leaves every test green. The edge case "git unavailable or W is not a repository worktree: the prompt shows that the state could not be read" is tested at the git layer (`worktree_loss.rs:80`, `GitCli` returns `Err`) and at the dialog layer (`features_agent_confirm.rs`, `unreadable: true` hand-built), but nothing checks that the daemon turns the error into `unreadable: true` in the prompt. A failing read would show an empty "nothing lost" prompt, the unsafe direction. | `crates/micold-daemon/src/mcp/tools.rs:909-913` |
| 4 | MED | Sleepy tests: absence is asserted after fixed sleeps (300 ms, 500 ms, 1 s, 400 ms, 3.5 s) and presence by polling loops written inline three times. Slow (about 5 s in one test) and timing-dependent under load. Should wait on a condition (the audit or window event) or share one `wait_until` helper. | `mcp_lifecycle_tools.rs:946,1150,1237,1267,1285,1287`; inline polls `:900-905,:962-967` duplicate `gone_from_catalog` `:866` |
| 5 | MED | SC-006 and the open-handle test (R6): `a_self_delete_removes_the_worktree_although_a_handle_stays_open_for_four_seconds` passes trivially on Linux and macOS (only Windows refuses to delete an open directory), and 23 tests in this file, including every self stop and self delete-session test, are `#[cfg(unix)]`. The Windows proof depends on a CI job this audit cannot see. | `mcp_lifecycle_tools.rs:1174-1198`; `#[cfg(unix)]` at `:879,913,931,952,1157,...` |
| 6 | LOW | Duplicated setup: `crates/micold-core/tests/worktree_loss.rs:8` hand-rolls `git()` and a repo builder. The workspace already has about nine per-file `git()` helpers (for example `crates/micold-core/tests/support/attach_fixture.rs:18`); this adds one more instead of a shared helper. Not a finding against the exemplar, which has none. | `worktree_loss.rs:8-38` |
| 7 | LOW | The client notice reads "Could not finish removing “<subject>”" for every `DeferredActionFailed`, including a failed self stop (`Deferred::StopSession`), and the one test only checks `contains` of subject and reason. The wording is wrong for the stop case and no test pins it. | `crates/micold-client/src/shell/daemon_sync.rs:2269`, test `:6003-6020` |
| 8 | LOW (flaky, unrelated) | `crates/micold-daemon/tests/structured_model.rs::of_two_picks_the_last_one_wins` failed once in the full run (`:522`) and passed on rerun. The file is not in the feature's diff. Because `cargo test --workspace` stops at the first failing binary, the binaries after it did not run in that run. | `structured_model.rs:522` |

Checked and clean: no assertion-free, tautological, doubled-subject, over-mocked, or self-approving-snapshot test in the feature's new tests. The daemon tests drive the real MCP HTTP entry point (`f.ok`/`f.err`, raw TCP for reply ordering); the only double is the answering window, which is the user. The core, client, and unit tests follow the profile's exemplars (`#[cfg(test)]` unit tests in `loss.rs`, integration files under `tests/`, `Fake*` types not re-rolled). The `loss.rs` unit tests and the `confirm_agent_loss_list_geometry.rs` gate assert specific values, including the 50-entry bound and sanitization.

## Mutation results

No mutation tool in the profile (`mutation: null`). Deliberate mutants, one at a time, each restored with `git checkout --` and verified green afterwards (`cargo test -p micold-daemon --test mcp --test mcp_audit_log`: 232 and 7 passed; `git status` clean). Sample of 8 mutants over the highest-risk paths (data loss on delete, confirmation, ordering). Score 6 of 8 caught = 75%, a sample, not comparable to a tool score.

| Mutant | Behavior | Survived | Judgment |
| ------ | -------- | -------- | -------- |
| M1 `tools.rs:856` `delete_branch && wt.branch.is_some()` to `wt.branch.is_some()` | FR-004, FR-008 (`delete_branch: false`) | No | Caught by `an_accepted_self_delete_without_delete_branch_keeps_the_branch` |
| M2 `tools.rs:823` drop the live-sessions guard | FR-002 | No | Caught by `a_self_delete_without_stop_sessions_is_the_live_sessions_conflict` |
| M3 `tools.rs:832` skip the confirmation when self-targeted | FR-001, FR-005 (nothing before the user accepts) | No | Caught by 2 tests (decline, accept prompt) |
| M4 `tools.rs:197` drop `!deleted.branch_delete_failed` | FR-009, edge `branch_delete_failed` | **Yes** | Not equivalent: the notice becomes unreachable. Finding 2 |
| M5 `tools.rs:911` `unreadable()` to an empty bounded summary | FR-006, edge git unavailable | **Yes** | Not equivalent: an unreadable repo would show a "nothing lost" prompt. Finding 3 |
| M6 `loss.rs:7` `LOSS_LIMIT` 50 to 51 | SC-003 | No | Caught by `each_list_is_cut_at_fifty_and_the_rest_counted` |
| M7 `server.rs:33` `DELIVERY_GRACE` 2 s to 0 | FR-003 (result first, then act) | No | Caught by `a_self_delete_stops_the_sessions_only_after_the_reply_and_the_peer_close` |
| M8 `tools.rs:192` deferred delete `stop_sessions` true to false | FR-003 | No | Caught by 2 tests |

Coverage: not available (`coverage: null`).

## Traceability

No test list, so the `traces` column does not exist. The mapping below is built from the spec, the tasks, and the test names, and each cited test exists and passes. "Real entry point" means the MCP HTTP endpoint (daemon `f.ok`/`f.err`/raw TCP) for the daemon-side criteria.

| Criterion | Tests | Real entry point |
| --------- | ----- | ---------------- |
| US1 s1, s2, FR-001, FR-003, FR-008 | `an_accepted_self_delete_replies_scheduled_then_removes_the_worktree_and_branch`, `a_self_delete_stops_the_sessions_only_after_the_reply_and_the_peer_close`, `mcp_policy::deleting_the_callers_own_worktree_waits_for_confirmation_and_is_self_targeted` | Yes |
| US1 s3, FR-004 | `an_accepted_self_delete_without_delete_branch_keeps_the_branch` | Yes |
| US1 s4, s5, FR-005, SC-002 | `a_declined_or_unasked_self_delete_changes_nothing`, `a_hangup_while_the_prompt_is_pending_removes_nothing` (the 60 s timeout itself is shared with `ask_user`'s existing tests, not re-run here) | Yes |
| US1 s6, FR-002 | `a_self_delete_without_stop_sessions_is_the_live_sessions_conflict` | Yes |
| US2 s1-s3, s5, FR-006, SC-003 | `the_prompt_lists_changed_files_and_unpushed_commits_of_another_worktree`, `the_prompt_of_a_clean_pushed_worktree_lists_nothing`, `the_prompt_bounds_the_file_list_and_counts_the_rest`; core `worktree_loss.rs` (5); client `features_agent_confirm.rs` and `confirm_agent_loss_list_geometry.rs` | Yes (daemon prompt); commit-list bound at unit level only |
| US2 s2 (no upstream) | `without_an_upstream_commits_on_no_remote_branch_are_listed` | Unit/integration against `GitCli`; no daemon-level case |
| US2 s4 | `allowing_a_prompt_that_lists_work_deletes_the_worktree` | Yes |
| Edge: git unreadable | `a_directory_that_is_not_a_repository_is_an_error`, `an_unreadable_repository_is_said_and_lists_nothing` | No: daemon mapping untested (finding 3) |
| US3 s1, FR-007, SC-004 | `a_default_session_is_refused_delete_worktree` | Yes |
| US3 s2 | `an_unknown_target_is_not_found_without_a_prompt` | Yes |
| US4 s1, s2, FR-011 | `an_accepted_self_stop_replies_scheduled_then_stops_the_session`, `an_accepted_self_delete_session_replies_scheduled_then_removes_the_session`, `mcp_policy::stop_session_on_self...`, `delete_session_on_self...` | Yes (unix only) |
| US4 s3 | `a_declined_or_unasked_self_stop_or_delete_keeps_the_session_running` | Yes (unix only) |
| FR-011 refusals kept | `self_targets_of_the_other_tools_are_refused_without_a_prompt` | Yes |
| FR-016 | `a_default_session_stops_itself_and_touches_no_worktree` | Yes (unix only) |
| FR-009 | `a_failed_deferred_removal_is_broadcast_to_every_window`, client `daemon_sync.rs:6003` | Daemon yes, but skips as root (finding 1); branch-failure path untested (finding 2) |
| FR-010 | `mcp_audit_log.rs` (2 new tests) | Yes; error line skips as root (finding 1) |
| FR-012 | `the_delete_worktree_description_allows_self_delete_with_confirmation`, `mcp_tools_catalog.rs` | Catalog yes; user guide not tested |
| SC-005, FR-013 | existing 034 tests (232 daemon `mcp` tests pass) | Yes |
| SC-006 | unix tests only; Windows via CI | Not verifiable here (finding 5) |

Untested criteria: none, but FR-009 (branch-delete failure) and the git-unreadable edge case are covered only below the entry point. Tests tracing to nothing: none.

## What was not audited

- Test-first ordering: no list, no log, history inseparable; waived by the requester, not assessed.
- Mutation: 8 hand-made mutants in 4 changed source files; no tool, no sweep of `policy.rs`, `messages.rs`, the client dialog, or `ops.rs`. Not exhaustive.
- Windows and macOS runs (SC-006), including whether the open-handle test bites; CI results were not read.
- The full workspace suite: stopped at one unrelated flaky failure, so binaries after `structured_model` ran only where listed. Feature binaries were run explicitly and pass.
- Coverage (no tool), performance and load (no criterion), and the visual passes (`visual-pass-m1.md`, `visual-pass-m2.md`).
- `docs/user-guide/agent-tools.md` and the 034 spec amendments (FR-012, FR-017): documentation, not tests.
- Tests not in the feature's diff that exercise the same paths (`mcp_confirmations.rs` withdrawal and timeout tests were taken on the strength of their names, not re-read).
