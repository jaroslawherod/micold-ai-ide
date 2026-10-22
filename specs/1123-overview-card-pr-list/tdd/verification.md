---
feature: 1123-overview-card-pr-list
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 804cc8de # short SHA audited
behaviors: 12 # task pairs of tasks.md (no tdd/test-list.md exists; tasks.md test tasks are the list)
proven: 0
likely: 8 # red recorded in the cycle log; history cannot order it (one commit per milestone)
test_after: 4 # T001-T018 group (green-only, compile-red), T022 and T032 overview.rs tests, no red recorded
no_test: 0
high_smells: 0 # smell pass was sampled, not exhaustive (see "What was not audited")
criteria_total: 17 # FR-001..FR-017
criteria_covered: 17
mutation_score: not measured # cargo-mutants absent; 25 deliberate mutants, 22 caught, 3 survived
mutants_survived: 3 # two inside DONE behaviors (F1, F2), one near-equivalent (F4)
suite: scoped 1123 suite green after every restore (core 1456 lib + 7 integration targets; client 1084 lib, 679 bin, overview 88, features_pr_status 30, layout_snapshot 79), 107s warm
---

# TDD Verification: Overview cards, pull request list with state, linked

**Verdict: FAIL.** Two deliberate mutants inside DONE behaviors survive (a third, near-equivalent, is F4) the whole scoped suite (a click on a
Tab-stop number takes the keyboard; the "checks failing" error tint is never asserted through `pr_rows`), and
the cycle log itself records the M1 rows (T001-T018) as green-only and two app-test rows as "not run red".

The audit was run from cold context by a worker that did not write the tests. History is one commit per
milestone (`93f0222a` M1, `74f0ca9d` M2, `aee0a06e` M3) plus fix commits, each carrying tests and source
together, so ordering is never provable from git; the cycle log is the only red evidence.

## Test-first evidence

Behaviors are the task pairs of `tasks.md` (the feature has no `tdd/test-list.md`; the cycle log says so).

| Behavior (tasks) | Class | Evidence |
|---|---|---|
| T001/T002 K1 order + dedupe, K2 phrase precedence, counts, rows, `Debug` | TEST_AFTER | log: "not captured; `pr_list` absent at baseline (compile red)"; one commit `93f0222a`. Compile-red only, no behavioural red |
| T003/T004/T006 Q1/Q2 query, parser from `o` + `r` + `m` | TEST_AFTER | log: "not captured"; Q1 assertion contradicts the baseline query (structural only) |
| T005/T006 K3 first entry is the selected pull request | TEST_AFTER | log: "not captured; needs `BranchPrList` (compile red)" |
| T007/T008 `State.lists` replaces counts | TEST_AFTER | log: "not captured; `State.lists` absent" |
| T010-T017 U1 rows, `PrMark::word`, card rows from `State.lists`, covered state | TEST_AFTER | log: "not captured" (compile red) |
| M1 fixes (dash tooltip on the expanded card) | LIKELY | red quoted: `the_dash_carries_its_reason_collapsed_and_expanded` `left: []`; one commit with the fix |
| M2 T020/T024 U2 click on a number | LIKELY | red quoted for 3 tests; 3 others honestly recorded as passing at red (regression guards) |
| M2 T021/T025 U4 pointer, one wake | LIKELY | red quoted (`left: ["rest 1"] right: ["link rest 1"]`, ...) |
| M2 T022/T026 mounted click, root opens the listed address | LIKELY | red quoted for `ui::overview` and `pull_request_link_*` |
| M2 T022 (`tests/overview.rs`) lookup tests | TEST_AFTER | log: "written with the implementation (not run red)" |
| M2 review A fixes (address check, label-only hit) | LIKELY | red quoted (`pull_request_link_opens_no_address_off_github`, `a_number_takes_the_pointer_on_its_label_only`) |
| M3 T029-T035 K5a-g, U3, U4 arrows, U5/U6 | LIKELY | red quoted per task (6 core + 4 + 2 + 4 FAILED) |
| M3 T032 (`tests/overview.rs`, ui mount) | TEST_AFTER at M3; later red by mutants | log: "written with the implementation (not run red)"; review B r1 F1 then ran three mutants red (`overview.rs:1270`, `:2244`, `:2289`). I count it TEST_AFTER for the original cycle |
| M3 fixes r1/r2 (Tab/Shift+Tab/refocus hand-offs) | LIKELY | red quoted for 4 tests (`main_tests.rs:15188` among them) |

Counts above are by row group; `behaviors: 12` merges the M1 rows into the four TEST_AFTER entries, the M2 and
M3 red rows into LIKELY. No row can be PROVEN: no commit orders a test before its source.

### Existing tests (weakening check)

Diffed `f1d7d1b5..HEAD` over every changed test file. No assertion was loosened, skipped, `#[ignore]`d or
excluded, no threshold lowered. The edits that look like removals are deliberate, specified changes:

- `crates/micold-core/tests/pull_request_counts.rs`: the old `oc`/`mc` query test and the 100-cap cases
  became the Q1 test (`pull_request_query.rs:12` still holds `mergeable`, FR-019) and the 10-cap cases
  (`pull_request_counts_cap_at_a_full_page_of_ten`, nine reads exact). Replacement is stronger (adds the
  fork-in-a-full-page case).
- `crates/micold-client/tests/features_pr_status.rs`, `pull_request_source.rs`: `counts` renamed `lists`, same
  assertions on the new type.
- `crates/micold-client/src/main_tests.rs`: `LinkFocusGained(l)` events gained `CardLink::Issue` (K5g, "the
  1122 cases unchanged"); the 1122 assertions are kept with the new signature.
- `crates/micold-client/tests/gates/containment.rs`: the allow-list gained `PR_NUMBER_GUTTER`, a precise
  per-path list for the `overview-pr-rows` state, not a blanket exclusion.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| F1 | HIGH | A click on a number that is a Tab stop takes the keyboard. `card_link` sets `.focus_on_press(false)` for the issue link and the numbers alike; flipping it to `true` leaves 1084 lib, 679 bin, `overview` 88 and `features_pr_status` 30 all green. `a_click_on_a_number_leaves_the_keyboard_with_the_card_that_holds_it` (`card_grid.rs:3135`) mounts `numbered_grid()` whose numbers carry no focus reports, so they are not focusable and `press_takes_focus` is moot. The M3 behavior (numbers as Tab stops, SC-003 "no ... selection") is untested on the click path | mutant M-C5, `overview_card.rs:651` |
| F2 | HIGH | `pr_rows` maps `failing: row.phrase == Some(StatusPhrase::ChecksFailing)` (the error tint of "checks failing", U1/FR-004) but nothing asserts it. `failing: false` survives every scoped target, `layout_snapshot` included (geometry, not colour). The only `failing: true` in tests is a hand-built `CardPrRow` at `card_grid.rs:3085`; `overview_card.rs:794` is tested with hand-built rows, never with rows from `pr_rows` | mutant M-C11, `ui/overview.rs:827` |
| F3 | HIGH (process) | Test-first not evidenced for M1 (T001-T018) and for the T022 and T032 `tests/overview.rs` cases: the log says "not captured" / "not run red". Rubric class TEST_AFTER; verdict rule FAIL. T032 was later run red by mutants and the compile-red evidence is real, so this is a discipline gap, not a coverage gap. Cannot be cured after the fact | `tdd/cycle-log.md` lines 19-25, 40, 60 |
| F4 | MED | An unknown state on the **merged** page is not tested to fail the chunk. `.map_err(|_| UNREADABLE)?` replaced by `.unwrap_or(BranchPrState::Merged)` survives (`pull_request_list.rs:148` covers the open/recent path through `listed`, and `branch_pr_state` itself). The query asks `states: MERGED`, so GitHub never returns another state there: close to equivalent, but "unknown state fails the chunk" (K-contract) names no exception | mutant M-M9, `pull_request.rs:692` |
| F5 | LOW | `pull_request_counts.rs` asserts `counts(1, 3 + 8)`: the `8` is the eight remaining merged nodes of the fixture, unexplained | `pull_request_counts.rs` (fork test) |

No other HIGH smell was seen in the files opened (assertions carry rule messages, fixtures are shared helpers
`lists()`/`listed_pr()`, the guard test `pull_request_is_never_stored.rs` reads the source text deliberately and
is in the repo's guard-test style). The pass was targeted at the sampled behaviors, not a full catalogue sweep.

## Mutation results

Tool: none (`cargo-mutants` not installed, `tdd-profile.md` `mutation: null`). Method: deliberate mutants,
one at a time, each restored with `git checkout -- <file>`; the scoped suite was re-run green at the end
(`git status --short` clean before this report). 25 mutants, 22 caught, 3 survived. Two harness slips (a bad
`cargo` argument list on the `*10+*` mutant, re-run correctly; a non-existent `style_snapshot` target on a
re-run of the `failing` mutant, dropped) are not counted. Every mutant was checked to fail by a named test, not
by a compile error. The sample covers all eight areas requested.

| Mutant | Behavior | Survived | Caught by / judgment |
|---|---|---|---|
| `pr_list.rs` `group()` counts Draft as merged | row order, open+draft first | No | `a_collapsed_card_shows_two_rows...`, `the_list_orders_open_first_then_newest_and_dedupes` |
| `card_order` newest-first args swapped | newest first in a group | No | same two tests |
| dedupe filter keeps every repeated number | one row per number | No | `a_repeated_number_keeps_the_first_given` + 1 |
| `StatusPhrase::of` changes-requested and conflict swapped | precedence (D3, FR-004) | No | `each_phrase_beats_the_next` |
| `counts()` drops Draft from open | counts from the list | No | `the_counts_come_from_the_list` |
| `overview.rs` `*10+*` flag dropped for open | D4/FR-008 | No | `the_pull_request_row_hides_zero_counts` |
| `PAGE_SIZE` 10 to 11 | a full page reads capped | No | `pull_request_counts_cap_at_a_full_page_of_ten`, fork test |
| `branch_pr_state` unknown state maps to Open | unknown state fails the chunk | No | `an_unknown_state_makes_the_read_passing_not_a_wrong_mark` |
| merged page unknown state tolerated | same, merged path | **Yes** | F4 |
| merged page forks kept | forks dropped | No | `pull_request_lists_leave_out_pull_requests_from_forks` |
| `Debug` adds the url | FR-014 | No | `debug_leaves_out_the_address` |
| `COLLAPSED_ROWS` 2 to 3 | collapsed two rows + "+N more" | No | `a_collapsed_card_shows_two_rows_and_the_rest_as_more` |
| `drawn()` ignores `expanded` | all rows expanded | No | same test |
| `Unavailable` keeps `lists` | unreadable states clear the list, dash reasons stay | No | `unavailable_released_and_disabling_clear_the_counts_with_the_statuses` |
| `card_pr_list` ignores `pr_status.enabled` | no rows with status off | No | `a_numbers_address_is_looked_up_in_the_cards_list` |
| `pull_request_link_for` drops the GitHub prefix check | U7 address check | No | `pull_request_link_opens_no_address_off_github` |
| `pull_request_link_for` finds any listed pull request | exactly that PR's address | No | `pull_request_link_opens_nothing_once_gone` |
| `card_link` passes `f(number + 1)` | press opens that number | No | 5 tests (`a_click_on_a_number_publishes_only_its_pull_request`, ...) |
| `card_link` `.focus_on_press(true)` | click leaves the keyboard | **Yes** | F1 |
| `card_link` `.focusable(false)` | the number's Tab stop | No | 5+ `overview_card` tests |
| `pr_rows` `failing: false` | error tint of "checks failing" | **Yes** | F2 |
| `link_focus_handoff` drops the "still holds the keyboard" guard | hands over only while the link holds it | No | `a_move_off_a_row_three_number_moves_no_focus_to_the_card` |
| `link_focus_after` `from_another` inverted | Shift+Tab / Tab hand-offs | No | 3 tests (`a_tab_onto_row_two_stays_on_it`, both Shift+Tab) |
| `link_focus_after` drops `card_last_link` entry | Shift+Tab onto a collapsed card goes to its last | No | both `shift_tab_*_collapsed_cards_number_goes_to_its_last` |
| `link_focus_after` drops the refocus-by-id | a moved number is re-focused | No | `a_reading_that_moves_the_focused_number_refocuses_it` |

## Traceability

Each criterion has at least one test through the real entry point (mounted widget, `tests/overview.rs`
reducer, or the root `App` in `main_tests.rs`), checked by test name and, for the sampled behaviors, by a
mutant that fails it.

| Criterion | Tests | End to end |
|---|---|---|
| FR-001 / FR-016 list per branch, derivation in core | `pull_request_list.rs`, `pull_request_parse.rs`, `pr_list::tests` | Yes (parser to `State.lists`) |
| FR-002, FR-003, FR-005 rows, order, "+N more" | `pr_list::tests`, `tests/overview.rs:762,799,833` | Yes |
| FR-004 pill word + one phrase | `each_phrase_beats_the_next`, `PrMark::word` tests, `tests/overview.rs:762` | Partly: the error tint through `pr_rows` is untested (F2) |
| FR-006, FR-008 counts from the list, *10+* | `pull_request_counts.rs`, `tests/overview.rs:665,675` | Yes |
| FR-007 dash reasons kept | `tests/overview.rs:579,615,625`, `features_pr_status.rs` | Yes |
| FR-009 click and Enter/Space open the number | `overview_card` U2/U3, `ui::overview` mounted click, `pull_request_link_*` in `main_tests.rs` | Yes |
| FR-010 Tab order, Shift+Tab reverse | U3 tests, `link_focus_*`, `shift_tab_*` in `main_tests.rs` | Yes |
| FR-011 press outside the number is the card's | U2 pill/words test, `a_press_and_release_split...` | Yes; the press-takes-focus part is F1 |
| FR-012 failure notification | `pull_request_link_failure_notifies` | Yes |
| FR-013 sidebar unchanged | `features::changes::pr_mark` reuse; sidebar suites unchanged in the diff | Indirect |
| FR-014 no address stored or printed | `pull_request_is_never_stored.rs`, `debug_leaves_out_the_address`, `pull_request_link_opens_no_address_off_github` | Yes |
| FR-015 shared components | showcase poses, `showcase_completeness`, contrast tests | Yes (visual pass recorded) |
| FR-017 user guide | docs tasks T009, T018, T027, T036 | Not tested (docs) |
| SC-004 request budget | `status_query_*` fixtures, `pull_request_query.rs` | Yes |
| SC-005 counts equal the list for every combination | `the_counts_come_from_the_list`, `tests/overview.rs:665` | Yes |

Untested criteria: none. Tests tracing to nothing: none seen. Test-list `traces` do not exist, so the
mapping is by name and by mutant, not from a declared column.

## What was not audited

- Order of test and source in git: every milestone is one commit with both; all red evidence is the cycle
  log's own account. Only the log's quoted failing outputs were cross-checked against test names, which exist
  and pass.
- Mutation was a hand-picked sample (25 mutants over the eight requested areas), not a tool score; no
  mutation score exists. Not mutated: `text_link.rs` hit areas beyond `hit_on_label`, `focus.rs`,
  `daemon_sync.rs`, `shell/pr_status.rs`, `overview_expansion.rs` (K5a-g, covered by the cycle log's own
  mutants), showcase and `card_grid.rs` internals other than the click path.
- The full smell catalogue was not walked file by file over the roughly 30 changed test modules; only the
  tests named above, the weakening diff and the guard tests were read.
- Coverage: the repo has no coverage tool. Performance: scoped suite ran in 107 s warm; not assessed beyond
  that. Visual outcomes (pill colours, rings, screenshots) are the visual-pass evidence, not audited here.
- Windows and macOS legs and the sandbox suites; `scripts/gate.sh` was not re-run.
