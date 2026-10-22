---
feature: 944-session-usage-subagents
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 79db5768
behaviors: 23 # test tasks of tasks.md; no tdd/test-list.md exists
proven: 0
likely: 17
test_after: 6
no_test: 0
high_smells: 1
criteria_total: 7
criteria_covered: 7
mutation_score: unmeasured # no mutation tool in the profile; 6 deliberate mutants, 6 caught
mutants_survived: 0
suite: 187 passed, 0 failed, feature-scoped run (7 test binaries), about 8 s warm; full workspace not re-run
---

# TDD Verification: 944 session usage and subagents

**Verdict: FAIL.** Existing test assertions were loosened inside the feature range
(`structured_transcript.rs`, SC-002 comparison), one feature test has an assertion that silently
skips (`structured_session.rs:218`), and six behaviors have no red recorded (admitted
test-after in the cycle log). Test strength itself is good: 6 of 6 deliberate mutants were caught.

Independence: this audit was run from a cold context; every cited line below was opened in this
run. No test-list.md exists, so per-behavior evidence is built from the test tasks of `tasks.md`
and the cycle log; the rubric table lists "no test list" as `BLOCKED`, this audit proceeded as the
skill allows and the verdict is therefore weaker on ordering.

## Test-first evidence

History: every feature commit adds tests and source in the same commit (no test-only commit exists
for any milestone), so history can never show `PROVEN`. M1's cycle log says its red was
reproduced afterwards by stubbing, not recorded as it happened.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| M1 T002, T004, T005, T007, T009, T011, T012, T015 | LIKELY | red recorded in cycle log, reconstructed after the fact on 520a4644; commit c579a366 holds tests and source together |
| M2 T016, T017, T018, T022 | LIKELY | red recorded with output; commits b6d27aa6 together |
| M3 T023, T024, T025 | LIKELY | red recorded (17 failed on stubs); 22b13ab9 together |
| M4 T029, T030 | LIKELY | red recorded; 92c3023a together |
| M5 T034, T036 | LIKELY | red recorded (18 failed); 09e76dd8 together |
| M6 T037, T038-T041 | LIKELY | red taken by stubbing; ccb68a36 together |
| `a_tool_call_held_again_is_counted_once` (T027 dedupe) | TEST_AFTER | cycle log: "passed at once: test-after" |
| `an_open_thread_survives_an_eviction_that_leaves_its_events` | TEST_AFTER | cycle log: "test-after" |
| `an_open_thread_is_rebuilt_when_the_bound_drops_some_of_its_events` | TEST_AFTER | cycle log: "added test-after (green at once)" |
| M6 `ToggleEntry` fold of a card inside a thread, `kind_icon`, `TranscriptMsg::Insights` session | TEST_AFTER | cycle log: "the three passes are test-after" |

Weakened or changed existing tests:

| # | File:line | Before | After | Judgment |
| --- | --- | --- | --- | --- |
| W1 | `crates/micold-core/tests/structured_transcript.rs` (~708, commit c54d81a2) | `assert_eq!(streamed, finished)` over entries, SC-002 | the same comparison over `split(..)` partitions, raw rows compared apart from the conversation, so interleaving of raw rows no longer pinned for any fixture | Weakened existing assertion (a #939 test changed by #944). Partly offset by the new strict `a_recording_with_no_raw_row_mid_stream_..._exactly` over #938's five fixtures. Still a FAIL condition by the rubric. |
| W2 | same file, `every_event_of_every_recorded_fixture_is_accounted_for` (~1138) | every `content_block_delta` must carry a typed delta | a `Delta::Other` or `None` delta is now accepted when it opened an entry (or is a signature_delta absorbed) | Loosened for all fixtures including the original five. |
| W3 | `crates/micold-daemon/tests/structured_session.rs:218` (commit 6241e499) | unconditional `assert_eq!(forward-record, ...)` for the terminal session | wrapped in `if env var is none` | Loosened, see finding 1. |

Other test-file edits to existing tests were additions or constant updates (`icons.rs` set size
77 to the new count with the two new icons pinned; `containment.rs` path generalisation for the
transcript viewport, an existing exemption widened; `layout_snapshot.txt` regenerated for the new
state and the longer Settings note). No test was skipped, ignored or filtered out; no threshold exists to lower.

`tasks.md` against evidence: T045 and T046 are ticked in the working tree (uncommitted edit).
T045 has a ledger entry ("M6 gate green"); T046 (converge) has no entry in cycle-log.md. LOW.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Conditional logic in the test: the terminal-launch assertion runs only when `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT` is unset in the test process, so the "a terminal session is not given the variable" half is silently skipped in such an environment, and the test depends on ambient env. Should clear the variable for the test (or launch with it explicitly removed) and assert unconditionally. | `crates/micold-daemon/tests/structured_session.rs:218` |
| 2 | HIGH (rubric: weakened existing test) | SC-002 and the typed-delta assertions loosened, W1 and W2 above. Should keep the strict ordered comparison for all fixtures that have no raw row mid-stream (done for five) and pin the exact expected interleaving for the four 944 recordings rather than comparing the two partitions. | `crates/micold-core/tests/structured_transcript.rs` around 708 and 1138 |
| 3 | MED | Re-implemented expectation: `expected_threads` recomputes per-parent message, tool-call and latest-tool counts from the events with the same grouping rule `ThreadIndex` uses (and without the dedupe the code has). Both would be wrong together. Pin literal counts for at least one recording. | `crates/micold-core/tests/structured_insights.rs:1176,1233` |
| 4 | MED | Real clock in assertions and a fixed negative wait: `started.elapsed() < 10s` after `usage_changes` already bounded the wait (tautological), `now()` from `SystemTime`, and a 2 s silence wait for the switch-off case. Use the helper's barrier only. | `crates/micold-daemon/tests/plan_usage_structured.rs:42,202-204,247` |
| 5 | MED | No red for 6 behaviors (TEST_AFTER above) and none of the milestones can show red in history; M1 and M6 red was reconstructed by stubbing afterwards. Cycles were batched per milestone. | `tdd/cycle-log.md:9-12,177-190` |
| 6 | MED | Regenerated snapshot accepted in the same commit as the code: `layout_snapshot.txt` +1037 lines in ccb68a36 (new state plus the bar above two existing states). The cycle log records that the containment gate flagged three folded bodies, which is evidence of some review, but the approval of the new geometry is not separately recorded. | `crates/micold-client/tests/fixtures/layout_snapshot.txt` |
| 7 | LOW | Branch inside a looping test, both branches assert: `if bad.is_object()`. | `crates/micold-core/tests/structured_insights.rs:757` |
| 8 | LOW | An orphaned doc comment ("Whether an entry was opened by the event `seq`.") sits above `subagent_parent` after c54d81a2 and no longer documents `opened_by`. | `crates/micold-core/tests/structured_transcript.rs` before `subagent_parent` |
| 9 | LOW | T046 ticked without a cycle-log or ledger entry in the checked files; T045/T046 ticks are uncommitted. | `specs/944-session-usage-subagents/tasks.md:475-478` |

Not found: assertion-free tests, vacuous truthiness-only assertions on a value (a few `is_some()`
checks at `features_structured_insights.rs:277,425,474,492` guard presence with specific
follow-up assertions elsewhere), doubled subjects, ignored tests, sleeps in the core and client
tests. Sleep polling loops in `structured_session.rs` wait on a condition with a deadline
(acceptable). Helpers: new tests use `support::` fixtures and the daemon `structured` stand-in as
the profile recommends; no hand-rolled fake duplicating a recorded helper was found in the files read.

## Mutation results

No mutation tool in the profile. Six deliberate mutants on high-risk behaviors, each restored with
`git checkout` and the touched files confirmed identical to HEAD; the core suites re-ran green afterwards.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `context.rs:49` `used >= threshold` to `>` | US1 warning threshold | No | caught by `the_warning_starts_at_the_threshold_or_at_90_percent_without_one` |
| `context.rs:50` `*10 >= *9` to `>` | 90 percent default | No | same test |
| `tasks.rs:59` drop the `!cli_running` guard | US5 ended-with-CLI rule | No | caught by two tests |
| `plan_usage.rs:119` `> obtained_at` to `>=` | US4 reset-time rule | No | caught by `an_event_with_no_usable_window_is_none` |
| `daemon/structured.rs:464` do not apply `structured_env` | D8 forward subagent text | No | caught by `a_structured_launch_forwards_subagent_text` |
| `subagents.rs:92` drop tool-owner dedupe | FR-018 | No | caught by the test-after dedupe test |

A first attempt at the `plan_usage.rs` mutant ran a filter that matched no test and was discarded;
the re-run used the three plan-usage test binaries. Sampled 6 mutants, not exhaustive: nothing was
mutated in the client view model, the daemon feed, or the progress bar.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| SC-001 | `structured_insights` per-recording context and footer tests, `the_insights_are_the_cores_for_every_recording` | Partial: core and reducer; the drawn meter in `transcript_insights_view` |
| SC-002 | `totals_are_the_sum_of_the_counted_turns_for_1_2_and_50_turns`, resume and restart tests (:511, :577) | Core level only |
| SC-003 | `every_parented_message_is_counted_in_its_parents_thread_and_no_other`, `every_thread_of_a_nested_recording_opens_at_any_depth` | Yes through the reducer and view |
| SC-004 | `plan_usage_structured` (4 tests through the daemon, two windows, 10 s) | Yes |
| SC-005 | `session_bar_fits`, `transcript_insights_view`, visual pass T044 | Geometry yes; the 5-second claim is manual evidence |
| SC-006 | `malformed_*` tests with `insights-tolerance`, `an_unreadable_event_leaves_the_current_reading_as_it_is` | Yes |
| SC-007 | existing suites, but W1 and W2 changed tests that SC-007 says must pass unchanged | Contradicted in part |

Untested criteria: none. Tests tracing to nothing: none found.

## What was not audited

- The full workspace suite and `mise run gate` were not re-run; only the 7 feature test binaries
  (187 tests) ran. The ledger records a green gate at 79db5768.
- Unit tests inside `crates/*/src` (`figures.rs`, `format.rs`, `progress_bar.rs`, `provider.rs`,
  `daemon/src/plan_usage.rs`) were read only by name through the cycle log, not opened line by line.
- `session_bar_fits.rs`, `plan_usage_rate_limit_event.rs`, the showcase and `settings_sections.rs`
  edits were scanned for smells by pattern, not read in full.
- Mutation was a 6-mutant sample; no tool exists. Coverage unavailable.
- Performance and flakiness: one run only; the daemon tests with real timers (about 4 s) were not repeated.
- The visual-pass evidence and the SC-005 five-second claim were not assessed.
- No fresh-context subagent was available; the smell pass was done in one pass by this auditor.
- `tasks.md` was not modified (`--no-tasks`).

## Remediation (close unit)

- Finding 1: fixed. `a_structured_launch_forwards_subagent_text` asserts the terminal launch
  unconditionally, expecting the value the test process itself holds, if any (run green with and
  without the variable set).
- Finding 3: fixed. `the_recorded_threads_hold_their_hand_counted_figures` pins each recording's
  thread counts as literals.
- Finding 7: fixed. `a_malformed_response_usage_leaves_the_reading_without_a_figure` asserts each
  case without a branch.
- Findings 2 and 8: #939's commit c54d81a2, owned by #939's flow. Findings 4, 5, 6: declined, with
  reasons in the ledger's *Declined review findings*. Finding 9: T045/T046 results are in the
  ledger's *Review rounds*.

