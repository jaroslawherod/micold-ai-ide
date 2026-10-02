---
feature: 040-worktree-pr-ci-status
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 42 # US1 1–14, US2 1–10, US3 1–6, US4 1–12
planned_at: 52ccc184
updated_at: 52ccc184
suite_baseline: green # fast subset `mise run test-core` at 9501106d: 1510 passed, 0 failed, 7 ignored; workspace suite: CI's green `main` at 3d52e83e (merge-base)
---

# Test List: Pull Request and Check Status for Each Worktree

Traces name spec.md ids: `US<story>-<scenario>` for an acceptance scenario, `FR-…`, `SC-…`. A trace
to a contract (`PS` = contracts/pull-request-source.md, `RW` = contracts/reading-and-wire.md, `UI` =
contracts/pull-request-ui.md, `DM` = data-model.md, `R#` = research.md) is added where the spec
leaves the detail to the plan. The task that writes each test is named above its table; the same
ids stand on the tasks of [tasks.md](../tasks.md).

## Outer loop: acceptance behaviors

One per acceptance scenario of spec.md, in spec order. The profile's acceptance runner
(`sandbox_real_*`) drives the container runtime and cannot see the sidebar, so these are **client
integration tests**, at the highest entry point the repository can test without a display, without
the network and without `gh`:

- **Shell level** (`crates/micold-client/src/main_tests.rs`, named `pr_status_…`): messages go
  through `update_inner` with test `Capabilities` whose pull request source factory returns a
  `FakePullRequestSource`; results are read from the application state and from the messages sent
  to the daemon. A10–A12, A14, A25, A27–A42.
- **Reader level** (`crates/micold-client/tests/`): the render-free readers the view calls — the row
  projection, `worktree_tooltip`, `worktree_menu_items` — and the open handler with a recording
  `LinkOpener`. A1–A9, A13, A15–A24, A26. This is weaker than the shell level: it composes the
  modules but does not start from a daemon message. tasks.md has no shell test task in M4 and M5,
  so story 1's row scenarios and story 2 stop here (reported by `/speckit.tdd.plan`).

A1–A9 feed the row projection with the statuses `parse_status` yields from the recorded fixtures of
`crates/micold-core/tests/fixtures/gh/`, so one test crosses parsing, selection, check reduction
and the projection. The rendered half — colour, both themes, the real `gh`, the browser, the delete
confirmation, the 100 ms response — is quickstart §B (T067), run by `visual-pass`.

### User Story 1 — the indicator

Tests: A1–A9 in `crates/micold-client/tests/features_sidebar.rs` (T030); A10–A12 and A14 in
`main_tests.rs` (T025); A13 in `crates/micold-client/tests/icons.rs` (T031).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | With the statuses read from `pr_three_branches.txt`, the row of the worktree whose branch has the open pull request projects `Some(RowPullRequest)` in state open | US1-1, FR-001, FR-002 | example | PENDING | |
| A2  | With the statuses read from `pr_draft.txt`, the row projects state draft, a value different from open | US1-2, FR-002 | example | PENDING | |
| A3  | With the statuses read from `pr_three_branches.txt`, the row of the branch whose only pull request is merged projects state merged, with no check status | US1-3, FR-002, FR-003 | example | PENDING | |
| A4  | With the statuses read from `pr_closed.txt`, the row projects state closed, with no check status | US1-4, FR-002, FR-003 | example | PENDING | |
| A5  | With the statuses read from `pr_checks_failing.txt` (a failed check beside others), the row projects check status failing | US1-5, FR-003, FR-008 | example | PENDING | |
| A6  | With the statuses read from `pr_checks_pending.txt` (no failed check, one still running), the row projects check status pending | US1-6, FR-003, FR-008 | example | PENDING | |
| A7  | With the statuses read from `pr_checks_passing.txt`, the row projects check status passing | US1-7, FR-003, FR-008 | example | PENDING | |
| A8  | With the statuses read from `pr_no_checks.txt`, the row projects state open and no check status | US1-8, FR-003 | example | PENDING | |
| A9  | With the statuses read from `pr_three_branches.txt`, the row of the branch without a pull request projects `None` | US1-9, FR-001 | example | PENDING | |
| A10 | With the switch on, `gh` not found — and, separately, the source answering `Unavailable` for a missing sign-in — after `Attached` + `CatalogChanged` the statuses are empty and the state holds no notice, dialog or error line | US1-10, FR-025, SC-004 | example | PENDING | |
| A11 | With `RemoteList` answering no github.com remote, the fake source records no call, the statuses are empty and no notice exists | US1-11, FR-025, FR-026 | example | PENDING | |
| A12 | While a reading is under way (the fake source has not answered), a selection message through `update_inner` is applied at once, and the update call that started the reading returned without the source having been called on the update loop | US1-12, FR-021, SC-005 | example | PENDING | |
| A13 | The four state glyphs and the three check glyphs are seven different codepoints, so every state and check status differs by shape | US1-13, FR-009 | example | PENDING | |
| A14 | With a github.com remote that is a fork, every call the fake source records names the project's own `owner/name` and no other, and a branch the source returns no entry for has no status | US1-14, FR-006, FR-031 | example | PENDING | |

### User Story 2 — the tooltip and Open pull request

Tests: A15–A18, A20, A24 in `features_sidebar.rs` (T041); A19 in
`crates/micold-client/src/main_tests.rs` as `pr_status_open_*` (T043); A21 in
`crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs` (T044); A22 in
`features_sidebar.rs` (T059); A23 in `crates/micold-client/tests/worktree_menu_pull_request.rs`
(T042).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A15 | For a row with an open, failing pull request, `worktree_tooltip` holds, after today's lines, `Pull request: #<number> <title>`, `PR state: open` and `Checks: failing`, each on its own line | US2-1, FR-010 | example | PENDING | |
| A16 | For each review decision — approved, changes requested, review required — the tooltip holds `Review: approved` / `Review: changes requested` / `Review: review required` | US2-2, FR-010, SC-002 | example | PENDING | |
| A17 | For a pull request with no review decision the tooltip has no `Review:` line | US2-3, FR-010 | example | PENDING | |
| A18 | With `None` for the pull request, `worktree_tooltip` returns today's expected strings byte for byte | US2-4, FR-011 | example | PENDING | |
| A19 | `WorktreeMsg::PullRequestOpenRequested(dir)` makes the recording `LinkOpener` receive exactly the stored address once, and selection, sessions and sidebar state are equal before and after | US2-5, FR-013, FR-014, SC-009 | example | PENDING | |
| A20 | A 200-character title is cut to 72 characters ending in `…`, with `#<number>` before it on the same line | US2-6, FR-011 | example | PENDING | |
| A21 | Source gate: `worktree_tooltip` and the row menu builder call nothing in `shell/`, so building a tooltip or a menu sends nothing and reads no disk | US2-7, FR-012, SC-008 | example | PENDING | |
| A22 | With `age_secs` above 600 the tooltip holds `Read: <n> min ago` after the review line | US2-8, FR-019 | example | PENDING | |
| A23 | For a row without a status, `worktree_menu_items` equals today's items entry for entry, with no **Open pull request** | US2-9, FR-013 | example | PENDING | |
| A24 | For a row with a status the tooltip is one string in which no line holds the pull request's address | US2-10, FR-013 | example | PENDING | |

### User Story 3 — the removal suggestion

Tests: A25, A27–A30 in `main_tests.rs`, named `pr_status_merged_…` (T049); A25 (row and tooltip)
and A26 in `features_sidebar.rs` (T050).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A25 | After a reading whose merged branch the daemon answers `Contained`, the branch is in `removable`, its row projects `removable: true` and the tooltip's last line is `Cleanup: merged — this worktree can be removed (right-click, Delete)` | US3-1, FR-015 | example | PENDING | |
| A26 | The menu of a row with the mark, and the message its **Delete** entry sends, equal those of a row without the mark | US3-2, FR-016, SC-010 | example | PENDING | |
| A27 | A reading that ends with a removable branch sends the daemon no message that deletes a worktree, a session or a branch | US3-3, FR-016, SC-010 | example | PENDING | |
| A28 | A merged branch the daemon answers `Beyond` keeps state merged and is not in `removable` | US3-4, FR-017 | example | PENDING | |
| A29 | A closed pull request is not asked about in `MergedBranchCheck` and is not in `removable` | US3-5, FR-017 | example | PENDING | |
| A30 | A branch with a merged pull request and a newer open one shows the open one, is not asked about in `MergedBranchCheck` and is not in `removable` | US3-6, FR-004, FR-017 | example | PENDING | |

### User Story 4 — staying current

Tests: A31, A40–A42 in `main_tests.rs`, named `pr_status_…` (T025); A32–A39 in `main_tests.rs`,
named `pr_status_tick_…` and `pr_status_refresh_…` (T058).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A31 | With the switch on, `DaemonMsg::Attached` for the active project followed by `CatalogChanged` makes the fake source record exactly one call, with no user message sent | US4-1, FR-018, SC-001 | example | PENDING | |
| A32 | `Message::PrStatusTick` with the project held and the switch on makes the source record one further call | US4-2, FR-018, SC-003 | example | PENDING | |
| A33 | A tick reading whose answer differs from the last replaces the branch's status, and selection, expansion, scroll, filter and sessions are equal before and after | US4-3, FR-020 | example | PENDING | |
| A34 | `WorktreeMsg::RefreshFinished` makes the source record one further call | US4-4, FR-018 | example | PENDING | |
| A35 | A tick and three refreshes that arrive while a reading is under way add no call until it ends, and exactly one call when it ends | US4-5, FR-022 | example | PENDING | |
| A36 | After `RefreshFinished` the refresh control is idle and "Worktree list refreshed." is shown while the reading it started has not answered | US4-6, FR-027 | example | PENDING | |
| A37 | After the source answers `RateLimited { until }`, a tick with `now < until` adds no call and the statuses are those of the last successful reading | US4-7, FR-024, SC-006 | example | PENDING | |
| A38 | After the source answers `Passing`, the statuses are unchanged, no notice, dialog or error line exists, and the next tick adds a call | US4-8, FR-019, FR-025 | example | PENDING | |
| A39 | After the source answers `Unavailable`, statuses and `removable` are empty, no notice, dialog or error line exists, and the next tick adds a call | US4-9, FR-025 | example | PENDING | |
| A40 | After a switch to another project, and after a disconnect, the old project's statuses are empty and no call for it is recorded however many ticks follow | US4-10, FR-018, SC-006 | example | PENDING | |
| A41 | With the switch off, `Attached` + `CatalogChanged` records no source call, sends no `RemoteList` for the reading and leaves the statuses empty | US4-11, FR-026, FR-030, SC-006 | example | PENDING | |
| A42 | `SettingsChanged` turning the switch on while the project is held and listed records one source call without a tick | US4-12, FR-018, FR-030 | example | PENDING | |

## Inner loop: unit behaviors

No property-based library is in the profile, so every invariant is sampled at its boundaries
(`kind: example`). `approval` is the profile's homegrown layout snapshot
(`UPDATE_LAYOUT_SNAPSHOT=1`), regenerated deliberately by the task that says so.

### `crates/micold-core/tests/fixtures/gh/pr_*.txt`: the recorded answers

Recorded by T001; the check itself sits in `crates/micold-core/tests/pull_request_parse.rs` (T003).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | Each of the 14 files of PS §5 is present, and none holds a token (`gho_`, `ghp_`, `github_pat_`) or an `Authorization` header | FR-028, FR-032, PS §5 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::every_recorded_answer_is_present_and_holds_no_credential` |

### `crates/micold-core/src/pull_request.rs`: `status_query`, `status_args`

Tests: `crates/micold-core/tests/pull_request_query.rs` (new, T002).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U2  | `status_query(1)` equals the pinned one-line document: `o0`, `r0`, `rateLimit`, the `pr` fragment with check counts by state and no check nodes | FR-023, PS §2 | example | DONE | `crates/micold-core/tests/pull_request_query.rs::u2_query_for_one_branch_is_pinned` |
| U3  | `status_query(50)` equals its pin and holds `o<i>` and `r<i>` for every `i` in 0..50 and none for 50 | FR-023, SC-006 | example | DONE | `crates/micold-core/tests/pull_request_query.rs::u3_query_for_fifty_branches_is_pinned` |
| U4  | No branch name appears in the query document | FR-031 | example | DONE | `crates/micold-core/tests/pull_request_query.rs::u4_no_branch_name_appears_in_the_document` |
| U5  | `status_args` yields exactly one `-f b<i>=<branch>` pair per branch for names holding `"`, `$`, a space, a leading `-`, `true` and `123` | FR-031, PS §2 | example | DONE | `crates/micold-core/tests/pull_request_query.rs::u5_branch_names_pass_through_unchanged` |
| U6  | `status_args` holds `--hostname github.com`, `--include`, `owner`, `name`, the query and the branches, and nothing else: no `-F`, no token, no header argument | FR-005, FR-028, FR-031 | example | DONE | `crates/micold-core/tests/pull_request_query.rs::u6_args_are_exactly_the_contract` |

### `crates/micold-core/src/pull_request.rs`: `split_response`, `parse_status`

Tests: `crates/micold-core/tests/pull_request_parse.rs` (new, T003), against T001's fixtures.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U7  | `split_response` yields status, headers and body for `\r\n` and for `\n` line ends, and finds a header whatever the case of its name | PS §5 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::split_response_reads_status_headers_and_body_with_crlf` (+2 more in the file) |
| U8  | `split_response` is `None` without a status line, and `None` without an empty line | FR-019, PS §5 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::split_response_without_a_status_line_is_none` (+1 more in the file) |
| U9  | `parse_status` on `pr_three_branches.txt` yields open, merged, and no entry for the third branch, keyed by the branches passed in | FR-001, FR-002, SC-002 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::parse_status_three_branches_keys_by_given_names` |
| U10 | `pr_draft.txt` and `pr_closed.txt` yield draft and closed with number, title, url and head as recorded | FR-002, SC-002 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::parse_status_draft` (+1 more in the file) |
| U11 | `pr_checks_failing.txt`, `pr_checks_pending.txt`, `pr_checks_passing.txt` and `pr_no_checks.txt` yield an open pull request whose checks are failing, pending, passing and none | FR-003, FR-008, SC-002 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::parse_status_reduces_checks` |
| U12 | `pr_review_states.txt`: `APPROVED`, `CHANGES_REQUESTED` and `REVIEW_REQUIRED` yield the three review states; `null` and any other value yield none | FR-010, SC-002 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::parse_status_reads_review_decisions_in_branch_order` (+2 more in the file) |
| U13 | `pr_truncated.txt` is a failure, never a partial map | FR-019 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::parse_status_truncated_answer_is_passing` |
| U14 | A missing alias, a `null` repository and an `errors` entry are each a failure, never a partial map | FR-019, FR-025 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::parse_status_missing_alias_is_passing` (+4 more in the file) |
| U15 | `pr_cross_repository.txt` (a fork's pull request with the same head-branch name) yields no entry for the branch | FR-005, FR-006 | example | DONE | `crates/micold-core/tests/pull_request_parse.rs::parse_status_only_cross_repository_is_empty` |

### `crates/micold-core/src/pull_request.rs`: `select_pull_request`

Tests: `crates/micold-core/tests/pull_request_select.rs` (new, T004).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U16 | An open node is selected over a newer merged one | FR-004 | example | DONE | `crates/micold-core/tests/pull_request_select.rs::an_open_node_is_selected_over_a_newer_merged_one` |
| U17 | Of two open nodes the one created later is selected | FR-004 | example | DONE | `crates/micold-core/tests/pull_request_select.rs::of_two_open_nodes_the_one_created_later_is_selected` |
| U18 | With no open node, the one created later of a merged and a closed node is selected | FR-004 | example | DONE | `crates/micold-core/tests/pull_request_select.rs::with_no_open_node_the_later_created_of_merged_and_closed_is_selected` |
| U19 | A cross-repository open node is ignored and the same-repository closed one is selected | FR-005 | example | DONE | `crates/micold-core/tests/pull_request_select.rs::a_cross_repository_open_node_is_ignored` |
| U20 | Only cross-repository nodes yield `Ok(None)` | FR-005, FR-006 | example | DONE | `crates/micold-core/tests/pull_request_select.rs::only_cross_repository_nodes_select_nothing` (+1 more in the file) |
| U21 | `OPEN` with `isDraft: true` is `Draft`, and with `false` is `Open` | FR-002 | example | DONE | `crates/micold-core/tests/pull_request_select.rs::an_open_node_is_a_draft_or_open_by_is_draft_with_its_checks_reduced` (+1 more in the file) |
| U22 | An unknown `state` is `Err(Unreadable)` | FR-019, PS §3 | example | DONE | `crates/micold-core/tests/pull_request_select.rs::an_unknown_state_is_unreadable` |

### `crates/micold-core/src/pull_request.rs`: `reduce_checks`

Tests: `crates/micold-core/tests/pull_request_checks.rs` (new, T005).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U23 | Each failed name alone (`FAILURE`, `CANCELLED`, `TIMED_OUT`, `ACTION_REQUIRED`, `STARTUP_FAILURE`; contexts `FAILURE`, `ERROR`) is `Failing` | FR-008 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::each_state_name_alone_reduces_to_its_group` |
| U24 | Each not-finished name alone (`QUEUED`, `IN_PROGRESS`, `PENDING`, `WAITING`, `STALE`; contexts `PENDING`, `EXPECTED`) is `Pending` | FR-008 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::each_state_name_alone_reduces_to_its_group` |
| U25 | Each finished name alone (`SUCCESS`, `NEUTRAL`, `SKIPPED`, `COMPLETED`; context `SUCCESS`) is `Passing` | FR-008 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::each_state_name_alone_reduces_to_its_group` |
| U26 | One failed count beside pending and passing counts is `Failing` | FR-008, US1-5 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::a_failed_count_beside_pending_and_passing_is_failing` |
| U27 | A pending count beside passing counts, with no failed one, is `Pending` | FR-008, US1-6 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::a_pending_count_beside_passing_is_pending` |
| U28 | Only skipped and neutral counts are `Passing` | FR-008 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::only_skipped_and_neutral_is_passing` |
| U29 | An unknown state name alone is `Pending` | FR-008, R4 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::an_unknown_state_name_alone_is_pending` |
| U30 | A `null` rollup is `None`, and counts that are all zero are `None` | FR-003, US1-8 | example | DONE | `crates/micold-core/tests/pull_request_checks.rs::no_rollup_is_no_checks` (+1 more in the file) |

### `crates/micold-core/src/pull_request.rs`: `reading_failure`, `rate_limit_pause`

Tests: `crates/micold-core/tests/pull_request_failure.rs` (new, T006), against T001's fixtures and
034's stderr fixtures.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U31 | `RunOutcome::TimedOut` is `Passing` | FR-019, FR-021 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::timed_out_is_passing` |
| U32 | `SpawnFailed` is `Unavailable` when `classify` says `ToolMissing`, else `Passing` | FR-025 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::missing_gh_is_unavailable` (+1 more in the file) |
| U33 | A GraphQL `RATE_LIMITED` error, HTTP 429 and an HTTP 403 that says "rate limit" are each `RateLimited { until }`, also when the same answer would read as no access | FR-024 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::graphql_rate_limited_error_pauses_until_reset` (+4 more in the file) |
| U34 | A `NOT_FOUND` error on `repository`, HTTP 401, and `classify` saying `NotSignedIn` or `NoAccess` are each `Unavailable` | FR-025 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::repository_not_found_answer_is_unavailable` (+2 more in the file) |
| U35 | `classify` saying `Offline` is `Passing` | FR-019 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::offline_is_passing` |
| U36 | HTTP 5xx and an answer `parse_status` cannot read are `Passing` | FR-019 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::http_502_is_passing` (+3 more in the file) |
| U37 | `Retry-After: 30` pauses until `now + 30` | FR-024 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::retry_after_adds_its_seconds` (+1 more in the file) |
| U38 | `X-RateLimit-Remaining: 0` pauses until the `X-RateLimit-Reset` time | FR-024 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::remaining_zero_waits_for_the_reset` |
| U39 | A secondary limit with remaining above 0 and no `Retry-After` pauses until `now + 60`, not until the reset header | FR-024 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::remaining_above_zero_ignores_the_reset` |
| U40 | A reset time in the past pauses until `now + 1` | FR-024 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::a_reset_in_the_past_pauses_one_second` (+1 more in the file) |
| U41 | A `Retry-After` or reset value that does not parse is skipped as if absent | FR-024, PS §5 | example | DONE | `crates/micold-core/tests/pull_request_failure.rs::unparsable_retry_after_is_skipped` (+3 more in the file) |

### `crates/micold-core/src/pull_request.rs`: `PullRequestSource`, `FakePullRequestSource`, chunking

Tests: `crates/micold-core/tests/pull_request_source.rs` (new, T007).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U42 | `FakePullRequestSource` returns its scripted answers call by call and records `(owner/name, branches)` of each call | FR-031, PS §1 | example | DONE | `crates/micold-core/tests/pull_request_source.rs::fake_answers_in_order_and_records_calls` |
| U43 | An empty branch list answers an empty map and makes no call | FR-023, FR-026 | example | DONE | `crates/micold-core/tests/pull_request_source.rs::no_branches_makes_no_request` |
| U44 | 50 branches are read in one chunk, 51 in two (50, 1) and 120 in three (50, 50, 20) | FR-023 | example | DONE | `crates/micold-core/tests/pull_request_source.rs::branches_are_read_in_chunks_of_fifty` |
| U45 | A failing second chunk fails the whole reading with that failure and returns nothing of the first chunk | FR-019, FR-023 | example | DONE | `crates/micold-core/tests/pull_request_source.rs::a_failing_chunk_ends_the_reading_with_nothing_half_read` (+1 more in the file) |

### `crates/micold-core/src/pull_request.rs`: `PullRequestStatus` is never stored

Tests: `crates/micold-core/tests/pull_request_is_never_stored.rs` (new, T008).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U46 | Source gate: `PullRequestStatus` derives neither `Serialize`, `Deserialize` nor `Debug` | FR-032, SC-011 | example | DONE | `crates/micold-core/tests/pull_request_is_never_stored.rs::the_status_derives_no_serialisation_and_no_debug` |
| U47 | Its hand-written `Debug` output holds the number and the enums and neither the title nor the address | FR-032, SC-011 | example | DONE | `crates/micold-core/tests/pull_request_is_never_stored.rs::debug_output_holds_the_number_and_the_enums_and_neither_title_nor_address` |

### `crates/micold-core/src/settings.rs`: `pr_status_enabled`

Tests: `crates/micold-core/tests/settings_roundtrip.rs` (extended, T014).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U48 | `pr_status_enabled` is `false` by default and when read from a `settings.json` written without it | FR-030 | example | PENDING | |
| U49 | `true` survives a write and a read, and `SETTINGS_VERSION` is unchanged | FR-030, DM §5 | example | PENDING | |

### `crates/micold-core/src/git.rs`: `containment`, `branch_tip`, `is_ancestor`

Tests: `crates/micold-core/tests/git_containment.rs` (new, T015).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U50 | `containment` is `Contained` for a tip equal to `head` and for a tip that is an ancestor of it | FR-015 | example | PENDING | |
| U51 | `containment` is `Beyond` for a tip that is not an ancestor of `head` | FR-017 | example | PENDING | |
| U52 | `containment` is `Unknown` with no tip and with unknown ancestry | FR-017 | example | PENDING | |
| U53 | `RealGit::branch_tip` on a temporary repository returns the branch's commit, and none for a missing branch | FR-015, RW §3 | example | PENDING | |
| U54 | `RealGit::is_ancestor` is true for a branch at and behind a commit, false for one ahead of it, and unknown for a commit id not in the repository | FR-015, FR-017 | example | PENDING | |
| U55 | `FakeGit` answers `branch_tip` and `is_ancestor` as scripted | RW §3 | example | PENDING | |

### `crates/micold-core/src/protocol/`: the wire change

Tests: `crates/micold-core/tests/protocol_roundtrip.rs` and `schema_hash.rs` (extended, T016).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U56 | `ClientMsg::MergedBranchCheck` and `OperationResult::MergedBranchCheck` survive a round trip | FR-015, RW §3 | example | PENDING | |
| U57 | `SettingsSet { pr_status_enabled }` (`Some` and `None`) and `DaemonSettings.pr_status_enabled` survive a round trip | FR-029, FR-030, RW §4 | example | PENDING | |
| U58 | The schema hash equals its new pin and `PROTOCOL_VERSION` is one above the previous | RW §5 | example | PENDING | |

### `crates/micold-daemon/src/server.rs`: the `MergedBranchCheck` arm

Tests: `crates/micold-daemon/tests/merged_branch_check.rs` (new, T017), against a temporary
repository.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U59 | A branch whose tip equals `head`, and one behind `head`, answer `Contained` | FR-015 | example | PENDING | |
| U60 | A branch with a commit after `head` answers `Beyond` | FR-017, US3-4 | example | PENDING | |
| U61 | A missing branch, and a `head` that is not a local object, answer `Unknown` | FR-017 | example | PENDING | |
| U62 | A `head` that is not 40 or 64 hexadecimal characters answers `Unknown` without git being run | RW §3 | example | PENDING | |
| U63 | The answers are one per query, in query order | DM §6 | example | PENDING | |
| U64 | 50 queries are answered and 51 are refused | RW §3 | example | PENDING | |
| U65 | A project that is not a repository is refused with the refusal `RemoteList` gives | RW §3 | example | PENDING | |
| U66 | No ref and no file of the repository differs after the call | FR-016, SC-010 | example | PENDING | |

### `crates/micold-daemon/src/server.rs`, `catalog.rs`, `state.rs`: the setting

Tests: `crates/micold-daemon/tests/pr_status_setting.rs` (new, T018).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U67 | `SettingsSet { pr_status_enabled: Some(true) }` is persisted and reported in the next `Welcome` | FR-030 | example | PENDING | |
| U68 | The change is broadcast as `SettingsChanged` to two connected clients | FR-029, FR-030 | example | PENDING | |
| U69 | `SettingsSet { pr_status_enabled: None }` leaves the stored value as it is | RW §4 | example | PENDING | |

### `crates/micold-client/src/features/pr_status.rs`: the first reading and the switch

Tests: `crates/micold-client/tests/features_pr_status.rs` (new, T023).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U70 | `Held` then `ListingArrived` yields one `Read { seq }`; a second `ListingArrived` yields none | FR-018 | example | PENDING | |
| U71 | `EnabledChanged { true }` after off, when held and listed, yields `Read` | FR-018, FR-030 | example | PENDING | |
| U72 | `EnabledChanged { true }` when not held, and while awaiting the listing, yields none | FR-018, RW §1 | example | PENDING | |
| U73 | `EnabledChanged` with the value the state already has yields none and changes nothing | RW §2 | example | PENDING | |
| U74 | `EnabledChanged { false }` empties `statuses` and `removable`, sets `read_at` and `pause_until` to `None` and the phase to `Idle` | FR-029, DM §3 | example | PENDING | |
| U75 | `Finished` with `Ok` replaces `statuses` and sets `read_at` to the time the reading started | FR-019 | example | PENDING | |
| U76 | `Finished` with `Unavailable` empties `statuses` and sets `read_at` to `None` | FR-025 | example | PENDING | |
| U77 | `Finished` with `Passing` leaves `statuses` and `read_at` as they were | FR-019 | example | PENDING | |
| U78 | `Finished` with `RateLimited { until }` leaves `statuses` as they were and sets `pause_until` | FR-024 | example | PENDING | |
| U79 | `Finished` with a `seq` other than the reading's changes nothing | FR-019, DM §3 | example | PENDING | |
| U80 | `Released` empties `statuses`, sets `held` to false and keeps `pause_until` | FR-024, SC-007 | example | PENDING | |
| U81 | After `Released`, no message but `Held` followed by `ListingArrived` yields a `Read`, and that pair yields exactly one | FR-018, SC-007 | example | PENDING | |
| U82 | `ListingArrived` with `now` before `pause_until` yields none | FR-024 | example | PENDING | |
| U83 | Over sampled message sequences, no two `Read` effects occur without a `Finished` or a `Released` between them | FR-022 | example | PENDING | |
| U84 | `Held` then `ListingArrived` with the switch off yields none | FR-026, FR-030 | example | PENDING | |

### `crates/micold-client/src/shell/`: where a reading may start (source gate)

Tests: `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs` (new, T024).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U85 | `shell::pr_status::start` is called only from the lines that handle S1 and S2 | FR-018, RW §1 | example | PENDING | |
| U86 | `Msg::ListingArrived` is sent from one line, inside the `CatalogChanged` arm | FR-018, RW §1 | example | PENDING | |
| U87 | `PullRequestSource::read` is called from one place | FR-021, FR-022 | example | PENDING | |
| U88 | No line of `crates/micold-client/src` logs a pull request's title or address | FR-032, SC-011 | example | PENDING | |

### `crates/micold-client/src/shell/pr_status.rs`, `daemon_sync.rs`, `workspace.rs`: one reading

Tests: `crates/micold-client/src/main_tests.rs`, named `pr_status_…` (T025), with
`FakePullRequestSource`. A10–A12, A14, A31 and A40–A42 share the file.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U89 | After `Attached` + `CatalogChanged`, `RemoteList` is sent, then the source is read with the listing's branches, deduplicated, in listing order, detached worktrees left out, and its answer is in `state.pr_status.statuses` | FR-007, FR-018a, FR-023 | example | PENDING | |
| U90 | A later `CatalogChanged` records no further call | FR-018, RW §1 | example | PENDING | |
| U91 | `RemoteList` unanswered for 10 s keeps the statuses (`Passing`) | FR-019, FR-021 | example | PENDING | |
| U92 | `Displaced` and `Refused { ProjectBusy }` for the active project each empty the statuses and start nothing | FR-018, SC-007 | example | PENDING | |
| U93 | A take-over (`Attached` + `CatalogChanged` after a displacement) records exactly one call | FR-018, SC-007 | example | PENDING | |
| U94 | `SettingsChanged` turning the switch off empties the statuses | FR-029 | example | PENDING | |
| U95 | A finished reading leaves selection, expansion, scroll, filter and sessions of the application state equal to before | FR-020 | example | PENDING | |
| U96 | After each failure kind (`Unavailable`, `Passing`, `RateLimited`) the state holds no notice, dialog or error line | FR-025, SC-004 | example | PENDING | |
| U97 | A reading sends no `ClientMsg::WorktreeRefresh` | FR-018a | example | PENDING | |

### `crates/micold-client/src/features/sidebar.rs`: the row projection

Tests: `crates/micold-client/tests/features_sidebar.rs` (extended, T030). A1–A9 share the file.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U98 | A worktree row whose branch is in `statuses` projects `Some(RowPullRequest)` with that status and `age_secs` equal to `now − read_at` | FR-001, DM §4 | example | PENDING | |
| U99 | A detached worktree, the "Default" entry (whatever branch the root has checked out) and session rows project `None` | FR-007 | example | PENDING | |
| U100 | With empty statuses every row's projection equals today's | FR-001, FR-011 | example | PENDING | |

### `crates/micold-client/src/icons.rs`: the indicator glyphs

Tests: `crates/micold-client/tests/icons_font.rs` and `icons.rs` (extended, T031). A13 shares them.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U101 | `PrOpen`, `PrDraft`, `PrMerged`, `PrClosed`, `ChecksPassing`, `ChecksPending` and `ChecksFailing` each have a codepoint in the shipped font | FR-009, FR-033 | example | PENDING | |

### `crates/micold-client/src/ui/material/pull_request_indicator.rs` and the showcase

Tests: `crates/micold-client/tests/showcase_completeness.rs` and `material_builder_api.rs`
(extended, T032).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U102 | `PullRequestIndicator` passes the builder-form gate | FR-033 | example | PENDING | |
| U103 | The catalogue holds a "Pull request indicator" entry with 12 poses: 4 states without checks, open and draft with each of 3 check statuses, and 2 stale forms | FR-033, FR-009 | example | PENDING | |

### `crates/micold-client/src/features/settings.rs`: the switch

Tests: `crates/micold-client/tests/features_settings.rs` and `settings_sections.rs` (extended,
T033).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U104 | `Msg::PrStatusToggled(v)` sets the draft to `v` | FR-029 | example | PENDING | |
| U105 | Applying a changed draft sends `SettingsSet { pr_status_enabled: Some(v) }`; an unchanged draft sends `None` | FR-029, FR-030 | example | PENDING | |
| U106 | The draft follows a `SettingsChanged` from the daemon | FR-029 | example | PENDING | |
| U107 | The section's title is "GitHub" | FR-029, UI §5 | example | PENDING | |

### Layout: the row with an indicator and the Settings section

Tests: `crates/micold-client/tests/support/covered_states.rs` (extended, T034) and the layout gates
over `tests/fixtures/layout_snapshot.txt` (regenerated in T039).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U108 | In the row with an indicator (open, failing) the indicator is inside the row, left of the action cluster, and the name does not overlap it; at the narrowest sidebar width the indicator's width is the same and the name is what shrank | FR-009, UI §6 | approval | PENDING | |
| U109 | The geometry of a row without a pull request with the switch on equals that row's with the switch off | FR-001 | approval | PENDING | |
| U110 | In Settings → GitHub the checkbox, its note and the issue mapping are not clipped, and in the showcase entry none of the 12 poses is clipped | FR-029, FR-033 | approval | PENDING | |

### `crates/micold-client/src/features/sidebar.rs`: `worktree_tooltip`

Tests: `crates/micold-client/tests/features_sidebar.rs` (extended, T041). A15–A18, A20 and A24
share the file.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U111 | With a status, the lines `Pull request:`, `PR state:`, `Checks:`, `Review:` follow today's lines in that order | FR-010 | example | PENDING | |
| U112 | A merged, a closed and a check-less open pull request have no `Checks:` line | FR-003, FR-010 | example | PENDING | |
| U113 | A title of 72 characters is kept whole; one of 73 is cut to 72 ending in `…` | FR-011, UI §3 | example | PENDING | |
| U114 | Control characters and line breaks in a title become spaces, so the title stays on its line | FR-010, FR-011 | example | PENDING | |

### `crates/micold-client/src/ui/mod.rs`: `worktree_menu_items`

Tests: `crates/micold-client/tests/worktree_menu_pull_request.rs` (new, T042). A23 shares the file.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U115 | For a row with a status the items hold **Open pull request** directly above **Delete** | FR-013 | example | PENDING | |

### `crates/micold-client/src/shell/pr_status.rs`: opening the pull request

Tests: `pr_status_open_*` in `crates/micold-client/src/main_tests.rs` (T043), with a recording
`LinkOpener` set through `Capabilities::with_link_opener`. A19 stands beside them.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U116 | An address that does not start with `https://github.com/` opens nothing | FR-014, UI §4 | example | PENDING | |
| U117 | A row that lost its status before the entry was chosen opens nothing | FR-014, UI §4 | example | PENDING | |

### `crates/micold-client/src/icons.rs`: the menu entry's glyph

Tests: `crates/micold-client/tests/icons_font.rs` (extended, T044). A21 is T044's gate.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U118 | The glyph of the **Open pull request** entry (`OpenInBrowser`, or the existing variant that draws `open_in_new`) is in the shipped font | FR-013, UI §1 | example | PENDING | |

### `crates/micold-client/src/features/pr_status.rs`: `removable`

Tests: `crates/micold-client/tests/features_pr_status.rs` (extended, T048).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U119 | `Finished` with `Ok` stores its `removable` | FR-015 | example | PENDING | |
| U120 | A branch in `removable` whose status is not merged, or that is not in `statuses`, is dropped | FR-017, DM §3 | example | PENDING | |
| U121 | `Unavailable`, `Released` and switching off each empty `removable` | FR-025, FR-029 | example | PENDING | |

### `crates/micold-client/src/shell/pr_status.rs`: the merged-branch question

Tests: `crates/micold-client/src/main_tests.rs`, named `pr_status_merged_…` (T049). A25 and
A27–A30 share the file.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U122 | After the source answers, `MergedBranchCheck` holds one query per merged branch with its `head`, and none for open, draft or closed ones | FR-015, FR-018a | example | PENDING | |
| U123 | With no merged branch, `MergedBranchCheck` is not sent | FR-023, RW §2 | example | PENDING | |
| U124 | A merged branch answered `Unknown` is not in `removable` | FR-017 | example | PENDING | |
| U125 | No answer within 10 s, a refusal, and an answer list of another length each apply the statuses with `removable` empty | FR-017, FR-021 | example | PENDING | |

### `crates/micold-client/src/features/sidebar.rs`: the removal mark

Tests: `crates/micold-client/tests/features_sidebar.rs` (extended, T050). A25 and A26 share the
file.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U126 | `RowPullRequest.removable` is true for a branch in `removable` and false for one that is not | FR-015 | example | PENDING | |
| U127 | A merged row that is not removable has no `Cleanup:` line | FR-017 | example | PENDING | |

### Layout: the can-be-removed chip

Tests: `crates/micold-client/tests/support/covered_states.rs` (extended, T051); snapshot
regenerated in T053.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U128 | In the row with an indicator and the **can be removed** chip, and in its narrow form, both are inside the row and neither overlaps the other or the name | FR-009, FR-016, UI §6 | approval | PENDING | |

### `crates/micold-core/src/pull_request.rs`: `is_stale`

Tests: `crates/micold-core/tests/pull_request_stale.rs` (new, T055).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U129 | `is_stale` is false 600 s after `read_at` | FR-019 | example | PENDING | |
| U130 | `is_stale` is true 601 s after `read_at` | FR-019 | example | PENDING | |
| U131 | `is_stale` is false when `now` is before `read_at` | FR-019, R13 | example | PENDING | |

### `crates/micold-client/src/features/pr_status.rs`: triggers, `again`, the pause

Tests: `crates/micold-client/tests/features_pr_status.rs` (extended, T056).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U132 | `Trigger { Interval }` and `Trigger { Refresh }` each yield `Read` from `Idle` | FR-018 | example | PENDING | |
| U133 | A `Trigger` while off, while not held, while awaiting the listing and while paused yields none | FR-018, FR-024, FR-026 | example | PENDING | |
| U134 | Three `Trigger { Refresh }` during a reading and its `Finished` yield exactly one further `Read` | FR-022 | example | PENDING | |
| U135 | `Trigger { Interval }` during a reading yields no `Read`, then or at its `Finished` | FR-022 | example | PENDING | |
| U136 | After `RateLimited { until }` a trigger at `until − 1` yields none, and a pending `again` is dropped | FR-024 | example | PENDING | |
| U137 | The first trigger at `until` yields `Read` | FR-024 | example | PENDING | |
| U138 | The pause is kept across `Released` and a new `Held` + `ListingArrived` | FR-024, SC-006 | example | PENDING | |
| U139 | A `Trigger` 60 s after a reading started abandons it and yields `Read` with a new `seq`, and the old answer is then dropped; at 59 s it does not | FR-021, FR-022, RW §2 | example | PENDING | |

### `crates/micold-client/src/shell/subscriptions.rs`: the interval timer

Tests: `crates/micold-client/tests/idle_subscriptions.rs` (extended, T057).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U140 | The 300 s subscription is absent with the switch off | FR-026, SC-006 | example | PENDING | |
| U141 | It is absent in a window that does not hold its project | FR-018, SC-007 | example | PENDING | |
| U142 | It is present with the switch on and the project held | FR-018 | example | PENDING | |

### `crates/micold-client/src/shell/daemon_sync.rs`: the refresh and tick triggers

Tests: `crates/micold-client/src/main_tests.rs`, named `pr_status_refresh_…` and `pr_status_tick_…`,
and `pr_status_is_read_only_on_named_events.rs` (T058). A32–A39 share `main_tests.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U143 | `RefreshTimedOut` starts a reading too, and a reading started by a refresh covers the branches the updated listing shows | FR-018, FR-018a | example | PENDING | |
| U144 | A reading that fails after a refresh changes neither the refresh control's state nor its notice, and adds no error | FR-027 | example | PENDING | |
| U145 | Source gate: `shell::pr_status::start` is called from exactly the lines that handle S1 to S5 | FR-018, FR-022 | example | PENDING | |

### `crates/micold-client/src/features/sidebar.rs`: the `Read:` line

Tests: `crates/micold-client/tests/features_sidebar.rs` (extended, T059). A22 shares the file.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U146 | With `stale` false (`age_secs` 600) the tooltip has no `Read:` line; with `stale` true (`age_secs` 601) it reads `Read: 10 min ago` | FR-019 | example | PENDING | |
| U147 | At 119 minutes the line reads `Read: 119 min ago`; from 120 minutes it reads `Read: 2 h ago` | FR-019, UI §3 | example | PENDING | |
| U149 | `RowPullRequest.stale` is false with `now` 600 s after `read_at` and true at 601 s | FR-019, DM §4 | example | PENDING | |

### Layout: the stale indicator

Tests: `crates/micold-client/tests/support/covered_states.rs` (extended, T060); snapshot
regenerated in T064.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U148 | The row with a stale indicator has the geometry of the row with the current form | FR-009, FR-019 | approval | PENDING | |

## Invariants and edge cases still to place

None open. Where each spec edge case is held:

- A project with no worktrees makes no request: U43. Hundreds of checks: U2 (counts by state, no
  check nodes). Very many worktrees: U44. Several pull requests for one branch, reopened, reused:
  U16–U18. Same branch name in a fork: U15, U19, U20.
- Worktree with no branch, the "Default" entry, several sessions in one worktree: U89, U99.
- Worktree removed, renamed or re-branched between readings, and a project closed during a reading:
  the join is by branch name only (U98, A9) and a late answer is dropped (U79, U80).
- Merged pull request whose last commits never reached this machine: U54, U61, U124.
- Application restarted, nothing stored: U46, U49 (the setting is the only stored field).
- Several windows: U80, U81, U92, U93, U141.

Held by no automated behavior, and why:

- **FR-034** (same on every OS): the feature adds no `cfg` arm; CI runs every suite above on Linux,
  macOS and Windows, and T068 confirms no arm was added.
- **FR-035** (user guide): T040, T047, T054 and T065, checked by CI's user-guide gate.
- **US1-13 in both themes, SC-005's 100 ms, the wording of the Settings note (FR-029), the tooltip
  closing when the cursor leaves the row (US2-10, 029's unchanged behaviour), the browser and the
  delete confirmation as rendered**: quickstart §B (T067). A12, A13 and A24 pin the testable part.

## Out of scope

- A pull request in the repository a fork was made from (spec Clarifications, FR-006; U15 and U20
  pin that it is not shown).
- GitHub Enterprise and other hosts; anonymous access (spec Assumptions, FR-028).
- Individual checks, their names and logs; individual reviewers and comments (spec Assumptions).
- Acting on a pull request, notifications, a configurable interval, clicking the indicator, a
  tooltip that stays open (spec Assumptions).
- Fetching, and examining uncommitted changes, for the removal suggestion (spec Assumptions).
- More than 10 newer pull requests with one head-branch name in a connection (PS §2 "Known bound").
- A pull request indicator for the "Default" entry (spec Assumptions; U99 pins its absence).

## Verification commands

From `.specify/memory/tdd-profile.md`, run from the workspace root:

- single: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- file: `scripts/build-lock.sh cargo test --test {file}`
- suite: `scripts/build-lock.sh cargo test --workspace`
- core-only inner loop: `mise run test-core`
- binary-module tests (`main_tests.rs`): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide pr_status`
- layout snapshot (approval), regenerated only by the task that says so: `UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`

A filter that matches no test exits 0: read the `N passed` count, never the exit code alone.
