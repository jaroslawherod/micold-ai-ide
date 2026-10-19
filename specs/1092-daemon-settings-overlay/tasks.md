---
description: "Task list for the daemon settings overlay (#1092)"
---

# Tasks: Daemon settings in an overlay with their own Save

**Input**: `specs/1092-daemon-settings-overlay/` (spec.md, plan.md, research.md, data-model.md, contracts/ui-contract.md, quickstart.md)
**Tests**: required (Constitution I): every test task precedes the code it covers. GUI layout glue is covered by geometry gates and quickstart Part B.
**Paths**: `crates/micold-client/src/...` and `crates/micold-client/tests/...`; written `src/` and `tests/` below.

## Phase 1: Setup

- [x] T001 Probe the open risk (research R5): add `tests/daemon_overlay_focus.rs` driving `Message::FocusMoved` with the existing `ConfirmDaemonRemovalDialog` open, asserting focus never lands on a widget of the page behind. Record the outcome (escapes or not) in the test's header comment; if it escapes, T013 applies. T013 also re-runs it once T009/T010 land, against the new form dialog.

## Phase 2: Foundational

- [x] T002 [P] Write failing widget tests for `material::ScrollingDialog` in `src/ui/material/scrolling_dialog.rs` (`#[cfg(test)]`, patterned on `src/ui/material/dialog_anatomy.rs`): the body scrolls when taller than the window, the actions row stays pinned and visible, width honours `SurfaceKind::Dialog`.
- [x] T003 Implement `material::ScrollingDialog` in `src/ui/material/scrolling_dialog.rs` and export it from `src/ui/material/mod.rs`: chainable builder, `Roles`, `From<..> for Element`, built on `Surface`/`Modal`/`dialog::actions` (Principle VIII, FR-011).
- [x] T004 [P] Make `form()` and `details_block()` `pub(crate)` in `src/ui/settings/daemons.rs` so the dialog modules reuse them (plan: Structure Decision).

## Phase 3: User Story 1 + 2 - Edit in a dialog; Cancel or dismiss discards (P1) MVP

**Goal**: Edit and Add open the daemon form in a modal dialog with Save daemon / Cancel; Escape, scrim click and Cancel discard.
**Independent test**: edit a daemon name, Save daemon: list shows it, page Save untouched; dismiss three ways: unchanged.

- [x] T005 [P] [US1] Failing registry test in `tests/overlay_registry.rs`: `DaemonFormDialog` is open exactly while `daemon_list.draft.is_some()`, on `Layer::Dialog`; Escape and scrim map to `DraftDiscarded`.
- [x] T006 [US1] [US2] Failing reducer tests in `tests/daemons_feature.rs`: `EditOpened` fills the draft from stored values; `Saved` with valid data stores and clears the draft (SC-001, SC-003); the daemon's connection is updated as before (FR-003); `Saved` with an error keeps the draft and sets `form_error` (FR-004); `DraftDiscarded` leaves stored values unchanged and a reopen shows stored values (SC-002); a daemon saved from the draft leaves page-level Settings dirtiness and save-enabled state unchanged (FR-007, SC-004).
- [x] T007 [P] [US1] Failing geometry cases in `tests/daemons_section_geometry.rs`: with a draft open the page view contains no daemon form fields (SC-006); the form dialog's Save daemon and Cancel are inside the window at the smallest supported size (SC-005; use the existing minimum-window constant). Add a FR-013 regression assertion: row status and the other row actions (Remove, Start/Stop) are still present.
- [x] T008 [US1] Failing reducer tests in `tests/daemons_feature.rs` (R6, FR-014): `DaemonRegistryChanged` does not replace `draft`; `Saved` for an edit whose id left the registry stores nothing and sets `form_error` "This daemon no longer exists".
- [x] T009 [US1] Create `src/ui/daemon_form_dialog.rs`: `ScrollingDialog` titled with the daemon name or "Add daemon", body = `form()`, actions `Save daemon` (filled, `Msg::Saved`) and `Cancel` (outlined, `Msg::DraftDiscarded`); re-export in `src/ui/mod.rs`.
- [x] T010 [US1] Add `DaemonFormDialog` (id `daemon_form`, `Layer::Dialog`, `open_in` reads `daemon_list.draft`, dismissal `DraftDiscarded`) to `src/features/daemons.rs` and one `register!` line plus surface-count comments in `src/overlay/registry.rs`; update the surface counts in `tests/popover_displacement.rs` and `tests/surface_registration_cost.rs` (bump `DIALOGS` in `tests/popover_displacement.rs` by one, 22 to 23; extend the count comment in `tests/surface_registration_cost.rs`).
- [x] T011 [US1] In `src/features/daemons.rs` make `save_checked` refuse an edit whose id is not in `model.registry` with a `FormError` (R6); keep `draft` untouched on `DaemonRegistryChanged`.
- [x] T012 [US1] Remove the inline `form(..)` placement from `src/ui/settings/daemons.rs` (`view` no longer pushes it); the Add and Edit buttons only send `AddOpened` / `EditOpened`.
- [x] T013 [US1] If T001 showed Tab leaving the overlay: scope `focus_next`/`focus_previous` to the overlay container id in the `Message::FocusMoved` handler (research R5). Otherwise tick this task with the note "not needed" and keep the T001 test.
- [x] T014 [US1] Update `docs/user-guide/settings.md` Daemons section: Add and Edit open a dialog with its own Save daemon and Cancel; daemon changes no longer need the page Save; the running-daemon confirmation appears inside the dialog and Escape answers No.

## Phase 4: User Story 5 and 3 - in-dialog confirmation, add path, focus return (P2)

T016, T017, T019, T020 (US5) ship in M1 so a running-daemon edit never shows a second modal over the form; T015, T018, T021-T023 ship in M2.

**Goal**: Add path verified end to end, focus returns to the opener, the running-daemon confirmation shows inside the form dialog, system prompts wait for the overlay.
**Independent test**: add a daemon via dialog (saved, or cancelled with nothing added); edit a running daemon's runtime: confirmation in-dialog, No keeps edits, Yes closes.

- [ ] T015 [P] [US3] Failing reducer tests in `tests/daemons_feature.rs`: `AddOpened` opens a blank draft; valid `Saved` stores and lists the new daemon without page save; `DraftDiscarded` adds nothing.
- [x] T016 [US5] Failing tests in `tests/daemons_feature.rs` and `tests/overlay_registry.rs`: `Saved` needing confirmation sets `Pending::Edit` and keeps the draft; `form_confirmation()` returns it and `standalone_prompt()` excludes `Pending::Edit`; `RemovalCancelled` returns to the form with edits kept; the running decision is taken at Save time, not at open (edge case); `RemovalConfirmed` stores and closes; `DaemonFormDialog` dismissal is `RemovalCancelled` while confirming, `DraftDiscarded` otherwise; `ConfirmDaemonRemovalDialog` is closed for `Pending::Edit`.
- [x] T017 [US5] Failing reducer tests in `tests/daemons_feature.rs` and `tests/features_daemons_kube.rs`: a Kubernetes drift prompt raised while `draft` is `Some` stays queued, is never applied to the draft, and shows after the overlay closes (R7).
- [ ] T018 [US3] Failing reducer tests in `tests/daemons_feature.rs` for FR-009: `Opener` (`Add` or `Row(id, RowAction)`) is set on open and taken on close.
- [x] T019 [US5] In `src/features/daemons.rs` add `State::form_confirmation()`, make `standalone_prompt()` exclude `Pending::Edit`, and make `DaemonFormDialog` carry `confirming: bool` with the state-dependent dismissal; gate `show_next_drift` on `pending.is_none()` and no open overlay and call it when the overlay closes (R7).
- [x] T020 [US5] In `src/ui/daemon_form_dialog.rs` render the confirmation panel (the prompt's confirm/decline labels in place of Save daemon / Cancel) when `form_confirmation()` is `Some`; adjust `src/ui/confirm_daemon_removal.rs` so `dialog()` skips `Pending::Edit`.
- [ ] T021 [US3] Add `Opener` and `RowAction` and `State::opener` in `src/features/daemons.rs`, set on `AddOpened`/`EditOpened`, taken on close; give the material `Button` an optional `.id(..)` in `src/ui/material/button.rs` and set ids on Add and Edit in `src/ui/settings/daemons.rs`.
- [ ] T022 [US3] In `src/shell/daemons.rs` issue `iced::widget::operation::focus(id)` for the taken opener after a close message (FR-009).
- [ ] T023 [US3] Update `docs/user-guide/settings.md`: focus returns to the button that opened the dialog.

## Phase 5: User Story 4 - Details in a dialog (P3)

**Goal**: Details opens a read-only modal with live content and a Close button; the inline block and Hide details are gone.
**Independent test**: open Details on a daemon with a log, a new line appears live; Close/Escape/scrim close it.

- [ ] T024 [P] [US4] Failing tests in `tests/daemons_feature.rs` and `tests/overlay_registry.rs`: `DetailsOpened(id)` sets `details`, `DetailsClosed` clears it; `DaemonDetailsDialog` is open while `details` is `Some` and its daemon is in the registry, closed when the daemon is removed; replace `DetailsToggled` use in `tests/daemons_feature.rs` (line near 1430).
- [ ] T025 [US4] In `tests/daemons_feature.rs` add a test that the Details view reflects a newly logged line without reopening (US4 scenario 2). Update `tests/daemons_section_geometry.rs` (two `DetailsToggled` uses): the page shows no details block and the row has no Hide details; a new Details-dialog case checks Close is inside the window.
- [ ] T026 [US4] In `src/features/daemons.rs` replace `details_shown: BTreeSet<DaemonId>` and `Msg::DetailsToggled` with `details: Option<DaemonId>`, `Msg::DetailsOpened(DaemonId)`, `Msg::DetailsClosed`; drop the `details_shown` pruning in `RemovalConfirmed`; set `Opener::Row(id, RowAction::Details)`.
- [ ] T027 [US4] Create `src/ui/daemon_details_dialog.rs` (`ScrollingDialog`, body = `details_block()` computed from live state, action `Close`), register `DaemonDetailsDialog` (id `daemon_details`, dismissal `DetailsClosed`) in `src/features/daemons.rs` and `src/overlay/registry.rs`; update surface counts in `tests/popover_displacement.rs` and `tests/surface_registration_cost.rs` (bump `DIALOGS` by one, 23 to 24).
- [ ] T028 [US4] In `src/ui/settings/daemons.rs` make the row's Details button send `DetailsOpened(id)` and remove the inline `details_block` placement and Hide details label.
- [ ] T029 [US4] Update `docs/user-guide/settings.md`: Details opens in a dialog with Close.

## Phase 6: Polish

- [ ] T030 Run quickstart Part B (`visual-pass` skill) and record results in `specs/1092-daemon-settings-overlay/quickstart.md`; tick tasks. No code (close unit).

## Dependencies and order

- Setup T001 and Foundational T002-T004 first; then US1/US2 (M1); US3+US5 (M2) depend on M1's dialog; US4 (M3) depends on `ScrollingDialog` and the opener from M2.
- Within a story: tests, then implementation. [P] tasks touch disjoint files, except those appending to `tests/daemons_feature.rs`, which are written by one hand in order.

## Implementation Strategy

MVP = M1: Add and Edit work in the dialog with Save daemon, Cancel, Escape and scrim; inline form gone. M2 adds the Add-path tests and focus return. M3 moves Details. Until M3 the inline Details toggle remains working (no half-wired UI).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 - Edit and add in a dialog 🎯 MVP

- **Tasks**: T001–T014, T016, T017, T019, T020
- **Deliverable**: Settings > Daemons Add and Edit open a modal dialog with Save daemon and Cancel; saving applies at once, Cancel/Escape/outside click discard, no inline form; a running daemon's runtime edit is confirmed inside the dialog and system prompts wait for it.
- **Satisfies**: US1 acceptance scenarios 1–4; US2 acceptance scenarios 1–3; US5 acceptance scenarios 1–3; FR-001–FR-008, FR-010, FR-011, FR-014; FR-009 in part (Tab stays in the dialog); SC-001–SC-006
- **Verify**: `mise run test` (`daemons_feature`, `overlay_registry`, `daemons_section_geometry`, `daemon_overlay_focus`); quickstart §B steps 1–5, 8
- **Depends on**: —
- **Tier**: full

### M2 - Add path and focus return

- **Tasks**: T015, T018, T021–T023
- **Deliverable**: Add is verified end to end and focus returns to the button that opened the dialog on close.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-009
- **Verify**: `mise run test` (`daemons_feature`); quickstart §B step 7
- **Depends on**: M1
- **Tier**: full

### M3 - Details in a dialog

- **Tasks**: T024–T029
- **Deliverable**: Details opens a live read-only dialog with Close; the inline block and Hide details are gone.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-012, FR-013
- **Verify**: `mise run test` (`daemons_feature`, `overlay_registry`, `daemons_section_geometry`); quickstart §B step 6
- **Depends on**: M2
- **Tier**: light

T030 is left to the close unit (no code).
