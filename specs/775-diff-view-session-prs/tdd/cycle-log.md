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
