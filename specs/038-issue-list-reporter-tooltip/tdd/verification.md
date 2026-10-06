---
feature: 038-issue-list-reporter-tooltip
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset present; extension copy graded against
verified_at: 89408097 # diff base 7e751a84~1 (91e72124); first code commit is 2e8ff0bb, so the base precedes all feature code
behaviors: 100 # inner U1-U100; the 28 outer A-rows are realised by those tests and were not classed separately
proven: 69
likely: 24
test_after: 2
no_test: 0
not_applicable: 5
high_smells: 0 # in the files read; see "What was not audited"
criteria_total: 28
criteria_covered: 28
mutation_score: unmeasured # no mutation tool in the profile; 6 deliberate mutants, 6 caught
mutants_survived: 0
suite: 5071 passed, 0 failed, 9 ignored, 429 binaries, 121s (warm shared target; MICOLD_SKIP_GH_LAUNCH_TEST=1)
---

# TDD Verification: Reporter, Labels and a Description Tooltip in the Issue List

**Verdict: FAIL.** Two behaviors are test-after by the cycle log's own admission (U44, U82 source half), and the test list still marks outer behaviors A16-A28 `PENDING` while every task that claims them is ticked `[x]`. The suite is green and no deliberate mutant survived, so the fixes are small: record the deviations and bring the list up to date. The audit was not independent in one respect only: it was run in a fresh context, not by the session that wrote the tests.

## Test-first evidence

History: the cycle-log commit hashes (`069165fa`, `ddb85c37`, `a34b07bc`, `b9ef217d`, `e841c7dd`, ...) resolve to objects but are not ancestors of HEAD, so the branch was rewritten after the log was written. Ordering was read from the rewritten commits instead (`b29d028d` tests and stubs, then `fda0208e` implementation; `2488ed23` then `400a3a81`; `15ad6cef` then `5822c88e`). M1 to M3 commits carry tests and implementation together, so for those the order rests on the log's red output.

| Class | Behaviors | Evidence |
| --- | --- | --- |
| PROVEN (69) | all U not listed below | red output in the cycle log (cycles 1-26) and, where the history is separable, the test commit precedes the implementation commit |
| LIKELY (24) | U6, U29, U30, U33, U39, U42, U47, U81 | passed on first run (guards of absence, or pre-existing behavior); the log records a deliberate-mutant failure instead of a red. Accepted as LIKELY on the strength of that mutant, not PROVEN |
| LIKELY | U63-U68, U70, U78-U80 | `15ad6cef` says "red not yet observed"; cycles 16-17 record a red afterwards against stubs. Order corroborated, red unverifiable |
| LIKELY | U71-U74, U83, U86 | `2488ed23` says unit-test red "not yet" observed; cycle 20 records it afterwards |
| TEST_AFTER (2) | U44 | cycle 6: "the gate was written after T010-T012, so it passed on first run". Red obtained only by two mutants (`Wrapping::None`, fixed row height) |
| TEST_AFTER | U82 (source-switch half) | cycle 21: "written after the green, so it had no red". Mutant check only. The loading half has a red (cycle 20) |
| NOT_APPLICABLE (5) | U10 (superseded by U16), U35, U48, U69, U77 | characterization baselines |

T013 (showcase pose) has no behavior id and no red (cycle log, "T013 -- no cycle").

## Existing tests changed by the feature

None was skipped, ignored, or had a threshold or exclusion lowered (checked: no `#[ignore]`, `#[allow]`, or CI/config changes in the feature commits). Changes to tests that existed before:

| Where | Before, after | Judgment |
| --- | --- | --- |
| `crates/micold-client/tests/idle_requests_no_frames.rs` (15ad6cef) | `the_frame_request_sits_behind_the_animating_guard` asserted exactly one frame request in `cdk/motion.rs`; renamed `the_frame_requests_are_the_guarded_one_and_the_timed_one`, asserts `(1, 1)` and keeps the literal `animating()` guard check, adds the `wake_at` body check | Intended by U63; the invariant is wider, not weaker. A renamed test: no filter references the old name |
| same file, `the_scan_actually_finds_the_rendering_layer` (5822c88e) | `redraw_call_sites().len() == 1` -> `== 2` | Same change as above |
| `crates/micold-client/tests/one_overlay_implementation.rs:160` (400a3a81), `material_boundary.rs:189` (0085bc33) | the `calls` scan counted `.tooltip(` as the rendering stack's `tooltip(`; now `if !preceded_by_ident` also excludes a method call | Narrowed guard. Positive controls added (`a_helper_ending_in_the_widget_name_is_not_a_use_of_it`, `a_method_with_a_widgets_name_is_not_a_widget_call`). No iced widget is built through a method called `tooltip`, so no hole found; still a loosened architecture gate |
| `crates/micold-client/src/main_tests.rs` (7adbfa96, 092b774d) | `issue_choosing_the_source_lists_open_issues` row text now has `ghost`; `constructed` expectations `[FAKE_GH]` -> `[FAKE_GH; 2]`, `; 2` -> `; 3` | Behavior change (U16, U95) with list items. The `constructed` counts pin how many sources were built, which no requirement states (implementation coupled) |
| `crates/micold-core/tests/github_parse.rs:68-85` (7adbfa96) | `row_text()` expectations gain `ghost` | Behavior change U16 |
| `crates/micold-client/tests/showcase_state.rs:250-270` (59788f8a) | highlight `None` -> `Some(1)`, `Some(0)` -> `Some(2)` | Behavior change, FR-028 |
| `tests/support/layout.rs`, `layout_snapshot.txt` | support refactor for floated layers; fixture only added | No assertion removed |

## Findings

| # | Sev | Needs | Finding | Evidence |
| --- | --- | --- | --- | --- |
| 1 | HIGH | docs | Test list says A16-A28 `PENDING` while T032-T055 (their tasks) are `[x]`; `planned_at`/`updated_at` still `d9deabff`. A completion claim with no evidence on the list, and `/speckit.implement` would redo them | `tdd/test-list.md:42-54`, `tasks.md` T032-T055 |
| 2 | HIGH | docs | U44 is test-after (gate written after implementation; red only by mutant) | `cycle-log.md:173` |
| 3 | HIGH | docs | U82 source half is test-after (no red; mutant only) | `cycle-log.md:601-603` |
| 4 | MED | code-test | The showcase's two-line rows of differing height (FR-028) are held by a screenshot only; `showcase_state.rs` asserts the seeded highlight, not row heights. Log records it as a follow-up | `cycle-log.md:200-207` |
| 5 | MED | code-test | FR-022 (Markdown removal) is carried by GitHub's `bodyText`; the only test of it parses a hand-written fixture that already holds the stripped text. The log says the fixtures were "written, not captured", but test messages say "the captured node" | `github_description.rs:141`, `github_description_pass.rs:174`, `cycle-log.md:521-522` |
| 6 | MED | code-test | Re-implemented expectation: expected label, spans and details are computed with `Issue::title_line()`/`emphasis()`, the functions `issue_rows` calls; only the #1100 label is a literal | `issue_picker_rows.rs:123-138` |
| 7 | MED | code-test | Implementation coupled: proves the view uses the hint by `include_str!` of `worktree_form.rs` with whitespace stripped | `issue_picker_rows.rs:174-179` |
| 8 | MED | docs | The loosened scan guards and `constructed` counts in the table above are recorded in the log as deviations but not in the test list | see table |
| 9 | LOW | code-test | Assertions without a message, against the profile's "every assertion carries a message" | `github_description.rs:389-393` |
| 10 | LOW | docs | Cycle-log commit hashes are not in HEAD's history; commit messages `15ad6cef`, `2488ed23` say red was not yet observed | `cycle-log.md:226,526,573` |
| 11 | LOW | docs | Profile baseline is stale: now 5071 passed in 429 binaries, profile says 2629 in 268 | `.specify/memory/tdd-profile.md` |

Smell pass over the files read (see below): no assertion-free, tautological, vacuous, doubled-subject, conditional-logic or skipped test; no sleeps or real clocks (`Instant::now()` is used only as an origin to which fixed offsets are added). Doubles are the profile's `FakeIssueSource`. Properties: the whole suite runs in about 2 minutes warm; the feature's core tests run in milliseconds; the layout gates are 0.3-0.4 s.

## Mutation results

No mutation tool in the profile (`mutation: null`). Deliberate mutants, one at a time, each restored with `git checkout` and the tree confirmed clean; afterwards the core suite (1828 passed, 0 failed) and the client `issue_source_state`, `layout_snapshot` and `picker` tests were green.

| # | Mutant | Behavior | Caught by |
| --- | --- | --- | --- |
| M1 | `tooltip.rs:60` `>= delay` -> `> delay` | U49 | 4 tests in `tooltip_rest` |
| M2 | `tooltip.rs:55` `> REST_TOLERANCE` -> `>=` | U52 | `a_move_of_exactly_the_tolerance...` |
| M3 | `github.rs:1137` `pages_read >= CAP` -> `>` | U94 | `the_pass_ends_where_the_list_would` |
| M4 | `worktree_form.rs:908` drop the seq check on a description page | U97 | `a_page_that_is_not_awaited_changes_nothing` |
| M5 | `github.rs:393` empty login no longer `ghost` | U2, U9 | `a_node_without_an_author_is_reported_by_ghost`, `row_text_shows_labels_only_when_present` |
| M6 | `picker.rs:136` `ROW_TOOLTIP_REST` 3 s -> 2 s | U74, U84, U88 | gate tests and `a_rows_tooltip_waits_three_seconds...` (the latter asserts the literal 3 s at `picker.rs:1169`) |

6 behaviors sampled across the reporter, rest-delay and description-pass paths; 0 survivors. Not exhaustive.

## Traceability

All 28 criteria (US1 1-9, US2 1-6, US3 1-13) trace to at least one existing test; every test name cited in the cycle log was found in `crates/` (the two not found, `the_match_text_omits_the_reporter` and `every_query_asks_for_the_body_text_in_the_shared_selection`, were replaced on purpose by U16 and U27).

| Criteria | Tests | End to end |
| --- | --- | --- |
| US1-1..5, 8, 9 | `issue_picker_rows.rs`, `gates/issue_rows_show_all_text.rs` over the real form view | Yes (headless real view) |
| US1-6, 7 | `picker_highlight_into_view.rs` (drives the real form and reducer) | Yes |
| US2-1..6 | `issue_source_state.rs`, `github_issue_lines.rs`, `issue_picker_rows.rs` | Yes |
| US3-1..5, 10, 11 | `tooltip_rest_glue.rs`, `gates/picker_row_tooltip_clears_its_row.rs` (the real form with a real cursor) | Yes for logic; the 3 s wall-clock and SC-003/004 timing only in quickstart B |
| US3-6..9 | `issue_picker_rows.rs`, `line_clamp.rs` tests, `tooltip_clamp.rs` | Yes |
| US3-12, 13 | `github_description.rs` on a hand-written fixture | Partly: depends on GitHub's `bodyText` (finding 5); confirmed in quickstart B |

Untested criteria: none. Tests tracing to nothing: none found.

## What was not audited

- Read in full: `github_issue_lines.rs`, `github_description.rs`, `github_description_pass.rs`, `github_privacy.rs`, `tooltip_rest.rs`, `tooltip_clamp.rs`, `tooltip_rest_glue.rs`, `issue_picker_rows.rs`. Not read for smells (only their cited behavior and the diffs of pre-existing files): `gates/picker_row_tooltip_clears_its_row.rs` (1061 lines), `gates/issue_rows_show_all_text.rs`, `picker_highlight_into_view.rs`, `support/tooltip.rs`, the in-crate tests in `ui/material/picker.rs` and `line_clamp.rs`, the new tests in `issue_source_state.rs`, `issues_are_requested_only_on_named_events.rs`, `main_tests.rs`. A smell count of 0 applies to the files read only. No fresh-context subagent was available for the smell pass.
- Mutation was a 6-mutant sample, not a tool run; no scoring.
- Coverage: no tool in the profile.
- The sandbox acceptance suite and the release-mode daemon half were not run; the feature does not use them.
- Windows and macOS: the suite ran on Linux only.
- Performance (SC-008) and the 3 s timing (SC-003/004): measured by quickstart B, not by tests; not re-measured.
- Quickstart Part B visual evidence was not reviewed.
- `specs/038-issue-list-reporter-tooltip/plan.md` has an uncommitted one-line edit that is not the auditor's; it was left as found.
