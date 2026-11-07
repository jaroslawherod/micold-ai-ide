---
feature: 1703-composer-layout-large-editor
verdict: FAIL # findings 1-4, 6-8 fixed in the close PR; see autopilot.md
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 18a4aa800 # plus one uncommitted change in crates/micold-client/src/ui/composer.rs (activity_indicator SM padding)
behaviors: 12 # no test-list.md; the 12 test tasks of tasks.md stand in (T002-T006, T014, T017-T020, T026, T027)
proven: 0
likely: 0
test_after: 12 # no red recorded for any; tests and source land in the same commits
no_test: 0
high_smells: 3
criteria_total: 19 # functional requirements FR-001 to FR-019
criteria_covered: 15 # FR-003, FR-004, FR-009 (tonal/40 dp), FR-014 have no effective test
mutation_score: unmeasured # no mutation tool in the profile; 3 deliberate mutants, 1 caught, 2 survived
mutants_survived: 2
suite: targeted only - micold-client --test main (112 passed), --lib + layout_snapshot filtered (14 + 28 passed), 0 failed; full workspace not run
---

# TDD Verification: Structured view composer layout and large editor

**Verdict: FAIL.** Two existing tests lost the assertion that exercised the real entry point (the selector press, the picker arrow keys), and two shipped behaviors (Bypass marker, tonal settings icon) survive deliberate mutants because no test pins them.

Not an independent audit of authorship: the auditor did not write these tests, and every file was re-read cold. No fresh-context subagent was used for the smell pass; findings below were checked line by line.

## Test-first evidence

No `tdd/` directory exists: no test list, no cycle log. Per the rubric, no recorded red means `TEST_AFTER`. Git history cannot rescue it: tests and source land together, and several commits are `wip(...)` snapshots ("not yet compiled" `c63202294`, "tests pending" `3a2f3aef6`, "unverified" `e54bc8657`, `6d0f5aa69`). Classification is by test task in `tasks.md`.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T002 large-editor reducer | TEST_AFTER | `c63202294` adds `large_editor.rs` and `features_large_editor.rs` together; no red |
| T003 chord in `composer_key` | TEST_AFTER | `c63202294`, same commit as source; no red |
| T004 open-button predicate | TEST_AFTER | `c63202294` |
| T005 shortcut sheet entry | TEST_AFTER | `c63202294` |
| T006 dialog registration | TEST_AFTER | `c63202294` |
| T014 activity icon view | TEST_AFTER | `3a2f3aef6` source + `composer_tab.rs` together |
| T017 settings field | TEST_AFTER | `3a2f3aef6` |
| T018 options reducer | TEST_AFTER | `3a2f3aef6` |
| T019 persistence | TEST_AFTER | `3a2f3aef6` |
| T020 options-row view | TEST_AFTER | `3a2f3aef6`; later patched twice (`04f2306ce`, `725ef5ec4`) to make it pass |
| T026 selector geometry gate | TEST_AFTER | `e54bc8657` (wip, unverified) adds source and gate together |
| T027 filter and unchanged behavior | TEST_AFTER | `e54bc8657` / `24b4507d8` |

`tasks.md` check: all of T001-T032 are ticked and the code exists; no ticked task lacks a test or a visual-pass record. T032 evidence screenshots are untracked in the working tree.

### Existing tests changed by the feature

| Test | Before | After |
| --- | --- | --- |
| `tests/gates/composer_fits.rs:749` `pressing_the_mode_control_opens_the_four_modes_and_picking_one_asks_for_it` (`24b4507d8`) | `lay::painted_text_pressing(..., &control)` pressed the drawn control and the list had to appear | `state.permission_prompts.toggle_mode_menu(id, 0)` opens the list in state (line 759); the second render is `painted_text_settled_at` and nothing is pressed |
| `tests/features_command_palette.rs:554` `the_picker_rows_end_with_the_typed_name...` (`4f8bc743b`) | 4 rows; `rows[1].item == Some(1)`; arrows highlight an item (`enter_row == Some(0)`); arrows reach the typed row (`Some(3)`) | 1 row; arrow navigation assertions deleted entirely |
| `tests/gates/composer_fits.rs:411,578` (`753c721d4`) | click at `transcript.x + 100.0` | `+ 200.0` (layout moved; a magic offset, same assertion) |
| `popover_displacement.rs`, `root_vocabulary_is_cross_cutting.rs` | counts 29 and 30 | 30 and 31; justified by the new dialog and variant |
| `tests/composer_tab.rs` `tab_from_the_text_area...` | Tab lands on Add attachment | lands on settings icon (order changed by #1764) |
| `tests/fixtures/layout_snapshot.txt` | | regenerated in `16d679971`, `3a2f3aef6`, `24b4507d8` (4498-line churn), `753c721d4`; accepted in the same commit as the code |

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Weakened existing test: the press that opens the selector list is no longer exercised. The list is pre-opened through state; `press_path` is still computed but only used for the pick step. No remaining test shows that a press on the trigger emits the toggle message (FR-016 open state, entry point). Should keep `painted_text_pressing` for the open step, or add a test that a press on the trigger yields the toggle message. | `crates/micold-client/tests/gates/composer_fits.rs:759-768` |
| 2 | HIGH | Weakened existing test: arrow-key navigation over the model picker rows (items and the typed row) is no longer asserted anywhere after `4f8bc743b`. A non-matching typed string was chosen, which removed the rows the arrows move over. Should keep a navigation assertion with a typed string that leaves several rows. | `crates/micold-client/tests/features_command_palette.rs:572-583` |
| 3 | HIGH | Vacuous test: `attachments_are_never_read_or_written` builds a draft with no images, so `images.len()` is 0 before and after; a `Done` that dropped the attachments would pass. Should put two images on the draft and assert the same two after `Done` and after `Cancelled`. | `crates/micold-client/tests/features_large_editor.rs:110-117` |
| 4 | HIGH | Untested criteria: FR-014 / US3.5 (Bypass warning marker) and FR-009 (settings icon tonal while open, 40 dp, label) have no test; only the quickstart visual pass. Deliberate mutants (A, D) survive. FR-004 placement (top-right) and FR-007 per-state tooltip/accessible label wiring are likewise visual-only. | `crates/micold-client/src/ui/composer.rs:331-345` (`.selected(open)`, `if bypass`); no test references `options_tooltip` |
| 5 | MED | No test-first evidence for any behavior and no `tdd/` artifacts; M1 Review B already noted it, not retrofittable. | whole feature |
| 6 | MED | `done_writes_to_a_session_that_ended_meanwhile` never ends a session (a comment stands in); `State::default()` has no sessions anyway, so it equals the ordinary Done test. FR-019 ended-session edge is unproven. | `tests/features_large_editor.rs:90-98` |
| 7 | MED | `the_activity_state_is_an_icon...` omits the Busy state from its `activity()` table and from the no-word check (only Idle, NotRunning), and expects values from the same constants the code uses. SC-003 claims 100% of states. | `tests/composer_tab.rs:337-366` |
| 8 | MED | Options-row order: the doc comment says "Permission is last" but only Model before Effort is asserted; Permission position (FR-010) is unchecked. | `tests/composer_tab.rs:369-394` |
| 9 | MED | Selector gate omits the focus state (3 dp secondary ring), the ripple, the chevron rotation and the disabled tooltip reason, yet SC-004 claims "all five states" and FR-016 lists the tooltip. | `crates/micold-client/src/ui/material/menu_select_anatomy.rs` (only Active, Hovered, Pressed, Disabled, open) |
| 10 | MED | Form-save exclusion for `composer_options_open` (T019) is covered only by the compile-time exhaustive destructure, with no behavioral test that a form save leaves the flag alone. | `crates/micold-client/src/shell/persist.rs` (`set_changed`) |
| 11 | MED | Uncommitted change to `activity_indicator` (`.padding(spacing::SM)`) has no test and was not regenerated against the layout fixture; the filtered layout run passed, so the fixture does not pin it. | `crates/micold-client/src/ui/composer.rs` working tree |
| 12 | MED | Layout fixture regenerated wholesale in the same commit as source four times; review of the diff is not demonstrable (self-approving risk). | `crates/micold-client/tests/fixtures/layout_snapshot.txt` |
| 13 | LOW | Magic layout numbers carry the rule: `+ 200.0` (x2), `settings.x - attach.x > 400.0`, `send.x - settings.x < 120.0`, glyph literals `"\u{e8b8}"`, `"\u{e316}"`. | `tests/gates/composer_fits.rs:416,583`; `tests/composer_tab.rs:270,~420-430` |
| 14 | LOW | Eager test: `a_model_effort_or_permission_refusal_opens_the_row_and_a_drop_notice_does_not` asserts three behaviors; `the_input_row_puts...` asserts four orderings. | `tests/features_composer.rs` (~1508); `tests/composer_tab.rs` |

Hard-rule notes: no secrets, no repository text issuing instructions. No sleeps or clocks; the tests are deterministic. Speed: the targeted runs took 0.2-4.4 s after build.

## Mutation results

No mutation tool (profile: `mutation: null`). Three deliberate mutants, sampled on the behaviors with the most direct acceptance criteria; one at a time, restored byte-for-byte (`cmp`) and re-verified green.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `ui/composer.rs` `if bypass` to `if !bypass` (Bypass marker) | FR-014 / US3.5 | Yes | Real gap: no test (finding 4). `composer_tab` + `composer_fits` + `composer_parts_fit`: 28 + 6 passed |
| `ui/composer.rs` `.selected(open)` to `.selected(false)` (tonal icon) | FR-009 / US3.2 | Yes | Real gap (finding 4) |
| `features/composer.rs` `lines > 1` to `lines > 2` (open-button predicate) | FR-004 | No | Caught by `the_open_editor_button_shows_for_several_lines_focus_or_hover` |

Sample size 3, not exhaustive. Reducer and persistence logic (large editor, auto-expand, settings) were read but not mutated. Score unmeasured.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-001 chord | `features_composer::the_large_editor_chord_opens_it...`; shell route in `shell/composer.rs` (no test of the routed message) | Partly (key map only) |
| FR-002 / US1.1-3 open, Done, Esc | `features_large_editor` reducer + `editor_key` | Reducer, no view test |
| FR-003 attachments | `attachments_are_never_read_or_written` | No (vacuous, finding 3) |
| FR-004 button | predicate unit test only | No (placement visual only) |
| FR-005 / SC-006 shortcut lists | `keys.rs` sheet test; docs not checked here | Yes for in-app sheet |
| FR-006 / FR-008 indicator and no permission selector in the input row | `composer_tab::the_input_row_puts...`, `the_options_row_holds...` | Yes (rendered view) |
| FR-007 words as tooltip/label | `composer_tab::the_activity_state_is...` (no visible word) | Partly (no Busy, tooltip wiring untested) |
| FR-009 toggle, keyboard | `the_settings_icon_toggles_from_the_keyboard` | Yes for Space/Enter; tonal/40 dp no |
| FR-010 / FR-011 / SC-005 | `composer_tab`, `settings` tests, `startup`, `persist`, `shell/composer` mock store | Yes (unit level; restore via `restore_folds`, not full boot) |
| FR-012 auto-expand | `features_composer`, `shell/composer` test | Yes |
| FR-013 Stop / note stay visible | `features_composer::stop_session_and_the_refusal_note...` asserts state only | No (view not checked) |
| FR-014 Bypass marker | none | No |
| FR-015 geometry | `menu_select_anatomy` (lib) | Yes (gate on showcase selector, not composer's) |
| FR-016 states | `menu_select_anatomy` (no focus, ripple, tooltip) | Partly |
| FR-017 filter | `features_composer::selectors`, `composer_fits` (pre-opened list) | Partly (finding 1) |
| FR-018 unchanged | `features_composer::selectors`, `composer_fits` mode list | Yes |
| FR-019 session binding | `done_writes_the_text_to_the_bound_session_only` | Yes (ended-session part no, finding 6) |

Untested criteria: FR-014, FR-009 tonal look, FR-004 placement. Tests tracing to nothing: none found (`hover_follows_the_pointer_per_session` backs FR-004's hover rule).

## What was not audited

- Full workspace suite (`mise run gate`): not run. Only `micold-client --test main` filtered to composer, large-editor, overlay and palette tests (112 passed), the filtered `--lib` plus `layout_snapshot` run (14 + 28 passed), and the mutation reruns. `micold-core` settings tests were read, not executed.
- The full `layout_snapshot` fixture comparison test and the `menu_select_anatomy`-adjacent showcase tests were not run.
- Docs (`structured-input.md`, `terminal-panes.md`) were not checked for the chord.
- Mutation: no tool; 3 deliberate mutants only, none on the reducer, persistence or settings logic.
- Coverage: no tool in the profile.
- The quickstart visual pass evidence (screenshots) was not re-judged.
- Test-first ordering across PRs #1705 #1717 #1754 #1770 was judged from commit contents only; squash history on `main` was not consulted.
- Performance and cross-platform (macOS Cmd branch compiled only via `cfg!`, no run on that target).
