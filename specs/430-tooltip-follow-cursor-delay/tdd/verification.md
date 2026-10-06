---
feature: 430-tooltip-follow-cursor-delay
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: cddd3ce5 # short SHA audited
behaviors: 10 # test tasks in tasks.md (no tdd/test-list.md exists)
proven: 0
likely: 0
test_after: 8 # fail closed: no red recorded anywhere
no_test: 0
not_applicable: 2 # T001, T004 characterization guards
high_smells: 1 # survivor M3, a vacuous tracking test (finding 2)
criteria_total: 10 # acceptance scenarios (US1 3, US2 5, US3 2); FR-001..007 and SC-001..006 mapped in the table
criteria_covered: 10
mutation_score: unmeasured # cargo-mutants absent; 6 deliberate mutants on changed files, 5 caught, 1 survived
mutants_survived: 1 # not equivalent
suite: 5127 passed, 1 failed (unrelated, environment), 9 ignored, 431 binaries, 120s
---

# TDD Verification: Tooltip follows the cursor and waits before showing

**Verdict: FAIL.** The feature has no `tdd/test-list.md` or `tdd/cycle-log.md`, so no test-first evidence exists for
any behavior (fail closed: `TEST_AFTER`), and a deliberate mutant that stops the follow panel being re-laid-out on
pointer movement survives the "panel tracks the pointer" tests.

Independence: this audit was run from a fresh context in a separate session from the one that wrote the code. Every
test file cited below was re-read at the audited SHA.

## Preflight

- Profile `.specify/memory/tdd-profile.md` read (suite, helpers, exemplars, conventions). No coverage tool, no mutation tool.
- The feature was not planned through the TDD extension: no `tdd/` directory. Behaviors are taken from the test tasks
  in `tasks.md` (T001, T003, T004, T006, T007, T009, T011, T013, T014, T015). Spec/plan/tasks were read.
- Suite (`scripts/build-lock.sh cargo test --workspace --no-fail-fast`): 5127 passed, 1 failed, 9 ignored, 431 binaries,
  120 s. The single failure, `micold-core/tests/github_locate_desktop_launch.rs::a_desktop_launch_finds_a_working_gh`,
  is environmental (`gh` installed under mise, not on the desktop-launch PATH) and unrelated to the feature. Without
  `--no-fail-fast` the first run stopped at that binary (3245 passed, client and daemon crates not reached).
- Baseline: the profile records 5071 passed, 0 failed at `cdc473ab`; the failure is new on this machine, not in the feature's files.

## Test-first evidence

History is two milestone commits that each add tests and source together: `559dc7c7` (M1: `tooltip.rs` +399 lines,
`core/src/tooltip.rs`, `tooltip_show.rs` 174 new lines, `tooltip_show_glue.rs`, `idle_requests_no_frames.rs`,
`support/tooltip.rs`) and `9f6f29c7` (M2). `559dc7c7` is titled "(wip, ungated)". `tasks.md` states "must be seen failing
for the right reason" but nothing records a red command or output: no cycle log, and `autopilot.md` has no red entry. The
history cannot show order within a commit. Rule: no red recorded means test-after.

| Behavior | Task | Class | Evidence |
| -------- | ---- | ----- | -------- |
| Existing tooltip tests unchanged as guard | T001 | NOT_APPLICABLE | Characterization guard, green by definition |
| `place_at_pointer` table | T003 | TEST_AFTER | tests and source in `559dc7c7`; no red recorded |
| Fixed placements clear of trigger at four edges | T004 | NOT_APPLICABLE | Declared guard, green on arrival |
| Follow glue (open at pointer, move, still, leave, press, subject, scroll) | T006 | TEST_AFTER | `559dc7c7`; no red recorded |
| No pointer / zero trigger / unknown window opens nothing | T007 | TEST_AFTER | `559dc7c7`; no red recorded |
| Follow idle: no frame | T009 | TEST_AFTER | `559dc7c7`; no red recorded |
| `ShowTimer` table | T011 | TEST_AFTER | `559dc7c7` adds `tooltip_show.rs` and the rule together; no red recorded |
| `show_delay` glue | T013 | TEST_AFTER | `9f6f29c7`; no red recorded |
| Last wait wins (cdk and material) | T014 | TEST_AFTER | `9f6f29c7`; no red recorded |
| Delay: one timed wake, no frame; builder step | T015 | TEST_AFTER | `9f6f29c7`; no red recorded |

### Pre-existing tests

Diff of the feature's range over `tooltip_rest_glue.rs`, `idle_requests_no_frames.rs`, `material_builder_api.rs`,
`support/tooltip.rs` and removed lines in `cdk/tooltip.rs`, `material/mod.rs`, `core/src/tooltip.rs`: no assertion
removed, loosened, skipped or excluded; no threshold changed. Two edits are widenings, not weakenings:
`idle_requests_no_frames.rs:430` (CALLERS reason string now names `ShowTimer`) and
`material_builder_api.rs:117,136` (test renamed to `the_tooltips_wait_modes_and_options_are_four_chainable_steps`, loop gained
`show_delay`; still matches the `tooltip` filter in tasks.md Verify). `tooltip_rest_glue.rs` is untouched. SC-004 holds. No weakened existing test.

### tasks.md against evidence

No `[X]` task lacks a test. T018 and T019 (visual pass) are unticked and `visual-pass.md` records "NOT RUN" (no Xvfb or
xdotool): honest, but the spec's visual outcomes are unverified, see "What was not audited". M1 was nonetheless merged (PR #600)
and M2 (PR #607) with T019/T018 open.

## Findings

| #  | Severity | Finding | Evidence |
| -- | -------- | ------- | -------- |
| 1  | HIGH | No red evidence for any behavior; tests and source land in the same commit; no test list, no cycle log. All 8 behavior tasks are `TEST_AFTER` by the fail-closed rule. | `git show --stat 559dc7c7 9f6f29c7`; no `tdd/` dir; `tasks.md:5-6` claims red-first |
| 2  | HIGH | Surviving mutant M3: replacing `if state.open && rest.open` with `if false && ...` in `update` (no `shell.invalidate_layout()` when the pointer moves while the panel is open) passes all 14 `tooltip_show_glue` and 14 `idle_requests_no_frames` tests. `a_pointer_move_while_open_moves_the_panel` reads the position through `Driven::panel()`, which builds a fresh overlay and layout itself, so it never observes whether the widget asked the runtime to relayout. In the real runtime the panel would stay where it was. Not an equivalent mutant: the source comment says that call is what gets the move painted. Behavior T006/T008 (FR-001, US1.1 "stays beside the pointer"). | `crates/micold-client/src/ui/cdk/tooltip.rs:284`; `crates/micold-client/tests/tooltip_show_glue.rs:60-70`; `crates/micold-client/tests/support/tooltip.rs:265` (`panel`), `:153` (`Seen` has no layout-invalidated flag) |
| 3  | MED | `EDGE_PADDING` is hard-coded as `5.0` in the glue test with a comment pointing at the private const; a change to the const breaks the glue tests without a stated reason (magic value, duplicated setup). `visible()` is duplicated between `tooltip_show_glue.rs:33` and `tooltip.rs:698`. | `tooltip_show_glue.rs:19-40`; `cdk/tooltip.rs:698-706` |
| 4  | MED | `material_builder_api.rs` checks the builder step by searching library source text for `pub fn show_delay(`, not by calling it: framework-style string scan, passes with a stub body. Follows the file's existing 038 pattern (Foreign style is not the issue); the behavior is covered by the `the_wait_set_last_wins` tests. | `crates/micold-client/tests/material_builder_api.rs:117-140` |
| 5  | LOW | `the_wait_set_last_wins` exists twice (`cdk/tooltip.rs:645`, `material/mod.rs:391`) with the same assertions on the `wait` field; the second pins forwarding at a different level, so not counted as redundant, but both read a private field (implementation coupled). | those two lines |
| 6  | LOW | `ShowTimer::observe`'s `checked_add` overflow branch (delay too long for the clock, `wake_at: None`) has no test. | `crates/micold-core/src/tooltip.rs:128-132` |
| 7  | LOW | Delay with `after_rest` mixed: only the last-wins builder rule is tested; no behavior test drives a `Wait::Rest` tooltip after `show_delay` through events (spec leaves it a plan decision). Delay cancel on leave is tested for the non-follow panel only (`leaving_cancels_the_delay`). | `tooltip_show_glue.rs:186` |

Smell pass, item by item, on `tooltip_show.rs`, `tooltip_show_glue.rs`, the added cases in `idle_requests_no_frames.rs`,
`support/tooltip.rs`, and `placement_tests`: no assertion-free, tautological, doubled-subject, over-mocked, vacuous or conditional-logic
tests; the loops in `a_pointer_panel_is_inside_...` and `every_fixed_placement_...` assert unconditionally on every iteration. No
snapshots, skips or ignores. Expectations in the placement tests are literal arithmetic (`pointer + GAP`), not the code's own
computation. Helpers: the new `follow_tooltip`, `delayed_tooltip` and `panel` extend the existing `support/tooltip.rs` rather than
hand-rolling a fixture; the style matches `tooltip_rest_glue.rs`. Core tests sit under `crates/micold-core/tests/` and the unit
tests in a `#[cfg(test)] mod` in the source file, as the profile's exemplars do. Assertion messages state rules. Determinism and
speed: all clocks are passed in (`Instant` values built from one `now`), no sleeps; the feature's test files run in under 0.1 s each.

## Mutation results

Tool: none (`cargo-mutants` not installed; profile records `mutation: null`). Score unmeasured. Six deliberate mutants, one at a
time, each restored with `git checkout` and the suites re-run green after (tree clean, `git status` empty). A sample, not
exhaustive: two files (`core/src/tooltip.rs`, `cdk/tooltip.rs`), the timing rule and the follow glue and placement.

| Mutant | Behavior | Survived | Judgment |
| ------ | -------- | -------- | -------- |
| `core/src/tooltip.rs:119` `>= delay` to `> delay` | T011 `ShowTimer` boundary | No | Caught by 4 tests, boundary pinned |
| `core/src/tooltip.rs:121` waiting branch restarts `since: now` | T011 never restarts | No | Caught by `repeated_observations_never_restart_the_wait`, `leaving_cancels_the_wait` |
| `cdk/tooltip.rs:543` `window - at >= at` to `<=` | T003 fits-neither side | No | Caught by `a_pointer_panel_that_fits_neither_side_takes_the_side_with_more_room` |
| `cdk/tooltip.rs:274` drop `state.show.press()` | T006 press rule | No | Caught by 2 glue tests |
| `cdk/tooltip.rs:279` `state.pointer != over` to `false` | T006/T013 tracking | No | Caught by 5 glue tests |
| `cdk/tooltip.rs:284` drop relayout on move while open | T006 move re-lays out | **Yes** | Not equivalent; finding 2, HIGH |

## Traceability

Real entry point for the glue tests is the widget's `update` and `overlay` driven by `Driven` with real iced events and
layout; the placement and timer tests are pure units beneath it. All claimed tests exist and run (feature tests: 14 + 14 + 20 lib + 12 core, green).

| Criterion | Tests | End to end |
| --------- | ----- | ---------- |
| US1.1, FR-001 | `the_follow_panel_opens_at_the_pointer`, `a_pointer_move_while_open_moves_the_panel`, `a_pointer_panel_sits_beside_the_pointer_offset_by_the_gap` | Yes, but see finding 2 (relayout request unobserved) |
| US1.2, FR-002, SC-001 | `a_pointer_panel_with_no_room_...flips_*`, `..._never_under_the_pointer` (41x31 grid), delayed edge test | Yes |
| US1.3 | `leaving_the_trigger_closes_the_follow_panel` | Yes |
| US2.1, SC-002, FR-003 | `nothing_shows_before_the_delay_...`, `ShowTimer` table | Yes |
| US2.2 | `leaving_cancels_the_delay` | Yes (non-follow only) |
| US2.3, FR-004 | `a_zero_delay_opens_at_once` | Yes |
| US2.4 | `a_delayed_follow_panel_opens_at_the_current_pointer_and_then_tracks_it` | Yes |
| US2.5, FR-007, SC-005 | `a_waiting_show_delay_asks_for_one_timed_wake_and_no_frame`, `a_follow_tooltip_..._requests_no_frame` | Yes |
| US3.1, FR-005, SC-003, SC-006 | `every_fixed_placement_keeps_clear_of_a_trigger_at_each_window_edge`, `a_delayed_follow_panel_never_covers_...` | Unit (placement) plus glue for follow; fixed placements with a delay are not driven end to end |
| US3.2 | `a_pointer_panel_in_the_corner_flips_on_both_axes` | Unit |
| FR-006, SC-004 | existing `tooltip_rest_glue.rs` and `idle_requests_no_frames.rs` unchanged | Yes |
| Edge cases: press, subject change, scroll, no pointer, zero trigger, tiny window | glue and `placement_tests` as listed in the file | Yes |

Untested criteria: none. Tests tracing to nothing: none. Edge cases not tested: a window resize while open (no test drives a
`Window::Resized`), "several windows" isolation (state is per widget tree; no test), logical-unit scale (no test).

## What was not audited

- The visual pass (T018, T019): no Xvfb or xdotool here; `visual-pass.md` records both as NOT RUN. Colour, elevation and look of the
  showcase poses are unverified by this audit.
- Mutation testing across the whole change: no tool; six deliberate mutants only, none in `material/mod.rs`, `showcase/sections/floating.rs`, or the `overlay`/`Panel::layout` path.
- Coverage: no tool in the profile.
- The `micold-daemon` and `micold-core` suites beyond the tooltip files were run (one unrelated environmental failure) but not read.
- Performance and idle CPU beyond the `RedrawRequest` assertions: not measured.
- Git order within a commit: not verifiable; reds may have occurred but are unrecorded.
