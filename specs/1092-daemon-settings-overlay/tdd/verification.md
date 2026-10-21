---
feature: 1092-daemon-settings-overlay
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 79cd9920a # short SHA audited (re-audit; previous FAIL at b56bf28d7)
behaviors: 0 # no tdd/test-list.md; audited per tasks.md task (40 tasks, 38 ticked) and spec criterion
proven: 1 # T039 (red recorded in the cycle log, test and fix in one commit)
likely: 0
test_after: 36 # T001-T029 (no red on record) + T031-T033, T035-T038 (tests over existing behavior, mutant-proven, no red)
no_test: 0
high_smells: 0 # previous findings 2, 3, 4 are cleared
criteria_total: 29 # 14 acceptance scenarios + 14 FR + 1 edge-case group; see Traceability
criteria_covered: 29 # every criterion reaches a test through the built view, the registry or app::State::update
mutation_score: unmeasured # no mutation tool; 9 deliberate mutants this run, 8 caught, 1 survived
mutants_survived: 1 # the shell's focus effect call (finding 2), not equivalent
suite: 4279 passed, 0 failed, 3 ignored (micold-client only; wall time 18 min, almost all waiting on the shared build lock; feature binaries under 15 s)
---

# TDD Verification: Daemon settings in an overlay (#1092)

**Verdict: FAIL.** The only blocking reason is the test-first record: M1-M3 and six of the seven M4 test tasks have no
red phase on record, and the rubric fails every `TEST_AFTER` behavior. Everything behavioral the previous audit
found is fixed: no HIGH smell, no weakened existing test, every criterion has an end-to-end test, 8 of 9 new
mutants caught. The remaining findings are evidence and test-strength items, not unbuilt behavior.

Independence: this audit ran from cold context in a session that did not write the tests; every cited file was
re-read at HEAD.

## What changed since the previous audit (b56bf28d7)

M4 (PR #1182, `6719cd3f2`, `932d611a0`) closed the previous findings 2 to 8 and added one behavior (drift prompts
wait while Details is open, and a queued drift prompt keeps focus return waiting). Verified by reading the tests
and by mutants below. The previous FAIL reasons 2, 3 and 4 (HIGH smells) no longer apply; reason 1 (no red) stands.

## Test-first evidence

No `tdd/test-list.md` exists; `tdd/cycle-log.md` now exists but covers M4 only. Fail closed.

| Tasks | Class | Evidence |
| --- | --- | --- |
| T001-T029 (M1-M3) | TEST_AFTER (rule) | Unchanged from the previous report: M1 tests committed uncompiled (`984ab4d4d` before `d751eec57`), M2 and M3 tests and source landed together. The cycle log says so itself ("no red phase on record"; T034 accepted in the ledger). |
| T039 drift waits for Details | PROVEN | Cycle log records the failing assertion (`no second modal over Details`, `features_daemons.rs:1309`); `6719cd3f2` carries test and fix together. Mutants M1 and M3 below fail the same test. |
| T031 button wiring, T032 details content, T033 scrim, T036 focus ids and return, T037 FR-013, T038 SC-003 | TEST_AFTER | The log is honest: the behavior already existed, so there was no red; the tests are proven by mutants instead (the log's claim holds for the ones I re-ran: M5-M8). That is not a red phase. |
| T035 (gate bound) | TEST_AFTER | Assertion added to an existing gate; no red. |
| Review-A follow-ups (`932d611a0`) | TEST_AFTER | Log: "red not run separately". Mutants M2 and M3 fail their tests, so they bite. |
| T030 visual pass | NOT_APPLICABLE | Still unticked; `visual-pass-m1.md` only; M2/M3 passes partial. |
| T034 | NOT_APPLICABLE | Process task. Resolved in tasks.md line 126 and the ledger, but its own checkbox (line 120) is still `[ ]`, so tasks.md says one thing in two places. |

`tasks.md` against reality: no ticked task lacks a test and every ticked behavioral task has a passing test. Unticked:
T030 (visual pass, a close-unit task) and T034 (see above).

## Existing tests changed by the feature (M4 delta)

| Where | Before | After | Judgment |
| --- | --- | --- | --- |
| `tests/daemons_section_geometry.rs:135-182` (details dialog) | dialog empty-state strings only | feeds `DialFailed("connection refused")` and two `LogLine`s through the reducer; asserts both painted in the dialog and absent from the page; empty-state strings asserted absent; second line appears live | Strengthened. Previous finding 3 cleared. |
| `tests/daemons_section_geometry.rs:1008-1038` (FR-013) | `has(before)==has(after)`, only Remove asserted | exact-label presence on the page before and after, 7 labels incl. Details | Strengthened. Previous finding 7 cleared. |
| `tests/overlay_registry.rs:895` | `escape_and_the_scrim_discard_the_draft` | renamed `escape_discards_the_draft`; the scrim is now `daemon_dialog_buttons::clicking_the_scrim_*` | Honest rename, coverage moved, not lost. |
| `tests/gates/daemon_form_fits.rs:66-77` | extent only | extent plus "dialog lies inside the TALL window" | Strengthened (weakly, finding 4). |

No test skipped, ignored, renamed out of a filter or excluded; no threshold exists. The 3 ignored tests in the client
suite are pre-existing (profile baseline). None is a 1092 test.

## Findings

Severity is the rubric's. "Kind" says whether the item is unbuilt behavior or test-strength, evidence or docs.

| # | Severity | Kind | Finding | Evidence |
| --- | --- | --- | --- | --- |
| 1 | HIGH | Evidence (process); not unbuilt behavior | No red-phase record for T001-T029 and for T031-T033, T035-T038. Cannot be earned retroactively: the commits are merged and the tests passed on first run. The rubric makes this a FAIL by itself; the options are an explicit waiver by the maintainer, or accepting the verdict. | `tdd/cycle-log.md`; `git show --stat 984ab4d4d d751eec57 bc6cdce8f 1a4813946 6719cd3f2` |
| 2 | MED | Test-strength | The shell's actual focus effect is untested. `tasks.push(iced::widget::operation::focus(id))` can be deleted (mutant M9) and the whole micold-client suite (lib, bin, all integration tests) stays green. The id selection (`focus_return_id`, mutant M4) and the button ids are pinned; only the one-line `Task` push is not. | `crates/micold-client/src/shell/daemons.rs:165-167`; test `shell::daemons::tests::a_closed_dialog_returns_...` calls `focus_return_id` only |
| 3 | LOW | Docs/tasks hygiene | T034's checkbox is `[ ]` while the same file records it resolved (line 126). T030 is unticked, the visual pass M2/M3 is partial. | `specs/1092-daemon-settings-overlay/tasks.md:120,126,68` |
| 4 | LOW | Test-strength | `daemon_form_fits` now only asserts the dialog is inside a 4000 px window (it is centred, so almost any placement passes); the position is constrained by `the_dialog_actions_stay_inside_the_smallest_window` instead, which is sound. | `tests/gates/daemon_form_fits.rs:66-77` |
| 5 | LOW | Test-strength | SC-003 test persists through the real `JsonFileSettingsStore` but takes the registry from the `Outcome`, so the shell's write of that outcome (`shell/daemons.rs`) is not exercised. Acceptable; reducer-to-store contract is what the spec states. | `tests/daemons_feature.rs:2145-2170` |
| 6 | LOW | Style / duplication | A third hand-rolled iced `Operation` recorder (`Ids` in `daemon_dialog_buttons.rs:160`, beside `Record` in `daemon_overlay_focus.rs:42` and the recorders in `focus_scroll.rs`, `settings_rail_motion.rs`) and `daemons_feature.rs` at 2404 lines with five `mod`s. No shared helper in `tests/support/`. Unchanged from the previous finding 9. | files cited |

Previous findings: 1 stands (now finding 1); 2, 3, 4 cleared; 5 reduced (finding 4); 6 reduced (finding 2); 7, 8
cleared; 9 stands (finding 6).

## Smell pass (new and changed M4 tests)

`daemon_dialog_buttons.rs` (read in full): drives the built view and sweeps a click grid, so a wrong message fails
on the message value; no doubles; assertions carry messages; scrim tests assert the exact message list
(`vec![DraftDiscarded]`), not truthiness. No HIGH or MED smell. The `contains` checks in the two button tests are
specific values, not truthiness. `features_daemons.rs` drift tests and the shell test assert specific states with
rule messages. Determinism: no clock, sleep or network; the SC-003 test uses a `tempfile` directory.

## Mutation results

No mutation tool in the profile (`mutation: null`). Nine deliberate mutants, one at a time, each restored with
`git checkout -- <file>`; `git status` clean afterwards and the affected binaries (`features_daemons`,
`daemon_dialog_buttons`, `daemons_section_geometry`, the `micold-ai-ide` bin) re-run green. Scope: the M4 code and
the dialogs' wiring in `features/daemons.rs`, `shell/daemons.rs`, `ui/mod.rs`, `ui/daemon_*_dialog.rs`,
`ui/settings/daemons.rs`. Sampled, not exhaustive. (The first run of M4 and M9 used `--lib`, which cannot reach the
`shell` module that lives in the bin; both were re-run against `--bin` and M9 against every target, and only those
re-runs are reported.)

| Mutant | Behavior | Survived | Caught by |
| --- | --- | --- | --- |
| M1 `show_next_drift` ignores Details (`features/daemons.rs:1447-1452`) | prompt waits for Details (T039) | No | `a_drift_prompt_raised_while_details_are_open_waits_for_them_to_close` |
| M2 `take_focus_return` ignores a pending prompt (`:438`) | focus returns after the queued prompt | No | `focus_returns_after_the_queued_drift_prompt_is_answered_not_under_it` |
| M3 Details of a removed daemon hold prompts (`:1449`) | stale Details hold nothing back | No | `details_of_a_daemon_gone_from_the_registry_hold_no_prompt_back` |
| M4 `focus_return_id` returns the Add id (`shell/daemons.rs:~473`) | focus goes to the opener | No | `shell::daemons::tests::a_closed_dialog_returns_the_keyboard_to_the_button_that_opened_it` |
| M5 `ui/mod.rs:775` scrim `on_dismiss` dropped | outside click dismisses (FR-005) | No | `clicking_the_scrim_discards_the_form_draft`, `clicking_the_scrim_closes_the_details_dialog` |
| M6 Close sends `AddOpened` (`daemon_details_dialog.rs:27`) | Close (FR-012) | No | `the_details_dialog_close_sends_details_closed` |
| M7 details dialog's error loop emits nothing (`settings/daemons.rs:219`) | errors painted (US4) | No | `the_details_dialog_paints_the_errors_and_the_log_...` |
| M8 Save daemon sends `DraftDiscarded` (`daemon_form_dialog.rs:60`) | Save (FR-002/003) | No | `the_form_dialog_buttons_send_save_and_discard` (was a survivor last audit) |
| M9 `operation::focus(id)` push removed (`shell/daemons.rs:165`) | focus actually moves (FR-009) | **Yes** | nothing in the micold-client suite; finding 2 |

Previous survivors (Save, Edit, Details wiring) are now caught: M8 re-run, and `the_page_buttons_open_the_dialog_for_their_row`
asserts Edit and Details per row (not re-mutated this run). Deliberate sample: 8 of 9 caught (89%). The one
survivor is inside a ticked task (T036) and is not equivalent, but it is a single glue line, hence MED.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 #1 / FR-001 | `edit_opened_fills_the_draft...`, `an_open_draft_paints_the_form_in_the_dialog_and_not_on_the_page`, `the_page_buttons_open_the_dialog_for_their_row` | Yes, incl. the button press |
| US1 #2 / FR-003 / SC-001 | `saved_with_valid_data_stores_and_closes`, `the_form_dialog_buttons_send_save_and_discard` | Yes |
| US1 #3 / FR-004 | `saved_with_an_error_keeps_the_draft_and_names_the_field`, `the_add_form_shows_a_refusal_beside_its_field` | Yes |
| US1 #4 / FR-007 / SC-003 / SC-004 | `a_daemon_saved_from_the_dialog_leaves_the_page_draft_alone`, `a_daemon_saved_from_the_dialog_survives_a_restart` | Yes (finding 5) |
| US2 #1-3 / FR-005 / SC-002 | `discarding_leaves_the_stored_values...`, `escape_discards_the_draft`, `clicking_the_scrim_discards_the_form_draft` | Yes, Escape, Cancel and scrim |
| US3 #1-3 | `add_opened_opens_a_blank_draft`, `a_valid_save_stores_and_lists...`, `discarding_a_filled_form_adds_nothing`, Add press in `the_page_buttons_...` | Yes |
| US4 #1 / FR-012 | `the_details_dialog_paints_the_errors_and_the_log_and_the_page_paints_neither`, `details_dialog_is_open_while...` | Yes |
| US4 #2 | same painted test, second `LogLine` appears | Yes |
| US4 #3 | `the_details_dialog_close_sends_details_closed`, `clicking_the_scrim_closes_the_details_dialog`, Escape in the registry test | Yes |
| US5 #1-3 / FR-008 | `a_running_daemons_edit_asks_inside_the_form`, `declining_...`, `accepting_...`, `while_confirming_escape_answers_no...` | Yes |
| FR-002 | `the_dialog_actions_stay_inside_the_smallest_window`, button-press tests | Yes |
| FR-006 modal | `it_is_open_exactly_while_the_draft_exists...` (layer) plus scrim tests | Yes |
| FR-009 | `daemon_overlay_focus` (3), `the_add_and_row_buttons_carry_the_ids_focus_returns_to`, `a_closed_dialog_returns_the_keyboard_...`, reducer tests | Id and decision yes; the focus `Task` itself no (finding 2) |
| FR-010 | existing form and section tests | Yes |
| FR-011 / SC-005 | `scrolling_dialog` unit tests, `the_dialog_actions_stay_inside_the_smallest_window`, `daemon_form_fits` | Yes |
| FR-013 | `the_rows_keep_their_status_and_actions_with_the_dialog_open` (exact labels), `a_removal_confirmation_is_still_its_own_dialog` | Yes |
| FR-014 | `a_registry_change_does_not_replace_the_draft`, `saving_an_edit_of_a_removed_daemon_is_refused` | Yes |
| SC-006 | `an_open_draft_paints_the_form_in_the_dialog_and_not_on_the_page` | Yes |
| Edge: drift queued while open | `drift_behind_the_form` (now five tests, covering form and Details) | Yes |

Untested criteria: none. Tests tracing to nothing: none (`expand_clipping` traces to the M1 visual defect recorded in autopilot.md).

## Properties

Isolation and determinism: no clocks, sleeps, network or shared files; one temp dir. Speed: feature binaries under
15 s; the micold-client package 4279 tests (the 18 min wall time was queueing on `build-lock.sh` behind other
worktrees, not test time; the previous run measured 159 s). Specificity: failures print the message or painted
strings. Refactor sensitivity: tests assert ids and `Message` values, which is the contract.

## What was not audited

- Mutation by tool (none installed); only 9 hand-picked mutants this run, plus the 11 of the previous run (not
  repeated). Not mutated: `shell/daemons.rs` beyond the focus lines, `scrolling_dialog.rs`,
  `confirm_daemon_removal.rs`, `button.rs`, `expand_clipping.rs`, the Edit/Details button wiring (previous survivors,
  covered now by `the_page_buttons_...` but not re-mutated).
- The core and daemon crates and the workspace suite (profile baseline 5071 tests): not run; 1092 changes the client
  and docs only. The M4 range also touched `features/changes.rs` and `micold-core/src/review/*`: those belong to #1149
  merges, not graded.
- Visual quality, themes and the quickstart Part B pass (T030 unticked; M2 and M3 passes partial).
- `docs/user-guide/settings.md` accuracy; coverage (no tool); performance beyond wall time.
- Per-behavior test-list ids: no list exists.
