---
feature: 1465-back-forward-navigation
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset found
verified_at: 9261978d5
behaviors: 19 # cycle-log cycles 1-18 plus 12b; no tdd/test-list.md exists
proven: 11
likely: 0
test_after: 8
no_test: 0
high_smells: 0
criteria_total: 20 # FR-001..FR-015 and SC-001..SC-005
criteria_covered: 20 # FR-013 only partly, see finding 5
mutation_score: 84 # manual deliberate mutants (no cargo-mutants); 32 of 38 killed; scope core history.rs + client features/history.rs, app.rs, keymap.rs, ui/mod.rs, shell/{history,subscriptions}.rs, changes/keys.rs, runs.rs
mutants_survived: 6 # 2 real gaps (findings 1, 2), 1 unpinned (3), 3 equivalent or redundant
suite: 11805 passed, 6 failed, 18 ignored, 597 binaries, 15m44s cold build (6 failures are unrelated, see Suite)
---

# TDD Verification: Back/forward navigation (#1465)

**Verdict: FAIL.** Two behaviors have surviving mutants that no test catches (Forward passing over the shown place in `History::step`, and the stale session-composer rule in `live_focused_field`), and eight M1 behaviors have no contemporaneous red.

The audit is independent: the auditor did not write this feature. Branch `1465-m5`, range `a3db2cf11^..9261978d5` (design commits through HEAD).

## Suite

`cargo test --workspace --no-fail-fast`: 11805 passed, 6 failed, 18 ignored. The six failures are `micold-core` `settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` (2), and `micold-daemon` `sessions::worktree_delete_blocked_by_an_unremovable_path...`, `settings::a_service_write_over_an_unreadable_settings_file_is_refused`. All six rely on file permissions that the root user of this sandbox ignores (unreadable file, undeletable path). None touches a file the feature changed. They are not feature failures, and no baseline entry in the cycle log covers them (the log has no baseline run of the full suite, see finding 7).

## Test-first evidence

There is no `tdd/test-list.md`; the cycle log is the only per-behavior record. The rubric lists "no test list" under `BLOCKED`, the skill says to audit against `spec.md` and expect a weaker verdict on ordering, which is what was done. Behaviors are the log's cycles.

| Behavior | Class | Evidence |
| --- | --- | --- |
| C1 record | TEST_AFTER | The log's baseline states M1 was written "alongside the code, not run red first". `a3db2cf11` adds `core/history.rs` source and tests in one commit. The red is a stub run recovered later (`TDD_STUB`). It shows the tests can fail, not that they came first. |
| C2 step | TEST_AFTER | Same as C1. |
| C3 observe | TEST_AFTER | Same; `19827d5a6` adds source and tests together. |
| C4 exists | TEST_AFTER | Same. |
| C5 gate | TEST_AFTER | Same. |
| C6 apply | TEST_AFTER | Same. |
| C7 finish a move | TEST_AFTER | Same. |
| C8 side-button decode | TEST_AFTER | Same; `12eb0ff5b` adds `side_button` and its test together. The terminal-pane guard "passed on arrival". |
| C9 Back with nothing shown | PROVEN | Red recorded with the fix reverted; `81b561c95` carries test and fix together. |
| C10 H1 decode and labels | PROVEN | `dcd7a2b86` (log cites `c520924b8`) adds the tests with stubs, then `8456c6baf` the code. |
| C11 routing | PROVEN | Same pair. |
| C12 key gate | PROVEN | Same pair. |
| C12b holding the key | PROVEN | Red observed once the gate was in; test and fix land in `8456c6baf`. |
| C13 guide rows | PROVEN | Test in `dcd7a2b86`, docs in the later commit. |
| C14 Review tab's own Back | PROVEN | `daf96ad25` (log cites `146392a08`) tests first, `ffe11f80d` code. |
| C15 runs.known | PROVEN | `4e7784a41` then `343deee7b`. |
| C16 other-project places | PROVEN | `e6671778a` then `2d438fdb1`. |
| C17 shell availability/lookup | PROVEN | `40a899f64` then `dd6b10896`. |
| C18 Review sheet rows | PROVEN | `d788de35b` then `be9d666d3`. |

The log cites `c520924b8` and `146392a08` for M2 and M3. Those commits exist in the object store but are not ancestors of HEAD; the branch carries rebased copies `dcd7a2b86` and `daf96ad25` with identical subjects. The citations cannot be followed from this branch (finding 8).

### What the change did to tests that already existed

No pre-existing assertion was removed or loosened, no test skipped or filtered, no threshold lowered.

- `tests/overlay_registry.rs` `EXEMPT` grew from 10 to 11 and `tests/root_vocabulary_is_cross_cutting.rs` went from 29 to 30 wrappers: both are inventories that must name the new variant. Legitimate.
- `tests/review_keys.rs` row counts 12 to 14 and 2 to 4: the behavior change the feature specifies (FR-014), with a new test for it.
- `layout_snapshot.txt`: only the two sheet states (`review-shortcut-sheet`, `...-single-keys-off`) changed. Regeneration is the project's mechanism and the diff is confined to those states.
- `main_tests.rs` `mod panes::app_with_sessions` (line 13041) now makes its sessions `Idle`. This changes the precondition of existing pane tests, and no 1465 test uses that helper (finding 6).
- `tests/history.rs`: `2d438fdb1` dropped `assert!(state.runs.compare.is_none())` from a test the feature itself added in `e6671778a`. The cycle log (cycle 16) explains it, and `main_tests.rs::back_to_another_projects_compare_reopens_it_and_forward_returns` asserts it. Not a weakening of a pre-existing test.

## Findings

Ordered by severity.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Surviving mutant in a DONE behavior (C2 step). Forward's pass-over of an entry equal to the shown place is not tested; replacing `at = next` with a Forward no-op passes every suite, and in the real code that mutant is an infinite loop (a UI freeze). Reachable: history `[A,B,A]` at position 0 after two Backs, B deleted, Forward. The Back-side twin is pinned by `an_entry_equal_to_shown_is_passed_over_and_kept` and `removal_merges_neighbouring_equal_entries`. The Forward side has no equal-entry test, and the SC-002 sweep uses distinct entries so it cannot produce the case. | `crates/micold-core/src/history.rs:140-143` (mutant M10); tests `history.rs:312-321, 396-422` |
| 2 | HIGH | Surviving mutant in a DONE behavior (T004/T022, research R8). Making a session composer or queued message count as stale-never (`FieldId::Composer(_) \| FieldId::QueuedMessage(_) => false`) passes the core, lib, `history`, `review_keys` and bin tests. Nothing pins that a composer left focused behind the Review tab does not block Alt+←/→ (US2 AS2 boundary, FR-009), nor that `review_holder` still treats it as Review. Only the `ReviewComposer` arm is tested (`a_key_press_acts_over_a_closed_comment_boxs_stale_focus`). | `crates/micold-client/src/app.rs:1510` (mutant A7); only test `crates/micold-client/tests/history.rs:834-843` |
| 3 | MED | `shown_place` returning the underlying place while Settings is open is not pinned (mutant A3 survives). Spec says Settings is not a place (Assumptions, FR-010). Today the recorded entry would equal the current one, so the effect is nearly invisible, but the documented rule has no assertion. | `crates/micold-client/src/app.rs:2054-2056` |
| 4 | MED | The M1 behaviors C1-C8 were not driven test-first. The log concedes it, and the stub-red recovers test strength only. It cannot be repaired after the fact. The remediation is a recorded waiver, and the loop must not repeat it for later milestones (M2-M5 did follow red-first). | `tdd/cycle-log.md:6-11` |
| 5 | MED | FR-013 is only partly pinned. `the_user_guide_lists_back_and_forward_with_their_keys_and_buttons` checks the two table rows. The required statement that a focused text field or terminal keeps the keys, the Review-tab-first sentence (T026), the pass-over-gone-places sentence (T033) and the Review-sheet sentence (T036) have no test. They are prose tasks, and the spec requires the first of them. | `crates/micold-client/tests/keymap.rs` (guide test, around the end of the Back/Forward block); `docs/user-guide/terminal-panes.md:91-99, 124` |
| 6 | LOW | Pre-existing fixture changed without a feature need: `mod panes::app_with_sessions` sessions made `Idle`. `mod history_tests` has its own near-identical `app_with_sessions` and `idle_at` (duplicated setup). | `crates/micold-client/src/main_tests.rs:13041-13048` and `:16610-16625, 16812-16816` |
| 7 | LOW | Sweeps in the core tests branch on the outcome (`if exists(&current)`, `if let Some(p) = got`), and the nothing-shown sweep asserts nothing for the `None` result beyond the invariants. They are exhaustive over a small domain and the else branch asserts in the second sweep, so the risk is low. The cycle log also has no full-suite baseline entry, so unrelated failures cannot be told apart from the log. | `crates/micold-core/src/history.rs:349-368, 399-421`; `tdd/cycle-log.md` |
| 8 | LOW | Cycle log cites commit hashes not reachable from HEAD (`c520924b8`, `146392a08`); the branch holds rebased copies. | `tdd/cycle-log.md:128, 149, 159, 184` |
| 9 | LOW | `history_rows()` in the review sheet test builds its expected labels with the same `history_key_label` the code uses, so that test cannot catch a wrong label. The labels themselves are pinned in `keymap.rs::the_back_and_forward_labels_name_both_platform_forms`, so the combination holds. Also `let _ = b;` pads a dead binding. | `crates/micold-client/tests/review_keys.rs:1053-1066`; `crates/micold-client/tests/history.rs:376` |
| 10 | INFO | `tasks.md` T037 (quickstart section B visual pass) is unticked, and `evidence/` holds only the four M5 sheet screenshots. The unticked box is honest. It is still an open completion item for the close unit. | `specs/1465-back-forward-navigation/tasks.md:180` |

Smell catalogue pass: no assertion-free, tautological (beyond finding 9), doubled-subject, over-mocked, vacuous, snapshot-approval or skipped test was found in `core/history.rs`, `tests/history.rs`, `tests/features_history.rs`, the 1465 blocks of `tests/keymap.rs`, `app_chord_tests.rs`, `overview_arrow_tests.rs`, the `subscriptions.rs` test, the `terminal_pane.rs` guard, `features_runs.rs` additions, `review_keys.rs` additions, and `main_tests.rs::history_tests`. They use the project's fixtures (`support::sidebar_fixture`, `FakeFolderScanner`, real git repositories in temp dirs) and carry assertion messages as the profile requires. Properties: deterministic, no sleeps, history/features tests run in tens of milliseconds, `main_tests` history block 0.2 s.

## Mutation results

Tool: none installed (`cargo mutants` absent). Method: 38 one-at-a-time deliberate mutants, each applied, run, reverted with `git checkout`, and the tree confirmed clean at the end. Test sets: `micold-core --lib history::`; `micold-client --lib`; client integration targets `history`, `features_history`, `features_runs`, `review_keys`, `keymap`; bin filters `history_tests side_button app_chord overview_arrow`. The core mutants M1-M10 first ran against `micold-core --lib history::` only. The client mutants first ran without the client `--lib` target, so every survivor, plus the `ui` mutants, was re-run with all targets listed (rows marked "re-run"). Coverage tooling is not in the profile, so none was run.

Caught (32): record equal-guard dropped (M1); truncate dropped (M2); cap off by one (M3); shown pass-over disabled (M4); position adjust on removal dropped (M5); merge dropped (M6); Back-with-nothing-shown start (M7); merge position `<=` to `<` (M9); `navigation::blocked` dropped (C1); key gate applied to buttons (C2); terminal dropped from key gate (C3); Review-first for Forward (C4); archived counted as existing (C5); availability ignored (C7); observe ignores `moving` (C8); `finish` does not observe (C9); `moving` never set (C10); worktree lookup ignored (C11); Compare reads active groups (C12); key move keeps terminal focus (A1); no project switch first (A4); no Compare close for Overview (A5); stale `ReviewComposer` live (A6); arrows swapped (K1); Shift allowed (K2); Ctrl allowed (K3); `runs.known` not recorded (R1); availability refresh dropped (S1); app_chord drops the history key (S2, re-run); Overview arrow takes the key (S3, re-run); sheet loses the history rows (S4); side button swapped (S5).

Survived (6):

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `core/history.rs:141` Forward `at = next` removed (M10, re-run with every target) | C2 step, FR-006 | Yes | Real gap, finding 1. In the unmutated code the case is handled; the mutant loops forever. |
| `app.rs:1510` composer/queued staleness `=> false` (A7, re-run) | C12 key gate, T004 | Yes | Real gap, finding 2. |
| `app.rs:2054` Settings guard removed in `shown_place` (A3, re-run) | C3 observe | Yes | Unpinned documented rule, finding 3. Near-equivalent today. |
| `core/history.rs:151` position clamp removed (M8, re-run) | C2 step | Yes | Equivalent. Every path that leaves the position past the end also ends with `found` setting it, or with every removal decrementing it. Defensive code. |
| `features/history.rs:117` `&s.location == location` dropped (C6, re-run) | C4 exists | Yes | Redundant: `location_exists(location)` runs first and `SessionId` is unique. Equivalent unless a session's location can change, which was not verified. |
| `app.rs:2058-2064` Overview and Compare precedence swapped in `shown_place` (A2, re-run) | C3 observe | Yes | Equivalent in reachable states: opening Compare closes the Overview (the test at `tests/history.rs:178-210` records Compare after Overview), so both are never open. |

Sampled behaviors: record, step (both directions), observe/finish/moving, exists (archive, location, availability, worktree lookup, Compare group source), the key and button gates, Review-first, the key decode and its modifier rules, routing, the Review sheet, and runs.known. Not mutated: the terminal-pane side-button guard (production code unchanged, test passes on arrival), the macOS arm of `history_key` (see below), the docs, the quickstart.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-001 one history per window, not persisted | `two_windows_keep_independent_histories`, `features_history::observing_...` | Yes (root `State`) |
| FR-002 every mover records | `a_sidebar_selection_...`, `a_quick_switcher_choice_...`, `a_navigation_chord_step_...`, `the_overview_compare_and_a_review_tab_...`, `the_shown_sessions_removal_...`, `background_sessions_and_catalog_updates_record_nothing` | Yes |
| FR-003 equal adds nothing | core `recording_the_current_entry_adds_nothing`; `re_selecting_the_shown_session_records_nothing` | Yes |
| FR-004 truncate | core `recording_after_back_drops_the_forward_entries`; bin `back_twice_then_a_new_selection...` | Yes (bin through root `update`) |
| FR-005 cap 50 | core `sixty_records_keep_the_newest_fifty` | Unit only, but the reducer is the rule's owner (SC-004) |
| FR-006 back/forward target | `back_and_forward_walk_the_sessions`, `back_reaches_another_projects_session_...`, bin cross-project tests | Yes; Forward over the shown place is untested (finding 1) |
| FR-007 no record while moving, gone entries removed | `finishing_a_move_...`, `an_archived_session_...`, `a_removed_worktrees_places_...`, `another_projects_worktree_missing_...` | Yes |
| FR-008 side buttons | `side_button_maps_...`, `a_side_button_press_in_a_focused_pane_never_reaches_the_process`, `a_button_press_acts_while_a_field_holds_the_keyboard` | Partly. The real iced subscription is not driven; the mapping function is. |
| FR-009 keys and focus rule | `keymap.rs` decode/near-miss tests, `app_chord_tests`, `a_key_press_does_nothing_while_...` (2), stale-focus test | Yes on this platform; macOS arm not compiled here; composer-stale arm untested (finding 2) |
| FR-010 surfaces open | `a_press_does_nothing_over_settings_a_dialog_the_switcher_or_the_review_sheet`, `a_key_press_does_nothing_over_settings` | Yes |
| FR-011 Review tab's own Back | `back_in_a_browsing_review_tab_is_the_tabs_own_back`, `..._without_a_browse_frame_...`, `forward_from_a_browsing_...`, bin `back_in_a_browsed_review_tab_...` | Yes |
| FR-012 once per press/repeat | `the_back_and_forward_keys_press_the_history_from_the_keyboard` (repeat), `three_key_presses_step_back_three_entries` | Yes |
| FR-013 user guide | `the_user_guide_lists_back_and_forward_with_their_keys_and_buttons` | Rows only (finding 5) |
| FR-014 Review sheet | `the_sheet_ends_with_back_and_forward_with_single_keys_on_and_off`, two updated sheet tests, layout snapshot | Yes |
| FR-015 reducer and routing tests | all of the above | Yes |
| SC-001 | `back_and_forward_walk_the_sessions`; keys via `three_key_presses...`; mouse via bin | Linux only; Windows and macOS rely on quickstart section C (T038) |
| SC-002 | core `every_step_lands_on_an_existing_other_entry_or_does_nothing` | Unit sweep |
| SC-003 | `the_terminal_encodes_the_back_and_forward_keys_as_before` and the existing input tests | Yes (Linux) |
| SC-004 | core sweep and cap test | Unit |
| SC-005 | bin `back_in_a_browsed_review_tab_returns_to_the_reading_then_leaves_the_tab` | Yes |

Untested criteria: none. Weakly tested: FR-013's focus-rule sentence, FR-006 Forward pass-over, FR-009's stale composer rule. Tests tracing to nothing: none found; the `known_keeps_each_projects_last_group_list` tests trace to research R5 (US4 across projects).

## What was not audited

- The macOS arm of `keymap::history_key` (`keymap.rs:405-421`) and its tests are `cfg(target_os = "macos")`; this Linux host compiles and mutates only the non-macOS arm. Windows likewise. SC-001 on those platforms rests on quickstart section C.
- The real iced event path for the side buttons (the `listen_with` subscription) and the real terminal widget under a live window: only the pure mapping function and the headless pane harness were exercised.
- The `visual-pass` and quickstart section B results (T037 is open); only the four M5 screenshots exist in `evidence/`.
- Mutation was manual and sampled (38 mutants), not tool-driven. The files `shell/history.rs`, `shell/subscriptions.rs`, `terminal_pane.rs`, `ui/mod.rs` were mutated at one or two sites each.
- Coverage: the profile has no coverage tool; none was run.
- The six sandbox-permission failures (root user) were not investigated beyond confirming that they do not touch feature files.
- The sandbox acceptance suite (`mise run test-sandbox`) and `mise run gate` (fmt, clippy, shell scripts) were not run; only `cargo test --workspace`.
- PR #1574's newer M4 review fixes are not on this branch and were not assessed, as instructed.
- No remediation tasks were appended to `tasks.md`: the audit was asked to leave the tree clean apart from this report. The tasks are listed in the hand-off message instead.

## Re-verification after remediation (close unit, 2026-10-28)

Added by the close unit, which wrote the remediation; the audit above is unchanged and its verdict
was given at `9261978d5`. Remediation tasks T039–T047 in `tasks.md`; details and red runs in the
cycle log's Cycle 20. Each surviving mutant was re-applied on top of the new tests, one at a time,
run and restored (logs in the session scratchpad).

| Finding | Remediation | Mutant re-run | Result |
| --- | --- | --- | --- |
| 1 HIGH | core `forward_passes_over_an_entry_equal_to_shown_and_keeps_it` (bounded by a thread timeout); client `forward_passes_over_the_shown_place_and_on_past_a_gone_one` | M10 | Killed: core test fails at its 10 s bound; client test hangs (killed by `timeout`) |
| 2 HIGH | `tests/history.rs` `a_key_press_acts_over_a_session_fields_stale_focus_behind_the_review_tab`; `tests/review_keys.rs` `a_session_fields_stale_focus_behind_the_tab_leaves_the_keyboard_to_the_review` | A7 | Killed by both |
| 3 MED | `the_shown_place_is_none_while_settings_is_open` | A3 | Killed |
| 4 MED | Waiver recorded in the cycle log (M1 cycles 1–8 test-after; not repairable) | — | Waived |
| 5 MED | Guide test checks the focus-rule, Review-tab-first, passed-over and Review-sheet sentences | each sentence altered in turn | Killed (4 of 4) |
| 6 LOW | `mod panes::app_with_sessions` restored; no test needed `Idle` | — | Reverted; `panes::` and `history_tests` pass |
| 7 LOW | Core sweeps assert an independently computed outcome (entry, position, kept entries); full-suite baseline recorded in the cycle log | — | Done |
| 8 LOW | Cycle-log citations rewritten to the commits merged to `main` (all reachable) | — | Done |
| 9 LOW | `history_rows()` writes the labels out; `let _ = b;` replaced by an entries assertion | `history_key_label` takes the other platform's label | Killed (2 sheet tests) |
| 10 INFO | T037 quickstart §B GUI pass run: PASS (B12 covered by M5's pass); evidence `evidence/qs-B*.png` | — | Done |

Survivors left: M8, C6 and A2, judged equivalent or redundant above. M8 was re-run and still
survives, as expected. With findings 1–3 killed, 5 and 9 pinned, and 4 waived, no HIGH finding
remains open; the test-after status of C1–C8 stands, under the waiver.
