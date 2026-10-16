# TDD cycle log: 837 M1

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T003 | `PlainText` neutralising | 4 tests panic `not yet implemented` against `todo!()` stubs (red run: 37 of 37 `issue_details` tests failed, plus `github_gh_cli::read_details_past_its_deadline_runs_no_gh`) | `cargo test -p micold-core --test issue_details`: 37 passed; `github_description` 17 passed unchanged |
| T004 | model invariants (`RelatedList`, `Reference`, `PrListState`, `is_blocked`, `Debug`) | same red run, `todo!()` panics | same green run |
| T005 | parse of every fixture | same red run | same green run |
| T006 | request args, timeline paging, fake source, `GhCli` past deadline | same red run (incl. `github_gh_cli`) | same green run; `github_gh_cli` 12 passed |
| T011 | `to_json` and `details_error_json` | same red run | same green run |

The tests were written by the first M1 unit (commit 7d2ed276); the second unit added the API as
`todo!()` stubs, ran them red (38 failures, all `not yet implemented`), then implemented T007–T010
and T014 and ran them green in one step.

## Unit 3: tool path and published text

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T012, T016 | `github_issue` result carries `issue`; calls without it read nothing; refused calls carry none | T016's code predates these tests (unit 2). Red shown by mutation: with `row.insert("issue", …)` removed, 7 of 18 `github_issue` tests fail; restored | `cargo test -p micold-daemon --test mcp_create_worktree`: 41 passed |
| T027, T029 | `not_readable`, `details_error`, 200 ms bound cut with the 10 s sentence, blocked issue still created | same mutation run (the 4 US3 tests among the 7) | same green run |
| T013, T017 | published text in the tool and `github_issue` schema descriptions | `create_worktree_publishes_the_issue_object` failed before T017 | `cargo test -p micold-core --test mcp_tools_catalog`: 48 passed |
| A-F4 (review) | typed error beside a null issue | `a_null_issue_with_a_typed_error_reports_that_error` failed in the gate run against the pre-fix lib | issue_details 38 passed |

# TDD cycle log: 837 M2

Every test below was written and run red before the code it tests (Review B of M1 noted M1's daemon
code predated its tests). Stubs (`display_lines` returning no lines, `issue_details_request`
returning `None`, `set_picked_issue` only assigning) let the tests compile, so each red is an
assertion, not a missing symbol.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T019, T022 | `IssueDetails::display_lines` | `cargo test -p micold-core --test issue_details`: 5 failed (every-kind order and texts, warning only when blocked, `and N more; N you can't see`, Not readable in place, `at least N`); the empty-issue test passed against the empty stub, as it should | 44 passed |
| T020, T023, T024 | pick starts one bounded read; stale, cleared and closed-form answers dropped; Create does not wait; other sources and Jira start nothing | `cargo test -p micold-client --bin micold-ai-ide details_`: 10 failed on `Idle` where `Loading` was expected, 0 reads recorded; `details_other_sources_start_no_read` passed (nothing started yet) | `issue_source`: 47 passed |
| T028 | failed read is `Failed` with `e.message(repo)`; Not readable line; blocked issue still created | same red run | same green run |
| T021, T030 | details rows add height; Create/footer keep x, width and never move up | `layout_snapshot the_details_rows…`: "expected at least 9 more nodes … (255 vs 255)" before the view existed | 62 passed after regenerating the fixture |
| T025 | view: rows, blocked warning, failure line, scroll past five rows | the red above; the first green run showed `panel_placement` failing (dialog 796 of 800 high) and then `containment` (scroll overhang), both answered in the view and the gate (`ISSUE_DETAILS_CONTENT`) | `layout_snapshot`: 62 passed |
