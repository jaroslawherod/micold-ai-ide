# Tasks: Structured View — Drop Files and "Add attachment" Button

**Input**: `specs/1464-structured-drop-attach/` (spec, plan, research, data-model, contracts/admission.md, quickstart)

**Tests**: mandatory (Constitution I): in every phase the test tasks come first and must fail before the code tasks.

**Documentation**: each user story's user-guide task ships in its own milestone (Constitution VII).

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [x] T001 Record the baseline: run `cargo test -p micold-client --test features_composer --test pane_drop_target --test features_session` green on this branch and note in `specs/1464-structured-drop-attach/research.md` (R7) the current drop route (`SplitView` -> `shell/drops.rs::read_for_composer` -> `composer::Msg::Dropped` -> `State::apply_drop`)

## Phase 2: Foundational

The admission guard every route needs (D1, contracts/admission.md). Part of M1 with US1.

- [x] T002 [P] Failing reducer tests in `crates/micold-client/tests/features_composer.rs`: `Dropped` for a session that cannot accept input changes nothing but `notice` ("<n> file(s) not attached: the session is not accepting input"); an empty plan (no images, no paths) leaves draft and `notice` untouched (FR-011, FR-016)
- [x] T003 `State::apply_drop(session, files, can_accept)` guard in `crates/micold-client/src/features/composer.rs` (D1, data-model)
- [x] T004 Compute `can_accept` from `app.core` (session half of `can_send`, no `connected`) where `Msg::Dropped` is routed in `crates/micold-client/src/shell/composer.rs`, with a shell-level test (in `crates/micold-client/src/main_tests.rs`: the shell is the binary's, out of reach of `tests/`) that a drop on an ended session is refused with a notice

## Phase 3: User Story 1 - Drop files anywhere on the structured view (P1) 🎯 MVP

**Goal**: files dropped on a structured view reach only that session's composer, with spec 941 behaviour kept and refusals told.

**Independent Test**: drop a PNG and a text file on the transcript; one chip and a quoted path appear; nothing reaches a terminal.

- [x] T005 [P] [US1] Characterisation tests (spec 941 behaviour, expected green on first run; failures are defects to fix in T008) in `crates/micold-client/tests/features_composer.rs`: several files dropped at once keep drop order; mixed images and other files give chips plus quoted paths; a rejected image is reported with file name and reason (FR-002, FR-003, SC-006)
- [x] T006 [P] [US1] Characterisation and routing tests (green or red on first run, as T005) in `crates/micold-client/tests/features_composer.rs` and `src/main_tests.rs`: a drop on a structured pane's transcript or composer reaches only that session's composer; with a terminal pane beside it no terminal receives input; with several structured sessions only the receiving one changes (FR-001, scenario 4)
- [x] T007 [US1] Investigate delivery (D6, FR-004): check that transcript and composer widgets do not swallow `FileDropped` before `SplitView::update` sees it, in `crates/micold-client/src/ui/material/split_view.rs` and `src/ui/panes.rs`; fix any defect found with a regression test, and record findings per platform (Wayland, X11, macOS, Windows) in `specs/1464-structured-drop-attach/research.md` R7
- [x] T008 [US1] Make T005 and T006 pass; fix whatever they show in `crates/micold-client/src/features/composer.rs`, `src/shell/drops.rs`, `src/shell/panes.rs`
- [x] T009 [US1] User guide: create `docs/user-guide/structured-input.md` (drop onto the view, quoted paths vs chips, refusal notice), list it in `docs/SUMMARY.md` and `docs/README.md`

## Phase 4: User Story 3 - "Add attachment" button with file picker (P1)

**Goal**: a composer-row icon button opens the native multi-file picker and admits the result exactly as a drop.

**Independent Test**: click the button, pick an image and a text file; chip and quoted path appear as for a drop.

- [x] T010 [P] [US3] Failing reducer tests in `crates/micold-client/tests/features_composer.rs`: `AttachRequested` twice opens one picker; `AttachPickerClosed` is silent; `AttachPicked` after the session ended is refused with a notice; the result lands only on the requesting session; the same ordered paths via `Dropped` and via `AttachPicked` give equal draft, images and notice; button press -> picker requested -> `AttachPicked` is the whole flow (FR-007, 008, 009, 012, 013, 015, SC-001, SC-002)
- [x] T011 [P] [US3] Failing geometry test in `crates/micold-client/tests/layout_snapshot.rs`: the Add attachment button fits the composer row beside Stop/Send; disabled when the session cannot accept input or a picker is open (FR-006, 011)
- [x] T012 [US3] Add `rfd` (async dialog, `xdg-portal` + `tokio` on Linux) to `/Cargo.toml` workspace deps and `crates/micold-client/Cargo.toml`
- [x] T013 [US3] `Msg::{AttachRequested,AttachPicked,AttachPickerClosed}` and private `picker_open` on the composer entry, with the transitions of data-model.md, in `crates/micold-client/src/features/composer.rs`
- [x] T014 [US3] Make `read_for_composer` `pub(crate)` in `crates/micold-client/src/shell/drops.rs`; new `crates/micold-client/src/shell/attach_picker.rs` (`open(session) -> Task<Message>` over `rfd::AsyncFileDialog::pick_files`, title "Add attachment", no filter)
- [x] T015 [US3] Route the new messages in `crates/micold-client/src/shell/composer.rs`: spawn the picker task, hand picked paths to `read_for_composer`, refuse a late result for a session that no longer accepts input
- [x] T016 [P] [US3] `Icon::Attach` (paperclip) with its glyph source, and a unit test in `icons.rs` that it has a glyph, in `crates/micold-material/src/icons.rs`; list it in `docs/user-guide/icons.md`
- [x] T017 [US3] Push `micold_material::IconButton` with an `iced::widget::tooltip` ("Add attachment") before Stop/Send in `crates/micold-client/src/ui/composer.rs`; disabled via `on_press_maybe(None)`; keyboard focus through its existing support (FR-006, FR-010 focus part)
- [x] T018 [P] [US3] Showcase poses of the button (normal, hover, focus, disabled; the focus pose is also where keyboard reach by Tab and Enter/Space is checked, T040) in `crates/micold-client/src/showcase/sections/`
- [x] T019 [US3] User guide: the button and the picker in `docs/user-guide/structured-input.md`

## Phase 5: User Story 4 - Keyboard shortcut (P2)

**Goal**: a composer-scoped chord opens the picker; the tooltip names it.

**Independent Test**: with the composer focused press Ctrl+Shift+A (Cmd+Shift+A on macOS); the picker opens.

- [ ] T020 [P] [US4] Failing tests: `composer_key` gives `KeyIntent::Attach` for the chord (Cmd on macOS via `logo`, Ctrl elsewhere) and `Edit` for plain `a` and `Ctrl+A`, in `crates/micold-client/tests/features_composer.rs`; and in `crates/micold-client/src/ui/app_chord_tests.rs` the app-level chord handler leaves Ctrl/Cmd+Shift+A to the focused composer (FR-010)
- [ ] T021 [US4] `KeyIntent::Attach` checked before the non-plain guard in `composer_key` (amend the R12 comment) in `crates/micold-client/src/features/composer.rs`; map it to `AttachRequested` in `src/shell/composer.rs`
- [ ] T022 [US4] Tooltip text "Add attachment (Ctrl+Shift+A)" / "(Cmd+Shift+A)" per platform in `crates/micold-client/src/ui/composer.rs`
- [ ] T023 [US4] User guide: the shortcut in `docs/user-guide/structured-input.md`

## Phase 6: User Story 4b - shortcut sheet and `/shortcuts` (P2)

**Goal**: the shortcut sheet lists the chord, and a structured session can open it (D7).

**Independent Test**: type `/shortcuts` in a structured composer; the sheet opens with a "Composer" group listing the attach chord.

- [ ] T024 [P] [US4] Failing core tests (red = does not compile until T027) in `crates/micold-core/tests/structured_commands.rs`: `/shortcuts` is in both the reported and the fallback command list as `CommandKind::App`, and `classify_draft` returns `DraftCommand::Shortcuts`
- [ ] T025 [P] [US4] Failing daemon tests in `crates/micold-daemon/src/state.rs` tests, beside the `DraftCommand::Model` cases: a switch to the terminal whose command is `/shortcuts` is refused as `SwitchRefusal::BadCommand`, and a user message `/shortcuts` is refused with `UserMessageRefusal::ShortcutsIsTheApps` (never sent to the CLI)
- [ ] T026 [P] [US4] Failing client tests (red = does not compile until T029–T031): `command_palette::classify` maps `DraftCommand::Shortcuts` to `Classified::Shortcuts` (unit test in `crates/micold-client/src/features/command_palette.rs`); `send_intent` returns `SendIntent::ShowShortcuts` (`crates/micold-client/tests/features_composer.rs`); `ShortcutsToggled(true)` makes `ShortcutSheet::open_in` true and `changes::Msg::ShortcutsClosed` clears both flags (unit test in `crates/micold-client/src/app.rs`); `SheetModel` groups "Review" (only with a Review view) and "Composer" (`COMPOSER_KEYS`) (unit test in `crates/micold-client/src/features/changes/keys.rs`)
- [ ] T027 [US4] `SHORTCUTS_COMMAND`, `DraftCommand::Shortcuts` and the `classify_draft` branch in `crates/micold-core/src/structured/commands/mod.rs`; add it to both list builders
- [ ] T028 [US4] `DraftCommand::Shortcuts` arms in both exhaustive `classify_draft` matches in `crates/micold-daemon/src/state.rs`: the switch-to-terminal validation (`SwitchRefusal::BadCommand`, beside `Send | Model(_)`) and the user-message validation (`Err(UserMessageRefusal::ShortcutsIsTheApps)`, beside `Model(_)`); add the variant and its message to `crates/micold-core/src/structured/user_message.rs`
- [ ] T029 [US4] `Classified::Shortcuts` in `crates/micold-client/src/features/command_palette.rs`; `SendIntent::ShowShortcuts` in `src/features/composer.rs`; turn it into `ShortcutsToggled(true)` in `src/shell/composer.rs`; nothing is sent to the CLI
- [ ] T030 [US4] `State.shortcuts_open` and `app::Message::ShortcutsToggled(bool)` in `crates/micold-client/src/app.rs`; the app's handler of `changes::Msg::ShortcutsClosed` also clears `shortcuts_open`; `ShortcutSheet::open_in` in `src/features/changes.rs` true when either flag is set
- [ ] T031 [US4] `SheetModel` as groups with `COMPOSER_KEYS` in `crates/micold-client/src/features/changes/keys.rs`; render the groups in `src/ui/shortcut_sheet.rs`
- [ ] T032 [P] [US4] Showcase poses of the sheet with both groups in `crates/micold-client/src/showcase/sections/`
- [ ] T033 [US4] User guide: `/shortcuts` and the sheet in `docs/user-guide/structured-input.md`

## Phase 7: User Story 2 - Drop-target cue (P2)

**Goal**: a cue shows on input-accepting structured panes while files hover, where the OS reports it.

**Independent Test**: drag files over the view; cue shows; leave or drop; it is gone.

- [ ] T034 [P] [US2] Failing reducer tests in `crates/micold-client/tests/pane_drop_target.rs`: `FilesHovering(true)` sets `files_hovering`, leave and drop clear it; a pane that cannot accept input and terminal panes get no cue (FR-005, SC-005)
- [ ] T035 [P] [US2] `crates/micold-material/src/drop_cue.rs`: shared presentational cue (outline plus tinted fill from theme `Roles`); export it from the crate; with a unit test that wrapping a child leaves the child's size unchanged
- [ ] T036 [US2] `SplitView` forwards `FileHovered`/`FilesHoveredLeft`/`FileDropped` as `PaneMsg::FilesHovering(bool)` in `crates/micold-client/src/ui/material/split_view.rs`; handle it in `src/shell/panes.rs`; `App.files_hovering` in `src/app.rs`
- [ ] T037 [US2] Wrap an input-accepting structured pane's body in `drop_cue` while the flag is set in `crates/micold-client/src/ui/panes.rs`
- [ ] T038 [P] [US2] Showcase pose of the cue in `crates/micold-client/src/showcase/sections/`
- [ ] T039 [US2] User guide: the cue (best effort) in `docs/user-guide/structured-input.md`

## Phase 8: Polish

- [ ] T040 Quickstart §B visual pass (`visual-pass` skill) and manual per-platform matrix, including keyboard reach of the button (Tab, Enter/Space, visible focus), SC-004 (UI responsive while the picker is open) and SC-003 per platform; record results in `specs/1464-structured-drop-attach/quickstart.md` (no code: done by the close unit)
- [ ] T041 Tick tasks and confirm every FR in the requirement map is covered (close unit)

## Dependencies & Execution Order

Phase 1, 2 then US1 (M1); US3 needs the guard (M1); the chord (US4) needs the button (M2); the sheet needs the chord (M3); the cue is independent of the button but follows in priority order. Within a phase tests first ([P] tasks touch disjoint files).

## Implementation Strategy

MVP = M1 (drops are admitted or refused with a notice, delivery verified). Then the button (M2), the chord (M3), the sheet and `/shortcuts` (M4), the cue (M5). The `/shortcuts` command (D7, T024–T033) is core + daemon + client: it is cut into M4 so the chord ships first.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Drop admission guard and delivery 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: Files dropped on a structured view reach only that session's composer, and a drop on a session that cannot accept input is refused with a notice naming the count; a user-guide page describes dropping.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002, FR-003, FR-004 (drop part), FR-011 (drop part), FR-016; SC-003 (tested part), SC-006
- **Verify**: `cargo test -p micold-client --test features_composer --test pane_drop_target --test features_session`
- **Depends on**: —
- **Tier**: full

### M2 — Add attachment button and picker

- **Tasks**: T010–T019
- **Deliverable**: The composer row has an Add attachment icon button (tooltip, focus, disabled state) that opens the native multi-file picker and admits the choice exactly as a drop.
- **Satisfies**: US3 acceptance scenarios 1–5; FR-006, FR-007, FR-008, FR-009, FR-011, FR-012, FR-013, FR-014 (button), FR-015; SC-001, SC-002, SC-004
- **Verify**: `cargo test -p micold-client --test features_composer --test layout_snapshot`, `cargo test -p micold-material`; quickstart §B step 3
- **Depends on**: M1
- **Tier**: full

### M3 — Attach shortcut

- **Tasks**: T020–T023
- **Deliverable**: Ctrl+Shift+A (Cmd+Shift+A on macOS) in a focused structured composer opens the picker, and the button tooltip names it.
- **Satisfies**: US4 acceptance scenarios 1–2; FR-010 (chord)
- **Verify**: `cargo test -p micold-client --test features_composer` and `cargo test -p micold-client app_chord`
- **Depends on**: M2
- **Tier**: full

### M4 — Shortcut sheet and /shortcuts

- **Tasks**: T024–T033
- **Deliverable**: Typing `/shortcuts` in a structured composer opens the shortcut sheet, which lists the attach chord in a Composer group.
- **Satisfies**: US4 acceptance scenario 3; FR-010 (sheet, and its reach through `/shortcuts`)
- **Verify**: `cargo test -p micold-core --test structured_commands`, `cargo test -p micold-daemon`, `cargo test -p micold-client --test features_composer`, `cargo test -p micold-client --lib`
- **Depends on**: M3
- **Tier**: full

### M5 — Drop-target cue

- **Tasks**: T034–T039
- **Deliverable**: While files hover over the window, every input-accepting structured pane shows a drop cue that is gone after leave or drop.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-005, FR-014 (cue); SC-005
- **Verify**: `cargo test -p micold-client --test pane_drop_target`, `cargo test -p micold-material`; quickstart §B step 1
- **Depends on**: M1
- **Tier**: full

Phase 8 (T040–T041) changes no code and is left to the close unit.
