---
feature: 941-structured-prompt-input
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: bc69abc0 # short SHA audited
behaviors: 13 # no test-list.md: cycles of tdd/cycle-log.md stand in
proven: 4
likely: 6
test_after: 3
no_test: 0
high_smells: 2
criteria_total: 39 # 31 acceptance scenarios + 8 success criteria
criteria_covered: 38 # SC-007 only by an #[ignore]d test
mutation_score: unmeasured # no cargo-mutants; 8 deliberate mutants by hand, 7 caught, 1 survived (M8)
mutants_survived: 1
suite: 10123 passed, 6 failed (root-only unreadable-file tests, not findings), 18 ignored, 16m14s
---

# TDD Verification: Structured prompt input (941)

**Verdict: FAIL.** Three cycles (9, 10, 11) are recorded as test-after, a deliberate mutant inside a
shipped behavior survived (M8), the SC-007 timing test is `#[ignore]`d with nothing running it, and the
feature has no `tdd/test-list.md`. Where tests exist they are mostly strong: the daemon tests drive the
real `DaemonState` and a stand-in CLI process, and the client gates measure the drawn window.

Independence: this audit was not written by the session that wrote the tests (cold read, no
subagent available; the smell pass was done by the auditor directly, a sample not every file).

## Test-first evidence

No test list exists, so the behaviors are the 13 cycles of the cycle log. Git cannot corroborate order:
every `feat(941)` commit adds tests and source together (`c560e68f`, `371e0fad`, `4e3cb2d1`, `4a2af812`,
`61614823`, `6d7e54e8`, `483f911d`, `490a968a`), so no cycle can be `PROVEN` by history alone.

| Cycle | Class | Evidence |
| --- | --- | --- |
| 1 capture modes | PROVEN | red command and count quoted (`17 cases, 6 failed`) |
| 2 core API | LIKELY | red is "did not compile"; mutants after green; no output quoted |
| 3 daemon until M2 | LIKELY | compile failure only, prose |
| 4 stand-in, delivery, queue | LIKELY | compile failure, then a mutant after green |
| 5 interrupt, overdue | LIKELY | compile failure, then a mutant after green |
| 6 saved queue | LIKELY | compile failure, then a mutant after green |
| 7 interrupt after its message | PROVEN | failure output quoted |
| 8 composer state | LIKELY | log says red "recorded at M4 verify" from a mutant set on finished code |
| 9 composer parts | TEST_AFTER | gate/showcase tests red first, but component unit tests "written with each component (test-after, recorded)" (cycle-log.md:157) |
| 10 composer view | TEST_AFTER | "order: test-after, recorded" (cycle-log.md:184) |
| 11 focus fix | TEST_AFTER | "test-after (the fix and the tests in one sitting)" (cycle-log.md:209) |
| 12 text area keeps place | PROVEN | red on old layout quoted (`Save` not drawn) |
| 13 withdraw, Tab | PROVEN | red output quoted ("focused field None"); T063 red shown by a mutant |

### Tests that already existed

Diffed every `(941)` commit over tests and test-bearing source. No assertion was loosened to match
code, none skipped, no filter or threshold touched. Items to know:

- `371e0fad` `daemon/tests/structured_session.rs:~734-960`: #938's `TurnInProgress` refusals and the
  M1 "Not available yet" test replaced. Planned (plan "Superseded assertions", T026), with the new
  behavior tested in `structured_input.rs`. Legitimate.
- `c560e68f` `core/src/structured/turn.rs` tests: the old `accept(text, true)` / `InTurn` tests were
  replaced by `TurnId` ones (T007). Legitimate.
- `8288205f` `core/../composer_large_draft.rs:~47`: `assert!(pasted < BUDGET, ...)` removed so the
  setup paste is not held to 100 ms. Documented; see finding 4.
- `5447d8fd`: protocol version assertions 62 to 60 (bump) follow the code. Legitimate.
- Layout fixture re-recorded in cycles 10, 12, 13 (cycle log). Fixture diffs not reviewed here.

Tasks: T001-T061 and T063 are ticked and each has a cycle or review behind it. T062 (quickstart Part A
on merged `main`) is unticked and not behavioral. No ticked task contradicts the log.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Survivor M8: `turn_closed` ignores whether the ended process is the current one. Replacing the guard with `true` leaves all 19 `structured_input` tests green. The comment promises a late end of an earlier process leaves a running later process's queue alone; no test pins it (FR-022, US4 AS4). | `crates/micold-daemon/src/state.rs:7697-7699` |
| 2 | HIGH | Three cycles are test-after: component unit tests (text_area, attachment_chip, queue_row, busy_indicator), the composer view gate, and the focus fix. Cycle 8's red is from mutants on finished code. | `tdd/cycle-log.md:157-161,184-191,209-211,129-133` |
| 3 | HIGH | Vacuous assertions on user-facing sentences: `unavailable(..).is_some()` and `notice.is_some()` / `overdue_notice().is_some()` assert presence where the rule is which sentence (FR-005, FR-006, FR-015). Mutant M2 (swap `NOT_CONNECTED` and `NOT_RUNNING`) passes all 29 `features_composer` tests; only the unrelated `layout_snapshot` fixture caught it. Should assert `Some(NOT_CONNECTED)` / `Some(NOT_RUNNING)` / `Some(OVERDUE)`. | `crates/micold-client/tests/features_composer.rs:380,388,490,578` |
| 4 | HIGH | The only SC-007 test is `#[ignore]` in the committed state and nothing runs it: not in `.github`, `scripts`, `mise.toml` or `docs`. The 100 ms budget has no gate. Its paste step is also no longer asserted. | `crates/micold-client/tests/composer_large_draft.rs:40`; commit `8288205f` |
| 5 | HIGH | No `tdd/test-list.md`: no behavior ids, no `traces`, so "each criterion has a test" rests on this audit's own name mapping. | `specs/941-structured-prompt-input/tdd/` |
| 6 | MED | History cannot show order (tests and source share each commit) and most reds are prose ("did not compile"), not command and output. | `tdd/cycle-log.md:28-30,45-47,62-65,83-84,97-98` |
| 7 | MED | Real clock: fixed 300 ms sleeps for "nothing is delivered" and wall bounds (1 s, 3 s) with real time; flake-prone on a loaded host. | `crates/micold-daemon/tests/structured_input.rs:311,1241,1046`; `structured_input_saved.rs:251,287,334,408` |
| 8 | MED | No end-to-end test through the real CLI or the sandbox (`sandbox_real_*`) for any 941 criterion; the CLI is a stand-in. No test joins the client's composer to the daemon over the wire. Interrupt and picture shapes rest on the M1 recorded fixtures. | `crates/micold-daemon/tests/support/structured.rs` |
| 9 | MED | Eager tests: the 20-interrupt loop and the six-case send test each pin several behaviors. The send test compares the stdin line to `encode_user_message` itself; independent pins of that encoding were not checked. | `crates/micold-daemon/tests/structured_input.rs:399-460,1028-1100` |
| 10 | LOW | M5 (`restored` sets `hold=false`) is caught by core `structured_input` but passes the daemon `structured_input_saved` suite: the daemon test does not pin the hold on its own. | `crates/micold-core/src/structured/input.rs:134` |

## Mutation results

No mutation tool in the profile. Deliberate mutants only: 8, one at a time, on changed files, each
restored with `git checkout` and `git diff` confirmed empty. Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| M1 `composer.rs:130` drop `trim()` in `Draft::sendable` | FR-004 | No | `can_send_needs_a_sendable_draft...` failed |
| M2 `composer.rs:492-494` swap the two unavailable sentences | FR-005/006 | Yes, in `features_composer`; caught only by `layout_snapshot` | Finding 3 |
| M3 `composer.rs history_forward` restores an empty draft | FR-024 | No | `stepping_forward_past_the_newest...` failed |
| M4 `turn.rs:101` an asked interrupt writes again | FR-016 | No | `an_interrupt_of_the_open_turn_is_written_once` failed |
| M5 `input.rs:134` restored queue not on hold | FR-022 | No | core `a_restored_queue_is_held...` failed (finding 10) |
| M6 `attachment.rs:192` `>=` to `>` | FR-029 | No | 3 unit tests failed |
| M7 `attachment.rs:195` `>` to `>=` | FR-029 | No | `pictures_over_the_message_total_are_refused` failed |
| M8 `state.rs:7697` stale-process guard to `true` | FR-022 | Yes | Not equivalent in intent; finding 1 |

After restore: core `--lib structured` 153 passed; daemon `structured_input` 19, `structured_input_saved`
5, `structured_session` 19; client `features_composer` 29, `layout_snapshot` 98, `composer_tab` 2. The
core `structured_input` integration target was filtered out by the name filter in that confirmation run
(source diff is empty; it ran 25 passed, 1 failed under M5 only), so rerun it.

## Traceability

Mapped by test name; criteria text read, tests not all opened. "E2E" means through the daemon's real
state and a stand-in process, or the window's real view tree.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 AS1-3, FR-003, FR-009, SC-001/002 | daemon `a_send_to_an_idle_session...`, `every_window_sees_busy_then_idle...`; `composer_fits::busy_and_idle...` | Yes (stand-in) |
| US1 AS4-6, FR-004/005/006, SC-005 | core `a_blank_message...`; `features_composer` can_send, refused, lost-connection tests | State only; sentence unpinned (finding 3) |
| US1 AS7, SC-008 | `a_terminal_session_has_no_composer`, `only_a_pane_showing_a_structured_primary...`, `pane_drop_target` | Yes |
| US2 AS1-7, FR-002/007/008/023-025, SC-006 | `features_composer` keys, history, per-session drafts; `composer_fits` growth | Yes (window view) |
| SC-007 | `composer_large_draft` | Not run: ignored (finding 4) |
| US3 AS1-5, FR-012-017, SC-003 | daemon interrupt tests (5), `escape_interrupts...`, `composer_fits` notices | Yes (stand-in) |
| US3 AS6 | `an_interrupt_withdraws_the_turns_pending_permission_request` | Yes |
| US4 AS1-6, FR-018-022, FR-032, SC-004 | daemon queue, edit, hold, saved-queue tests; core scenario table | Yes; stale-process hold unpinned (finding 1) |
| US5 AS1-7, FR-026-030 | `features_composer` drops, `attachment` unit tests, daemon picture sends, `composer_fits` chips and transcript | Yes (stand-in) |
| FR-033 | user guide edited | No test; not audited |

Untested criteria: none by name except SC-007 as noted. Tests tracing to nothing: none found in the
files read.

## What was not audited

- Every smell was checked only in `structured_input` (daemon and core), `structured_input_saved`,
  `features_composer`, `composer_fits`, `composer_tab`, `composer_large_draft`; component unit tests
  (`text_area`, `attachment_chip`, `queue_row`, `busy_indicator`), `structured_wire`,
  `terminal_history_*`, `pane_drop_target` were mapped, not read line by line.
- Mutation: 8 hand mutants, no tool, no score; core `structured_input` not re-run green after restore.
- Coverage: no tool.
- Re-recorded layout fixtures were not diffed or judged.
- The acceptance runtime suite (`sandbox_real_*`) was not run; no image was built.
- Windows and macOS legs, performance beyond the suite's wall time, and the user guide text (FR-033).
- Six suite failures are root-only unreadable-file tests (a directory that cannot be emptied, refused
  writes over unreadable files, leftover report cap, worktree delete blocked); the two named flakes did
  not fail this run.
