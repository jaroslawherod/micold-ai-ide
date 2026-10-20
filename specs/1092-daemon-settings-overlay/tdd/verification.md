---
feature: 1092-daemon-settings-overlay
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: b56bf28d7 # short SHA audited
behaviors: 0 # no tdd/test-list.md; audited per tasks.md task and spec criterion instead (30 tasks, 29 ticked)
proven: 0
likely: 0
test_after: 29 # by rule: no red recorded for any task (no cycle log)
no_test: 0
high_smells: 3 # findings 2, 3, 4 (finding 1 is evidence, not a smell)
criteria_total: 29 # 5 stories x scenarios = 14 acceptance scenarios + 14 FR + 1 edge-case group; see Traceability
criteria_covered: 28 # FR-009 focus return only at reducer level, no shell test (finding 6)
mutation_score: unmeasured # no mutation tool; 11 deliberate mutants, 8 caught, 3 survived
mutants_survived: 3 # all inside DONE tasks, none equivalent
suite: 4227 passed, 0 failed, 3 ignored, 159s (micold-client only); 8 feature test binaries 26s
---

# TDD Verification: Daemon settings in an overlay (#1092)

**Verdict: FAIL.** The tests are real and the reducer/registry behavior is well pinned (8 of 8 reducer and
registry mutants caught), but there is no red-phase evidence for any task, a pre-existing details test was
loosened, and the dialogs' buttons are wired to nothing a test checks (3 of 3 button-message mutants
survive the whole client suite).

Not independent in the sense that matters least: this audit ran from cold context in a different session
from the one that wrote the tests; every file cited was re-read.

## Test-first evidence

No `FEATURE_DIR/tdd/test-list.md` and no `tdd/cycle-log.md` exist: the feature was planned and run through
`speckit-autopilot`, not the TDD extension. There is therefore no behavior id and no recorded red command.
Fail closed: every task is `TEST_AFTER` by rule. The git history, which outranks the log, gives this
ordering for the milestones (range `0db3cf9cc..HEAD`, 1092 commits only):

| Milestone / tasks | Class | Evidence |
| --- | --- | --- |
| M1 T001-T014, T016, T017, T019, T020 | TEST_AFTER (rule); ordering favourable | `984ab4d4d` "M1 tests, docs and handover (uncompiled)" lands the tests before `d751eec57` "M1 implementation". The tests were committed uncompiled, so no red was ever observed. `scrolling_dialog.rs` (T003, with its T002 unit tests in the same file) is in the tests commit. |
| M1 fix-up `9a4ba62c0` (expand clip) | TEST_AFTER | `expand_clipping.rs` test and source in one commit, no red recorded. |
| M2 T015, T018, T021-T023 | TEST_AFTER | `bc6cdce8f` adds tests, source and docs in one commit; the follow-up fixes `6d21f313a`, `bb57df73b` have no test change. |
| M3 T024-T029 | TEST_AFTER | `1a4813946` "wip ... from interrupted unit" adds tests and source together; `6646970b1` then edits two of those tests (see finding 3). |
| T030 (visual pass) | NOT_APPLICABLE | Unticked; `visual-pass-m1.md` exists, M2 and M3 passes are partial (autopilot.md: Esc/Save/Edit/Tab "not judged"). |

`tasks.md` against reality: every task but T030 is ticked and its test exists and passes. No ticked task
lacks a test. T013 was ticked as required (T001 probe showed Tab escaped); `ui::dialog_focus_move` exists.

## Existing tests changed by the feature

| Where | Before | After | Judgment |
| --- | --- | --- | --- |
| `crates/micold-client/tests/daemons_section_geometry.rs:132-160` (`a_row_with_its_details_open_paints_its_errors_and_log` renamed) | with details open, asserted the page paints `"connection refused"` and `"Hide details"` | asserts the dialog paints `"No connection errors"` and `"Nothing logged yet"`; page-level `!has("connection refused")` became `!has("Log")` | Loosened. Finding 3. |
| `crates/micold-client/tests/gates/daemon_form_fits.rs:~50-72` | Kubernetes add form's absolute `Cancel` bottom `<= 800` | dialog extent (title top to Cancel bottom) `<= 800`, window forced to `TALL` | Measure changed with the placement; defensible but no longer checks position. Finding 5. |
| `crates/micold-client/tests/root_state_is_shared.rs:~214` | `window.window_size` listed in `COMPONENT_LOCAL` with a justification | entry removed | Forced by the file's own rule (the field is now read by the dialog views, `ui/daemon_*_dialog.rs`), the test passes. Accepted. |
| `crates/micold-client/tests/popover_displacement.rs` | `DIALOGS` 22 | 24 | Matches T010/T027. Accepted. |
| `crates/micold-client/tests/daemons_section_geometry.rs:~160` and `tests/daemons_feature.rs` (`DetailsToggled` use) | inline toggle | `DetailsOpened`/`DetailsClosed` | Behavior change per spec. Accepted. |

No test was skipped, ignored, renamed out of a filter, or excluded; no threshold exists to lower.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | No red-phase evidence for any task: no test-list, no cycle log. M1 tests were committed uncompiled, so they could not have been run red; M2 and M3 tests and source landed in single commits. Fails closed to `TEST_AFTER`. | `git show --stat 984ab4d4d d751eec57 bc6cdce8f 1a4813946`; `specs/1092-.../tdd/` absent before this report |
| 2 | HIGH | The dialogs' buttons are not tested. Mutants changing what the Save daemon button, the row's Edit button and the row's Details button send all survived the full `micold-client` suite (4227 tests). Every test injects `Msg::Saved`/`EditOpened`/`DetailsOpened` directly or only checks painted text, so the pointer path of FR-002 and US1/US3/US4 "user presses ..." is unverified. Should: build the view, find the button, and assert the message it produces (or drive `iced_test`-style press), for Save daemon, Cancel, Close, Edit, Details, Add. | `crates/micold-client/src/ui/daemon_form_dialog.rs:60` (Saved), `src/ui/settings/daemons.rs:191` (EditOpened), `:196` (DetailsOpened); tests: `tests/daemons_section_geometry.rs` only counts painted strings |
| 3 | HIGH | The details dialog's real content (errors and log lines) is never asserted painted. The pre-existing test asserted a populated error; the replacement asserts only the two empty-state strings, and its fixture has no diagnostics (`with_daemons()` feeds `DaemonLinks::apply`, but the dialog reads `State::diagnostics`). The name still says "paints_errors_and_log". Live update (US4 #2) is checked on `daemons::details()` data only, not on the view. Should: record an error and a log line through `daemons::daemon_event`/`Msg::LogLine`, then assert the dialog's painted strings contain them. | `tests/daemons_section_geometry.rs:132-160`; `src/ui/settings/daemons.rs:211-230`; `tests/daemons_feature.rs` `details_open_and_close_and_follow_a_new_log_line` |
| 4 | HIGH | `escape_and_the_scrim_discard_the_draft` never exercises the scrim. It asserts `registry::escape` and `on_escape`; outside-click dismissal (FR-005, SC-002 third path, US2 #2) is wired in `ui/mod.rs:771` through `on_escape` and nothing tests the form or details dialog's `Modal::on_dismiss`. The name claims coverage the body lacks. Should: build the view with a draft open and assert the modal's dismiss message, or rename and add the test. | `tests/overlay_registry.rs:895-905`; `src/ui/mod.rs:771-776` |
| 5 | MED | Height gate changed from "form ends within 800 px" to "dialog is 800 px tall"; it no longer fails if the dialog is placed off-window. Also the dialog is centred in a forced `TALL` window, so position is never constrained. SC-005 is covered separately by `the_dialog_actions_stay_inside_the_smallest_window`, which is sound. | `tests/gates/daemon_form_fits.rs:~50-72` |
| 6 | MED | FR-009 focus return is tested only as reducer state (`take_focus_return`), not the shell effect `iced::widget::operation::focus(id)` in `shell/daemons.rs`, nor the ids on the buttons (`opener_id`). Recorded by M2 review B as MINOR; mutating the id mapping would not fail a test. | `tests/daemons_feature.rs` mod `add_dialog`; `src/shell/daemons.rs`; `src/ui/settings/daemons.rs:123` |
| 7 | MED | `the_rows_keep_their_status_and_actions_with_the_dialog_open` compares `has(before, x) == has(after, x)`, which passes when a needle is absent in both; only `Remove` is asserted present. `has` is a substring match, so `"Edit"`, `"Start"` also match the dialog's own text. | `tests/daemons_section_geometry.rs` (FR-013 test, ~L969-982), `:113` |
| 8 | MED | Reducer tests in `daemons_feature.rs`'s `form_dialog`/`add_dialog` drive `daemons::Msg` through a `World` fixture; end-to-end through `app::State::update` exists for one case only (`a_daemon_saved_from_the_dialog_leaves_the_page_draft_alone`). SC-003 "values survive a restart" is not tested (no persistence round-trip). | `tests/daemons_feature.rs:2043-2420` |
| 9 | LOW | Foreign style: `daemons_feature.rs` is now >2400 lines with 5 `mod`s, `overlay_registry.rs` nests `mod daemon_form`; a shared `Opener`/`Msg` import preamble is repeated. Probe test `daemon_overlay_focus.rs` hand-rolls an `Operation` recorder (~150 lines) rather than reusing `support/`. | `tests/daemon_overlay_focus.rs:1-120` |

## Mutation results

No mutation tool in the profile (`mutation: null`). Deliberate mutants on 11 high-risk spots, one at a time,
each restored with `git checkout -- <file>` and the tree verified clean. Scope: `features/daemons.rs` (8),
`ui/daemon_form_dialog.rs` (1), `ui/settings/daemons.rs` (2). Sampled, not exhaustive. Test set: five
binaries (`daemons_feature`, `overlay_registry`, `daemons_section_geometry`, `features_daemons`,
`daemon_overlay_focus`); the three survivors were re-run against the whole `micold-client` suite.

| Mutant | Behavior (FR) | Survived | Judgment |
| --- | --- | --- | --- |
| `features/daemons.rs:1210` guard `registry.get(id).is_none()` disabled | save of a removed daemon (FR-014) | No | Caught by `saving_an_edit_of_a_removed_daemon_is_refused` |
| `:1446` (show_next_drift) drift loop ignores `draft.is_none()` | prompts wait for overlay (R7) | No | Caught by two `drift_behind_the_form` tests |
| `:520` `standalone_prompt` no longer excludes `Pending::Edit` | no second modal (FR-008) | No | Caught by `a_running_daemons_edit_asks_inside_the_form` |
| `:2250` dismissal condition negated | Escape while confirming = No | No | Caught by two overlay tests |
| `:435-438` `opener.take()` -> `clone()` | focus return once (FR-009) | No | Caught by `the_opener_is_set_on_open_and_taken_on_close` |
| `:2294-2298` details dialog stays open after removal | no empty modal | No | Caught by `details_dialog_is_open_while_its_daemon_is_in_the_registry` |
| `:2303` form removed from `traps_focus` | Tab trapped | No | Caught by `both_daemon_dialogs_ask_for_the_scoped_traversal` |
| `:2195` `DetailsClosed` no-op | Close (FR-012) | No | Caught by `details_open_and_close_and_follow_a_new_log_line` |
| `daemon_form_dialog.rs:60` Save daemon sends `DraftDiscarded` | Save button (FR-002/003) | **Yes** | Not equivalent. Finding 2 |
| `settings/daemons.rs:191` Edit sends `AddOpened` | Edit button (FR-001) | **Yes** | Not equivalent. Finding 2 |
| `settings/daemons.rs:196` Details sends `EditOpened` | Details button (FR-012) | **Yes** | Not equivalent. Finding 2 |

Mutation score: unmeasured by tool. Deliberate sample: 8 of 11 caught (73%), all 3 survivors in ticked tasks.

## Traceability

"End to end" means through `app::State::update`, the registry or the painted view, not a reducer called
directly.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 #1 / FR-001 | `edit_opened_fills_the_draft...`, `an_open_draft_paints_the_form_in_the_dialog_and_not_on_the_page`, `it_is_open_exactly_while_the_draft_exists...` | Yes (button press not covered, finding 2) |
| US1 #2 / FR-003 / SC-001 | `saved_with_valid_data_stores_and_closes` | Reducer; app level via `a_daemon_saved_from_the_dialog...` |
| US1 #3 / FR-004 | `saved_with_an_error_keeps_the_draft_and_names_the_field`, `the_add_form_shows_a_refusal_beside_its_field` | Yes |
| US1 #4 / FR-007 / SC-003 / SC-004 | `a_daemon_saved_from_the_dialog_leaves_the_page_draft_alone` | Yes for FR-007/SC-004; restart persistence not tested (finding 8) |
| US2 #1-3 / FR-005 / SC-002 | `discarding_leaves_the_stored_values...`, `escape_and_the_scrim_discard_the_draft` | Escape yes; scrim click no (finding 4) |
| US3 #1-3 | `add_opened_opens_a_blank_draft`, `a_valid_save_stores_and_lists...`, `discarding_a_filled_form_adds_nothing` | Reducer only |
| US4 #1 / FR-012 | `the_page_has_no_inline_details_and_the_details_dialog_paints...`, `details_dialog_is_open_while...` | Partial: populated errors and log not painted (finding 3) |
| US4 #2 | `details_open_and_close_and_follow_a_new_log_line` | Data only (finding 3) |
| US4 #3 | `details_dialog_is_open...` (Escape); Close button not pressed | Partial (findings 2, 4) |
| US5 #1-3 / FR-008 | `a_running_daemons_edit_asks_inside_the_form`, `declining_...`, `accepting_...`, `while_confirming_escape_answers_no...` | Yes |
| FR-002 | `the_dialog_actions_stay_inside_the_smallest_window` (painted) | Painted only; press not tested (finding 2) |
| FR-006 modal | provided by existing `Layer::Dialog` modal; `it_is_open_exactly...` asserts the layer | Indirect |
| FR-009 | `daemon_overlay_focus` (3 tests), `the_opener_is_set_on_open...` | Tab trap yes; focus-return effect no (finding 6) |
| FR-010 | existing form tests, section tests | Yes |
| FR-011 / SC-005 | `scrolling_dialog` unit tests (4), `the_dialog_actions_stay_inside_the_smallest_window`, `daemon_form_fits` | Yes |
| FR-013 | `the_rows_keep_their_status_and_actions_with_the_dialog_open`, `a_removal_confirmation_is_still_its_own_dialog` | Yes (weak, finding 7) |
| FR-014 | `a_registry_change_does_not_replace_the_draft`, `saving_an_edit_of_a_removed_daemon_is_refused` | Yes |
| SC-006 | `an_open_draft_paints_the_form_in_the_dialog_and_not_on_the_page` | Yes |
| Edge: drift queued while open | two `drift_behind_the_form` tests | Yes |

Untested criteria: none outright; FR-005 (outside click), FR-009 (focus effect) and SC-003 (restart) lack a
real-entry-point test. Tests tracing to nothing: none (`expand_clipping` traces to the M1 visual defect, a
side fix recorded in autopilot.md).

## Properties

Isolation and determinism: no clocks, sleeps, network or shared files in the new tests; the feature
binaries run in 26 s, the client package in 159 s. Specificity: geometry failures print the painted
strings, reducer tests carry rule messages. Refactor sensitivity: `daemon_overlay_focus` and registry tests
assert ids (`"daemon_form"`, `"daemon_details"`) and `Debug`-derived opener ids; acceptable.

## What was not audited

- Mutation by tool: none installed; only 11 hand-picked mutants, none in `shell/daemons.rs`, `scrolling_dialog.rs`,
  `confirm_daemon_removal.rs`, `button.rs` or `expand_clipping.rs`.
- The full workspace suite (core and daemon crates; profile baseline 5071 tests): not run, the feature
  touches `micold-client` only. The 1092 commits' diff was restricted to that crate plus docs; the broader
  `0db3cf9cc..HEAD` range contains many unrelated merges from `main`.
- Visual quality, themes and the quickstart Part B pass (T030 unticked; M2 and M3 passes partial).
- `docs/user-guide/settings.md` accuracy; coverage (no tool); performance beyond wall time.
- Behaviors per test-list id: impossible, no list existed.
