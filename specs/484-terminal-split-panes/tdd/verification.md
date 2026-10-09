---
feature: 484-terminal-split-panes
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: a0f059ec # short SHA audited (diff base 909f4b23)
behaviors: 15 # test tasks T002 T004 T006 T006a T008 T010 T012 T015 T018 T021 T024 T026 T029 T031 T033 (no tdd/test-list.md exists)
proven: 0
likely: 0
test_after: 15 # no red recorded anywhere; every commit lands tests and source together
no_test: 0
high_smells: 0 # in the files read; see "What was not audited"
criteria_total: 25 # user-story acceptance scenarios
criteria_covered: 23 # 2 only partly (US2-5, US5-3)
mutation_score: unmeasured # quick mode, no mutation tool in the profile
mutants_survived: unmeasured
suite: 5922 passed, 6 failed (all unrelated, run as root), 9 ignored, 350s
---

# TDD Verification: Terminal split panes (484)

**Verdict: FAIL.** There is no test-first evidence for any behavior (no `tdd/` directory, no red recorded; the 15 feature commits each add tests and source together), and two acceptance scenarios have no test that checks what they say.

The tests themselves are good. No weakened existing test, no skipped test, and no HIGH smell in the files read. The FAIL comes from the rubric's fail-closed rules: no red recorded means `TEST_AFTER`, and an acceptance criterion with no test fails the feature. Neither finding says the code is wrong.

Independence: this audit did not write the tests. The feature's autopilot run did, in another session; this session read every cited file cold.

## Suite

`CARGO_INCREMENTAL=0 scripts/build-lock.sh cargo test --workspace --no-fail-fast` at `a0f059ec`: 5922 passed, 6 failed, 9 ignored, 460 binaries, 5m50s. The 6 failures are all permission tests that need a file the process cannot read, and this audit ran as uid 0, which ignores the mode bits. None touches panes:

- `a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched`
- `a_service_write_over_an_unreadable_settings_file_is_refused`
- `a_refused_write_is_logged_as_a_failure_with_the_reason`
- `a_directory_that_cannot_be_emptied_names_what_survived`
- `the_leftover_report_is_capped`
- `worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports`

Every pane test passed (about 250 tests match pane, split, divider, focus_dir or chord). The profile baseline was 5071 passed over 429 binaries; the feature adds roughly 850 tests.

## Test-first evidence

No cycle log and no test list exist. Evidence is git history only: `8020fe3b`, `1f18a742`, `cf876609`, `548a5e6a`, `f80776c0`, `f620ad0d` each change tests and source together, and no commit holds tests alone. The review-fix commits (`0477e900`, `dd9dd6f3`, `733dde40`) change both sides again. `tasks.md` requires "seen to fail for the right reason", and `autopilot.md:77` claims one test "fails with the fix mutated away", but neither records a red command or output. That is self-report with no output.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T002 core layout ops | TEST_AFTER | tests in `pane_layout.rs` added with source in `8020fe3b`, no red recorded |
| T004 wire change 36 to 37 | TEST_AFTER | `protocol_roundtrip.rs`, `schema_hash.rs` change with `messages.rs` in `8020fe3b` |
| T006 / T006a daemon multi-attach, isolation | TEST_AFTER | `pane_terminals.rs` added with `state.rs`/`server.rs` in `8020fe3b` |
| T008 client grids per terminal, FR-016/017 | TEST_AFTER | `main_tests.rs` with `shell/panes.rs` in `1f18a742` |
| T010 / T012 SplitView, split flow | TEST_AFTER | `cf876609` |
| T015 focus mark and key routing | TEST_AFTER | `cf876609`, `0477e900` |
| T018 per-pane resize | TEST_AFTER | `1f18a742`, `cf876609` |
| T021 chords | TEST_AFTER | `548a5e6a` |
| T024 / T026 close, swap, drag | TEST_AFTER | `f80776c0` |
| T029 / T031 / T033 persistence | TEST_AFTER | `f620ad0d` |

No behavior is `NO_TEST`: every test task has tests. The class is "no red recorded", not "source provably first". If the autopilot transcript or unit logs hold the reds, copy them into a `tdd/cycle-log.md` and these become `LIKELY`.

### Existing tests

Diffed `909f4b23..HEAD` for removed or changed lines in `crates/*/tests/*` and `main_tests.rs`. Nothing is weakened. Every removed line is an API migration:

- the `grids` key changes from `SessionId` to `TerminalRef` (`main_tests.rs`, `app_at_the_tail`);
- the new `process` field is added to `SessionInput`, `SessionResize` and `GridFrame` (`protocol_roundtrip.rs`);
- `Icon::ALL.len()` goes from 47 to 49 for the two new icons, with the `expected()` arms added (`icons.rs:35-40, 93`);
- `FEATURE_026_PROTOCOL_VERSION` 36 to 37 (`schema_hash.rs`).

No `#[ignore]`, no config exclusion, no threshold change. `tasks.md` has every behavioral task ticked and a test present for each. T036 and T037 are unticked, which is accurate: the quickstart pass and the idle-CPU probe are not recorded.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (blocking) | No test-first evidence for any of the 15 behaviors. Fail closed: `TEST_AFTER`. | no `tdd/cycle-log.md`; commits listed above |
| 2 | HIGH (blocking) | US5-3 has no test. "A closed or replaced terminal keeps its last size until shown again, then resizes to the pane that shows it" is claimed by the doc comment on `a_terminal_not_shown_is_not_resized`, but the body only checks that resizing pane 1 sends one `SessionResize` for session 0. It never replaces a terminal and never re-shows it. | `main_tests.rs:10200-10219` |
| 3 | HIGH (blocking) | US2-5 is only half tested. "A dialog, menu or text field takes the keyboard ... returns to the same pane" has no test. `focus_returns_to_the_same_pane` covers only window focus release and regain via `TerminalFocusReleased`/`TerminalFocused`, which the comment itself says is the same mechanism. | `main_tests.rs:10153-10163` |
| 4 | MED | The daemon cap of 6 viewed terminals (`MAX_VIEWED`, FR-001 / T006 "at most 6 entries") is untested. `take(MAX_VIEWED)` could be removed and nothing fails. | `server.rs:1126`, `server.rs:2416`; no 7-entry case in `pane_terminals.rs` |
| 5 | MED | The visible reason for the minimum-size refusal is tested only at the core level (`Refusal::TooSmall`). The client test covers only the 7th-split message. T021 requires "a refused action shows a visible reason". | `pane_layout.rs:1034`; `main_tests.rs:10014` |
| 6 | MED | The shadowing test pins what the spec forbids. FR-009 says pane chords MUST NOT shadow a chord feature 006's key map forwards. `pane_chords_shadow_the_old_forwarded_encoding` states that `Ctrl+Shift+D` was EOF (0x04) and `Ctrl+Shift+Arrow` a modified cursor key, and asserts the pane action wins. Research R6 does not record this as an accepted deviation. Needs a spec decision, not a test fix. | `crates/micold-client/tests/keymap.rs` (added test); `spec.md` FR-009 |
| 7 | MED | `no_forwarded_chord_is_a_pane_chord` cannot fail in practice. Its modifier set contains no Shift+Ctrl or Shift+Cmd combination, and every pane chord needs one, so the check holds by construction. T021 asked for an enumeration of feature 006's forwarded chords. The Ctrl+Shift letters that matter are never exercised. | `keymap.rs` tests, `mods` array |
| 8 | MED | Redundant test: `pane_chords_shadow_the_old_forwarded_encoding` repeats the first assertion of `pane_chords_are_app_actions_never_bytes` (same inputs, same expected output). | `keymap.rs` tests, added at end |
| 9 | MED | `an_exited_terminal_shows_its_state_in_its_pane_only` asserts `terminal_status(...) == "exited"` on a helper. It does not check that the pane shows it, or that another pane's rendering is unchanged. The "other panes unaffected" half is `assert_ne` against a different terminal. | `main_tests.rs:10088-10107` |
| 10 | MED | No acceptance test through the real runtime. No `sandbox_real_*` test touches panes, so US5-1 ("`stty size` in each pane") is verified only by asserting PTY grid dimensions after `resize_terminal` on `cat` stand-ins, and the GUI path only through `update()`. The profile's acceptance command was not run. | `pane_terminals.rs:207-212` |
| 11 | MED | Timing-window assertions: `frames_within(.., 300..800 ms)` with negative checks (`is_empty()`, "no frames from X") is the only way to prove absence, but it is a fixed sleep and can false-green under load. | `pane_terminals.rs:266-283, 342-355` |
| 12 | LOW | Assertion messages: the profile asks for a message stating the rule on every assertion. Most of `pane_layout.rs` tests (`:946-1241`) and the keymap tests have bare `assert_eq!`. | `pane_layout.rs:946-1241` |
| 13 | LOW | `invariants()` builds `d` (a clone of `shown`) and dead-drops it. | `pane_layout.rs:929-934` |
| 14 | LOW | Unclear name: `a_terminal_not_shown_is_not_resized` says one thing and checks another (see finding 2). | `main_tests.rs:10202` |
| 15 | LOW | "Rebindable where other shortcuts are" (spec edge case, T021/T022) is untested. Research R6 says "through the same mechanism as existing chords", but `keymap.rs` has no such mechanism visible, so this may be vacuous. | `research.md:27`; `keymap.rs:221-247` |
| 16 | NOTE | FR-015 and SC-004 (idle CPU, 6 panes, +10%) have no test by design (manual) and the manual probe (T036) is not recorded. | `tasks.md:103-104` |

Smell catalogue results for the files read:

- **Foreign style, bypassed utility, redundant test:** none beyond finding 8. New tests use `app_with_sessions`, `support::*` and `DrivenTerm`-style helpers consistently with the exemplars.
- **Conditional logic in a test:** the table-driven loops (`an_unusable_layout_degrades_to_none...`, `no_forwarded_chord...`, `a_key_reaches_only...`) always assert inside the loop. Fine. The `while l.len() < 4` setup loop in `pane_layout.rs:1111` is setup only.
- **Self-approving snapshot:** none. This diff adds no fixture or snapshot files.
- **Eager test:** `an_unusable_layout_degrades_to_none...` packs 10 cases, but each case carries a name in its message, so a failure says which. Acceptable.
- **Properties:** the property-style tests (`random_operation_sequences...`, `rects_tile...`) use a seeded LCG, so they are deterministic. The core tests are fast. The daemon tests spawn real `cat` PTYs with 300 to 1000 ms windows, so they are the slow ones.

## Mutation results

Not run (`quick`). The profile has no mutation tool, so deliberate mutants were also skipped as instructed. Test strength is unmeasured. Mutation survival cannot be ruled out for any behavior. Candidate mutants for a later run, by risk:

- keep the `take(MAX_VIEWED)` guard (finding 4);
- invert the focus check in the unfocused-press branch (`terminal_pane.rs:1285`);
- drop `Release` coalescing in `a_divider_drag...`;
- return `Some` for a duplicate terminal in `show`.

## Traceability

The test list is absent, so claims come from `tasks.md` and test comments. "E2E" below means the daemon boundary or `update()`, since no GUI/real-runtime test exists.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1-1, 1-2 split, new pane | `split_adds_a_pane_after_and_focuses_it`, `a_split_then_one_choice...`, `a_new_pane_opens_on_an_unshown_terminal` | Yes (`update()`) |
| US1-3 both live | `set_viewed_terminals_streams_exactly...`, `a_frame_updates_only_its_own_terminals_grid` | Yes (daemon) |
| US1-4 pick any terminal | `selecting_a_terminal_calls_show_or_focus`, `selecting_a_shown_terminal_focuses_its_pane` | Yes |
| US2-1 key to one pane | `a_key_reaches_only_the_focused_panes_terminal` (2 to 6 panes) | Yes |
| US2-2 press in unfocused pane | `terminal_pane.rs` widget test (press captured, publishes `FocusPane`) | Widget level |
| US2-3 focus mark | `ui/panes.rs:300`, both themes | Geometry |
| US2-4 window refocus | `focus_returns_to_the_same_pane` | Yes |
| US2-5 dialog returns focus | none | **No** (finding 3) |
| US3-1, 3-2, 3-3, 3-4 | `pane_chords_split_and_move_focus`, `the_close_chord_closes_the_focused_pane`, `focus_dir_...`, `keymap.rs` tests | Yes (weak, findings 6 to 8) |
| US4-1 drag, minimum | `dragging_a_divider...`, `a_drag_never_shrinks...`, `set_ratio_is_clamped...` | Yes |
| US4-2 close keeps session | `closing_a_pane_leaves_every_session_running` | Yes |
| US4-3 last pane refused | `the_last_pane_cannot_be_closed_and_says_why` | Yes |
| US4-4 swap | `swapping_terminals_sends_no_session_lifecycle_message`, `a_header_dropped_on_another_pane_swaps_them` | Yes |
| US4-5 double press | `a_double_press_on_the_divider_restores_equal_sizes` | Yes |
| US5-1 own size | `each_pane_sizes_its_own_terminal`, `input_and_resize_with_a_process...` | Daemon PTY size, no real `stty` (finding 10) |
| US5-2 coalesced, final sent | `a_divider_drag_sends_pane_sizes_once_on_release` | Yes |
| US5-3 keeps size until shown | none | **No** (finding 2) |
| US6-1 restore | `the_layout_is_restored_from_the_snapshot_on_connect`, `a_layout_round_trips...` | Yes |
| US6-2 per project | `each_project_restores_its_own_layout`, `layouts_are_per_project` | Yes |
| US6-3 gone terminal | `a_gone_terminal_becomes_an_empty_pane_on_restore`, `terminals_that_no_longer_resolve_are_kept` | Yes |
| US6-4 corrupt or newer | `an_unusable_layout_degrades_to_none...`, `no_stored_layout_and_a_corrupt_one_both_give_one_pane` | Store and client; no full-start test |

Tests tracing to nothing: none found. `icons.rs` and `schema_hash.rs` changes trace to the new icons and the wire bump. FR-017 (single-pane behaviour unchanged) is covered by `a_project_with_one_pane_sends_no_viewed_terminals` plus the whole pre-existing suite staying green.

## What was not audited

- Mutation and deliberate mutants: skipped (`quick`). Test strength is unmeasured.
- Coverage: the profile has none.
- Not read line by line: the bodies of most `main_tests.rs` additions beyond the ones cited, the `split_view.rs` gesture tests (`:600-720`), `ui/panes.rs:294+`, `pane_layout_store.rs` in the daemon, and the `terminal_pane.rs` widget tests other than the press test. The "no HIGH smell" count applies to what was read.
- The acceptance suite (`sandbox_real_*`) and the release daemon build: not run.
- The six unrelated failing tests were not re-run as a non-root user.
- Idle CPU (FR-015, SC-004) and the visual pass: no test, and T036 is not recorded.
- Performance and flakiness: the suite ran once; no repeat runs.
- The scratchpad logs under the session directory belong to another feature (runs/483) and were not used as evidence.
