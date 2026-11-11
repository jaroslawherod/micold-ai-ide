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
- [x] T005 [P] [US1] Test in `src/ui/composer.rs` tests: Model, Effort and Permission controls are all `MenuSelect` with the same trigger and keep `MIN_TRIGGER_WIDTH` and `MODEL_PICKER_WIDTH` (FR-003); `ModelPicked`, effort and permission picks and `*Closed`/`*Moved` messages unchanged (FR-010, SC-004).

### Implementation

- [x] T006 [US1] Extract the `Select` trigger (container shape, fill or outline, label, value, arrow, state layers, ripple) into a shared piece in `src/ui/material/` (new `select_trigger.rs`, exported through `ui/material/mod.rs`); tokens and `Roles` only, no literals; takes `open` as an input.
- [x] T007 [US1] `src/ui/material/select.rs`: `Select` draws its trigger through the shared piece; behaviour unchanged; its existing tests stay green.
- [x] T008 [US1] `src/ui/material/menu_select.rs`: `MenuSelect` trigger uses the shared piece in place of the text `Button`; builder API kept, ends in `.into()`; filter, chips and keyboard highlight untouched. Makes T002 to T005 green. Also touches `crates/micold-material/src/button.rs` (`Button::bare`: the press, keyboard and ripple without a look of its own), and, for the disabled look of T003, `crates/micold-material/src/form_field.rs` and `filled_field.rs` (`FormField::disabled`); T005's test lives in `crates/micold-client/tests/gates/composer_fits.rs`.
- [x] T009 [P] [US1] `src/showcase/`: the `MenuSelect` and options-row entries show the new trigger beside `Select`.
- [x] T010 [P] [US1] Docs: `docs/user-guide/structured-input.md` (session options row look) and the component catalogue docs for the shared trigger.

## Phase 4: User Story 2 - Material 3 menu for the select options (P2)

**Goal**: the opened menu of every structured select is the Material 3 `picker` menu, pinned by a parity test.

**Independent test**: `scripts/build-lock.sh cargo test -p micold-client picker_parity`.

- [ ] T011 [US2] Characterization test (a green first run is acceptable, then T012 is skipped; note it in the cycle log) in `src/ui/material/picker_parity.rs`: `MenuSelect` and `Select` menus use the same surface, item layout, selected emphasis and item state layers (FR-004, US2 scenarios 1-2).
- [ ] T012 [US2] Only if T011 fails: align `src/ui/material/menu_select.rs` with `picker::animated_menu` / `row_surface`, no new menu widget.

## Phase 5: User Story 3 - Expand and collapse animation for the settings (P2)

**Goal**: the options row grows from the trigger and collapses with `MEDIUM_2` and the standard easing; instant under Reduce motion.

**Independent test**: `scripts/build-lock.sh cargo test -p micold-client composer`.

### Tests (write first, see them fail)

- [ ] T013 [US3] Red tests in `src/ui/composer.rs` tests (`drawer_flags_tests.rs` style): expand duration is `tokens::motion::duration::MEDIUM_2`, and `Duration::ZERO` when `settings.reduce_motion` (FR-005, FR-006, FR-013); stepping the track settles at that duration (within one frame) with no single-step full-height jump (SC-002).
- [ ] T014 [US3] In `src/ui/composer.rs` tests: red test: toggling mid-animation reverses from the current height without a jump (FR-007), and rapid repeated toggles leave no stuck half-open state.
- [ ] T015 [US3] In `src/ui/composer.rs` tests: red test: two sessions' composers keep separate expand tracks (FR-008).
- [ ] T016 [US3] In `src/ui/composer.rs` tests: red test: a collapsed row's controls are not reachable by keyboard or pointer (research R3, contract §2; no FR, spec edge case "Rapid repeated toggling" and FR-005 imply it).
- [ ] T017 [US3] Red test in the tests of the module that owns the `options_open` toggle and `*Closed` messages (`src/features/composer.rs`): collapsing the row closes any open model, effort or permission picker (research R3, contract §2: `Expand::overlay` has no hidden guard; no FR of its own).
- [ ] T018 [US3] In `src/ui/composer.rs` tests: red geometry test at a narrow width: the end height equals the row's wrapped height, and in a window shorter than the row it ends at the reachable (clipped) height (FR-011, R5; clip is the behaviour, contract §2).

### Implementation

- [ ] T019 [US3] `src/ui/composer.rs`: build `options_row` always and wrap it in `micold_material::animation::expand(row, options_open, over)`; `over` = `MEDIUM_2`, `Duration::ZERO` when `state.settings.reduce_motion`, read in `composer::view`; no literal duration or easing (SC-003).
- [ ] T020 [US3] Toggling the row closed also emits the existing model, effort and permission `*Closed` messages for any open picker (in `src/features/composer.rs` / `src/shell/composer.rs` where `options_open` is toggled). Makes T017 green.
- [ ] T021 [P] [US3] Docs: `docs/user-guide/structured-input.md` (expand and collapse of the options row) and `docs/user-guide/settings.md` (Reduce motion also applies to it).

## Phase 6: Polish and cross-cutting (close unit; changes no code)

- [ ] T022 Run quickstart Part A and the Part B visual pass (skill `visual-pass`); record the result in `specs/1792-structured-select-m3/evidence.md` (FR-012, SC-001, SC-002).
- [ ] T023 Tick tasks; close or narrow #1765 (spec Assumptions) with a link to the merged work; confirm no literal colour, shape, duration or easing in `select_trigger.rs` and `composer.rs` via `tests/motion_tokens.rs` and `tests/ui_glyph_literals.rs` (SC-003); run `mise run duplication`; confirm `tests/material_boundary.rs` and `tests/one_overlay_implementation.rs` pass.

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

T022–T023 change no code (quickstart results, task ticks, duplication check): the close unit does them in the close PR.
