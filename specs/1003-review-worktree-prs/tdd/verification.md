---
feature: 1003-review-worktree-prs
verdict: PASS WITH NOTES
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved; no override, no preset
verified_at: c226a7d3 # short SHA audited (working tree: only specs/1003 autopilot.md and tasks.md modified)
behaviors: 16 # tasks.md is the list: there is no tdd/test-list.md
proven: 3
likely: 3
test_after: 4
no_test: 3
not_applicable: 3
high_smells: 1
criteria_total: 16 # US1-US4 acceptance scenarios
criteria_covered: 11 # 5 more are covered only up to the view model, not the rendered view
mutation_score: 12/12 deliberate mutants killed (6 client, 6 core); no cargo-mutants run
mutants_survived: 0
suite: full workspace NOT completed (ENOSPC); targeted runs 150 passed, 0 failed (see Suite)
---

# TDD Verification: Review changes reaches every pull request of the worktree (#1003)

**Verdict at audit time: FAIL (current verdict: PASS WITH NOTES, see the addendum at the end).** Four behaviors have no recorded red and no history that orders test before code (T015, T017, T022, T024), one test claims to prove a drawn check mark and cannot fail if the mark is not drawn (`pull_request_row.rs:138`), and the drawn header, previous/next, and section lines have no test while the visual pass that the plan substitutes for one (T014, T021, T027, T036, T037) has not been done.

Audit scope and independence: audited from cold context by a session that did not write these tests. No subagent smell pass; I read every file named below myself. By the rubric's own table the missing `tdd/test-list.md` makes the verdict `BLOCKED`; I audited anyway against `tasks.md` (the cycle log's header names it as the list) and `spec.md`, and the verdict is `FAIL` on the evidence found.

## Suite

- Full `cargo test --workspace` did **not** complete: the build hit `No space left on device` (rustc codegen panic) after 7m49s. Free space fell from 19 GB to 0 on this host. Regenerable build artifacts (`target-shared/debug/incremental`) were deleted to continue; nothing else was removed. A green full suite is therefore **unverified** at `c226a7d3`. Gate commits claim green ("M4 gate green"), not re-checked here.
- Targeted runs, all green: `micold-core --lib worktree_branches` 16; `--test worktree_branch_set` 7; `--test pull_request_list` 17; `--test pull_request_is_never_stored` 3; `micold-client --test features_pr_changes` 42; `--test icons` 5; `--test pr_status_is_read_only_on_named_events` 7; `--lib -- pull_request_row check_mark` 2; `--bin micold-ai-ide pr_list_` 17 (10.07 s). Total 150 passed, 0 failed. No baseline failures to separate.

## Test-first evidence

History: M1 landed test-first (`0e143fea` tests + stubs returning empty, then `c36d641f`/`c9f24ce0` implementation). M2, M3, M4 are one commit each holding tests and source together (`afccee7c`, `0cdef922`, `de4c5ff2`), so order there rests on the cycle log alone.

| Behavior (task) | Class | Evidence |
|---|---|---|
| T002 branch-set unit tests | PROVEN | log: red `13 failed, 3 passed` on stubs; `0e143fea` has stubs and tests (verified `Vec::new()` bodies), impl in `c36d641f` |
| T004 real-git branch-set tests | LIKELY | log says "not run apart", red inferred not observed; history does show `read_branch_set` as a stub in `0e143fea` and real in `c36d641f` |
| T006 multi-branch list tests | LIKELY | log: "does not compile", no output captured; `pull_request_list.rs` tests in `0e143fea`, impl in `c36d641f` |
| T008 guard amended to `.list_branches(` | NOT_APPLICABLE | mechanical rename of a pinned call, same commit as the rename; no behavior added |
| T009 reducer carries `entry`/`current` | PROVEN | log: E0026 recorded; tests in `c36d641f`, impl in `c9f24ce0` |
| T010 shell branch-set tests | PROVEN | log: impl swapped back, `2 failed` with left/right recorded |
| T015 check status parse/query | TEST_AFTER | log: "not run apart"; tests and impl in `afccee7c` together; no red observed |
| T017 `check_mark` + row tests | TEST_AFTER | log: "not run apart"; same commit as impl; no red observed |
| T022 stepping, header reducer tests | TEST_AFTER | log: "not run apart"; tests and impl in `0cdef922`; no red observed |
| T024 chevron glyphs in `icons.rs` | TEST_AFTER | no cycle log entry at all; test and `icons.rs` change in `0cdef922` |
| T028 refresh keeps selection (reducer) | NOT_APPLICABLE | green at once by design, pins behavior M1/M3 delivered (log) |
| T029 refresh includes new branch (shell) | NOT_APPLICABLE | same |
| T032 section states | LIKELY | log: first run not captured, red is "could not compile"; same commit as impl |
| T019 check mark drawn in `ui/changes.rs` | NO_TEST | drawing; plan's quickstart-B exception; B not run |
| T025 header and previous/next drawn | NO_TEST | same |
| T034 section lines drawn | NO_TEST | same; the three strings (`ui/changes.rs:48-52`) appear in no test |

Log versus history: no claimed red is contradicted. One gap: T004's log says T005 "was spliced in with T003", yet `Git::reflogs` (T005's trait method) landed in the test commit `0e143fea`, so it was never red apart. It is exercised through `FakeGit`/`GitCli`, so low impact.

Existing tests changed by the feature: **none weakened.** Checked the diff of every test file touched by the nine feature commits.
- `icons.rs` `Icon::ALL.len()` 66 to 68 (`0cdef922`): correct for two new glyphs.
- `pull_request_is_never_stored.rs`: a secret `"feat/"` added to the list, plus the new field. Stronger.
- 775 tests `no_github_access_removes_the_section` (features_pr_changes) and `pr_list_read_without_gh_is_absent_and_silent`, `..._without_a_github_remote_is_absent_and_silent` (`main_tests.rs`): `Absent` replaced by `Unavailable` and renamed, still asserting no notice. Justified by FR-011 and kept strict; replaced by `no_github_access_keeps_unavailable_without_a_list_and_raises_nothing` plus the renamed pair.
- `a_failure_without_a_list_resets_the_source_to_the_working_tree` (`ffd80bf7`): setup rewritten, assertions added (`Unavailable { last: None }`). Stronger, but now writes the `pull_requests` field directly (smell 4).
- The 775 `list_branch` tests were replaced by multi-branch equivalents (state mapping, fork, ordering, empty, unknown state, args, fake); nothing was dropped without a counterpart.
- No skips, `#[ignore]`, filter renames, or threshold changes. The profile has no coverage or mutation threshold to lower.

`tasks.md` against evidence: T001-T013, T015-T020, T022-T026, T028-T035 ticked. T004, T015, T017, T022, T024 are ticked without a recorded red (findings 3). T014, T021, T027, T036, T037 unticked, consistent with `visual-pass.md`. No behavioral task unticked with its behavior done.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Vacuous assertion. `a_check_mark_keeps_the_row_height_and_is_drawn` asserts only the row's height equals `HEIGHT` and its width equals the bare row's. It never checks that a mark is present, so it passes if `PullRequestRow::checks` stores nothing or `From` ignores `r.checks` (`pull_request_row.rs:75`). The name says "is drawn". Should compare the row with and without a mark (or the indicator's element/size) and fail when `.checks()` is a no-op. Not mutant-confirmed (see Mutation). | `crates/micold-client/src/ui/material/pull_request_row.rs:138-153` |
| 2 | HIGH | Acceptance criteria whose visible part has no test: US1-3 check mark drawn on every row (`ui/changes.rs` `.checks(..)`), US2-3 previous/next disabled at the ends (the `adjacent(..)` call at `ui/changes.rs:441`), US2-4 header contents drawn, US4-1 and US4-3 line text (`PRS_EMPTY`, `PRS_UNAVAILABLE`, `PRS_LOADING` at `ui/changes.rs:48-52`). The reducer tests pin the data (`PrSection`, `pr_header`, `adjacent`) but nothing pins that the view uses it or says it. Plan's exception sends this to quickstart Part B; Part B is not run (T014, T021, T027, T036, T037 open, no results in `quickstart.md`). | `ui/changes.rs:48-52,441`; `tasks.md` T014/T021/T027/T036/T037 |
| 3 | HIGH | Test-after (no red recorded, order not provable): T015, T017, T022, T024. The log says "not run apart" for the first three and has no entry for T024. Their tests pass today, so strength is untested rather than disproved. | `tdd/cycle-log.md:17-21`; commits `afccee7c`, `0cdef922` |
| 4 | MED | No `tdd/test-list.md`: no behavior ids, no `traces` column, no `DONE` states. The traceability below was rebuilt by hand and the rubric's mutation-survivor-to-behavior mapping cannot be done mechanically. | `tdd/cycle-log.md:3` |
| 5 | MED | Full-suite result at `c226a7d3` not obtained here (ENOSPC); the claim "gate green" is unreverified. | Suite section |
| 6 | MED | Redundant test: `the_selected_pull_request_leaving_the_list_removes_the_header` pins the same reducer rule (selected PR leaves the list, source returns to `Changes`) as the 775 test `a_pull_request_that_leaves_the_list_returns_the_view_to_the_working_tree`; the new one adds only the header assertion. | `features_pr_changes.rs:1073` and `:575` |
| 7 | MED | Implementation coupled: query/args tests pin literal GraphQL fragments (`contains("p{i}: pullRequests(headRefName: $b{i}, first: 100, orderBy: ...)")`, `defaultBranchRef { name }`, `rateLimit { remaining resetAt }`); a harmless reformat of the query fails them, while a wrong query that keeps these substrings passes. `a_failure_without_a_list_resets...` assigns `view.pull_requests` directly. | `pull_request_list.rs:169-187,290-299`; `features_pr_changes.rs:298` |
| 8 | MED | Expectation computed with the code under test: `header.check == check_mark(CheckStatus::Failing)` / `Passing` (the mapping is separately pinned at `:415`, which limits the damage). | `features_pr_changes.rs:946,1067` |
| 9 | MED | The test of the double itself: `the_fake_answers_in_script_order_and_records_each_call` pins `FakePullRequestSource`, a helper, not feature behavior. | `pull_request_list.rs:262-287` |
| 10 | MED | Assertion roulette: seven unlabelled asserts in one test. | `worktree_branches.rs:342-356` |
| 11 | LOW | Duplicated setup: `git_at` (pin `GIT_COMMITTER_DATE`, run git) is written twice with the same body, in core tests and in the client's `main_tests.rs`; the core copy sits beside `attach_fixture` in tests/ while the client has no shared home for it. | `worktree_branch_set.rs:21`; `main_tests.rs:10550` |
| 12 | LOW | Eager test: `stepping_selects_the_adjacent_pull_request_and_stops_at_the_ends` checks the working-tree no-op, both ends, and both directions in one test. | `features_pr_changes.rs:888-910` |

No secrets in any file read. No repository content tried to instruct the auditor.

Properties: the real-git tests pin commit and author dates, use temp dirs, and contain no sleeps or network. The 7 git tests run in 0.5 s and the 17 shell tests in 10 s. They are isolated and deterministic. The reducer tests drive `update` directly (insensitive to drawing changes). Failure messages are specific (rule ids in assert messages). No performance concern seen.

Positives worth keeping: the branch-set unit tests (boundary pairs at `MATCH_WINDOW_SECS` plus and minus one) and the real-git tests pin the hardest rule (created-here versus checked-out-here) with independent expectations, not re-implemented logic.

## Mutation results

At audit time not measured (since run: 12 of 12 mutants killed, see the sections below). The profile has `mutation: null` (no `cargo-mutants`), and the instruction was to skip mutation testing when disk is short: free space fell to 0, was reclaimed to under 1 GB, against a 15 GB floor. Deliberate mutants were not attempted either, because each needs a test rebuild the disk could not safely hold, and a mutant must never be left unrestored. Test strength was unmeasured at audit time; see the two sections at the end (12 deliberate mutants run afterwards, 0 survivors).

Mutants to try first once disk allows (not run): `worktree_branches.rs:181` `<=` to `<`; `pull_request.rs:819` 25 to 26; `pull_request.rs:889-893` tie-break `b.number.cmp(&a.number)` to ascending; `pull_request.rs:926` fork filter removed; `pull_request.rs:922` default-branch skip removed; `features/changes.rs:230-236` `Previous`/`Next` swapped; `features/changes.rs:132-146` `Unavailable` to `Absent`; `pull_request_row.rs:75-77` `checks` ignored (expected to survive, finding 1).

## Traceability

Real entry point here means the reducer `update`/shell `update` (no rendered-GUI test exists); "view" marks criteria whose drawing is untested.

| Criterion | Tests | End to end |
|---|---|---|
| US1-1 three branches, three PRs, newest first | `worktree_branch_set::every_branch_created_in_the_worktree_counts...`, `pull_request_list::every_pull_request_of_every_branch_is_listed...`, `main_tests::pr_list_branch_set_asks_every_branch_created_in_the_worktree` | Yes (shell with scripted `gh`) |
| US1-2 same branch reopened | `pull_request_list::a_branch_reopened_under_the_same_name...` | Parse only |
| US1-3 check marks | `every_state_carries_the_check_status...`, `a_check_status_maps_to_a_mark...`, `a_check_mark_keeps_the_row_height...` (vacuous) | No (view) |
| US1-4 other worktree excluded | `two_worktrees_of_one_repository_get_disjoint_sets`, `pr_list_branch_set_two_worktrees_each_ask_only_their_own` | Yes |
| US2-1, US2-2 own diff per PR (SC-002) | `choosing_the_second_then_the_third_shows_each_ones_own_files` | Yes (reducer) |
| US2-3 previous/next | `adjacent_follows_list_order...`, `stepping_selects_the_adjacent...`, `next_from_the_first_of_thirty...` | Reducer; disabled state: No (view) |
| US2-4 header | `the_header_names_the_shown_pull_request...` | Data only; drawing: No (view) |
| US2-5 working tree again | same test, `choosing_the_working_tree_returns_to_it` | Yes |
| US3-1, US3-3, US3-4 selection kept, state change, PR leaves | `a_refresh_adding_a_pull_request_keeps...`, `a_refresh_changing_the_selected_ones_state...`, `the_selected_pull_request_leaving_the_list...` | Yes (reducer) |
| US3-2 new branch appears at next trigger | `pr_list_refresh_branch_set_includes_a_branch_created_after_the_view_opened` | Yes (shell) |
| US4-1 empty line | `the_section_follows_the_states_table` (`PrSection::Empty`) | Data only; text: No (view) |
| US4-2 loading | same, `a_reread_with_a_list_in_hand_never_shows_the_loading_line` | Data only |
| US4-3 unavailable, no notice | `no_github_access_keeps_unavailable...`, `pr_list_read_without_gh_is_unavailable_and_silent`, `..._without_a_github_remote_...` | Yes (shell); text: No (view) |

Covered with a rendered/shell test: US1-1, US1-4, US2-1, US2-2, US2-5, US3-1, US3-2, US3-3, US3-4, US4-3 plus US4-2 and US2-3 at the model level = counted 11 in the frontmatter (US1-2 parse-level counted as covered). Untested in the view: US1-3, US2-4, and the display halves of US2-3, US4-1, US4-3.

Success criteria spot check: SC-007 (25 branches, 25 PRs) is covered as "one request for 25" (`pull_request_list.rs:217`) and not as "25 PRs listed"; FR-013 (working tree not blocked) is covered by `opening_reads_the_list_first_and_the_pull_requests_apart_from_it`. Tests tracing to nothing: none found. `traces` pointing at absent tests: not applicable (no list); every test named in `tasks.md` exists and runs.

## What was not audited

- Full workspace suite, clippy, fmt, `scripts/tests/*.test.sh` and the sandbox suite: not run (disk exhausted; see Suite).
- Mutation and deliberate mutants: not run (no tool, disk under the 15 GB floor). Test strength unmeasured.
- Coverage: profile has none.
- `main_tests.rs`: read only the branch-set tests and their helpers (`worktrees_with_history`, `ask_branch_set`, `asked_sets`, lines 10550-10760); the other 13 `pr_list_*` tests and the `rig` fixture were run, not read. Fixtures under `tests/fixtures/gh/` were not opened; their content was inferred from the assertions that pass against them.
- Source files (`pull_request.rs`, `features/changes.rs`, `shell/pr_status.rs`, `ui/changes.rs`) were read only where cited.
- Windows and macOS legs (CI only); the cross-platform path-slash gates.
- Documentation tasks (T013, T020, T026, T031, T035) and the user guide.
- Performance and load: no criterion, no test.
- No fresh-context subagent smell pass was run; findings come from one reader.

## Mutation results (client mutants, run after the audit)

Deliberate mutants in `micold-client`, each restored and the targets rerun green (`--test features_pr_changes --test icons`, `--lib -- pull_request_row`). Core mutants are in the next section.

| Mutant | Result | Killed by |
|---|---|---|
| `features/changes.rs` `Previous`/`Next` swapped | KILLED | `adjacent_follows_list_order_and_stops_at_the_ends`, `stepping_selects_...`, `next_from_the_first_of_thirty_reaches_the_last`, `a_failed_or_too_large_diff_keeps_the_header_and_stepping` |
| `section`: `Unavailable` to `Absent` | KILLED | `no_github_access_keeps_unavailable_without_a_list_and_raises_nothing` |
| `PullRequestRow::checks` ignored | KILLED after finding 1 was fixed (T038); the old assertions could not fail | `a_check_mark_keeps_the_row_height_and_is_drawn` |
| `check_mark` returns `None` | KILLED | `a_check_status_maps_to_a_mark_and_none_has_no_mark` |
| `adjacent` returns `None` | KILLED | the four stepping tests above |
| `Icon::ChevronLeft` glyph changed | KILLED | `icons.rs` `all_covers_every_variant_without_duplicates`, `glyph_maps_every_variant_to_its_pinned_codepoint` |

No client survivors. Findings 1, 3 (T017, T022, T024), 6 and the client part of 8, 12 are addressed by T038, T040, T044, T045.

## Core mutants (run after the audit)

Deliberate mutants in `micold-core`, each restored (file compared byte for byte with its backup) and the targets rerun green (`--test pull_request_list --test worktree_branch_set`, `--lib worktree_branches`).

| Mutant | Result | Killed by |
|---|---|---|
| `worktree_branches.rs:181` `<= MATCH_WINDOW_SECS` to `<` | KILLED | `the_match_window_is_two_seconds_either_way` |
| `pull_request.rs` `LIST_BRANCHES_PER_REQUEST` 25 to 26 | KILLED | `twenty_five_branches_are_one_request_and_twenty_six_are_two`, `the_chunks_are_merged_newest_first_then_by_number`, `a_failing_second_chunk_fails_the_whole_read` |
| `sort_newest_first` tie-break `b.number.cmp(&a.number)` to ascending | KILLED | `equal_creation_times_put_the_higher_number_first`, `the_chunks_are_merged_newest_first_then_by_number` |
| `parse_list` fork filter (`is_cross_repository`) removed | KILLED | `a_pull_request_from_a_fork_is_dropped` |
| `parse_list` default-branch skip removed | KILLED | `the_default_branchs_pull_requests_are_dropped` |
| `parse_list` without `reduce_checks` (T015) | KILLED | `every_state_carries_the_check_status_of_its_last_commit` |

No core survivors. Mutation results overall: 12 deliberate mutants (6 client, 6 core), 12 killed, 0 survived. Finding 8 (core part) and 12: the list-query tests also assert balanced braces and one alias per branch (T045); the `worktree_branches.rs` `copies_remote` asserts carry labels; `worktree_branch_set.rs` keeps its own `git_at` (a different date contract from `main_tests.rs`); `the_fake_answers_in_script_order_...` stays, since the client tests rely on the fake's script-order and `Passing` default.

## Close-unit addendum (2026-10-19)

Remediation after this audit: finding 1 fixed (the check-mark test now compares laid-out bounds with and without `.checks(..)` and is red against a no-op); finding 2 covered by the recorded visual pass (`visual-pass.md`, stand-in `gh`); finding 3: red after the fact recorded for T015, T017, T022, T024 in `cycle-log.md`; finding 4: tasks.md is the test list; finding 6 merged; 12 mutants run, 12 killed (sections above). Finding 5 (full gate) is recorded in `quickstart.md` Part A. Residual: reds for T015–T024 are after-the-fact breaks of the implementation, not original test-first runs (no history orders them), and the pass used a stand-in `gh`. Verdict after remediation: **PASS WITH NOTES**.
