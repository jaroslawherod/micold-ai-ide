---
feature: 775-diff-view-session-prs
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 4ec233b8
behaviors: 7 # cycle-log rows (M1 x2, M2 x4) plus M3 with no row; no tdd/test-list.md exists
proven: 0
likely: 0
test_after: 7
no_test: 0
high_smells: 0
criteria_total: 14 # FR-001..FR-014; SC-001 and visual criteria have no automated test
criteria_covered: 14
mutation_score: unmeasured # no mutation tool; 21 hand-applied mutants, scope: pr_diff.rs, diff_failure, SourceChosen
mutants_survived: 13 # of 21 run: 10 real, 3 equivalent or low value; 1 no-op by construction not counted
suite: 6559 passed, 0 failed, 10 ignored, 513 result lines (cargo test --workspace, 150 min wall incl. build-lock waits)
---

# TDD Verification: PRs in the diff view (775)

**Verdict: FAIL.** No behavior has a recorded red run, and several surviving mutants sit inside
behaviors the cycle log marks green: `SourceChosen(None)` does not forget the remembered PR,
`SourceChosen(Some)` guards are untested, and `split_pr_diff` boundary and `ModeOnly` paths are
unpinned.

The audit was not independent in one respect: the auditor is a fresh session, but the report relies
on the cycle log's honest statements (it concedes no red for M1 T006-T013 and all of M2).

## Preflight

- No `tdd/test-list.md`: the feature was not planned through the tdd extension, so there are no
  behavior ids or `traces`. Behaviors below are the cycle-log rows; the spec's FR list is the
  traceability base. Ordering evidence is correspondingly weaker.
- Profile has `mutation: null` and `coverage: null`: deliberate mutants only.
- Suite at HEAD `4ec233b8`: green (6559 passed, 0 failed, 10 ignored). No baseline failures.
- Worktree verified clean after every mutant (`git status --short` empty); the targeted tests were
  re-run green after the last restore (see "Restore verification").

## Test-first evidence

| Behavior (cycle-log row) | Class | Evidence |
| --- | --- | --- |
| M1 T001-T005 `parse_list`, `list_args`, `list_branch` | TEST_AFTER | Log claims "tests written before the implementation (commit order)" with no red output. History contradicts it: `78bc94fa` adds `pull_request_list.rs` and `pull_request.rs` in one commit. Discrepancy (log overstates). |
| M1 T006-T013 reducer, shell, row, mark | TEST_AFTER | Log: "not shown"; tests and implementation in `ae3e7f51`. |
| M2 T016-T019 `split_pr_diff`, `diff_args`, 406, Fake | TEST_AFTER | Tests, fixtures and implementation all in `c4bb7661` (uncompiled WIP); no red run. |
| M2 T020 `SessionLocation` ordering | TEST_AFTER | Derive and test in `c4bb7661`; log "red not shown". |
| M2 T021-T022 `SourceChosen`, `PullRequestDiffRead`, memory | TEST_AFTER | Same commit as reducer; no red. |
| M2 T023-T023b `pr_diff_read_*` | TEST_AFTER | Tests in `aa23047a` after the wiring (`c4bb7661`); log admits no red. |
| M3 T027-T031 refresh triggers, stale mark | TEST_AFTER | No cycle-log entry exists for M3 at all. `5a718971` lands tests and source together. |

Every class is `TEST_AFTER` under the rubric (no red recorded). The cycle log is honest about M1
T006+ and M2; it is wrong about M1 T001-T005 and silent about M3.

### Existing tests changed

`pr_status_is_read_only_on_named_events.rs`: the one-line pins (`reads.len() == 1`, `factory.len() == 1`)
became `== 3` plus new `.list_branch(` and `.diff(&repo` pins (lines 263-295). Extended with named
new call sites, not weakened. `pull_request_is_never_stored.rs` removals are a refactor of the
fixture list, replaced by wider coverage. No skipped, ignored or filtered tests, no thresholds touched.

### tasks.md vs evidence

T015, T026, T033 (visual passes) unticked: consistent (visual pass was "not observed"). All other
ticked tasks have tests present. No tick without a test.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Surviving mutant in a DONE behavior: `SourceChosen(None)` stops forgetting the remembered PR; no test fails. Switching away and back would re-open the PR the user left. `choosing_the_working_tree_returns_to_it` asserts only `source == Changes`. | `features/changes.rs:600-602`; `features_pr_changes.rs:346` |
| 2 | HIGH | Surviving mutants: `SourceChosen(Some(n))` guards. Dropping `!listed` and dropping `current` both pass: choosing an unlisted number, and re-choosing the shown PR (no duplicate read), are untested. | `features/changes.rs:618`; tests at `features_pr_changes.rs:300-345` |
| 3 | HIGH | Surviving mutant: removing `p.diff != PrDiffLoad::Failed` passes: the retry-after-failure path of `SourceChosen` is untested. | `features/changes.rs:616` |
| 4 | HIGH | Surviving mutant: `diff_failure` returning a constant `ReadingFailure::Passing` for every non-406 failure passes. `any_other_failure...` checks `matches!(Failed(_))` for 404 and one exact case that equals the constant. A rate limit or unavailable `gh` via the diff path is unpinned. | `pull_request.rs:896`; `pull_request_diff.rs:61-71` |
| 5 | HIGH | Surviving mutant: the large-file boundary `>` to `>=` passes. Fixture has 5001 changed lines, one above `MAX_CHANGED_LINES` (5000); exactly 5000 and 4999 are not pinned. | `review/pr_diff.rs:111`; `tests/fixtures/gh/pr_diff_large.txt` |
| 6 | HIGH | Surviving mutant: `ChangeKind::ModeOnly` branch disabled passes. No mode-change fixture exists although T016 lists the cases and the code has the branch. | `review/pr_diff.rs:94` |
| 7 | MED | Surviving mutant: dropping `GIT binary patch` detection passes; the binary fixture uses only `Binary files`. | `review/pr_diff.rs:78`; `pr_diff_binary.txt` |
| 8 | MED | `patch` bytes are never asserted: `trim_end()` on the patch passes; only `parse_unified` hunk count is checked. | `review/pr_diff.rs:114`; `pr_diff_split.rs:82-88` |
| 9 | MED | `SourceChosen(None)` early return when already on the working tree has no test (mutant removing it passes). With a remembered PR awaiting its list, the user's choice is not forgotten. | `features/changes.rs:596` |
| 10 | HIGH (process) | No red evidence for any behavior; M3 absent from the cycle log; M1 T001-T005 log claim contradicted by history. Not repairable after the fact. | cycle-log.md; `78bc94fa`, `c4bb7661`, `5a718971` |
| 11 | MED | No `tdd/test-list.md`: no ids, no `traces`, no way to check the claim set mechanically. | `specs/775-diff-view-session-prs/tdd/` |
| 12 | LOW | `an_added_and_a_deleted_file_are_told_apart` and `any_other_failure_is_failed...` hold two behaviors each (eager). `a_too_large...and_a_failed_read` loops over cases with the assertions inside a `for`; both iterations assert, so it is not vacuous. | `pr_diff_split.rs:33`, `pull_request_diff.rs:61`, `features_pr_changes.rs:413` |

No assertion-free, tautological or vacuous tests found in the files read. `pr_diff_read_*` shell
tests use `FakePullRequestSource` from `micold-core` (the recorded helper), and assertions name the
rule. Fixture reads use `CARGO_MANIFEST_DIR`, deterministic, no clock or network.

## Mutation results

Tool: none (profile `mutation: null`). Method: deliberate single mutants, one at a time, source
restored by writing back the original bytes after every run. Scope: `split_pr_diff`
(`review/pr_diff.rs`), the 406 mapping (`pull_request.rs` `diff_failure`/`diff_outcome`),
`Msg::SourceChosen` (`features/changes.rs`). Tests run: `pr_diff_split`, `pull_request_diff`,
`features_pr_changes`, `features_changes`. Not exhaustive.

| Mutant | File:line | Result | Judgment |
| --- | --- | --- | --- |
| `>` to `>=` large threshold | `pr_diff.rs:111` | SURVIVED | Real (finding 5) |
| ModeOnly branch disabled | `pr_diff.rs:94` | SURVIVED | Real (finding 6) |
| `deleted_file = true` to `false` | `pr_diff.rs:66` | caught | `an_added_and_a_deleted_file...` |
| `rename_to.or(new_path).or(old_path)` order swapped new/old | `pr_diff.rs:82` | SURVIVED | Equivalent for all fixtures (both sides same path), low value |
| odd-length header guard disabled | `pr_diff.rs:144` | SURVIVED | Malformed-header path; low value, note only |
| octal decode `*8` to `*7` | `pr_diff.rs:176` | caught | quoted fixture |
| removed counter dropped | `pr_diff.rs:57` | caught | rename/delete |
| `patch` trimmed | `pr_diff.rs:114` | SURVIVED | Real (finding 8) |
| section start condition `&& offset > 0` | `pr_diff.rs:30` | caught | several |
| `GIT binary patch` dropped | `pr_diff.rs:78` | SURVIVED | Real (finding 7) |
| `/dev/null` literal broken | `pr_diff.rs:122` | caught | add/delete |
| 406 check requires long stderr | `pull_request.rs:892` | caught | `http_406_is_too_large` |
| `contains("406") \|\| contains("HTTP 404")` | `pull_request.rs:892` | caught | `any_other_failure...` |
| 406 also matched on `TimedOut` | `pull_request.rs:889-895` | SURVIVED | Equivalent in practice (a timeout carries no HTTP status); not a finding |
| `Failed(reading_failure(..))` to constant `Passing` | `pull_request.rs:896` | SURVIVED | Real (finding 4) |
| `if !listed \|\| current` to `if current` | `changes.rs:618` | SURVIVED | Real (finding 2) |
| same to `if !listed` | `changes.rs:618` | SURVIVED | Real (finding 2) |
| drop `&& p.diff != Failed` | `changes.rs:616` | SURVIVED | Real (finding 3) |
| `remembered.remove` dropped in `SourceChosen(None)` | `changes.rs:600` | SURVIVED | Real (finding 1) |
| drop early return `source == Changes` | `changes.rs:596` | SURVIVED | Real (finding 9) |
| `remembered.insert` dropped in `SourceChosen(Some)` | `changes.rs:~622` | caught | `each_entry_remembers_its_own_choice` |
| `code: 0` made `_` in `diff_outcome` | `pull_request.rs:~902` | NOT RUN | text matched two sites; mutant script refused it |
| `TooLarge` mapped to `Failed` in `PullRequestDiffRead` | `changes.rs:649` | NOT RUN | killed to free the shared lock; the `matches!(p.diff, TooLarge)` assertion at `features_pr_changes.rs:431` is expected to catch it, not demonstrated |

A mutant that makes the source mutated by a no-op edit (`let _ = view.diff_offset;`) was also run
and survived by construction; ignore it.

Tally: 21 mutants run (2 further not run), 8 caught, 10 real survivors, 3 equivalent or low-value
survivors. A hand sample, not comparable to a tool run.

### Restore verification

`git status --short` was empty after each pass. A final run of `features_pr_changes`,
`features_changes`, `pr_diff_split`, `pull_request_diff` on the restored tree is recorded in the
chat report (see below); the full workspace suite was green at HEAD before any mutant.

## Traceability

| Criterion | Tests | Real entry point |
| --- | --- | --- |
| FR-001, FR-002 list for branch / no-session worktree | `pr_list_read_*`, `a_worktree_with_no_session_lists_the_same`, `pull_request_list.rs` | Yes (shell test) |
| FR-003 four states | `the_four_states_have_four_distinct_marks`, row component test | Yes |
| FR-004 diff base...head, incl. deleted head | `pr_diff_split.rs`, `pull_request_diff.rs`, `pr_diff_read_runs_the_source_and_lists_the_files` | Partial: gh itself is faked, deleted-head not exercised against GitHub (manual) |
| FR-005 return to working tree | `choosing_the_working_tree_returns_to_it` | Reducer only; forgetting untested (finding 1) |
| FR-006 default working tree | existing `features_changes` | Yes |
| FR-007 live update | `pr_list_refresh_*` | Yes |
| FR-008 no notice | `*_raises_no_notice`, `nothing_was_reported` | Yes |
| FR-009 no blocking | `opening_reads_the_list_first_and_the_pull_requests_apart_from_it` | Reducer level |
| FR-010 no cross project | `pr_list_read_*` two projects | Yes |
| FR-011 newest first | `pull_request_list.rs` | Unit |
| FR-012 stale | `pr_list_refresh_failure_keeps_the_list_and_marks_it_stale_by_age` | Yes |
| FR-013 per-entry / vanished PR | `each_entry_remembers_its_own_choice`, `a_pull_request_that_leaves_the_list...` | Reducer |
| FR-014 setting off | `the_setting_off_gives_no_section...`, `the_setting_turned_off_returns...` | Yes |

All FRs have at least one test; 3 of 14 only at reducer level. SC-001 (two interactions) and the
visual criteria have no automated test (T015, T026, T033 not done). Tests tracing to nothing: none
found.

## What was not audited

- Rendering and the three visual passes (T015, T026, T033): "not observed" per the ledger.
- UI code (`ui/changes.rs`, `ui/material/pull_request_row.rs`), `shell/changes.rs`,
  `shell/pr_status.rs`, `github.rs` `GhCli::diff` and `list_branch` bodies: no mutants run.
- M1 list code (`parse_list`, `list_args`, `list_branch`) and M3 refresh logic: read, not mutated.
- Real `gh` behavior (HTTP 406 text, deleted-head PR): fakes only.
- Coverage: no tool.
- Suite wall time of 150 minutes is mostly build-lock contention with other worktrees, not a
  speed measure; per-test speed not assessed.
- Smell pass was done by the single auditing session, no subagent; only the feature's files were read.

## Remediation (close PR)

This file is the point-in-time audit at `4ec233b8`. Findings 1-9 got tests in the close PR (tasks R001-R005): 1, 9 in `features_pr_changes.rs` (`choosing_the_working_tree_forgets_the_remembered_pull_request`, `choosing_the_working_tree_while_on_it_keeps_a_remembered_pull_request`); 2, 3 there too (`choosing_an_unlisted_number_does_nothing`, `choosing_the_shown_pull_request_again_reads_nothing`, `choosing_a_pull_request_whose_diff_failed_reads_it_again`); 4 in `pull_request_diff.rs`; 5-8 in `pr_diff_split.rs`. Findings 10 and 11 stay open.
