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
