# TDD cycle log, feature 775

## M1 (T001-T015)

| Tasks | Behaviour | Red | Green |
|---|---|---|---|
| T001-T005 | `parse_list`, `list_args`, `list_branch` (core) | tests written before the implementation (commit order in part 1) | `tests/pull_request_list.rs` green |
| T006-T013 | reducer, shell wiring, `PullRequestRow`, mark mapping | **not shown** | `features_pr_changes`, `pr_list_read_*` (8 tests), component and showcase tests green |

Deviation: the tests of T006-T009 were written beside the implementation in the same commit
(ae3e7f51), not shown failing first; T007's shell tests followed (cd437a73). No red run exists and
none is invented here. Review B raised it as MAJOR; it is recorded, not repaired, because a red
phase cannot be produced after the fact without mutating the code. Review A's two MAJOR fixes
(disconnect ends the list read; no notice without a route) did get a test written for the first
(`pr_list_read_a_disconnect_ends_the_read_and_raises_no_notice`) alongside the fix; the second is
covered by the existing no-notice assertions.

## M2 (T016-T025)

| Tasks | Behaviour | Red | Green |
|---|---|---|---|
| T016-T019 | `split_pr_diff`, `diff_args`, 406 to `TooLarge`, Fake diff answers | tests (`pr_diff_split`, `pull_request_diff`) written before the implementation in the part 1 WIP commit; red run not shown | both green |
| T020 | `SessionLocation` ordering | written with the derive; red not shown | green |
| T021-T022 | `SourceChosen`, `PullRequestDiffRead`, memory, vanished PR | written with the reducer; red run not shown | `features_pr_changes` green |
| T023-T023b | `pr_diff_read_*` (4 tests: source runs and lists files, failure no notice, remotes timeout, disconnect) | written after the wiring (part 1 wrote the wiring uncompiled); red not shown, none invented | green |

Deviation as for M1: part 1 hit its context cap mid-milestone and handed over uncompiled code, so
no red phase was observed for M2; tdd-verify mutation testing in the close unit is the remedy.

## M3 (T027-T031) and close

M3 had no cycle-log entry when it merged: tests and source landed together in one commit, no red
run was observed. `tdd/verification.md` (close unit) records the audit: verdict FAIL for missing red
evidence, which cannot be repaired after the fact. Its 10 real mutation survivors (findings 1-9)
got tests in the close PR; those tests were written against existing source and pass, so they are
test-after by construction.
