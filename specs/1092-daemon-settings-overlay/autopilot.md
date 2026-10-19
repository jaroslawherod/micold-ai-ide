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

M1 implement unit 2 stopped at 150k. Branch ahead of origin/main by the WIP commits only; nothing pushed, no M1 PR yet. Everything now COMPILES (`cargo test -p micold-client --no-run --all-targets` clean).
**Done (impl, uncommitted before this commit)**: T003 `material::ScrollingDialog` (+ showcase Entry/render `sections::floating::scrolling_dialog`); T004/T012 (`form`, `details_block` pub(crate), form dropped from the page, `view()` lost its `facts` arg, form title/Save/Cancel row removed from `form()`); T009 `ui/daemon_form_dialog.rs` (+ confirmation panel T020); T010/T019 `DaemonFormDialog{confirming}`, `form_confirmation()`, `standalone_prompt()`, `traps_focus(id)` in `features/daemons.rs`, registry line, `DIALOGS` 23; T011 stale-id refusal; R7 `show_next_drift` gated on `draft.is_none()`. T013 APPLIES: probe showed Tab escapes (21 focusables, 2 in the dialog); fix = every dialog `Surface` carries container id (`ui::focus::dialog_scope`), `ui::dialog_focus_move_task` scopes focus_next/previous, main.rs `FocusMoved` uses it when `ui::dialog_keeps_focus(&app.core)` (daemon_form and confirm_daemon_removal only). T001 header filled in `tests/daemon_overlay_focus.rs`.
**Last full `cargo test -p micold-client --no-fail-fast` failures still to fix** (re-run first; some may be fixed already: daemon_overlay_focus was just fixed for iced not wrapping, `root_vocabulary_is_cross_cutting` fixed by moving the daemons check out of the main.rs arm into `ui::dialog_keeps_focus`, `root_state_is_shared` window.window_size allowlist entry deleted, showcase completeness entry added):
- `tests/daemons_section_geometry.rs`: `every_kind_paints_name_runs_in_its_sections_in_order_and_the_actions` (line ~592 "the form is painted") and `the_add_form_shows_a_refusal_beside_its_field` (line ~166): the dialog paints but the page-wide `painted()` list lacks the form texts; `with_daemons` window_size is now (1280, 4000). Check whether `ui::view` paints the dialog (maybe the Modal entrance animation hides it: use `painted_tall`/`painted_text_settled_at` instead of `painted`, as the other passing form tests do). Then T007 new tests (page has no form fields; dialog Save daemon/Cancel inside MIN_WINDOW_SIZE 640x480; FR-013 row actions present) are NOT yet written.
- `tests/features_daemons.rs` `drift_behind_the_form::a_queued_drift_prompt_also_shows_after_a_save`: `state.draft.is_none()` fails after `Saved` for a Host-runtime add named Plain (I added RuntimeChosen(Host)); find why save refuses (print `state.form_error`; maybe the world already has a daemon named so, or Host not allowed twice: use a Container/other name).
- `tests/gates/daemon_form_fits.rs` (layout_snapshot): now measures title-to-Cancel extent; it is 2312 tall, past 800, because the dialog body is the whole Kubernetes form: decide (the form is unchanged by this feature; the old 800 bound was the form's own height with the page header above it). Re-baseline to the extent the form had (measure on origin/main: old bottom) or assert the pinned actions only; record the choice in the test header.
- Re-run the whole package and then `scripts/build-lock.sh cargo test -p micold-core` not needed (core untouched).
**Still to do after green**: tick T001-T014, T016, T017, T019, T020 in `specs/1092-daemon-settings-overlay/tasks.md`; docs re-read (`docs/user-guide/settings.md`, mention Tab stays in dialog optional); scoped gate (`scripts/autopilot/scoped-gate.sh`, detached), review A (code-review high) in its shadow, review B (conformance, sonnet), visual pass (quickstart Part B steps 1-5, 8) via an autopilot-worker, full gate (`mise run gate`), PR `feat(1092): edit and add daemons in a dialog (#1092)` ending `Refs #1092`, record PR in ledger. Run builds detached and wait with `scripts/autopilot/hold.sh`. The T000-style test command that works: `scripts/build-lock.sh cargo test -p micold-client --no-fail-fast` (about 4 min, detached with output grep to a scratchpad log).

## Open escalation

None.

## Follow-ups not done

None.
