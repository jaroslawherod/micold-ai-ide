---
feature: 1554-structured-effort-switch
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset present
verified_at: a486d8a6b # range f2b9dd525..946232f2c; crates/ identical at HEAD
behaviors: 14 # test-bearing tasks (no tdd/test-list.md exists)
proven: 5
likely: 4
test_after: 5 # task groups holding at least one test with no recorded red
no_test: 0
high_smells: 0
criteria_total: 17 # FR-001..FR-014 incl. 010a, SC-001..SC-006 minus FR-014/SC-005 (manual)
criteria_covered: 15
mutation_score: unmeasured # no mutation tool; deliberate mutants not run (see below)
mutants_survived: n/a
suite: 3478 passed, 0 failed (16 targeted binaries, cold build behind the build lock)
---

# TDD Verification: Structured session reasoning effort (#1554)

**Verdict: FAIL.** Test quality is good and every behavior has a test, but 15 daemon tests and 5 window tests
(the review-fix and M2 tests) have no recorded red, which the rubric classes `TEST_AFTER`, and test strength is unmeasured.

Not an independent audit of authorship: the auditor did not write these tests. Every test file cited was re-read.

## Test-first evidence

History cannot corroborate order below milestone level: M1-M3 are one feat commit each (tests and source together),
M4-M6 are squash-merged PRs (#1596, #1599, #1604). So `PROVEN` needs a logged red plus a same-commit test; squashed = `LIKELY`.

| Task group | Class | Evidence |
| --- | --- | --- |
| T002/T003 effort types | PROVEN | cycle 2: 6 named tests red (`todo!()` stubs); `421ce0f8d` |
| T004-T007 commands, control | PROVEN | cycle 3 named reds; cycle 6 red for `9af55f29f` fix |
| T008/T009 notes, saved | PROVEN | cycle 4 (two byte-pin tests passed at red, stated) |
| T011/T012 store, wire, argv | PROVEN | cycle 7: lenient read red; others compile-fail red |
| T014/T015 daemon M2 (`d931a65b9`) | TEST_AFTER (partly) | cycle 8: "written beside the implementation", red shown by after-the-fact mutation for 5 of 15 tests |
| T018/T019 daemon M3 (`1ec82983c`) | PROVEN | cycles 10-11 stubbed reds, named, with the 2 that passed at red said so |
| M2/M3 review-fix tests (`6fe35cace`, `99051554a`, `afc61687b`) | TEST_AFTER | 10 tests, no cycle-log entry (finding 1) |
| T022-T024 picker (`196125f1e`) | LIKELY | cycle 13 compile-fail red; squashed |
| M4 review A tests | TEST_AFTER | 5 tests, no entry (finding 2) |
| T027-T029 `/effort` (`5a109a961`) | LIKELY | cycle 16 reds named; squashed |
| M5 review A1 | LIKELY | cycle 16 |
| T033/T034 (`946232f2c`) | LIKELY | cycle 17 red `left: [low, high, low]`; squashed |
| T001, T005 data/docs/structural (cycles 1, 5, 9, 12, 14) | NOT_APPLICABLE | no behavior |

Existing tests: none weakened, skipped or excluded. Removed: `a_busy_sessions_pick_is_refused_for_now`, the planned M2 interim
test (T014/T018 say it goes). `composer_tab.rs:257` loop 6 to 7 adds the new tab stop; its assertion is unchanged (benign).
Struct literals gained `effort: None` in 12 client/daemon tests (mechanical). Gates, thresholds, filters: untouched.
tasks.md vs. evidence: every ticked task has its tests and they pass. No unticked-but-done tasks.

## Findings

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | 15 daemon tests have no red in cycle-log.md (TEST_AFTER, fail closed): `an_idle_pick_claude_refuses...`, `a_pick_whose_cli_ends_unanswered...`, `a_pick_for_a_session_not_running...`, `a_pick_that_changes_nothing...`, `the_notes_of_quick_picks...`, `a_late_answer_never_replaces...`, `a_terminal_sessions_pick...`, `another_clis_pick...`, `of_picks_from_two_windows...`, `one_sessions_pick_never_touches...`, `an_idle_pick_before_the_turns_end...`, `a_refused_idle_pick_after_a_refused_deferred_one...`, `a_model_change_while_an_idle_pick...`, `a_refused_pick_goes_back_to_a_written_one...`, `a_pick_left_unwritten...` | `crates/micold-daemon/tests/structured_effort.rs:570,593,621,633,649,667,688,698,713,733,1098,1128,1205,1234,1258` |
| 2 | HIGH | 5 window tests added by review A (M4) have no red: in-flight pick, terminal keyboard, three composer tests | `crates/micold-client/tests/features_effort_picker.rs:281,351`; `crates/micold-client/src/shell/composer.rs:599,619,634` |
| 3 | MED | M2 daemon reds (cycle 8) are mutations run after the implementation existed; not test-first, and only 5 of the 15 M2 tests are named | `specs/1554-structured-effort-switch/tdd/cycle-log.md:70-98` |
| 4 | MED | Test strength unmeasured: no mutation tool; deliberate mutants were not run (the sandbox denied scripted source edits, and the caller said not to edit source). Fail-closed: not a pass | n/a |
| 5 | MED | Implementation-coupled: five tests drive `pick_effort`, `finish_effort_pick`, `follow_effort_answer` directly instead of the wire | `structured_effort.rs:667,965,1205,1234,1258` |
| 6 | MED | Tolerant assertion: any number of trailing `None` re-applies passes (`asked[2..].all(is_none)`), so a repeated null write is not caught | `structured_effort.rs:1155,1199` |
| 7 | MED | 15 fixed `thread::sleep` (200-300 ms) for negative assertions, inside async tests (blocks a runtime worker); flake risk (T035 already fixed one race) | `structured_effort.rs:158,882,894,931,1002,1023,1045,1057,1115,1148,1185,1193,1221,1223,1314` |
| 8 | MED | No `tdd/test-list.md`: no behavior ids, `traces` or states; traceability below is rebuilt from tasks.md | `specs/1554-structured-effort-switch/tdd/` |
| 9 | LOW | Comment says the busy case is covered; the test's session is never busy (it is, in `shell/composer.rs:490`) | `crates/micold-client/tests/features_command_palette.rs:622` |
| 10 | LOW | `the_note_says_when_the_model_does_not_take_the_choice` packs 7 assertions, 4 unlabelled (eager, roulette) | `features_effort_picker.rs:310-341` |
| 11 | LOW | `layout_snapshot.txt` regenerated (1959 lines, shared with #1550); no per-feature evidence that the effort rows were reviewed, not just accepted | `crates/micold-client/tests/fixtures/layout_snapshot.txt` |

No HIGH smells: no tautologies, doubled subjects, vacuous or assertion-free tests; assertions are specific and labelled; the stand-in
is a real process, not a double of the unit. Style matches the exemplars (`mod support`, `Fake*`, libtest). Suite: deterministic across this run.

## Mutation results

Not run. No `cargo-mutants` (profile `mutation: null`). Planned deliberate mutants (not executed): `effective_level` ignores support;
`parse_effort_arg` ignores `offered`; `follow_model_answer` skips `reapply_effort` (`state.rs:4235`); `reapply_effort` never holds back (`state.rs:4525`).
Nothing is claimed about survival.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-001/002, US1 s1-2 | `features_effort_picker` (16), layout gate | Window state only; draw is the GUI exception (visual-pass) |
| FR-003, FR-010a | `effort.rs` tables, `structured_effort` 608, 816, 985 | Yes (stand-in) |
| FR-004, SC-001, SC-002 | 530, 871, 916 (1 s bound asserted) | Yes |
| FR-005 | 570, 593, 940, 965, 1128, 1163 | Yes |
| FR-006, FR-011 | 530, 752 | Yes |
| FR-007/008/009, SC-003 | palette 621-691, `composer.rs:490`, daemon 1284, 1336 | Yes (window to wire; daemon backstop) |
| FR-010, SC-004 | 752, 776, 816, `session_store` | Yes |
| FR-012 | 688, 698, `features_effort_picker` 137 | Yes |
| FR-013 | covered by the rows above | Yes |
| SC-006 | whole-suite green | gate |
| FR-014, SC-005 | none automated | manual (user guide, quickstart B) |

Untested: Edge Case "Sandboxed sessions" (plan III justifies it, no test); FR-014 docs and SC-005 two-click path (manual only).
Tests tracing to nothing: none.

## What was not audited

- Mutation and deliberate mutants (finding 4). Coverage: no tool.
- The sandbox acceptance suite (`sandbox_real_*`), the `--release` daemon half, and the full workspace: not run.
- Only 16 targeted binaries ran (core lib and 6 test targets, daemon `structured_effort`/`structured_permission`, client picker/palette/session);
  Windows and macOS legs not run. Other features in the range (1550, 1504, 1400, 1463, 1464) were out of scope.
- Draw code and screenshots (`visual-pass/`) were not judged.
- Performance: no criterion; `structured_effort` ran in 3.8 s.

## Remediation (close unit, after this audit)

The verdict above is the audit's, on 946232f2c. What the close unit did with each finding:

| # | Done |
| --- | --- |
| 1, 2, 3 | Each of the 20 tests shown red after the fact against a one-line mutant of the code it guards (`cycle-log.md` cycle 18), except `one_sessions_pick_never_touches_another`: no one-line mutant crosses sessions. They stay `TEST_AFTER`, as the rubric classes them; the reds show only that each test fails without its behaviour |
| 4 | The four planned mutants run and killed (cycle 18) |
| 5 | Kept: those tests force an interleaving (a late answer, an answer handled out of order) the wire cannot order; noted as a follow-up |
| 6 | `structured_effort.rs`: the M6 test asserts the exact list `[low, high]`; the M3 one accepts exactly `[low, high]` or `[low, high, None]` (the handling order decides; a repeated null write fails). 100 runs each, and the whole binary 40 more, no failure |
| 7 | Kept: each sleep waits for something not to happen; the rig has no event to wait on instead. Noted as a follow-up |
| 8 | Not added: the traceability table above is the record (ledger D22) |
| 9 | Comment corrected: the test's session is idle, the busy case is in `shell::composer` |
| 10 | The four unlabelled assertions labelled |
| 11 | Kept: the visual passes (`visual-pass/`) reviewed the effort rows as drawn |
