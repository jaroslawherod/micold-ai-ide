# Tasks: Structured view composer layout and large editor

**Input**: `specs/1703-composer-layout-large-editor/` (spec.md, plan.md, research.md, data-model.md, contracts/composer-layout.md, quickstart.md)

**Tests**: mandatory (Constitution I). Each story lists failing tests before the code they cover. `ui/` glue that only a pixel check can prove uses the quickstart §B visual pass.

Format: `- [ ] T### [P?] [US?] description with file path`. `[P]` = disjoint files, no dependency on an open task.

## Phase 1: Setup

- [x] T001 Add `Icon::OpenInFull` (codepoint, `Icon` list entry, mapping; `Icon::Settings` already exists) in `crates/micold-material/src/icons.rs`, with the icon-list test that file's tests keep (R6)

## Phase 2: Foundational

None. The persisted flag belongs to US3; the editor needs no shared plumbing.

## Phase 3: User Story 1 - Large editor (P1) 🎯 MVP

**Goal**: Ctrl/Cmd+Shift+M (or an icon button) opens a modal editor on the draft; Done writes back, Cancel leaves it.
**Independent test**: type a draft, press the chord, edit, Done: the composer holds the edited text; Cancel: unchanged.

### Tests first (must fail before T008+)

- [x] T002 [P] [US1] Reducer and key-mapping (Ctrl/Cmd+Enter -> Done, Esc -> Cancel) tests for `LargeEditor` in `crates/micold-client/tests/features_large_editor.rs`: `Requested(session)` opens with the session's draft text and cursor at end (FR-002); empty draft opens empty; second `Requested` while open is ignored; `Done` writes text to the bound `session` only, also when it ended (FR-019); `Cancelled` leaves the draft unchanged; attachments never read or written (FR-003)
- [x] T003 [P] [US1] Chord tests in `crates/micold-client/tests/features_composer.rs`: `composer_key` yields `LargeEditorRequested` for Ctrl+Shift+M on Linux/Windows and Cmd+Shift+M on macOS only while the text area is focused, and not for plain, Ctrl-only or other-modifier forms (FR-001, VI)
- [x] T004 [P] [US1] Unit test of the pure open-editor-button visibility predicate (lines > 1 or focused or hovered) in `crates/micold-client/src/features/composer.rs` tests module (FR-004)
- [x] T005 [P] [US1] Test that the shortcut sheet lists the chord next to Add attachment in `crates/micold-client/src/features/changes/keys.rs` tests (FR-005, SC-006)
- [x] T006 [P] [US1] Registration test for the large-editor dialog in `crates/micold-client/tests/overlay_registration.rs`

### Implementation

- [x] T007 [US1] Create render-free `crates/micold-client/src/features/large_editor.rs`: `Option<LargeEditor { session: SessionId, content }>` with `Requested`/`Edited`/`Done`/`Cancelled` reducer per data-model.md; add the module and `Message::LargeEditor` routing in `crates/micold-client/src/main.rs`
- [x] T008 [US1] In `crates/micold-client/src/features/composer.rs`: `LargeEditorRequested` intent from `composer_key` (before the non-plain guard, like the attachment chord), the visibility predicate, and the `Composer(LargeEditorRequested(session))` message
- [x] T009 [P] [US1] Dialog body `crates/micold-client/src/ui/large_editor.rs` (`Modal`, `TextArea` large `max_height`, Done and Cancel `Button`s; Ctrl/Cmd+Enter = Done, Esc = Cancel; the key-to-message mapping is a pure fn tested in T002's file) and register it in `crates/micold-client/src/overlay/registry.rs`
- [x] T010 [US1] Open-editor `IconButton` (`Icon::OpenInFull`, label "Open large editor", platform tooltip like Add attachment) at the text area's top-right in `crates/micold-client/src/ui/composer.rs`, shown by the predicate
- [x] T011 [P] [US1] Shortcut-sheet entry in `crates/micold-client/src/features/changes/keys.rs` (next to Add attachment, ~l.480)
- [x] T012 [P] [US1] Showcase entry for the large-editor dialog body in `crates/micold-client/src/showcase/sections/surfaces.rs` (edit; VIII)
- [x] T013 [P] [US1] User guide: document the chord and editor in `docs/user-guide/structured-input.md` and the shortcut table in `docs/user-guide/terminal-panes.md` (FR-005, VII)

**Checkpoint**: US1 works alone.

## Phase 4: User Story 2 - Activity indicator beside the input (P1)

**Goal**: the activity state is an icon-only 24 dp indicator in the input row, state words as tooltip and accessible label; the status text row loses the indicator.
**Independent test**: hover the indicator in Ready/Busy/Waiting: tooltip names the state, no visible status text.

- [ ] T014 [P] [US2] View-structure test in `crates/micold-client/tests/composer_tab.rs` (style of the existing tests): input row contains the indicator between text area and attach; no visible state word in the status row; tooltip/accessible label per state (FR-006, FR-007, SC-003)
- [ ] T015 [US2] In `crates/micold-client/src/ui/composer.rs` move the activity indicator into the input row as an icon-only 24 dp `BusyIndicator` with its label as tooltip and accessible label; `status_row` keeps Stop session and the refusal note (FR-006, FR-007)
- [ ] T016 [P] [US2] User guide: describe the icon indicator in `docs/user-guide/structured-input.md`

**Note**: US2 scenario 1 (no permission selector in the input row) needs the options row of US3 to re-home it, so US2 and US3 ship in one milestone (M2).

## Phase 5: User Story 3 - Session options row behind a settings icon (P1)

**Goal**: model/effort/permission selectors live in a row toggled by a settings icon; state persists per window; refusals auto-expand it.
**Independent test**: fresh settings show one control row; click the icon, row appears; restart keeps the state.

### Tests first

- [ ] T017 [P] [US3] Core test in `crates/micold-core/src/settings.rs` tests: `composer_options_open` defaults false, round-trips, an old file without the key and an unreadable file read false (FR-011)
- [ ] T018 [P] [US3] Reducer tests in `crates/micold-client/tests/features_composer.rs`: `OptionsToggled` flips `options_open`; a model, effort or permission refusal sets it true while an attachment-drop notice does not; Stop session and the permission refusal note stay in `status_row` when collapsed (FR-009, FR-012, FR-013)
- [ ] T019 [P] [US3] Persistence tests: `persist_composer_options_open` and the form-save exclusion in `crates/micold-client/src/shell/persist.rs`; boot reads the flag in `crates/micold-client/src/shell/startup.rs`; `crates/micold-client/src/shell/composer.rs` test with a mock `SettingsStore` that toggle AND auto-expand both persist (FR-011, FR-012)
- [ ] T020 [P] [US3] View-structure tests in `crates/micold-client/tests/composer_tab.rs`: no permission selector in the input row, options row holds Model, Effort, Permission in that order when open, settings `IconButton` 40 dp labelled "Session options" and `.selected()` when open, Bypass marker on it (FR-008, FR-009, FR-010, FR-014); the icon is a focusable `IconButton` whose Space/Enter fires `OptionsToggled` (keyboard path, FR-009)

### Implementation

- [ ] T021 [US3] Add `composer_options_open: bool` (`#[serde(default)]`, default false, serde mirror; `settings_version` unchanged) in `crates/micold-core/src/settings.rs`
- [ ] T022 [US3] `persist_composer_options_open` and the form-save exclusion in `crates/micold-client/src/shell/persist.rs`; read it into core state at boot in `crates/micold-client/src/shell/startup.rs`
- [ ] T023 [US3] `options_open` flag, `OptionsToggled`, auto-expand at the model/effort/permission refusal notice sites in `crates/micold-client/src/features/composer.rs`; call `persist_composer_options_open` on every change in `crates/micold-client/src/shell/composer.rs`
- [ ] T024 [US3] Layout in `crates/micold-client/src/ui/composer.rs`: settings `IconButton` at the input row start, options row (Model, Effort, Permission) when open, Bypass warning marker; `crates/micold-client/src/ui/mode_control.rs` `seams` drops the permission control from the input row (FR-008)
- [ ] T025 [P] [US3] User guide: options row, settings icon, persistence, auto-expand, Bypass marker in `docs/user-guide/structured-input.md`

## Phase 6: User Story 4 - One consistent selector style (P2)

**Goal**: model, effort, permission share one trigger style and state treatments; model list opens the floating list with a filter first row.
**Independent test**: hover/focus/press/open/disable each selector: identical treatments (quickstart §B 4).

- [ ] T026 [P] [US4] Geometry gate test `crates/micold-client/src/ui/material/menu_select_anatomy.rs` (new, modelled on `select_anatomy.rs`): 32 dp height, 8 dp radius, XS/SM padding, 1 dp outline-variant, min 140 dp, five states, disabled tooltip (FR-015, FR-016, SC-004)
- [ ] T027 [P] [US4] Tests that `MenuSelectRow::filter` filters the list by typed text, and that model, effort and permission choices, change behaviour and disabled rules are unchanged (FR-017, FR-018) in `crates/micold-client/tests/features_composer.rs`
- [ ] T028 [US4] Extend `crates/micold-client/src/ui/material/menu_select.rs`: shared trigger style, disabled state with tooltip, `MenuSelectRow::filter` first row (builder API kept)
- [ ] T029 [US4] Migrate model (`Typeahead`/`Button::text`) and permission (`Select`) to `MenuSelect` in `crates/micold-client/src/ui/composer.rs` (`model_control`, `effort_control`) and `crates/micold-client/src/ui/mode_control.rs`; `Select` stays for other callers
- [ ] T030 [P] [US4] Showcase `MenuSelect` disabled and filter variants in `crates/micold-client/src/showcase/sections/controls.rs` (and `catalogue.rs` if an entry is needed)
- [ ] T031 [P] [US4] User guide: selector look and model filter in `docs/user-guide/structured-input.md`

## Polish (close unit, no code)

- [ ] T032 Run quickstart §B (`visual-pass` skill) and record results in `specs/1703-composer-layout-large-editor/quickstart.md`; tick tasks

## Dependencies & Execution Order

- T001 and T008 before T010; T009 before T012. US1 independent otherwise.
- US2 and US3 are one milestone (M2): US2 scenario 1 removes the permission selector from the input row, and only US3's options row re-homes it. US2 touches `ui/composer.rs` input row: after US1 (M2 depends on M1).
- US3 after US2 within M2 (same file).
- US4 after US3 (migrates the selectors that US3 places in the options row).
- Within a story: tests first (red), then implementation.

## Parallel Examples

- US1: T002-T006 together; T009, T011, T012, T013 after T007/T008.
- US3: T017-T020 together.
- US4: T026, T027 together; T030, T031 after T028.

## Implementation Strategy

MVP = US1 (M1). Each later story ships alone behind main being green; no half-wired UI (see US2 note).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Large editor 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: Ctrl/Cmd+Shift+M or the open-editor icon opens a modal large editor on the draft; Done writes back, Cancel discards, and the chord is in the shortcut sheet and user guide.
- **Satisfies**: US1 acceptance scenarios 1–6; FR-001, FR-002, FR-003, FR-004, FR-005, FR-019; SC-001, SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test main -- features_large_editor features_composer overlay_registration`; quickstart §B step 6 (covers SC-001)
- **Depends on**: —
- **Tier**: full

### M2 — Activity indicator and session options row

- **Tasks**: T014–T025
- **Deliverable**: The input row shows an icon-only activity indicator (tooltip names the state, no status text); a settings icon toggles a Model/Effort/Permission row whose state survives restart and which a refusal auto-expands; the permission selector is gone from the input row.
- **Satisfies**: US2 acceptance scenarios 1–3; US3 acceptance scenarios 1–7; FR-006 to FR-014; SC-002, SC-003, SC-005
- **Verify**: `mise run test-core` and `scripts/build-lock.sh cargo test -p micold-client features_composer composer_tab persist startup`; quickstart §B steps 1, 2, 3, 5
- **Depends on**: M1
- **Tier**: full

### M3 — One selector style

- **Tasks**: T026–T031
- **Deliverable**: Model, effort and permission selectors share one trigger and state treatments; the model list is a floating list with a filter row.
- **Satisfies**: US4 acceptance scenarios 1–4; FR-015, FR-016, FR-017, FR-018; SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib menu_select_anatomy`; `scripts/build-lock.sh cargo test -p micold-client --test features_composer`; quickstart §B steps 4, 7
- **Depends on**: M2
- **Tier**: full

(T032 is the close unit's: quickstart results, no code.)
