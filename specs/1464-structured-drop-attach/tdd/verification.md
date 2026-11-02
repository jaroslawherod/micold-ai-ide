---
feature: 1464-structured-drop-attach
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: edc9f7521 # short SHA audited (branch reset to origin/main after M1-M5 merged)
behaviors: 0 # no tdd/test-list.md by design; see "Test-first evidence"
proven: 0
likely: 0
test_after: 0 # not classifiable: no list, no cycle log; all 5 milestones UNPROVEN
no_test: 0
high_smells: 1 # F1; F2-F5 are surviving mutants or an untested criterion, also HIGH
criteria_total: 16 # US1 5 + US2 3 + US3 5 + US4 3 acceptance scenarios (FR-001..016, SC-001..006 mapped below)
criteria_covered: 11 # at some test level; 1 has no test, 4 only below the real entry point
mutation_score: unmeasured # no cargo-mutants; 9 deliberate mutants, 5 caught, 4 survived
mutants_survived: 4 # none equivalent
suite: 12389 passed, 0 failed, 20 ignored, 870s (cargo test --workspace, incl. build)
---

# TDD Verification: Structured View drop and "Add attachment"

**Verdict: FAIL.** Four of nine deliberate mutants survived inside ticked tasks (the picker result route, the attach chord route, the button tooltip, the drop-cue wrapping), and the headline drop/picker parity test is tautological.

The audit was run from cold context in a session that did not write these tests. Hard rules observed: no existing test or source file was edited; every mutant was restored (`git status` clean after each) and the affected targets re-run green.

## Test-first evidence

No `tdd/test-list.md` or `tdd/cycle-log.md` exists, by design (`autopilot.md` Decisions, Review B M2 F3). Per the rubric, "no red recorded" is not `PROVEN`/`LIKELY`; there are no behavior ids to classify, so the table is per milestone from `tasks.md` and git history.

| Unit | Class | Evidence |
|---|---|---|
| M1 T002-T008 | UNPROVEN | `7d95a5731` adds tests, source and docs in one commit; no red recorded. `tasks.md` T002/T005/T006 say "failing/characterisation tests first", not evidenced |
| M2 T010-T019 | UNPROVEN | `53633e7d8` ("WIP, gate pending") one commit for tests and source |
| M3 T020-T023 | UNPROVEN | `22d4adf67` one commit |
| M4 T024-T033 | UNPROVEN | `0d8792b6e` ("unverified") one commit, then two `wip(1464)` commits and test fixups (`abd3e1ec0`, `a4205f769`) |
| M5 T034-T039 | UNPROVEN | `e742fb3b8` one commit, then three fix commits |

History is milestone-granular, so ordering cannot be shown either way (not a proof of test-after). Fail-closed: not `PROVEN`.

Existing tests changed by the feature (checked every removed assertion line in the feature commits):
- `composer_tab.rs:~214` doc comment reworded and an Add-attachment Tab stop added: strengthened.
- `icons.rs` `Icon::ALL.len()` 85 to 86 plus a pinned-glyph test: legitimate.
- `review_keys.rs` `model.rows` to `model.rows_of("Review")`, same counts and values: legitimate (shape change).
- `command_palette.rs` last App command `model` to `shortcuts`: legitimate (new command sorts last).
- `pane_drop_target.rs` `drop_at`/hover helper refactors (`e742fb3b8`, `5f589c547`, `5091215eb`): behavior kept.
- No skips, ignores, exclusions or lowered thresholds. No weakened existing test.

`tasks.md`: T001-T039 ticked; T040 and T041 unticked (see F9). There is no test list to cross-check ticks against.

## Findings

Test strength first, then docs, then unbuilt behavior. Severity per rubric.

| # | Sev | Type | Finding | Evidence |
|---|---|---|---|---|
| F1 | HIGH | test-strength | Tautological parity test: `the_same_ordered_files_by_drop_or_by_picker_give_the_same_composer` feeds the identical `drop_of(&images,&paths)` to `apply_drop` on both arms, so it passes whatever `read_picked` does. FR-008, FR-013, SC-001 are not verified; the picker arm never touches `shell/composer.rs::read_picked` or `shell/drops.rs::read_for_composer`. Should drive the shell `update` with `AttachPicked` and a drop and compare composers (as `main_tests.rs` panes do) | `crates/micold-client/tests/features_composer.rs:1389` |
| F2 | HIGH | test-strength | Mutant M4 survives: `AttachPicked` made to read regardless of `attach_picked` (late result for an ended session, FR-012) and nothing fails. No test routes `AttachRequested`, `AttachPicked` or `AttachPickerClosed` through the shell; only the reducer half and `attach_picker::answer` are tested | `crates/micold-client/src/shell/composer.rs:147-157` |
| F3 | HIGH | test-strength | Mutant M9 survives: the attach chord no longer reaches `AttachRequested` and all of `features_composer`, `app_chord` and the bin tests stay green. `composer_key` yields `KeyIntent::Attach` (tested) but no test shows the shell opens the picker for it (US4 AC2, FR-010) | `crates/micold-client/src/shell/composer.rs:55-57` |
| F4 | HIGH | untested criterion | US3 AC4 / US4: tooltip names action and shortcut. Mutant M6 (`Ctrl+Shift+A` to `Ctrl+Shift+Q`) survives lib, layout_snapshot, composer_tab, features_composer. `ATTACH_TOOLTIP` is asserted nowhere; the keys.rs sheet row is tested, the tooltip is not. Also no test that pressing the rendered button emits `AttachRequested` | `crates/micold-client/src/ui/composer.rs:469-473`, `:526-530` |
| F5 | HIGH | test-strength | Mutant M8 survives: `DropCue.active(...)` replaced by `false` in the pane view and no test fails. `shows_drop_cue` (predicate) and the hover publish are tested, the wiring between them and the drawn pane is not (US2 AC1-2, SC-005); only the M5 screenshots speak for it | `crates/micold-client/src/ui/panes.rs:262-263` |
| F6 | MED | test-strength | `a_drop_on_a_structured_pane_beside_a_terminal_sends_no_terminal_input` asserts only that no terminal input was sent and `pending_drops` is empty; it passes if the drop is ignored entirely. Its doc comment says the composer reads the file, nothing asserts it (US1 AC4 composer half, FR-001) | `crates/micold-client/src/main_tests.rs:14260` |
| F7 | MED | test-strength | `a_picked_result_lands_only_on_the_requesting_session` names the target session in the `apply_drop` call itself, so it cannot show routing; `press_attach` is a one-line wrapper over `attach_requested` that adds nothing (LOW, bypassed naming) | `crates/micold-client/tests/features_composer.rs:1376`, `:1311` |
| F8 | MED | test-strength | Re-implemented expectation: expected text for `/d/my notes.txt` comes from `plan_insertion`, the function `apply_drop` itself uses, so a quoting regression passes here (the pre-existing 941 test uses the same pattern; real quoting is pinned only in `path_insert` tests, not named by this feature) | `crates/micold-client/tests/features_composer.rs:1156` |
| F9 | MED | test-strength | Mutant M7 (button never disabled) is caught only by the `the_layout_matches_the_committed_fixture` snapshot, incidentally. The composer_fits disabled test asserts `state.composers.can_attach`, not that the drawn button is inert (US3 AC5, FR-011) | `crates/micold-client/tests/gates/composer_fits.rs` (`the_add_attachment_button_is_disabled_...`) |
| F10 | MED | docs | `tasks.md` T040 and T041 unticked; quickstart Part B has no recorded results (steps 2-4: real app per platform, keyboard reach, UI responsive during picker, SC-003, SC-004). `research.md` R7 records Wayland, X11, macOS, Windows as "not run". Visual-pass evidence exists only for the showcase (`visual/`, `visual-pass-m4.md`) | `specs/1464-structured-drop-attach/tasks.md:95-96`, `quickstart.md`, `research.md` R7 |
| F11 | LOW | docs | `tasks.md` T029/T030 name `app::Message::ShortcutsToggled(bool)` and `State.shortcuts_open`; the code routes `/shortcuts` as `Changes(ShortcutsShown)` with the flag in `changes::State` (`840e66b09`). Tasks are stale against the final design | `tasks.md:~82`, `crates/micold-client/src/shell/composer.rs:370` |
| F12 | LOW | process | No test list or cycle log (accepted by design); consequently the ordering evidence is empty for every milestone and `wip(1464)`/"unverified"/"WIP, gate pending" commits sit in the history | git log |
| F13 | LOW | unbuilt | SC-004 (UI responsive within 100 ms while the picker is open) has no test; satisfied by construction (`Task::future` over `rfd::AsyncFileDialog`) and by the unrun manual step | `crates/micold-client/src/shell/attach_picker.rs:15` |

No secrets found in any test or fixture. No instruction-like text in tests.

Smell pass (other items of the catalogue): no assertion-free, empty, skipped or conditional-logic tests found. `for` loops in `an_empty_drop_leaves_...`, `the_attach_chord_...` and `...fits_the_composer_row...` always assert per iteration. Style matches the exemplars (inline `#[cfg(test)]`, `tests/` per concern, stock asserts with rule messages, shared `composer_in`/`drop_of` helpers). `drop_of` is a feature-local fixture, the `DroppedFiles` build is repeated in `main_tests.rs::composer_drop` (LOW duplicated setup). Suite properties: deterministic and fast (feature targets under 15 s each); the tests are tolerant of refactors apart from F1/F7 which pin reducer internals (`picker_open`, `attach_picked` count argument).

## Mutation results

Tool: none (`mutation: null` in the profile). Deliberate mutants, 9 sampled of the feature's high-risk behaviors: admission guard (FR-011), picker single-instance (FR-015), picker result route (FR-008/012), drop route guard in shell, attach chord route (FR-010), tooltip (FR-006), button disabled (FR-011), cue predicate and cue wrapping (FR-005). Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| M1 `features/composer.rs` `apply_drop`: `if !can_accept` to `if false && ...` | refuse drop, FR-011 | No | Caught by `a_drop_on_a_session_that_cannot_accept_input_changes_only_the_notice` |
| M2 `features/composer.rs` `attach_requested`: `can_attach` to `accepts_input` | one picker, FR-015 | No | Caught by `attach_requested_twice_opens_one_picker` |
| M3 `features/session.rs` `shows_drop_cue`: drop `accepts_input` | cue only if accepting, FR-005 | No | Caught by `only_a_structured_pane_that_accepts_input_wears_the_cue_...` |
| M4 `shell/composer.rs` `AttachPicked`: read regardless of `attach_picked` | late result refused, FR-012 | Yes | Real gap, F2 |
| M5 `shell/composer.rs` `Dropped`: `can_accept = true` | shell asks the session, FR-011 | No | Caught by `a_composer_drop_on_an_ended_session_...` |
| M6 `ui/composer.rs` tooltip chord text changed | tooltip, US3 AC4 | Yes | Real gap, F4 |
| M7 `ui/composer.rs` button press always set | disabled button, FR-011 | No | Caught only incidentally by the layout snapshot, F9 |
| M8 `ui/panes.rs` `.active(false)` | cue shown, US2 | Yes | Real gap, F5 |
| M9 `shell/composer.rs` chord routing disabled | attach chord, FR-010 | Yes | Real gap, F3 |

An earlier M7 variant did not compile and was discarded and redone. Coverage: not available (`coverage: null`).

## Traceability

Real entry point here means the shell `update`/view or a drop event through the pane view; `Partial` is reducer or predicate only.

| Criterion | Tests | End to end |
|---|---|---|
| US1 AC1 (transcript drop, chip, sent) | `pane_drop_target::a_file_dropped_on_the_pane_view_...`, `a_pane_showing_a_structured_primary_...`, `features_composer` drop tests | Partial: no test joins the pane event to the composer draft |
| US1 AC2 (non-image to caret) | `dropped_paths_are_inserted_at_the_caret_...`, F8 | Partial |
| US1 AC3 (order) | `several_pictures_dropped_at_once_attach_in_drop_order` | Partial |
| US1 AC4 (only that composer, no terminal) | `a_drop_on_one_session_leaves_...`, `main_tests::..._sends_no_terminal_input` (F6) | Partial |
| US1 AC5 (rejects named) | `twenty_two_pictures_...`, `a_too_large_picture_...` | Partial |
| US2 AC1-2 (cue on, then off) | `only_a_structured_pane_...`, `the_pane_view_reports_a_hover_...`, `drop_cue` unit tests | Partial (F5) |
| US2 AC3 (none when cannot accept) | `only_a_structured_pane_...` | Partial |
| US3 AC1 (button opens picker, no freeze) | `attach_requested_twice_...` | Partial (F2, F13); picker itself untestable headless |
| US3 AC2 (same admission as a drop) | `the_same_ordered_files_...` | No: F1 |
| US3 AC3 (cancel is silent) | `a_closed_picker_changes_nothing_...`, `attach_picker::answer` | Partial |
| US3 AC4 (tooltip) | none | No: F4 |
| US3 AC5 (disabled) | composer_fits disabled test, `attach_requested_on_a_session_that_cannot_accept_input_...` | Partial (F9) |
| US4 AC1 (Tab reach, focus) | `composer_tab` (Tab stop added) | Partial: Enter/Space activation not exercised |
| US4 AC2 (chord opens picker) | `the_attach_chord_opens_the_picker_...`, `app_chord::the_attach_chord_is_left_...` | Partial (F3) |
| US4 AC3 (sheet lists chord) | `keys.rs` sheet-model test, `sending_slash_shortcuts_...`, core/daemon `/shortcuts` tests, `app.rs` toggle test | Partial: render of the sheet via `/shortcuts` through the shell not driven |

Untested criteria: US3 AC4 (tooltip). Tests tracing to nothing: none (`icons` pin, `drop_cue` size test trace to FR-006/FR-005). `traces`-style claims in `quickstart.md` Part A ("drop/picker parity (FR-013)") overstate coverage (F1).

## What was not audited

- Test-first ordering: no list or log; milestone-granular commits make it unprovable. Not "test-after", unproven.
- Mutation was a 9-mutant hand sample on 5 files, not a tool run; no score. No coverage tool.
- Real platform behavior (Wayland, X11, macOS, Windows drop delivery, the native `rfd` picker, SC-003, SC-004) and the real GUI visual pass: not run; no Xvfb pass here. Screenshots in `visual/` were not re-judged.
- The sandbox acceptance suite (`sandbox_real_*`) and `mise run gate` (fmt, clippy, scripts tests): not run; only `cargo test --workspace`.
- Daemon-side `/shortcuts` refusal tests and `micold-material` showcase/gallery gates were read, not mutated.
- Windows/macOS `cfg` arms (`ATTACH_TOOLTIP`, `attach_chord` on macOS) compiled only on Linux.
