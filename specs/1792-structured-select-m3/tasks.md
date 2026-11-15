# Tasks: Structured view Material 3 select and expand animation

**Input**: `specs/1792-structured-select-m3/` (spec.md, plan.md, research.md, contracts/ui-contract.md, quickstart.md)

**Tests**: mandatory (Constitution I). Each story lists its red tests before the code they cover.

**Documentation**: each story that changes behaviour carries its user-guide task (Constitution VII).

Paths are under `crates/micold-client/` unless they start with `docs/` or `specs/`.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [x] T001 Read `src/ui/material/select.rs` and `menu_select.rs`; confirm in `specs/1792-structured-select-m3/research.md` R1 how the shared trigger takes `open` as an input (resolves R1 "to confirm") and list the draw pieces to extract.

## Phase 2: Foundational

None beyond T001: the trigger extraction is the P1 story's own work.

## Phase 3: User Story 1 - Material 3 select for model, effort and permissions (P1) 🎯 MVP

**Goal**: the three structured selects show `Select`'s Material 3 trigger in every state.

**Independent test**: `scripts/build-lock.sh cargo test -p micold-client menu_select`.

### Tests (write first, see them fail)

- [x] T002 [US1] Red geometry test in `src/ui/material/menu_select_anatomy.rs`: `MenuSelect` trigger geometry, shape, fill/outline, label, value and arrow equal `Select`'s per state rest, hover, focus, pressed, open (FR-001, FR-002, FR-009).
- [x] T003 [US1] Red test in `src/ui/material/menu_select_anatomy.rs`: a disabled trigger shows the disabled look and does not react to hover, focus or press (FR-002, US1 scenario 3).
- [x] T004 [US1] Red tests in `src/ui/material/menu_select_anatomy.rs`: long value ellipsized with the arrow visible; value absent from the options renders its text; empty option list opens no menu (FR-011).
- [x] T005 [P] [US1] Test in `crates/micold-client/tests/gates/composer_fits.rs` (the composer view is built there; `src/ui/composer.rs` is unchanged): Model, Effort and Permission controls are all `MenuSelect` with the same trigger and keep `MIN_TRIGGER_WIDTH` and `MODEL_PICKER_WIDTH` (FR-003); `ModelPicked`, effort and permission picks and `*Closed`/`*Moved` messages unchanged (FR-010, SC-004).

### Implementation

- [x] T006 [US1] Extract the `Select` trigger (container shape, fill or outline, label, value, arrow, state layers, ripple) into a shared piece in `src/ui/material/` (new `select_trigger.rs`, exported through `ui/material/mod.rs`); tokens and `Roles` only, no literals; takes `open` as an input.
- [x] T007 [US1] `src/ui/material/select.rs`: `Select` draws its trigger through the shared piece; behaviour unchanged; its existing tests stay green.
- [x] T008 [US1] `src/ui/material/menu_select.rs`: `MenuSelect` trigger uses the shared piece in place of the text `Button`; builder API kept, ends in `.into()`; filter, chips and keyboard highlight untouched. Makes T002 to T005 green. Also touches `crates/micold-material/src/button.rs` (`Button::bare`: the press, keyboard and ripple without a look of its own), and, for the disabled look of T003, `crates/micold-material/src/form_field.rs` and `filled_field.rs` (`FormField::disabled`); T005's test lives in `crates/micold-client/tests/gates/composer_fits.rs`.
- [x] T009 [P] [US1] `src/showcase/`: the `MenuSelect` and options-row entries show the new trigger beside `Select`.
- [x] T010 [P] [US1] Docs: `docs/user-guide/structured-input.md` (session options row look) and the component catalogue docs (`docs/development/component-library.md`) for the shared trigger.

## Phase 4: User Story 2 - Material 3 menu for the select options (P2)

**Goal**: the opened menu of every structured select is the Material 3 `picker` menu, pinned by a parity test.

**Independent test**: `scripts/build-lock.sh cargo test -p micold-client picker_parity`.

- [x] T011 [US2] Characterization test (a green first run is acceptable, then T012 is skipped; note it in the cycle log) in `src/ui/material/picker_parity.rs`: `MenuSelect` and `Select` menus use the same surface, item layout, selected emphasis and item state layers (FR-004, US2 scenarios 1-2).
- [x] T012 [US2] Only if T011 fails: align `src/ui/material/menu_select.rs` with `picker::animated_menu` / `row_surface`, no new menu widget. Skipped: T011 was green on its first run (no drift).

## Phase 5: User Story 3 - Expand and collapse animation for the settings (P2)

**Goal**: the options row grows from the trigger and collapses with `MEDIUM_2` and the standard easing; instant under Reduce motion.

**Independent test**: `scripts/build-lock.sh cargo test -p micold-client composer`.

### Tests (write first, see them fail)

- [X] T013 [US3] Red tests in `src/ui/composer.rs` tests (`drawer_flags_tests.rs` style): expand duration is `tokens::motion::duration::MEDIUM_2`, and `Duration::ZERO` when `settings.reduce_motion` (FR-005, FR-006, FR-013); stepping the track settles at that duration (within one frame) with no single-step full-height jump (SC-002).
- [X] T014 [US3] In `src/ui/composer.rs` tests: red test: toggling mid-animation reverses from the current height without a jump (FR-007), and rapid repeated toggles leave no stuck half-open state.
- [X] T015 [US3] In `src/ui/composer.rs` tests: red test: two sessions' composers keep separate expand tracks (FR-008).
- [X] T016 [US3] In `src/ui/composer.rs` tests: red test: a collapsed row's controls are not reachable by keyboard or pointer (research R3, contract §2; no FR, spec edge case "Rapid repeated toggling" and FR-005 imply it).
- [X] T017 [US3] Red test in the tests of the module that owns the `options_open` toggle and `*Closed` messages (`src/features/composer.rs`): collapsing the row closes any open model, effort or permission picker (research R3, contract §2: `Expand::overlay` has no hidden guard; no FR of its own).
- [X] T018 [US3] In `src/ui/composer.rs` tests: red geometry test at a narrow width: the end height equals the row's wrapped height, and in a window shorter than the row it ends at the reachable (clipped) height (FR-011, R5; clip is the behaviour, contract §2).

### Implementation

- [X] T019 [US3] `src/ui/composer.rs`: build `options_row` always and wrap it in `micold_material::animation::expand(row, options_open, over)`; `over` = `MEDIUM_2`, `Duration::ZERO` when `state.settings.reduce_motion`, read in `composer::view`; no literal duration or easing (SC-003).
- [X] T020 [US3] Toggling the row closed also emits the existing model, effort and permission `*Closed` messages for any open picker (in `src/features/composer.rs` / `src/shell/composer.rs` where `options_open` is toggled). Makes T017 green.
- [X] T021 [P] [US3] Docs: `docs/user-guide/structured-input.md` (expand and collapse of the options row) and `docs/user-guide/settings.md` (Reduce motion also applies to it).

## Phase 6: Polish and cross-cutting (close unit; changes no code)

- [x] T022a Run quickstart Part A and record it in `specs/1792-structured-select-m3/evidence.md` (FR-012, SC-001, SC-002); Part B is recorded per milestone: steps 2, 3, 5, 6 in `visual-pass-M3.md`; steps 1 (rest, hover, disabled, open) and 4 in `visual-pass-M1.md`, whose focus and pressed rows were not checked (T022b).
- [ ] T022b Part B step 1 focus and pressed, and step 7 (narrow window, light and dark), as a visual run. NOT RUN: the forked `visual-pass` skill got no arguments from the close unit twice, so it ran nothing. Both states are pinned by tests with product mutants (T025, `evidence.md`); step 7's wrap by `at_a_narrow_width_the_row_ends_at_its_wrapped_height`.
- [x] T023a Ticked tasks; confirmed `tests/motion_tokens.rs` and `tests/ui_glyph_literals.rs` (SC-003, no literal colour, shape, duration or easing in `select_trigger.rs` and `composer.rs`), `tests/material_boundary.rs` and `tests/one_overlay_implementation.rs` pass (see `evidence.md`).
- [ ] T023b Close or narrow #1765 (spec Assumptions) with a link to the merged work (#1842, #1851, #1902), and run `mise run duplication`. NOT DONE: neither is possible from the cloud thread (no `mise`, no authenticated `gh`); listed under the ledger's Follow-ups not done.

## Dependencies

T001 → T002..T005 (red) → T006 → T007, T008 → T009, T010. M2 (T011, T012) after T008. M3 (T013..T018 red) → T019 → T020 → T021; independent of M1 code, but ordered after M2 by priority. T022, T023 last.

## Parallel examples

- US1: T005 (composer tests) is disjoint from T002-T004 (menu_select_anatomy.rs); T009 and T010 are disjoint from each other.
- US3: T021 is docs only; T013-T018 share one test module, so run them in order.

## Implementation Strategy

MVP first: M1 (US1) ships the shared trigger on all three controls. M2 pins the menu. M3 adds the animation. Each is one PR; `main` stays green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Material 3 trigger for the structured selects 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: the model, effort and permission selects in the structured view, and the showcase `MenuSelect`, draw the same Material 3 trigger as `Select` in every state.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-009, FR-010, FR-011 (trigger edge cases); SC-003, SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client menu_select`; showcase `MenuSelect` beside `Select`.
- **Depends on**: —
- **Tier**: full

### M2 — Material 3 option menu pinned by parity

- **Tasks**: T011–T012
- **Deliverable**: (a test-only PR if T011 is green) a test proves the structured select's menu equals `Select`'s Material 3 menu; any drift fixed.
- **Satisfies**: US2 acceptance scenarios 1–2; FR-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client picker_parity`
- **Depends on**: M1
- **Tier**: light

### M3 — Expand and collapse animation for the options row

- **Tasks**: T013–T021
- **Deliverable**: toggling the settings icon grows or collapses the options row over the standard motion duration, reversing mid-flight, per session, instant under Reduce motion; collapsing closes open pickers.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-005, FR-006, FR-007, FR-008, FR-011 (small window), FR-013; SC-002
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client composer`; quickstart Part B steps 2, 3, 5, 6.
- **Depends on**: M1
- **Tier**: full

### Close unit (not a milestone)

T022a–T023b change no code (quickstart results, task ticks, duplication check; T022b and T023b stay open): the close unit does them in the close PR.

## Phase 7: TDD remediation (from `tdd/verification.md`, verdict FAIL)

The feature is not done until T024 to T026 (the HIGH findings) are cleared. Paths are under `crates/micold-client/`.

- [x] T024 [US3] Finding 1: pin `around_options` gaps. In `src/ui/composer_expand_tests.rs` add a pose with parts above and below the row (a notice plus a draft image or chip): collapsed height equals the height with no row, and the open row adds both gaps. Prove with the survivors: `ui/composer.rs:300` `between` to `false` and `:302` `gap(under)` to `gap(false)` must each turn it red, then restore. Run `scripts/build-lock.sh cargo test -p micold-client --lib -- expand_tests`.
- [x] T025 [US1] (done: pressed and ring tests plus product mutants for each, deviation in `spec.md` FR-002) Finding 3: add tests in `src/ui/material/menu_select_anatomy.rs` for the `MenuSelect` trigger's keyboard-focus and pressed layers against `Select`'s (FR-002, FR-009), or, if `MenuSelect` is meant to show the button's ring, record that deviation in `spec.md` FR-002 and test the ring. Run `scripts/build-lock.sh cargo test -p micold-client --lib -- menu_select_anatomy`.
- [x] T026 [US1] Finding 2: no red can be recovered for M1. Re-prove each M1 test by deliberate mutant (trigger geometry, hover layer, disabled guard, long-value ellipsis, empty-list guard) and record the results in `tdd/cycle-log.md` under an M1 heading, noting it is after the fact. Run `scripts/build-lock.sh cargo test -p micold-client --lib -- menu_select_anatomy`.
- [ ] T027 [US2] (DECLINED, as at the M2 review: the parity test is geometry only by decision; `menu_panel` and `row_surface` are shared functions both controls call; ledger *Declined review findings*) Finding 6: extend `src/ui/material/picker_parity.rs` (`the_structured_select_menu_has_the_selects_panel_and_rows`) with a paint-level check of the selected row emphasis and an item state layer (US2 scenario 2, FR-004). Run `scripts/build-lock.sh cargo test -p micold-client --lib -- picker_parity`.
- [x] T028 [US1] Finding 7: in `tests/gates/composer_fits.rs:1103` assert the model, effort and permission triggers by name (the pose's structured Claude Code session offers effort, so all three are named, in order) instead of `chevrons >= 3`. Run `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot -- the_model_and_permission_selectors`.
- [x] T029 [US3] Findings 4 and 5: mutate the `reduce_motion` branch of `options_motion` (`ui/composer.rs:317`) and `palette.close_pickers()` / `close_mode_menus()` (`shell/composer.rs`) once each, confirm a test fails, restore, and update `tdd/cycle-log.md` (also fix the red command naming the removed `a_picker_opened_by_command_opens_the_options_row`). Run `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- shell::composer::tests`.
- [x] T030 [US3] Finding 8: split `a_toggle_mid_way_reverses_from_the_current_height` (`src/ui/composer_expand_tests.rs:130`) into reversal and rapid-toggle tests, and name each entry point of `a_list_opened_with_the_row_collapsed_opens_the_row` (`src/shell/composer.rs:634`) in its own test. Run both targeted commands above.
- [x] T031 Findings 9 and 10 (LOW): name the magic values in `composer_expand_tests.rs` and `menu_select_anatomy.rs`; correct `drift-M3.md` on the containment paths.
