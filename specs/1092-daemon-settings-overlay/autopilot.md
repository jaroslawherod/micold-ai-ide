# Autopilot ledger — #1092 daemon-settings-overlay

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Daemon settings should open in an overlay with its own Save, not rely on the Settings save button (issue #1092); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1092
- **Worktree branch**: feat/1092_daemon-settings-should-open-in-an-overlay-with-its
- **Started**: 2026-10-19
- **Phase**: milestone M1 (implementing)
- **Next step**: M1 implement, see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1137 | Design | merged | 0db3cf9ccbebc1b98662a5becfc4f2c4c3b96504 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T016, T017, T019, T020 | full | Add/Edit daemon in a modal dialog, own Save/Cancel, in-dialog confirmation, no inline form | - | in progress |
| M2 | T015, T018, T021–T023 | full | Add-path tests, focus return | - | pending |
| M3 | T024–T029 | light | Details in a dialog | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | e6163bef65f6d91e605c7395cc01c429cae5abb0:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CHANGES: 3 MAJOR (false premise, details mis-described, confirmations) - fixed |
| Spec | 2 | 264ca5b1ee1be4763aa21ea336b6d3cead721c30:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 | 1182d8f167058d85b22495f71b0e31c065e4d5a4:14c1b91abf503dd9792cfcdeee7e7091799e87f8 | CLEAN (3 MINOR fixed, prose only) |
| Plan | 1 | 148e0e9fc90d89309565b866272acec9556384c0:e22746bdb8932f04732f387ba677726415c65416 | CLEAN (4 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 implement unit 1 stopped at 150k. Branch reset to origin/main (branch-start.sh ok); WIP committed locally, nothing pushed, no M1 PR yet.

**Written but NEVER COMPILED** (the first test build was killed; expect compile errors to fix):
- `crates/micold-client/src/ui/material/scrolling_dialog.rs`: only the `#[cfg(test)]` tests (T002), no impl, not declared in `src/ui/material/mod.rs` (add `mod scrolling_dialog;` and `pub use scrolling_dialog::ScrollingDialog;`). Planned API: `ScrollingDialog::new(title: impl Into<String>, body: impl Into<Element>, actions: impl Into<Element>, roles)` with `.width(f32)` (default 560; Surface Dialog caps at MAX_WIDTH), `.max_height(f32)` (window height), `.actions_height(f32)` (default 40; confirmation panel about 120). Layout: `Surface(Dialog, dialog::body(dialog::fields(column![title Headline, container(material::Scrollable::new(body)).max_height(max_height - chrome)]), actions))`, chrome = 2*PADDING + TITLE_TO_BODY + title line (about 32) + BODY_TO_ACTIONS + actions_height + about 32 modal margin. The scroll container must NOT be Length::Fill (a Fill child makes a Shrink column take the full height). Test `the_actions_stay_pinned_inside_the_window` reads the actions at path `[0, 1]`; adjust if the tree differs.
- Tests added (uncompiled): `tests/daemons_feature.rs` (module `form_dialog`; needs `State::form_confirmation()`, `State::standalone_prompt()`, the stale-id refusal; the FR-007 test uses `app::State`, check `settings_draft` derives PartialEq/Default), `tests/features_daemons.rs` (module `drift_behind_the_form`; `world`/`send`/`dial` in `mod kubernetes` made `pub(super)`), `tests/overlay_registry.rs` (module `daemon_form`; deliberately NOT added to `dialogs()` so the 20/42 counts stay), `tests/daemon_overlay_focus.rs` (T001 probe against the removal dialog: records focusables with a custom Operation and Tab-walks `focus_next`; header holds `PROBE_OUTCOME_PLACEHOLDER` for the result; after T009/T010 add a second test for the form dialog. If the probe shows Tab escapes, T013 applies: scope focus_next/focus_previous in `src/main.rs` `Message::FocusMoved` to the overlay container id; `tests/field_focus_call_sites.rs` scans that arm for `focus_next()`/`focus_previous()`).
- `docs/user-guide/settings.md` Daemons section updated (T014 done; re-read once for wording).

**Still to do, in order**: T003 ScrollingDialog impl; T004 `form()`, `details_block()`, `missing_cli_notice` `pub(crate)` in `ui/settings/daemons.rs`; drop the Save daemon/Cancel row at the end of `form()` (the dialog's pinned actions replace it) and the inline `form(..)` placement in `view` (T012; keep the Add button in the page always, empty state shown on `list.empty` only); T007 geometry tests in `tests/daemons_section_geometry.rs` (page has no form fields; dialog Save daemon/Cancel inside `MIN_WINDOW_SIZE` 640x480; FR-013 row actions present); the existing geometry tests there that paint the form must paint the dialog body now: make its `view()` helper return `ui::daemon_form_dialog::dialog(..)` when `daemon_list.draft` is Some and set `with_daemons` window_size to (1280, 4000) so the scroll does not clip. T009 `src/ui/daemon_form_dialog.rs` (`pub fn dialog<'a>(state: &'a State, scheme, _env) -> Option<Element>`; draft = `state.daemon_list.draft`; facts = `MachineFacts{draft: settings.settings_draft.as_ref().map(|d| &d.daemon), legacy_default: state.daemon_rows().legacy_default, missing_cli: missing_cli_notice(state.session.availability.home())}`, focused = `state.window.focused_field`; title = daemon name or "Add daemon"; Save daemon filled `Msg::Saved`, Cancel outlined `Msg::DraftDiscarded`; re-export in `ui/mod.rs`; max_height from `state.window.window_size.1`). T010/T019 in `src/features/daemons.rs`: `DaemonFormDialog{confirming: bool}`, hand-written `FloatingSurface` (id `daemon_form`, Layer::Dialog, `DismissalRules::for_layer(Layer::Dialog).cancelled_by(RemovalCancelled if confirming else DraftDiscarded)`), `Registered::open_in` from `daemon_list.draft` and `form_confirmation()`, one `register!` line plus comment in `src/overlay/registry.rs` (`crate::features::daemons::DaemonFormDialog => crate::ui::daemon_form_dialog::dialog,`), `DIALOGS` 22 to 23 in `tests/popover_displacement.rs` and the count comment in `tests/surface_registration_cost.rs`; `State::form_confirmation()` (Pending::Edit prompt) and `standalone_prompt()` (excludes it); `ConfirmDaemonRemovalDialog::open_in` and `ui/confirm_daemon_removal::dialog` use `standalone_prompt()`. T011 `save_checked`: early `Err(FormError{field:"name", message:"This daemon no longer exists"})` when the edited id is not in `model.registry`. R7: `show_next_drift` returns early while `state.draft.is_some()`, and is called after `Msg::Saved` Ok with the draft cleared and after `Msg::DraftDiscarded`. T020 confirmation panel in the form dialog (prompt text and confirm/decline buttons `RemovalConfirmed`/`RemovalCancelled` replace the actions row). T013 per the probe. Tick T001-T014, T016, T017, T019, T020 in `specs/1092-daemon-settings-overlay/tasks.md` when done. Then scoped gate, review A (code-review high) in its shadow, review B (conformance), visual pass (quickstart Part B steps 1-5, 8) via an autopilot-worker, full gate (`mise run gate`), PR `feat(1092): edit and add daemons in a dialog (#1092)` ending `Refs #1092`.

Notes: no page dirtiness concept exists (Save is always enabled), so FR-007 is tested as the settings draft being unchanged. The `mise run test -- -p ...` passthrough is unverified; try `scripts/build-lock.sh cargo test -p micold-client --test <name>`. Builds take minutes: start detached and use `scripts/autopilot/hold.sh`. Never `pkill -f` a pattern that appears in your own command line.

## Open escalation

None.

## Follow-ups not done

None.
