---
feature: 971-overview-card-ripple
verdict: FAIL # resolved at close, see Resolution
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: ee269b6d
behaviors: 9 # no test-list.md; derived from tasks.md T002-T008, T016-T017 (see below)
proven: 0
likely: 4
test_after: 5
no_test: 1
high_smells: 0
criteria_total: 15 # FR-001..009, SC-001..006
criteria_covered: 13
mutation_score: n/a # no tool; 3 deliberate mutants, 2 caught, 1 survived
mutants_survived: 1 # killed after the fix, see Resolution
suite: scoped run, 785 lib + 587 bin + 44 overview + 15 ripple_state passed, 0 failed, ~16s warm build
---

# TDD Verification: Overview card ripple

**Verdict: FAIL.** The delayed-close wiring in `main.rs::overview_message` (T012, US3-1, SC-005, FR-007) has no test: a mutant that drops the `ActivationDue` follow-up (an activated card would never close the Overview) passed every client test. Test-first is also unproven for M1 (no red recorded; tests and code in one commit) and only retroactive for M2.

The audit was run in a session that did not write the tests, but with no `tdd/test-list.md` the behaviors below are derived from `tasks.md`. Scope: the five 971 commits (`549ad621`, `47e382d1`, `f32c8d56`, `875caab2`, `ee269b6d`). The `47dff0df..HEAD` range also contains other features (937, 802, 806, 938...) that were not graded.

## Test-first evidence

| Behavior | Class | Evidence |
|---|---|---|
| T002/T007 `request`/`due` gating (`features/overview.rs`) | TEST_AFTER | M1 has no cycle log; `549ad621` adds tests and source together |
| T003/T004 pointer ripple, plain card (`overview_card.rs`) | TEST_AFTER | same commit, no red |
| T005 clipping (`ripple_clipping.rs`) | TEST_AFTER | same commit, no red |
| T006 independent/restart/drag-off | TEST_AFTER | same commit, no red |
| T008 press emits `ActivationRequested` (`ui/overview.rs`) | TEST_AFTER | same commit, no red |
| T016 Enter/Space centred ripple | LIKELY | `cycle-log.md` red (feature disabled afterwards); `f32c8d56` has test and source together; log says "written in one pass" |
| T016 keyboard ripple fades | LIKELY | as above |
| T017 captured key presses at the centre | LIKELY | as above |
| T017 repeat does not restart | LIKELY | red recorded, but the `repeat: false` guard in `is_activation_key` landed later in `875caab2` ("auto-repeat guard"); `an_auto_repeat_event_starts_nothing` is a negative case with no red |
| T012 timer start and `ActivationDue` follow-up in `main.rs` | NO_TEST | see mutant 3 |

The red for M2 was produced by disabling the feature after the fact. That shows the tests can fail, not that they came first.

Existing tests: the 971 commits add only lines to test files (`ripple_state.rs` +10; no deletions, skips, renames, or thresholds changed). Feature 813's `tests/overview.rs` is unchanged and passes (44). No weakened tests.

`tasks.md`: T001-T021 are all `[x]`; there is no list to compare against, but T012 is ticked with no test behind it (HIGH, finding 1).

## Findings

| # | Sev | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | The delay wiring is untested end to end. `ActivationRequested` starting a 300 ms timer and `ActivationDue` calling `due()` then `CardActivated` appear in no test (no `ActivationDue` outside `main.rs`). Mutant 3 survived 587 bin tests. US3-1, FR-007's 300 ms close and SC-005 are covered only by the state-level `request`/`due` unit tests and the manual visual pass. | `crates/micold-client/src/main.rs:661-680`; `crates/micold-client/src/main_tests.rs` has no case |
| 2 | HIGH (class) | M1 behaviors are TEST_AFTER: no red recorded, tests and code in one commit. FAIL per rubric. | `549ad621`; no M1 section in `tdd/cycle-log.md` |
| 3 | MED | No `tdd/test-list.md`; behaviors, `traces` and DONE states are unverifiable. Constitution I ordering ("must fail first") was not followed for M2 either (log admits one pass). | `specs/971-overview-card-ripple/tdd/` |
| 4 | MED | `the_ripple_is_drawn_in_the_states_content_colour` only checks the pixel lies between fill and content colour and moved by more than 2; it does not pin the pressed opacity, so FR-003 "at the pressed opacity" and US1-4 are loosely held. | `overview_card.rs` (test at ~line 607 of the diff, `moved`/`lo..hi` check) |
| 5 | MED | Keyboard activation is tested only at the widget level; no test that Enter on a card in the real Overview view publishes `ActivationRequested` (T008 covers the pointer only). Escape/Space during the delay is checked only via `Msg::Closed`/`Toggled`, not the key subscription path. | `ui/overview.rs` test `a_card_press_requests_its_activation` |
| 6 | LOW | `a_plain_card_does_not_ripple` asserts the widget tree tag is not `RippleState` (implementation coupled; the `press(...).is_empty()` half carries the behavior). | `overview_card.rs` `a_plain_card_does_not_ripple` |
| 7 | LOW | `EXPAND`/`FADE` constants and the `settle` loop (120 frames) are redefined per test module (`overview_card.rs`, `ripple.rs`, `tests/ripple_state.rs`); magic numbers `1000`, `120`, `2` px. | tests above |
| 8 | LOW | `ui/overview.rs` T008 compares `Debug` strings of messages (works, but couples to derive output). | `ui/overview.rs` end of `mod tests` |

No Assertion free, Tautological, Vacuous, Over-mocked, Self-approving snapshot, or Always-skipped HIGH smells found. Loops in tests (`for state in CardState::ALL`, pixel scan) assert unconditionally in the loop body. Tests are fast (the ripple/overview subset runs in under 1 s), deterministic, and use `Mounted` from the existing `test_support`.

## Resolution (close unit)

- Finding 1: fixed. `an_overview_activation_follows_once_its_delay_is_due` (`main_tests.rs`) drives `ActivationRequested` then `ActivationDue` through `update_inner`; with mutant 3 re-applied it fails, restored it passes.
- Finding 2: accepted. M1's code and tests shipped in one commit; a red cannot be recorded after the fact. M1's tests are the ones M1 Review B checked against the diff.
- Finding 3: accepted. The feature predates a `test-list.md`; behaviors trace to `tasks.md`.
- Findings 4-8: declined as taste or already held by neighbouring tests (the pressed opacity is pinned by the shared ripple tests of feature 018; keyboard activation is pinned at the widget level, the only place the Enter path differs).

## Mutation results

No mutation tool in the profile. Deliberate mutants, 3 sampled (card keyboard origin, activation gating, delay wiring); not exhaustive.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `overview_card.rs`: remove `.centred_on_keys()` | T016/T019 | No | caught by `enter_and_space_on_the_focused_card_ripple_from_the_centre`, `the_keyboard_ripple_fades_to_idle` |
| `features/overview.rs` `request`: drop `\|\| self.activating.is_some()` | T007/FR-007 | No | caught by `a_pending_activation_ignores_further_input` |
| `main.rs` `ActivationDue`: `Some(_) => return Task::none()` (activation never follows) | T012/US3/SC-005 | **Yes** | real gap, finding 1 |

Each mutant was restored from a backup copy; the final run (lib, bins, `overview`, `ripple_state`) is green and `git status` is clean.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, SC-001 | `a_press_ripples_from_the_press_point_in_every_state` | Yes (widget) |
| FR-002, SC-002 | `enter_and_space_on_the_focused_card_ripple_from_the_centre`, `a_captured_activation_key_presses_at_the_centre` | Widget only (finding 5) |
| FR-003, SC-003 | `a_card_ripple_does_not_paint_outside_the_card`, colour test (loose, finding 4) | Yes |
| FR-004 | `after_the_fade_a_pointer_press_leaves_no_layer`; keyboard half rests on M1 tests (per log) | Partial |
| FR-005, SC-004 | independence, restart, repeat, fade-to-idle tests; `the_overview_delay_is_the_ripples_expand_time` | Yes |
| FR-006 | `a_plain_card_does_not_ripple` | Yes |
| FR-007, SC-005 | `request`/`due` unit tests only | **No** (finding 1) |
| FR-008, SC-006 | `the_activation_after_the_delay_closes_as_before`; 813 `tests/overview.rs` unchanged (44 pass) | Partial |
| FR-009 | no automated test; visual pass PNGs only | **No** |

Untested through the real entry point: FR-007/SC-005 and FR-009 (automated). Tests tracing to nothing: none.

## What was not audited

- Mutation tooling: none exists; only 3 deliberate mutants. `ripple.rs` `is_activation_key`/`captured_before` and `ripple_clipping.rs` were not mutated.
- The full workspace suite and `scripts/gate.sh`; only the client lib, bin, `overview` and `ripple_state` binaries ran.
- Other features in the `47dff0df..HEAD` range, and the layout snapshot churn in that range.
- Coverage (not configured), performance, and the manual visual-pass images.
- Remediation tasks were not appended to `tasks.md` (`--no-tasks`).
