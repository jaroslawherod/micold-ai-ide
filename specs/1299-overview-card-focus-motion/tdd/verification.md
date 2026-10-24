---
feature: 1299-overview-card-focus-motion
verdict: FAIL # resolved at close, see Resolution
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override in .specify/templates/overrides or presets
verified_at: 84ed10da # origin/main after M2 (#1331), plus the ledger commit
behaviors: 9 # no tdd/test-list.md or cycle-log.md; the test tasks of tasks.md (T002-T005, T012-T016)
proven: 0
likely: 0
test_after: 9 # tests and code landed in one commit per milestone, no red recorded
no_test: 0
high_smells: 2 # conditional logic in two tests; fixed at close
criteria_total: 18 # FR-001..FR-013, SC-001..SC-005
criteria_covered: 16 # FR-006 is a recorded decision (not animated), FR-013 is user-guide prose
mutation_score: unmeasured # no cargo-mutants; 12 deliberate mutants by hand, 11 caught, 1 survived (O1), killed after the fix
mutants_survived: 1
suite: "micold-client --lib: 1203 passed, 0 failed, 34 s warm (before the close fixes); 1204 passed after"
---

# TDD Verification: Overview card focus and expansion motion (#1299)

**Verdict: FAIL.** The Overview's Reduce motion wiring (`ui/overview.rs` `view`, T009, FR-007)
had no test: a mutant passing `.instant(false)` to the grid passed every client lib test. Test-first
is also unproven for every behavior: there is no cycle log, and each milestone's tests landed in the
same commit as the code (`7b6ae910`, `f0824062`).

Independence: the close unit wrote none of these tests, but it ran in the same autopilot run that
wrote them and had no subagent to hand the smell pass to; every file below was read cold at
`84ed10da`.

## Preflight

- Profile `.specify/memory/tdd-profile.md`: no coverage or mutation tool; conventions and exemplars
  as recorded there. `tdd/test-list.md` and `tdd/cycle-log.md` are absent: the behaviors below are
  the test tasks of `tasks.md`.
- `mise` is not installed in this cloud session. The suite was run as
  `scripts/build-lock.sh cargo test -p micold-client --lib` (all of this feature's tests are in-file
  `mod tests` of the client library).

## Test-first evidence

| Behavior | Class | Evidence |
|---|---|---|
| T002 fade in/out, reversal, two at once (`keyboard_focus.rs`) | TEST_AFTER | `7b6ae910` adds tests and source together; no red recorded |
| T003 press drops the indicator; built-focused starts full (`keyboard_focus.rs`) | TEST_AFTER | same commit; the fading-out press case added with its fix in `8ee1f750` |
| T004 `CARD_MOTION`, card fade, instant card, link not faded (`overview_card.rs`) | TEST_AFTER | `7b6ae910` |
| T005 instant grid, default grid fades, Reduce motion saved mid-fade (`card_grid.rs`) | TEST_AFTER | `7b6ae910` |
| T012 `eased_shadow` (`style.rs`) | TEST_AFTER | `f0824062` adds tests and source together |
| T013 `Lifts` (`card_lift.rs`) | TEST_AFTER | `f0824062` (new file, tests and code in one) |
| T014 draw order, nothing moves, rest pose, zoom (`card_grid.rs`) | TEST_AFTER | `f0824062`; the rest-scope case added with its fix in `031d0194` (ledger: fails without the fix) |
| T015 the Reduce motion test (`card_grid.rs`) | TEST_AFTER | `f0824062` |
| T016 owner-drawn shadow paints nothing outside (`overview_card.rs`) | TEST_AFTER | `f0824062` |

Existing tests changed by the feature, each read: two ring pixel tests in `overview_card.rs` gain a
`frames(0, DONE, …)` line that runs the fade to its end before the unchanged assertions (ledger
Decision 5, not a weakening); `showcase_completeness.rs` `the_overview_card_and_grid_are_posed` now
expects both entries interactive (`031d0194`), a behavior change of the catalogue that the
assertion still pins exactly; `field_focus.rs` loses its private `rebuild`, moved to
`test_support::Mounted`. No assertion removed or loosened, nothing skipped.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | The Overview's hand-over of Reduce motion to its grid is untested: `.instant(false)` passes every test (mutant O1). FR-007/SC-002 were proven only on a grid built `instant` by hand | `ui/overview.rs:213` |
| 2 | HIGH | Conditional logic: `focus_and_lift_ease_in_together` asserts only inside `if ms == MIDWAY` / `if ms == DONE` in a loop, so a change to either constant that skips the tick asserts nothing | `card_grid.rs:3590-3596` |
| 3 | HIGH | Conditional logic: `a_reversed_fade_turns_back_where_it_is_and_two_run_together` asserts the monotonic step only `if` a value changed; the guard is redundant (the predicate holds for equal values) | `keyboard_focus.rs:1065` |
| 4 | MED | Test-first unproven for all 9 behaviors (see table) | git history |
| 5 | MED | Magic tolerance `reached - next < 0.3` in the lift reversal test, where the focus reversal test derives its bound from the steepest step | `card_lift.rs:208` |
| 6 | LOW | No `tdd/test-list.md` or cycle log; traceability below is built by hand | feature directory |

## Resolution (close unit)

- Finding 1: fixed. `the_overview_grid_follows_reduce_motion` (`ui/overview.rs` tests) mounts the
  real Overview `view` with Reduce motion on and off, Tabs onto a card and reads the focus indicators
  ~75 ms on: none part-way with it on, the card's part-way with it off. Mutant O1 (`.instant(false)`)
  and O1b (`.instant(true)`) each fail it; restored, it passes.
- Findings 2 and 3: fixed. The first records each tick and asserts at `MIDWAY` and `DONE` through a
  lookup that panics on a missing tick; the second asserts on every tick. Mutant K1 still fails both.
- Finding 4: accepted. Code and tests shipped in one commit per milestone; a red cannot be recorded
  after the fact. Each milestone's tests were checked against its diff by review B (CLEAN, M1 and M2).
- Finding 5: declined as taste: the bound is loose, but a restart from none or full (the bug it
  guards) is far outside it, and the test also asserts `next < reached` and the end state.
- Finding 6: accepted, as for features 971 and 972.

## Mutation results

No mutation tool. 12 deliberate mutants, one at a time, each restored from a copy and the suite
re-run (`micold-client --lib`, 1203 then 1204 passed). Sampled: the fade, the press guard, the
instant flags at grid, card and Overview, the lift track, the settling draw, the rest-scope reset,
the zoom rule, the shadow scaling. Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| K1 `keyboard_focus.rs` fade `on_frame` → `restart_at(target)` (no fade) | T002/T004/T005 | No | 8 tests fail |
| K2 press guard `value() > 0.0` → `>= 1.0` | T003, R7 | No | `a_press_on_a_control_fading_out_drops_its_indicator_at_once` |
| G1 `card_grid.rs` grid `instant` not passed to cards | T005, FR-007 | No | 3 tests |
| G2 zoom no longer makes lifts instant | T014, C3 | No | `a_zoom_ends_the_lifts_at_once` |
| G3 rest-scope change keeps the old lifts | M2 review fix | No | `a_new_rest_scope_drops_the_old_sets_lifts` |
| G4 settling cards never drawn after the others | T014, FR-003 | No | `a_settling_card_is_drawn_over_its_neighbours_and_nothing_moves` |
| L1 `card_lift.rs` settled tracks kept | T013, FR-009 | No | `a_lift_eases_in_and_out_and_is_dropped_at_rest` |
| L2 `instant` ignored in `Lifts::step` | T013/T015 | No | 4 tests |
| S1 `style.rs` eased shadow keeps the full offset | T012 | No | `an_eased_shadow_scales_the_levels_shadow_by_its_strength` |
| O1 `overview.rs` `.instant(false)` | T009, FR-007 | **Yes** | real gap, finding 1; caught after the fix |
| O1b `overview.rs` `.instant(true)` | T009 | No (after the fix) | `the_overview_grid_follows_reduce_motion` |
| O2 `overview_card.rs` card fades even when `instant` | T004, FR-007 | No | 4 tests |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, SC-001 | `a_faded_indicator_eases_in_and_out_and_then_asks_for_nothing`, `an_activatable_cards_focus_fades_in`, `a_default_grids_card_focus_fades` | Widget and grid; Overview view by `the_overview_grid_follows_reduce_motion` (off half) |
| FR-002 | `a_faded_indicator_eases_in_and_out…` (press elsewhere), `the_cards_link_is_not_faded`, `an_instant_grid_shows_and_drops…` | Widget and grid |
| FR-003 | `a_lift_eases_in_and_out_and_is_dropped_at_rest`, `a_settling_card_is_drawn_over_its_neighbours_and_nothing_moves`, `focus_and_lift_ease_in_together` | Grid |
| FR-004 | `a_reversed_fade_turns_back_where_it_is_and_two_run_together`, `a_reversed_lift_continues_from_its_value` | Widget, unit |
| FR-005 | the same reversal test (two controls), `two_lifts_run_independently` | Widget, unit |
| FR-006 | none: decision R1 (growth not animated), posted on #1299 | n/a |
| FR-007, SC-002 | `an_instant_*`, `instant_lifts_reach_their_targets_on_the_first_step`, `reduce_motion_makes_card_focus_and_expansion_instant`, `the_overview_grid_follows_reduce_motion` | Yes (Overview view) |
| FR-008 | `reduce_motion_saved_mid_fade_ends_it_on_the_next_frame`, `reduce_motion_saved_mid_lift_ends_it_on_the_next_tick` | Grid |
| FR-009, SC-003 | the fade and lift tests' `RedrawRequest::Wait` checks; `tests/idle_requests_no_frames.rs` | Yes |
| FR-010 | `a_faded_indicator_at_full_draws_todays_indicator`, `an_expanded_card_at_rest_paints_the_shadow_it_cast_itself`, `an_eased_shadow_scales…`, the ring pixel tests | Grid (pixels) |
| FR-011, SC-004 | `a_press_on_a_faded_control_drops_its_indicator_at_once`, `the_cards_link_is_not_faded`, the unchanged ripple/menu/link/navigation suites | Yes |
| FR-012 | `reduce_motion_makes_card_focus_and_expansion_instant` | Grid |
| FR-013 | none: `docs/user-guide/settings.md` *Reduce motion* read, it names the focus outline and the expanded card's shadow | n/a |
| SC-005 | `a_settling_card_is_drawn_over_its_neighbours_and_nothing_moves` (every tick, every slot) | Grid |

Tests tracing to nothing: none.

## What was not audited

- Mutation was by hand, 12 mutants in the feature's six source files; no tool.
- Only `micold-client --lib` was run for the mutants; the full workspace suite ran once, in the gate.
- Perceived smoothness and frame pacing: no criterion is testable here; see `visual-pass.md`.
- The application's `update` path (hover rest, focus messages) that decides which card is expanded
  belongs to feature 1121 and was not re-audited.
