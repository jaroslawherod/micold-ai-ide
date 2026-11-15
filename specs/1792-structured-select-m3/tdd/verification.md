---
feature: 1792-structured-select-m3
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset exists)
verified_at: 0ac1001b8 # short SHA audited; tree clean before and after
behaviors: 15 # no test-list.md: the rows of the table below, from tasks.md T002-T005, T011, T013-T018 plus D1
proven: 0
likely: 6
test_after: 7
no_test: 1
not_applicable: 1
high_smells: 0
high_findings: 3 # survivors, M1 test-after, FR-002 focus/pressed untested
criteria_total: 17 # rows of the Traceability table
criteria_covered: 14 # not covered: US1.2 focus/pressed, US2.2 item layers, SC-003 (gate not run)
mutation_score: unmeasured # no cargo-mutants; 9 deliberate mutants on M3 files, 7 caught, 2 survived (same root cause)
mutants_survived: 2
suite: targeted only, 0 failed (lib expand_tests 7, bin shell::composer::tests 8, layout_snapshot 180, main layout_snapshot_regeneration 6); full workspace not run
---

# TDD Verification: Structured view Material 3 select and expand animation (#1792, M1-M3)

**Verdict: FAIL.** Two deliberate mutants in the M3 row layout (`around_options`) survive every test, M1's
tests were written after its code by the ledger's own account, and the focus and pressed state layers
of the structured select (US1 scenario 2, FR-002) have no test and no visual check.

Audit independence: the auditor did not write these tests. Every cited file was re-read cold. The smell
pass was done directly, not through a subagent.

Scope as given: M1 (#1842) and M2 (#1851) are squashed upstream and absent from `git log`; M3 is
`aa9968e9a^..aa9968e9a`, one squashed commit. Resolver note: `check-prerequisites.sh` returned
`specs/1816-supervisor-verify-report` (the checked-out branch); the feature directory was taken from the
user's input instead.

## Test-first evidence

There is no `tdd/test-list.md`; the feature was not planned through `speckit-tdd-plan`. The cycle log
covers M3 only and says so. Behavior ids below are task ids. Git cannot corroborate order for any of
them: M3 is a single commit (tests and source land together), M1 and M2 are not in the history.

| Behavior | Class | Evidence |
|---|---|---|
| T002 trigger geometry/paint vs `Select` (rest, open, hover) | TEST_AFTER | No cycle log for M1. Autopilot ledger, review B round 1 on M1: "no tdd cycle-log, tests after implementation" |
| T002 focus and pressed layers | NO_TEST | Ticked `[x]` but no test exists (Finding 3) |
| T003 disabled trigger | TEST_AFTER | As T002 |
| T004 long value, value missing, empty list (3 tests) | TEST_AFTER | As T002 |
| T005 three selects share a trigger | TEST_AFTER | As T002; test also weak (Finding 7) |
| T011 menu parity | NOT_APPLICABLE | Characterization, green on first run by design (tasks.md T011/T012) |
| T013 duration token and Reduce motion (2 tests) | LIKELY | Cycle log red recorded with message for the duration test; history squashed |
| T013 `under_reduce_motion_the_row_switches_in_one_step` | TEST_AFTER | Log: "passed red"; mutation check recorded afterwards (Review B F2), not re-run here (Finding 5) |
| T014 reversal | LIKELY | Red "mid-way: 160" |
| T015 two sessions | LIKELY | Red "still on its way: 160 vs 160" |
| T016 collapsed row unreachable | TEST_AFTER | Log: red only on the probe's own threshold, "pins `expand`'s hidden guard rather than driving code" |
| T017 collapse closes pickers | LIKELY | Red recorded at features level; the test later moved to `shell::composer::tests` and "ran green at once" |
| T018 narrow width end height | LIKELY | Red "grew by 90.2, not row 82.2 plus gap" |
| D1 `/model`, `/effort` expand the row | LIKELY | Red recorded for a test (`a_picker_opened_by_command_opens_the_options_row`) that no longer exists (Finding 4) |
| D1 effort/mode trigger press reopens the row | TEST_AFTER | Added after review A F1; the shell test ran green at once |

### Existing tests changed by the diff

No assertion was removed, loosened or skipped. Checked:

- `tests/gates/composer_fits.rs`: `find` became `find_under` for three call sites (`:134-150`, `:736`, `:789`).
  The assertion is unchanged; the search now excludes matches above the transcript. Narrower input, same
  check. Not a weakening.
- `tests/gates/containment.rs:68-82`: a new allowlist entry, `OPTIONS_ROW_REVEAL` (one path, transcript
  states, bottom edge only), exempts the collapsed row from `no_layout_node_escapes_its_parent`. This is an
  exclusion added to a gate. It is justified and pinned by `the_recorded_options_row_escape_is_its_reveal`
  (ran, green), and it fails closed if the path stops firing. Recorded, not a finding. `drift-M3.md` also
  says "plus the shifted recorded child paths"; the diff shows no change to any existing path (Finding 10).
- `tests/fixtures/layout_snapshot.txt`: +171 lines, regenerated. The removed lines are path shifts and one
  widened row (`443.8` to `666.7`). Mutant M9 turned this snapshot red, so it discriminates; I did not
  re-derive the fixture independently (see "not audited").
- `shell/composer.rs`, `features/composer.rs`: `mod tests` became `pub(crate) mod tests` to share
  `structured_state`; no test touched.
- No coverage or mutation threshold exists to lower.

### tasks.md against evidence

T013-T021 ticked and each has a test (or code) that exists and passes. T017's task text names
`features/composer.rs`; the test lives in `shell/composer.rs` (feature write isolation, explained in the
cycle log). T002 is ticked `[x]` though its focus and pressed states are untested (Finding 3). T022-T023
are unticked: correct, they are the close unit. No behavioral task is unticked with a done behavior.

## Findings

| # | Sev | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Two deliberate mutants survive every test, `layout_snapshot` included: `let between = over && under;` to `false` (M2), and the trailing `gap(under)` to `gap(false)` (M7). With a part above and a part below the row (a notice plus a draft image or chip), the 8 px gaps vanish or shift, so the doc comment's "a collapsed row leaves the composer exactly as tall as before" is unpinned. Every test poses a notice above and nothing below. Not an equivalent mutant: it changes laid-out height. Should assert collapsed height equals the no-row height, and the open row's gaps, with both sides present. | `ui/composer.rs:300,302`; `ui/composer_expand_tests.rs` (no pose with `below` parts) |
| 2 | HIGH | M1 (T002-T005) has no red evidence and the ledger admits tests came after the implementation. No cycle log exists for it, history is not available. Fail closed: TEST_AFTER for every M1 behavior. | `autopilot.md` review table row "B 1 M1"; `tdd/cycle-log.md` has only M3 |
| 3 | HIGH | FR-002 and US1 scenario 2 name hover, focus and pressed layers. Only hover, open and disabled are tested on the `MenuSelect` trigger. `SelectTrigger::focused` is passed by `Select` (`select.rs:320`) but never by `MenuSelect` (`menu_select.rs:416-420`); `research.md:8` records that its focus "shows the button's keyboard ring, not the filled field's focus layer". The M1 visual pass marked focus and pressed NOT CHECKED. So the structured select's focus and pressed state is neither tested nor seen, while T002 is ticked and FR-009 says "match in every state". | `menu_select_anatomy.rs:90-215`; `menu_select.rs:416`; `visual-pass-M1.md:16,30`; `tasks.md:29` |
| 4 | MED | Cycle log's red command filters on `a_picker_opened_by_command`, a test that no longer exists (moved and renamed to the shell test). Re-running the logged command today matches 7 of the 9 tests it claims. The moved tests have no red of their own. | `tdd/cycle-log.md:8,22,33-41`; `grep` finds no such test |
| 5 | MED | Two M3 pins were never red: `under_reduce_motion_the_row_switches_in_one_step` (passed red) and `a_collapsed_rows_controls_are_not_reachable` (red only on its own threshold). The first has a recorded post-hoc mutation check that I did not repeat; the second was accepted without one because the guard lives in `micold-material/animation.rs` (outside the feature's files, so I did not mutate it either). Unmeasured, not passing. | `composer_expand_tests.rs:117,247`; `cycle-log.md:19,43-46` |
| 6 | MED | M2 menu parity is geometry only; elevation, selected emphasis and per-item state layers (FR-004, US2) are inferred from both controls calling the same helpers, which the test's own doc admits. A change in `MenuSelect`'s row paint would pass. | `picker_parity.rs:559-602` |
| 7 | MED | The "all three selects share one trigger" test (FR-003) checks two labels' y and `chevrons >= 3` over the whole pane. The effort control is never named ("built alike" in the doc), and `>= 3` also passes if a fourth chevron is elsewhere and one select lost its own. Should assert each of the three triggers by name. | `tests/gates/composer_fits.rs:1103-1122` |
| 8 | MED | Eager tests: `a_toggle_mid_way_reverses_from_the_current_height` covers reversal, monotonic collapse and nine rapid toggles; `a_list_opened_with_the_row_collapsed_opens_the_row` covers four entry points in one body (messages are labelled, so roulette is mitigated). A failure names the line, not the behavior. | `composer_expand_tests.rs:130-171`; `shell/composer.rs:634-696` |
| 9 | LOW | Magic values: 12 px click grid, `between > 3`, pixel column range `160..198`, `step = (full - shut) / 3.0`, `TOLERANCE` redeclared in two test files. | `composer_expand_tests.rs:228-241,105,135`; `menu_select_anatomy.rs:182` |
| 10 | LOW | `drift-M3.md` claims `containment.rs` has "shifted recorded child paths"; the diff shows none. Record inaccuracy. | `drift-M3.md`, `containment.rs:65-82` |
| 11 | LOW | No `test-list.md`: the loop had no per-behavior id, so traces and states cannot be checked mechanically. | `tdd/` contains only `cycle-log.md` |

Smell catalogue result for the files read (`composer_expand_tests.rs`, `menu_select_anatomy.rs`,
`picker_parity.rs` T011 block, `shell/composer.rs` new tests, `composer_fits.rs:1103`, `containment.rs`
new test, `select_trigger.rs` test): no HIGH smell found. No tautological assertions, doubled subjects,
conditional logic deciding assertions, skipped or empty tests. Style matches the exemplars (in-crate
`#[cfg(test)]` modules, `Mounted` support, `FakeSettingsStore` reused, message strings state the rule).
One mild re-implemented expectation: `menu_select_anatomy.rs:121-128` converts `primary` to bytes in the
test; acceptable as a quantisation, not the code's logic. Properties: deterministic (frame-stepped clock,
no sleeps), fast (expand tests 0.1-0.3 s), insensitive to refactoring except pixel-column probes in M1.

## Mutation results

Tool: none in the profile (`mutation: null`, no cargo-mutants). Deliberate mutants only: 9 distinct
mutants on 5 behaviors in files M3 changed (`ui/composer.rs`, `shell/composer.rs`,
`shell/permission_prompts.rs`, `features/composer.rs`), one at a time, restored with `git checkout --`,
suite re-run green after. 12 test runs (three mutants were also run against `layout_snapshot`). This is a sample, not a
score. M1 and M2 files were not mutated: sampling concentrated on M3, where behavior is state logic.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `ui/composer.rs` duration `MEDIUM_2` to `MEDIUM_2 / 2` | T013 | No | 4 tests fail |
| `ui/composer.rs` `expand(.., !options_open, ..)` | T013/T014/T016/T018 | No | 5 tests fail |
| `ui/composer.rs` leading `gap(over && !under)` to `gap(false)` | T018 | No | caught by `at_a_narrow_width...` and by `layout_snapshot` |
| `ui/composer.rs:300` `between` to `false` | T019 gap logic | **Yes** | Not equivalent (changes height). Finding 1 |
| `ui/composer.rs:302` trailing `gap(under)` to `gap(false)` | T019 gap logic | **Yes** | Not equivalent. Finding 1 |
| `shell/composer.rs` drop `effort_picker.close_all()` | T017 | No | 2 shell tests fail |
| `shell/permission_prompts.rs` drop `expand_options()` | D1 mode trigger | No | 1 test fails |
| `features/composer.rs` drop `/effort` `expand_options()` | D1 | No | 1 test fails |
| `features/composer.rs` drop `/model` `expand_options()` | D1 | No | 1 test fails |

Not mutated: the `reduce_motion` branch of `options_motion` (log claims it was done, not repeated),
`palette.close_pickers()` and `close_mode_menus()` individually, `persist_options_if_changed`, and all M1
and M2 sources.

## Traceability

Acceptance scenarios and requirements, mapped to the tests that run today. "E2E" means the real entry
point (`view`, `update`, or the rendered pane).

| Criterion | Tests | E2E |
|---|---|---|
| US1.1 / FR-001 select look at rest | `menu_select_anatomy::the_trigger_has_the_container_geometry...`, `...paints_the_selects_container...` | Unit only (trigger built directly); the pane check is Finding 7 |
| US1.2 / FR-002 hover | `hovering_the_trigger_shades_its_container` | No |
| US1.2 / FR-002 focus, pressed | none | none (Finding 3) |
| US1.3 disabled | `a_disabled_trigger_looks_disabled_and_ignores_the_pointer` | No |
| US1.4 / FR-003 same look | `composer_fits::the_model_and_permission_selectors_share_one_trigger_on_one_line` | Yes, weak (Finding 7) |
| FR-009 match showcase every state | `menu_select_anatomy` rest/open/hover/disabled vs `Select` | Partial: focus, pressed missing |
| US2.1 / FR-004 menu look | `picker_parity::the_structured_select_menu_has_the_selects_panel_and_rows` | Geometry only (Finding 6) |
| US2.2 menu item state layers | none directly | none |
| US3.1, US3.2 / FR-005, FR-006, SC-002 | `opening_and_closing_take_the_standard_duration_without_a_jump`, `the_row_takes_the_standard_duration...` | Yes (`ui::composer::view`) |
| US3.3 / FR-007 | `a_toggle_mid_way_reverses_from_the_current_height` | Yes |
| US3.4 / FR-013 | `under_reduce_motion_the_row_switches_in_one_step` | Yes |
| FR-008 | `two_sessions_keep_separate_tracks` | Yes |
| FR-010, SC-004 | `composer_fits` pick tests, unchanged and green; no new test | Yes (pre-existing) |
| FR-011 empty list, missing value, long text | `an_empty_option_list...`, `a_value_missing...`, `a_long_value...` | Unit |
| FR-011 small window | `at_a_narrow_width_the_row_ends_at_its_wrapped_height` | Yes |
| FR-012 visual pass | `visual-pass-M1.md`, `visual-pass-M3.md` (manual) | Manual; M1 left focus, pressed, open list unverified |
| SC-003 no literals | `tests/motion_tokens.rs`, `ui_glyph_literals.rs` named in T023, not run here | Not run |

Criteria with no test: US1.2 focus and pressed, US2.2 item state layers. Tests tracing to nothing:
`containment::the_recorded_options_row_escape_is_its_reveal` (gate support, no criterion),
`a_list_opened_with_the_row_collapsed_opens_the_row` (decision D1, not a spec requirement; worth a spec
note). Counts in the frontmatter treat US1.2 as one criterion partly covered.

## What was not audited

- Full workspace suite: not run. The profile's suite is 5071 tests over 429 binaries; this cloud box has no
  `mise` or `cargo-nextest`, and a workspace link would write tens of GB. Targeted runs only: first
  `micold-client` lib/bin test build 3m23s cold, targeted reruns 40-50 s each. Baseline against `cdc473ab`
  not compared.
- Mutation tool runs: none exist. Deliberate mutants covered M3 only; no M1 or M2 source was mutated and
  `micold-material` (`animation.rs`, `button.rs`, `form_field.rs`) was out of the feature's file set for
  M3 and untouched.
- Git ordering for M1 and M2: unavailable (squashed upstream); M3 is one commit. No class above is
  `PROVEN`.
- `tests/motion_tokens.rs`, `ui_glyph_literals.rs`, `material_boundary.rs`, `one_overlay_implementation.rs`
  (T023) were not run.
- The regenerated `layout_snapshot.txt` was not independently re-derived or visually reviewed.
- Visual pass PNGs and `visual-pass-M3.md` claims were not re-checked; no display here.
- Coverage: none available (`coverage: null`). Performance and load: no criterion, not assessed. The
  `sandbox-real-runtime` acceptance suite is not related to this feature and was not run.
- `showcase/` entries (T009) and docs tasks (T010, T021) were not audited for test coverage.

## Remediation by the close unit (not a re-audit)

The verdict above is as audited (FAIL). Post-remediation, after review round 1 of the close PR, by finding:

- 1 (T024): cleared; both `around_options` survivors run red in the close unit.
- 2 (T026): cleared; all five M1 mutants run (`tdd/cycle-log.md`). Two first survived (disabled chevron colour, long-value ellipsis) and each is now killed by a strengthened test.
- 3 (T025): cleared; pressed and ring tests proved red by product mutants (the `Ripple` wrap, the focus `Indicator`); the ring deviation is in `spec.md` FR-002.
- 4, 5 (T029): cleared; `close_pickers()` and `close_mode_menus()` mutants both red; `reduce_motion` already proved.
- 6 (T027): DECLINED, as at M2 (geometry-only parity by decision; ledger *Declined review findings*). Criterion US2.2 stays covered by geometry only.
- 7 (T028): cleared; Model, Effort and Permissions are named, in order.
- 8 (T030): cleared; the two eager tests are split, one test per entry point.
- 9, 10 (T031): cleared; magic values named.
- 11 and the manual rows: not re-audited. Still open: the focus and pressed visual run and step 7 (T022b), #1765 and `mise run duplication` (T023b); see the ledger's Follow-ups not done.

The mutants were run by the close unit; this is not an independent re-audit, so the verdict is not re-issued as PASS here: the open items are outside the test-strength findings.
