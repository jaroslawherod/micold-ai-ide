---
feature: 1272-review-source-dropdown
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no override, no preset
verified_at: b92d372b8 # short SHA audited (origin/main after PR #1418)
behaviors: 13 # cycle-log rows (tasks.md stands in for the absent test-list.md)
proven: 9
likely: 0
test_after: 0
no_test: 0
not_applicable: 4 # T001, T008 (structural), T024 (snapshot recording), T026 (manual visual pass)
high_smells: 1
criteria_total: 20 # the acceptance scenarios of US1-US4; FRs and SCs traced below
criteria_covered: 20
mutation_score: 67 # scope: 9 deliberate mutants by hand (no mutation tool): 6 caught, 3 survived. Not a tool score
mutants_survived: 3 # all three non-equivalent, all in the T021 view wiring
suite: "scoped to -p micold-core -p micold-client: 9389 passed, 4 failed (root-only permission tests, unrelated), 15 ignored, 479 binaries; 164 s warm"
---

# TDD Verification: Review UI source dropdown and two-column layout (#1272)

> **Remediated in the close PR** (after this audit): T028–T033 cleared findings F1–F7 and recorded the review F8 asked for; the three
> surviving header mutants and the 240 px column mutant now fail the new tests
> (`tdd/cycle-log.md`, *Close*). This report stays as audited at `b92d372b8`.

**Verdict: FAIL.** Three non-equivalent deliberate mutants survive in `T021`'s view wiring (`ui/changes.rs`):
no test sends a gesture through the real review header, so the dismiss, arrow-key and tooltip hookups can break unseen.

The TDD discipline itself held: every behavior's test shipped in the commit that made it pass, with a recorded red.
The core model (`review::source_menu`) and the reducer picks are strongly tested (all six mutants there died).
The failure is confined to the glue between the reducer, the `MenuSelect` component and the header, plus two
assertions in the layout gate that compare against the production constants they test.

No `tdd/test-list.md` exists: the feature was not planned through this extension. `tdd/cycle-log.md` and
`tasks.md` (whose test tasks name the behaviors) stood in. The audit is independent of the session that wrote
the tests (it read every cited file cold). The feature merged as four rebase-merged PRs (#1355 design, #1368 M1,
#1383 M2+M3, #1418 M4), so per-commit order is visible in the history (not squashed).

## Test-first evidence

History source: `git log --stat origin/main --grep='(1272)'`. Each behavior row is one cycle-log row (tasks in the
first column). A feature commit adds the tests and the source together, so `PROVEN` rests on the recorded red.

| Behavior (tasks) | Class | Evidence |
| --- | --- | --- |
| T002/T005 `rows()`, one test per FR-005 state | PROVEN | `ea3132ef5` adds tests and source; red: bodies `todo!()`, 16 FAILED |
| T003/T006 `stops()`, `keeps_open` | PROVEN | same commit and red run |
| T004/T007 `label`, `full`, `tooltip` | PROVEN | same commit and red run |
| T009, T012-T014 menu open/move/pick/dismiss, refresh, label | PROVEN | `4666b25b1`; compile red (types absent); recorded mutants (no close in `SourceChosen`, no re-seat) FAILED two tests. I re-ran an analogous mutant (T012 reset): caught |
| T010, T015 chips, `show_working_tree` | PROVEN | `4666b25b1`; recorded mutant FAILED two tests. I removed `show_working_tree` from `CommittedToggled`: both named tests FAILED |
| T011 Files pick, Files across source change | PROVEN | `4666b25b1`; compile red |
| T016-T018 `MenuSelect`, `menu_select_fits`, showcase poses | PROVEN | `81d350dfa`; compile red; recorded mutants: 7 of 8 FAILED (the 8th is the library button's own keys) |
| T019/T021/T022 left column holds only the list, header holds the control | PROVEN | `1fd8a4bc8`; red run on the pre-M4 view, FAILED at the one-child assertion |
| T020/T023 two-column body, widths | PROVEN | same commit; red FAILED at the 260 px assertion |
| T001, T008 moves under green suite | NOT_APPLICABLE | structural, no new behavior; existing suites green unmodified |
| T024 covered states, re-recorded layout snapshot | NOT_APPLICABLE | a recording has no red by construction (see finding 8) |
| T026 visual pass | NOT_APPLICABLE | manual; `visual-pass/m4.md` records it and caught a real wrap bug |

Fix commits without a cycle-log row: `dd2a4150f` (a kept Review tab shown again opens with the list closed) has
no test in its own commit, but `opening_a_view_resets_the_control` (`features_pr_changes.rs`) already covered it
and fails without the fix (mutant 4). `28a7c46d3` and `37437b583` change drawing and one constant after visual
passes; geometry gates and the snapshot cover them.

**Sources agree.** Cycle log counts match the files: `source_menu` tests 10 (`features_pr_changes`) and 7
(`features_changes`), `menu_select_fits` 8. No discrepancy found between log and history.

**Existing tests changed by the feature** (diff of `crates/*/tests` outside the feature's own new files):

- `crates/micold-client/tests/gates/review_bar_fixed.rs:298-302` (finding 4): the 1270 assertion "the drawer is half the area, area under 720" became `wanted = full.min(area.width / 2.0)`.
  Before: `assert!(area.width < 2.0 * full)` and `drawer.width == area.width / 2.0`. After: `drawer.width == min(360, area/2)`.
  Justified (the fixed file column widens the area past 720), still an exact value, but the half-area branch is no longer pinned anywhere in this gate.
- `crates/micold-client/tests/gates/session_area_fits.rs` (1fd8a4bc8): the six removed lines are the `covered_states` import, a doc comment, a `placed.push` tuple, `PROGRESS_BAR` and the `(slot, words, bar)` bindings, re-pathed for the new header (`[1,0,0,1]` to `[2,0,0,1]`). No assertion line was removed; I did not diff every use of the changed paths. No weakening found.
- `crates/micold-client/tests/support/layout.rs`: `resolve` split into `walk_layers`, `settled` added; no assertions.
- `crates/micold-client/tests/fixtures/layout_snapshot.txt`: re-recorded in `1fd8a4bc8`, `37437b583`, `b92d372b8` (finding 8).
- `crates/micold-client/tests/zz_vp_scratch.rs`: added in `98130d7d4`, deleted in `37437b583` (scratch test, not left in the tree).
- No skipped, `#[ignore]`d or filtered-out test, no threshold or config exclusion added.

Task checkboxes: T001-T026 are `[x]`/`[X]` and each has a cycle-log row or visual-pass record. `T027` is unticked, which is right: the close unit has not recorded the quickstart. No behavioral task is unticked with its behavior done.

## Findings

Ordered by severity. Evidence is `file:line` as of `b92d372b8`.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Surviving mutants in T021's view wiring.** Nothing sends a press, `Esc` or arrow key through the real review header, so (a) `on_dismiss` sending `SourceMenuToggled` instead of `SourceMenuDismissed`, (b) the arrow hookup sending `SourceMenuToggled` instead of `SourceMenuMoved`, and (c) an empty tooltip all pass the full `layout_snapshot` target (138 passed). The gate's tooltip check is conditional and only calls the helper that builds the string: `if let Some(base) = base_line(view)` asserts `source_control_tooltip(..)` contains `base_line(..)`, never what the widget was given. Affects US1 s6, US4 s3, FR-009, FR-011 end to end. | mutated `crates/micold-client/src/ui/changes.rs:185`, `:189`, `:190`; weak test `crates/micold-client/tests/gates/session_area_fits.rs:844-850` |
| 2 | MED | **Gate compares layout to the constant that builds it.** `(file.width - FILE_COLUMN_WIDTH).abs() <= TOLERANCE` and the same for `SOURCE_CONTROL_WIDTH` import the production constants, so `FILE_COLUMN_WIDTH = 240.0` passes `session_area_fits`. Only the committed snapshot fixture killed that mutant (`the_layout_matches_the_committed_fixture` FAILED). FR-008 says "about 260 px"; the spec value is pinned only by a generated fixture. Should assert the literal `260.0` (and `360.0`) or a named expectation in the test. | `crates/micold-client/tests/gates/session_area_fits.rs:928`, `:944` |
| 3 | MED | **Re-implemented expectation.** `chips_follow_toggles_and_availability` recomputes `toggles.committed && committed_available` with the code's own formula over `every_input`. Mutant 1 (drop `&& available`) died only here. No test states an explicit `Chip { on: false, available: false }` for a default-entry row, so a wrong reading of FR-004 would pass. Also the `note` expectation (`== !input.committed_available`) mirrors the code. | `crates/micold-core/src/review/source_menu.rs:580-594` |
| 4 | MED | **Pre-existing 1270 assertion rewritten in step with the layout** and now mirroring the production rule `min(360, width/2)`; the half-area case it used to pin is unpinned in the gate. See the evidence section. | `crates/micold-client/tests/gates/review_bar_fixed.rs:298-302` |
| 5 | MED | **Loose bound.** "the chip is on the option's line" accepts a chip up to its own height above the option (`chip.y + 0.5 >= option.y - chip.height`), so a chip drawn on the line above passes. Not mutated; read only. | `crates/micold-client/tests/gates/menu_select_fits.rs:206-209` |
| 6 | MED | **Weak and eager reducer tests.** After the pull request leaves the list, the re-seat is asserted only as `at < stops.len()`, not where; `picking_a_source_is_choosing_it` pins two behaviors (pull request pick, then working-tree pick) and `moving_walks_...` walks a magic `0..7`. | `crates/micold-client/tests/features_pr_changes.rs:2338`, `:2265`, `:2213` |
| 7 | LOW | **Real clock in fixtures.** `SystemTime::now()` sets `read_at`/`now` so the list is never stale; fixed instants would be deterministic. No stale pose exists for the real view (stale is unit-tested in core only). Also duplicated setup: `branch_pr` copies `pr` of `features_pr_changes.rs:47` and core's `pr`. | `crates/micold-client/tests/support/covered_states.rs:3578`, `gates/session_area_fits.rs:840`, `features_changes.rs:4380` |
| 8 | LOW | **Self-approving snapshot, three re-records.** `layout_snapshot.txt` was regenerated in `1fd8a4bc8` (4,813 lines), `37437b583` (852) and `b92d372b8` (267, after rebasing onto #1271 M4); neither the cycle log nor the ledger records a review of the changed pre-existing states. Mitigated: the snapshot did kill a mutant (8), and independent geometry gates and the m4 visual pass saw the same poses. | `crates/micold-client/tests/fixtures/layout_snapshot.txt` |

Smell pass, item by item, for the changed test files: no assertion-free, empty, skipped, doubled-subject or over-mocked test; the reducer tests drive the real `update`, the core tests the real pure functions.
HIGH count is 1 (conditional logic in the T019 tooltip check, finding 1). `row_invariants_hold_for_every_input` has an `if *row == Row::Heading` inside a loop (`source_menu.rs:523-534`); I judged it the invariant's antecedent, pinned by exact-vector tests (`:421`, `:539`), and did not count it.
Properties: the whole scoped suite ran in 164 s warm; the core module in under 0.1 s; no sleeps; no order dependence; the only non-determinism is the clock in finding 7. Failure messages name the state (`{what}`, `{context}`), which makes them specific.

## Mutation results

No mutation tool (profile: `mutation: null`). Nine deliberate mutants, one at a time, each restored with `git checkout -- <file>` and verified (`git status --short` showed no source change; the final confirming run of the three client targets plus the core module: 17 + 162 + 81 + 138 passed, 0 failed).
Sampled behaviors: `rows` chips (2), `stops` (1), `label` (1), opening a view (1), chip-while-PR (1), header wiring (3). Not exhaustive; not sampled: `MenuSelect` internals, `keeps_open`, `PullRequestsRead` re-seat, Files re-read.

| # | Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- | --- |
| 1 | `source_menu.rs:187` committed chip `on` without `&& available` | T002 chips | No | caught by `chips_follow_toggles_and_availability` only (finding 3) |
| 2 | `source_menu.rs:266` drop `if committed_available` guard on the `(false, true)` label arm | T004 Default-entry label | No | caught by `default_entry_label_counts_only_what_it_can_list` |
| 3 | `source_menu.rs:236` Committed stop always enabled | T003 stops | No | caught by `stops_run_working_tree_kinds_pull_requests_files` |
| 4 | `features/changes.rs:1887` remove `close_source_menu` in `opened` | T012 reset on open | No | caught by `opening_a_view_resets_the_control` |
| 5 | `features/changes.rs:2298` remove `show_working_tree` from `CommittedToggled` | T015 chip while PR shown | No | caught by two tests |
| 6 | `ui/changes.rs:185` `on_dismiss` sends `SourceMenuToggled` | T021 dismiss (FR-011, US1 s6) | **Yes** | non-equivalent; no view-level gesture test (finding 1) |
| 7 | `ui/changes.rs:190` tooltip replaced by `String::new()` | T021 tooltip (FR-009, US4 s3) | **Yes** | non-equivalent; the gate tests the helper, not the widget input (finding 1) |
| 8 | `ui/changes.rs:69` `FILE_COLUMN_WIDTH` 260 to 240 | T023 | Gate: yes; suite: no | `session_area_fits` passed; only the snapshot fixture failed (finding 2) |
| 9 | `ui/changes.rs:189` arrow hookup sends `SourceMenuToggled` | T021 keyboard (FR-011) | **Yes** | non-equivalent; arrows tested only at the reducer, never from the header (finding 1) |

Score: 6 of 9 caught (67%), scope: hand-picked mutants in the feature's changed files, no tool or version. Survivors 6, 7, 9 sit inside `T021`, a behavior marked done, which is a `FAIL` condition.
One likely-equivalent case was reasoned, not run: dropping the `true` in `Some((stop, true))` at `features/changes.rs:2173` is masked because `CommittedToggled` re-checks `committed_available` (line 2293).

## Traceability

Entry point column: R = reducer `changes::update` (the application's message entry), C = component via the runtime's `UserInterface`, V = the real review view laid out under `session_area_fits`/snapshot. "Wiring" means the header hooks messages to the component.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 s1 header control, left column bare | `session_area_fits::the_left_column_starts_with_the_file_list_and_the_header_holds_the_source_control` (8 poses, both themes) | Yes (V) |
| US1 s2 list order and marks | core `stale_list_...`, `list_rows_keep_input_order`, `row_invariants_...`; snapshot `review-source-open*` | Yes (V via snapshot, geometry only) |
| US1 s3-s4 pick closes, same as `SourceChosen` | `picking_a_source_is_choosing_it`; 1003 suites unmodified | Yes (R) |
| US1 s5 previous/next changes label | `stepping_changes_the_label` | Yes (R) |
| US1 s6 `Esc`/outside closes, source unchanged | `dismissing_closes_...` (R), `outside_esc_and_tab_dismiss_the_open_list` (C) | **Partial**: wiring unproven (mutant 6) |
| US2 s1 chips with state | core `chips_follow_...`; `the_working_tree_option_holds_its_chips_and_note_...` (C) | Yes |
| US2 s2 chip keeps list open | `a_chip_flips_its_kind_and_keeps_the_list_open` | Yes (R) |
| US2 s3 chip while PR shown | `a_chip_while_a_pull_request_is_shown_returns_to_the_working_tree` (4 cases) | Yes (R) |
| US2 s4 label by kinds | core `working_tree_label_names_the_listed_kinds`, `default_entry_label_...` | Yes (core) |
| US2 s5 Default entry note | `the_default_entrys_committed_chip_changes_nothing`; core note in `chips_follow_...`; C DefaultEntry pose | Yes |
| US3 s1 Files pick | `picking_files_toggles_the_files_pane_and_closes_the_list` | Yes (R) |
| US3 s2 Files over a PR, label | `files_stays_on_across_a_source_pick_...`; core `pull_request_label_...` | Yes (R) |
| US3 s3 Back / Open diff | 1150 suites, unmodified (SC-004); not rewritten here | Yes (existing) |
| US3 s4 Files stays on | `files_stays_on_across_...`, `a_chip_while_a_pull_request_is_shown_reads_the_tree_...` | Yes (R) |
| US3 s5 Files again turns off | `picking_files_toggles_...` (second pick) | Yes (R) |
| US3 s6 Whole file | 1150 suites, unmodified | Yes (existing) |
| US4 s1 two columns, diff 70% | `the_file_column_keeps_its_width_and_the_diff_takes_the_rest` | Yes (V) |
| US4 s2 file column keeps its width | same (1280/1600/900) | Yes (V), constant-based (finding 2) |
| US4 s3 tooltip carries base line | `session_area_fits.rs:844-850` | **Partial**: helper only (mutant 7) |
| FR-005 / SC-005 every list state | core `loading_unavailable_and_empty_...`, `stale_list_...`, `a_list_that_holds_nothing_...`, `absent_...` | Yes |

FR-011 arrows: reducer `moving_walks_...` and the component's `Enter`; the component's arrow-key path and the header hookup (mutant 9) are unproven.
FR-012 (user guide): documentation, no automated test; evidence is `visual-pass/m4.md` and the guide diff in `1fd8a4bc8`.
Untested criteria: none. Tests tracing to nothing: none. Every `traces`-style claim (cycle-log task pairs) points at a test that exists and ran.

## What was not audited

- The full workspace and `micold-daemon` suite: scoped to `-p micold-core -p micold-client` as instructed (M4's full gate passed at `88213c041`, CI green at `72c45fba`; not re-run).
- Four failures in the scoped run are not feature-related and were not investigated: `settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` (two tests). They match the container's known root-only permission failures, but I did not confirm the cause.
- Mutation was 9 hand-picked mutants, not a tool run; `MenuSelect` internals (`menu_select.rs`), `keeps_open`, the `PullRequestsRead` re-seat and the Files re-read were not mutated.
- Coverage: no tool in the profile.
- The visual pass, the user guide's wording and the quickstart Part A/B were not re-run (T027 is still open).
- Performance beyond the suite's wall time: no criterion, not assessed. Windows and macOS legs: not runnable here.
- During the audit, `spec.md`, `quickstart.md`, `autopilot.md` and `tdd/cycle-log.md` carried uncommitted edits from outside this audit; I read `spec.md` and `cycle-log.md` before and during that time, so the criteria above follow the version I read.
