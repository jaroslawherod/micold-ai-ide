---
feature: 938-structured-session-mode
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: a47882cc # short SHA audited
behaviors: 23 # 14 on the test list (U1-U14) + 9 in the cycle log only (U15-U23)
proven: 12
likely: 1
test_after: 10 # U13, U14 (2 of 7 cases), U16-U19, U20-U23
no_test: 0
high_smells: 3
criteria_total: 37 # 30 FRs (incl. 006a, 021a-c, 022a) + 7 SCs
criteria_covered: 36 # FR-023's user-guide half has no automated check
mutation_score: unmeasured # no mutation tool in the profile; 8 deliberate mutants, 7 caught, 1 survived
mutants_survived: 1 # client log bound, not equivalent
suite: feature-scoped only, 304 passed, 0 failed on the first run, ~160 s incl. build (full workspace not run, see end); 1 flaky test, 2 failures in about 16 runs
---

# TDD Verification: 938 structured session mode

**Verdict: FAIL.** A gate test was weakened to pass (`structured_row_marker_fits.rs` now returns early instead of asserting), the M7 and M8 behaviors were written test-after, and a reducer test asserts nothing a mutant cannot satisfy.

The audit was run from a fresh context and was independent: no test in this feature was written in this session. Every cited test file was re-read.

## Test-first evidence

Stage that failed: question 1 (did the tests come first?) and question 2 (do they assert behavior?). Tests and coverage of the criteria are otherwise strong.

| Behavior | Class | Evidence |
| --- | --- | --- |
| U1 capture script scrub | PROVEN | cycle 1 red `7 cases, 7 failed`; `e633c09d` adds test and script together |
| U2-U5 kind, args, splitter, parser | PROVEN | cycle 2 red recorded against stubs; `e633c09d` (tests + stubs) precedes `f0003fa1` (feat) |
| U6, U7 history, wire delta | PROVEN | cycle 4 red recorded (stubbed `append`, `missing field kind`); tests and source share `9bf45832` |
| U8, U9 daemon session, lifecycle | PROVEN | cycle 5 red; `92c2fcbf` (tests, red) precedes `b86b64c5`/`af5aae79` |
| U10, U11 turn, user message, wire | PROVEN | cycle 7 red; `44141a90` (tests) precedes `db6ed335` |
| U12 daemon user messages, first prompt | LIKELY | cycle 8 red is "does not compile"; tests and source land together in `724b23d8`; the right-reason red was taken afterwards by mutation |
| U13 resume | TEST_AFTER | cycle 9 says "red: none observed ... Recorded as test-after"; `39bd39c5` lands after the code it covers. Three deliberate mutants were run, all caught |
| U14 late and multiple viewers | TEST_AFTER (2 of 7 cases) | `a_late_viewer_gets_exactly_the_history_then_live_events` and `two_viewers_both_receive_each_new_event` "passed at once"; 5 other cases have a red recorded. Mutants recorded and caught |
| U15 refused terminal-only requests (M6, not on list) | PROVEN | `2ee23b2b` tests red before `643a8326`; `the_same_requests_to_its_shell_instance_are_served` is test-after |
| U16-U19 saved event history (M7, not on list) | TEST_AFTER | cycle 12: "the tests were written with the code, so red was shown afterwards" by stubbing `saved.rs` after the fact; `c4c2e17e` has source and in-file tests together. The daemon file (`e3aeafdd`, 9 of 10 red against the unchanged daemon) and the store file (`621d7c25`, compile red) are LIKELY at best |
| U20-U23 UI entry, marker, log, settings note (M8, not on list) | TEST_AFTER | cycle 13: "written with the code, so red is shown afterwards, by mutation"; history confirms: feat `68b663aa`, then `7d74ec9a` "M8 tests follow the structured entry". `log_line.rs` (`2ed73160`) has in-file tests in the same commit as the code, no red |

Cycle log versus history: no log entry claims a red the history contradicts. The log is candid about every test-after case. The weakness is the test list, not the honesty of the log.

### Existing tests changed by the feature

| File:line | Before | After | Judgment |
| --- | --- | --- | --- |
| `crates/micold-client/tests/gates/structured_row_marker_fits.rs:69-73` (commit `7d74ec9a`) | `assert!(!changed.is_empty(), "marking a session structured changed no geometry: the marker is not drawn, or this gate reads the wrong state")` | `if changed.is_empty() { return; }` | WEAKENED. The gate now passes when the marker is not drawn at all, which is the failure its own message described. See finding 1 |
| `crates/micold-client/tests/terminal_bar_stability.rs:295` | `tab.contains("session_provider(state, id).provider().command()")` | `tab.contains("session_provider(state, id)") && tab.contains("command_caption(")` | Source-text pin loosened; the message still says "by `command()`". See finding 4 |
| `crates/micold-client/tests/terminal_tabs.rs:295` | `ai.contains(".provider().command()")` | `ai.contains("command_caption(") && ai.contains("session_provider(")` | Same loosening. See finding 4 |
| `crates/micold-client/tests/fixtures/layout_snapshot.txt:8258-8259` | row width `588.2` / `556.2` | `865.2` / `833.2` | Committed snapshot regenerated in `7d74ec9a`. The change is one row, consistent with the longer caption, but no test states why. See finding 5 |
| `main_tests.rs` (`offers_a_choice` helper, entry list), `directory_availability.rs`, `features_session.rs` | `AiCli::ALL` entries; chevron only with 2 CLIs | one extra Claude Code structured entry; chevron for Claude alone | Legitimate behavior change (FR-022), each has a new test that pins the new value. `offers_a_choice` changes meaning from the product rule to a local `known_clis(..).len() >= 2`, and its comment says so. Not a finding. Note SC-004 says existing terminal tests pass "unchanged"; these are the intended exceptions |
| `schema_hash.rs` `FEATURE_026_PROTOCOL_VERSION` 51 to 54 (3 commits) | pinned version | follows `PROTOCOL_VERSION` | Intended; the pin exists to force this edit |
| `fixtures/stream-json/tool-turn.jsonl`, `error-result.jsonl` (`766e4196`) | split tool-input deltas unscrubbed | scrubbed | Secret scrub of recorded data, not a weakening |

`tasks.md` against the test list: 55 of 56 tasks ticked; T048 (close unit, no code) is correctly open. The list stops at U14 and has no rows for M6, M7 and M8 (T033-T036, T037-T047, T049-T056, 31 ticked tasks). Their evidence exists only in the cycle log. See finding 3.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Gate weakened, now conditional and assertion-free in the case it exists to catch. If the marker is not drawn or the gate reads the wrong state, `changed` is empty and the test returns green. Should: assert `changed` is non-empty (the caption does differ, the snapshot row moved by 277 px), or drive a width where it provably moves. | `structured_row_marker_fits.rs:69-73`, commit `7d74ec9a` |
| 2 | HIGH | Vacuous assertion: `rows[0].contains('a') && rows[1].contains('b')`. The line format is `"unreadable line: <text>"` (`log_line.rs:58`), and "unreadable" contains both letters, so the assertion holds for any two rows. A reversed or swapped log passes this test (other tests catch it by accident). Should: assert `rows == [log_line(&event(0,"a")), log_line(&event(1,"b"))]` or the full strings. | `features_structured_log.rs:26` |
| 3 | HIGH | Test list is stale, so ticked tasks have no list evidence. U15-U23 (M6, M7, M8: `SessionRequestRefused`, saved history, entry, log pane, settings note) exist only in the cycle log, `test-list.md` header still says "Scope so far: M1". 31 ticked tasks sit behind behaviors with no `DONE` row. Should: add rows U15-U23 with their test files and states, mark U13/U14/U16-U23 TEST_AFTER honestly. | `tdd/test-list.md:3-21`; `tasks.md` T033-T047, T049-T056 |
| 4 | HIGH (survivor) | Mutation survivor in a ticked behavior: the client log's bound is untested. `if excess > 0` changed to `if excess > 0 && false` and all 10 `features_structured_log` tests pass. The client log then grows without bound, contradicting its own doc ("stays as bounded as the service's history"). `MAX_EVENTS` appears in no client test. Should: a test applying `MAX_EVENTS + k` events and asserting the retained rows, the marker text and `omitted`. | `crates/micold-client/src/features/structured_log.rs:57-61` |
| 5 | MED | Source-text guards loosened (see table). Each now passes if both substrings appear anywhere in the function body, not as the label expression, and the failure message still describes the old rule. Also the regenerated layout snapshot has no test that states why a row grew by 277 px. | `terminal_bar_stability.rs:295`, `terminal_tabs.rs:295`, `fixtures/layout_snapshot.txt:8258` |
| 6 | MED | Flaky test: `messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal` failed 2 of about 16 runs (one with the two daemon binaries in parallel, one alone) at `support/conn.rs:197` (`the request is answered`, a 20 s timeout), so some request in the sequence is sometimes never answered. 8 further reruns in series were green. The cycle log records other flakes fixed, not this one. | `structured_session.rs:614-667`, `support/conn.rs:185-197` |
| 7 | MED | Sleepy test and negative assertion on a timer: `sleep(300ms)` then "never sent again" and "the other session read nothing". The same pattern was replaced in `structured_resume.rs` by review A (waits until the stand-in read a follow-up message) but left here. Passes if the bug lands later than 300 ms. | `structured_session.rs:582-583`, `739-746` |
| 8 | MED | Foreign style: the profile's convention is "every assertion carries a message explaining the rule". `features_structured_log.rs` (lines 25, 34, 43, 53, 63, 72, 80-82, 91, 99, 107) and `structured_start_menu.rs:44-49, 75, 81` have bare `assert_eq!` or `assert!`. Eight of these assert only `rows.len()`: they pin the count, not the order or text of rows. | files as listed |
| 9 | MED | Eager test: `session_create_keeps_terminal_the_default_and_lists_each_kind` checks decode without kind, the PTY path, the structured kind and no PTY in one body, so one failure does not say which behavior broke. | `structured_session.rs:330-368` |
| 10 | LOW | Real-clock timing assertion: `result.at - assistant.at >= 1000 ms` over a `pause 1500` stand-in. Lower bound only, so it will not flake, but it ties SC-006 to wall time. | `structured_session.rs:231-234` |
| 11 | LOW | Dead setup: `clients` are built, zipped and discarded (`let _ = client;`). The comment "the viewer was attached from the start" is never asserted, so the test says less than it appears to. | `structured_session.rs:442-453` |
| 12 | LOW | Stale verify command: `tasks.md` M8 Verify and T039 name `--test structured_log`, which `7d74ec9a` renamed to `features_structured_log`. The command errors rather than false-greens. | `tasks.md:481, 498, 701` |
| 13 | LOW | `FR-023` (user guide and Settings note) is covered for the Settings note only. The guide text in `docs/user-guide/worktrees-and-sessions.md` and `settings.md` has no automated check. | spec FR-023 |

Properties: the daemon tests use a stand-in CLI and real sockets, are isolated per tempdir and run in 0.5 to 11 s per file (`structured_lifecycle` is the slow one at 11 s: it waits for the crash-loop give-up). Waits are deadline polls, not fixed sleeps, except in finding 7. Failure messages are specific.

Smell items worked through for the new files and found clean: assertion free, tautological, doubled subject, over-mocked, self-approving snapshot, empty/skipped (`#![cfg(unix)]` on the daemon tests is stated and reasoned in-file), re-implemented expectation (the argv test compares against a real terminal launch rather than recomputing it), magic values, mystery guest (fixtures are named and documented in `fixtures/stream-json/README.md`).

## Mutation results

Tool: none in the profile (no `cargo-mutants`). Deliberate mutants only, 8 of them, one at a time, each restored with `git checkout` and re-verified green (daemon, client and core files re-run after restore; the flaky test in finding 6 failed once on the restore run and passed on all but one of 14 reruns, so it is not a mutant leftover: `git diff -- crates` was empty).

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `history.rs:91` `len() >` to `>=` on `max_events` | U6 | No | 3 tests fail; the bound is pinned |
| `history.rs:91` `bytes >` to `>=` on `max_bytes` | U6 | No | 1 test fails (`lower_bounds_evict...`); caught, but by only one test |
| `turn.rs:29` mid-turn guard replaced by `false` | U10 | No | `a_message_mid_turn_is_refused_and_the_turn_goes_on` |
| `turn.rs:46` `== InTurn` to `!= InTurn` | U10 | No | `an_exit_mid_turn_ends_it_with_a_note...` |
| `state.rs:6021` refuse for `Terminal` instead of `Structured` | U15 | No | `terminal_only_requests_to_a_structured_primary_are_refused_to_the_sender` |
| `structured.rs:429` `log_stderr` discards stderr | U8 | No | `the_clis_stderr_is_logged_and_never_an_event` |
| `structured_log.rs:58` bound disabled (`&& false`) | U20-U23 | Yes | Not equivalent: unbounded client log. Finding 4 |
| Reading of `features_structured_log.rs:26` with a reversed log | U20-U23 | Not run | Assertion is vacuous by inspection of `log_line.rs:58`; neighbouring tests catch a reversal, this one does not. Finding 2 |

Coverage: the profile has none. Mutants sampled across 5 behaviors (U6, U8, U10, U15, U20-U23); this is a sample, not a score.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-001-004, SC-004 | `session_kind.rs` (10), `structured_session::session_create_keeps_terminal...`, `a_structured_copilot_session_is_refused...` | Yes (daemon `SessionCreate`) |
| FR-005, FR-006, FR-006a | `a_structured_launch_is_the_terminal_launch_behind_the_structured_args` | Yes |
| FR-007, FR-008, FR-025 | `the_first_prompt_is_delivered_once...`, `a_user_message_is_one_line...`, `messages_are_refused...` (flaky) | Yes |
| FR-009 | `structured_lifecycle.rs` (4 of 7) | Yes |
| FR-010 | `the_clis_stderr_is_logged_and_never_an_event` | Yes |
| FR-011-FR-015, SC-001, SC-002, FR-024 | `structured_parse.rs` (7), `every_recorded_fixture_reaches_two_viewers_in_order`, `no_line_of_the_tolerance_fixture...` | Yes |
| FR-016, FR-017, FR-018, SC-005 | `structured_history.rs` (7) | Yes |
| FR-019 | `structured_lifecycle` refusal and served tests, `structured_wire` | Yes |
| FR-020 | `session_create_keeps_terminal...`, `structured_wire` | Yes |
| FR-021, FR-021a, SC-003 | `structured_resume.rs` (4) | Yes |
| FR-021b, FR-021c, SC-007 | `structured_saved_history.rs` (12), `terminal_history_store` (44), `saved.rs` in-file tests | Yes |
| FR-022, FR-022a | `structured_start_menu.rs` (6), `features_structured_log.rs` (10), `structured_row_marker_fits.rs` (weakened), visual-pass notes | Reducer and menu are state-level; the pane is shown by a human-style visual pass, not by a test |
| FR-023 | Settings note only | No for the guide (finding 13) |
| SC-006 | `an_assistant_event_arrives_before_the_turns_result_is_written` | Yes |

Untested criteria: none outright; FR-023's guide half has no check. Tests tracing to nothing: none found in the feature files.

The test list's `traces` column resolved for U1-U14: every named file and test exists and ran green. U15-U23 have no `traces` because they are not on the list.

## What was not audited

- The full workspace suite and `mise run gate` were not run. The disk filled during the first attempt (a 26 GB shared target dir), so the audit ran only the feature-scoped targets (core: `session_kind`, `structured_parse`, `structured_wire`, `terminal_history_store`, `--lib structured::`; daemon: 5 `structured_*` files; client: `features_structured_log`, `structured_start_menu`, `features_session`, `layout_snapshot`). About 920 cached test binaries were deleted from `target-shared/debug/deps` to free 21 GB; they relink on demand.
- The 4 root-only baseline failures in the cycle log were not re-run.
- Mutation was a hand-picked sample of 8, not a tool run; modules not mutated include `parse.rs`, `saved.rs`, `log_line.rs`, `user_message.rs`, the history chunking in `server.rs` and all of `ui/`.
- Windows and macOS legs (`#![cfg(unix)]` tests, Windows `PATHEXT` follow-up in `autopilot.md`) are not assessed.
- The visual pass (`visual-pass/*.png`, `notes.md`) was not re-run or judged.
- Performance beyond per-file wall times: no criterion, not assessed.
- The `sandbox_real_*` acceptance suite was not run.
- Not checked: whether `layout_snapshot.txt` was regenerated in the same step it was accepted (finding 5).

## Resolution (close unit, after the audit)

The verdict above is the audit as run; the close unit acted on it:

- 1 fixed: the row-marker gate no longer returns green on no change. At the narrowest width the
  caption is clipped and moves no geometry (restoring the old `!changed.is_empty()` assert fails),
  so the gate then asserts the structured and terminal captions differ.
- 2 and 4 fixed: `the_first_history_message_fills_the_log` compares whole rows;
  `the_log_stays_bounded_and_says_how_many_events_it_dropped` pins the client log bound.
- 3 fixed: `test-list.md` gains U15–U23 with honest states. 12 fixed: the M8 verify command names
  `features_structured_log`.
- 5, 8–11 declined as MINOR/LOW (taste or already pinned elsewhere). 7: the two 300 ms negative
  waits stay; each asserts "nothing was sent" and a later bug would fail the next test's ordering.
- 6 (flaky `messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal`, 2 of ~16 runs)
  and 13 (no check of the user-guide text) are recorded as follow-ups in `autopilot.md`.
