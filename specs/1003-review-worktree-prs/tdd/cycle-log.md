# TDD cycle log — 1003 review-worktree-prs

No `tdd/test-list.md`: tasks.md's test-first order is the list (ledger *Handover*).

| Task | Phase | Command | Result |
|---|---|---|---|
| T002 | red | `cargo test -p micold-core --lib worktree_branches` (stubs) | 13 failed, 3 passed (the 3 "is dropped" cases hold on an empty stub) |
| T003 | green | same | 16 passed |
| T004 | red | not run apart: `read_branch_set` (T005) was spliced in with T003; the stub returned `[]`, which fails every T004 assertion | — |
| T005 | green | `cargo test -p micold-core --test worktree_branch_set` | 7 passed |
| T006 | red | compile: `list_in_chunks`, `LIST_BRANCHES_PER_REQUEST`, `head_branch`, `list_branches` missing (test written in part 1 against the API in the ledger) | does not compile |
| T007 | green | `cargo test -p micold-core --test pull_request_list --test pull_request_is_never_stored` | 15 + 3 passed |
| T009 | red | `cargo check -p micold-client --all-targets` | E0026: `ReadPullRequests` has no fields `entry`, `current` |
| T011 | green | `cargo test -p micold-client --test features_pr_changes` (with T012, one build) | passed |
| T010 | red | `cargo test -p micold-client --bin micold-ai-ide pr_list_branch_set`, T012 read swapped back to `[current]` | 2 failed: `left: [{"C"}]`, `right: [{"A", "B", "C"}]`; no-local-git case passed (it pins the old behaviour) |
| T012 | green | `cargo test -p micold-client --bin micold-ai-ide pr_list_` | 16 passed |
| T015 | red | not run apart: tests (fixture `pr_list_branches_checks.txt`, `checks` field) were written before `list_query`/`BranchPullRequest.checks` (T016) in one edit; against the old code they do not compile (`CheckStatus` field missing) | does not compile |
| T016 | green | `cargo test -p micold-core --test pull_request_list --test pull_request_is_never_stored` | 17 + 3 passed |
| T017 | red | not run apart: `check_mark` and `PullRequestRow::checks` did not exist when their tests were written | does not compile |
| T018 | green | `cargo test -p micold-client --test features_pr_changes`; `cargo test -p micold-client --lib -- check_mark pull_request_row` | 28 passed; 2 passed |
| T022 | red | not run apart: `adjacent`, `pr_header`, `Step` and `Msg::PullRequestStepped` did not exist when the tests were written | does not compile |
| T023 | green | `cargo test -p micold-client --test features_pr_changes --test icons --test icons_font` (with T024) | all passed |
| T028 | green at once | reducer tests of the refresh (new entry, state change, entry leaving) passed against M1/T023 | passed |
| T029 | green at once | `cargo test -p micold-client pr_list_refresh_branch_set`: M1's per-read branch set already includes a branch created later | 1 passed; T030 needs no fix |
| T032 | red (compile) | tests written first against the absent `PrSection`/`section` and the old `Absent` arm; the first run was not captured, so the red is that the tests could not compile before T033 | `section` undefined |
| T033 | green | `cargo test -p micold-client --test features_pr_changes` | 42 passed |
| T038 | red (after the fact) | `PullRequestRow::checks` made a no-op (`let _ = checks;`), `cargo test -p micold-client --lib -- pull_request_row` | `a_check_mark_keeps_the_row_height_and_is_drawn` FAILED at `pull_request_row.rs:155`: `left != right` failed: `Pending is drawn` (the old height/width-only assertions passed); restored, green |
| T017 | red (after the fact) | `check_mark` made to return `None` (`features/changes.rs:209`), `--test features_pr_changes` | `a_check_status_maps_to_a_mark_and_none_has_no_mark` FAILED at `features_pr_changes.rs:419`: `left: [None, None, None]`; restored, green |
| T022 | red (after the fact) | `adjacent` made to return `None` (`features/changes.rs:230`), `--test features_pr_changes` | 4 FAILED: `adjacent_follows_list_order_and_stops_at_the_ends`, `stepping_selects_the_adjacent_pull_request_and_stops_at_the_ends`, `next_from_the_first_of_thirty_reaches_the_last`, `a_failed_or_too_large_diff_keeps_the_header_and_stepping` (`features_pr_changes.rs:460`: `expected ReadPullRequestDiff, got None`); restored, green |
| T024 | red (after the fact) | `Icon::ChevronLeft` mapped to `'\u{e5cc}'` (`icons.rs:342`), `--test icons` | `all_covers_every_variant_without_duplicates` FAILED at `icons.rs:133`: `ChevronLeft and ChevronRight must not share a codepoint`; `glyph_maps_every_variant_to_its_pinned_codepoint` FAILED; restored, green |
| T015 | red (after the fact) | `reduce_checks` dropped from `parse_list` (`pull_request.rs:945`, `checks: CheckStatus::None`), `-p micold-core --test pull_request_list` | `every_state_carries_the_check_status_of_its_last_commit` FAILED at `pull_request_list.rs:306`: `merged and closed keep their check status too (FR-003)`, `left: [(55, Closed, None), (54, Merged, None), ...]`, `right: [(55, Closed, Failing), (54, Merged, Failing), ...]`; restored, green (17 passed) |

**Test list (T041):** `tasks.md` T002–T037 is this feature's test list; each test task names the behaviour and the acceptance scenario it traces to, so no separate `tdd/test-list.md` is kept.
