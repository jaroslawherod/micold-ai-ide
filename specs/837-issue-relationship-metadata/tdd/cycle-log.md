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
