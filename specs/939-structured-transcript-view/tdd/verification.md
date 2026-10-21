---
feature: 939-structured-transcript-view
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: a9dfd402
behaviors: 11 # no tdd/test-list.md: one batch per milestone, plus each fix that added behaviour
proven: 0
likely: 6
test_after: 4
no_test: 0
not_applicable: 1
high_smells: 0 # one reported HIGH, judged MED on review (finding 1)
criteria_total: 6 # SC-001..SC-006; FR-001..FR-024 traced through tasks.md only
criteria_covered: 6
mutation_score: null # no mutation tool (tdd-profile: cargo-mutants absent); 9 deliberate mutants, 9 caught
mutants_survived: 0
suite: full gate (scripts/gate.sh) — see the close PR's Local gate line
---

# TDD Verification: Structured session transcript view

**Verdict: FAIL — four fixes made after review or the visual pass (M2, M3 ×2, M5) have no red
recorded: test-after by the rubric.** Every one of them was mutant-checked in this audit and its
test caught the mutant; no `HIGH` smell stands, every success criterion has a test through the real
entry point, and no existing test was weakened against the spec. The verdict is the rubric's fail
closed on test-first evidence, not a gap in what the suite proves.

The audit was run by the close unit of the same autopilot run, so it is not independent of the
authoring context; the smell pass went to a fresh-context reviewer (`autopilot-reviewer`), and every
finding it returned was opened and vetted here.

## Test-first evidence

No `tdd/test-list.md` was planned; the cycle log records one red/green per milestone. Each
milestone landed as one `feat` commit adding tests and source together, so the history cannot show
order inside a milestone: `LIKELY` at best.

| Behavior | Class | Evidence |
|---|---|---|
| M1 transcript model, finished events (T001–T015) | LIKELY | cycle log: lib 0/14, `structured_transcript` 0/25 red on `todo!()` stubs; `18e2f0c4` tests + source together |
| M2 streaming, supersede, bound, re-sync (T016–T019) | LIKELY | cycle log: 28 passed, 8 failed; `892b974f` together |
| M2 fix: close stops a block superseded by another kind | TEST_AFTER | `8b95b3ad` adds test and fix with no red in the cycle log; mutant C1 caught |
| M3 window transcript state (T020–T023) | LIKELY | cycle log: 1 passed, 16 failed on stubs; `afbb98c5` together |
| M3 fix: open cards the bound dropped are forgotten | TEST_AFTER | `2f7cfd6a`, no red recorded; mutant C4 caught |
| M3 fix: a toggle of a card that has left opens nothing | TEST_AFTER | `2f7cfd6a`, no red recorded; mutant C3 caught |
| M4 DisclosureCard, slots, monospace, icons (T024–T030) | LIKELY | cycle log: 8 failed in `ui::material`, 1 in `layout_snapshot`; icons red by compilation only |
| M5 view behind the renderer seam (T031–T037) | LIKELY | cycle log: reds quoted (figures 2 failed, entry view 3 failed, release budget 178 ms) |
| M5 fix: tabs expand to 4-column stops (`plain_text`) | TEST_AFTER | cycle log says so itself; `d3e40ecc`; mutant C2 caught |
| M6 pane shows the transcript (T038–T043) | LIKELY | cycle log: tiled tests red twice, gate red on lookup; single-pane red not observed (stated) |
| Close: #944's recordings in `RECORDED` | NOT_APPLICABLE | tests only, no source change |

**Existing tests changed by the feature.**

- `features_structured_log.rs` (147 lines) deleted with the plain log (T041, research R13): the
  behaviour it tested was removed by FR-001, not loosened.
- Close, `structured_transcript.rs::replaying_a_recorded_fixture_one_event_at_a_time_changes_only_the_streamed_entry`:
  the final comparison was entry-for-entry over #938's five; it is now the conversation and the raw
  rows each in order over all nine, because in #944's recordings a `thinking_tokens` row arrives
  while a thinking block streams and the two orders differ by construction (contract G2 amended).
  The entry-for-entry comparison over #938's five is kept in
  `a_recording_with_no_raw_row_mid_stream_streams_into_its_turn_without_deltas_exactly`, so no
  fixture lost strength. The same test file's `content_block_delta` arm now also asserts an absorbed
  `signature_delta` opens nothing and a typed delta opens nothing (stronger than before).
- Close: findings 3–9 below split or labelled tests; every assertion was kept.

**tasks.md against the cycle log**: T001–T043 all ticked, each inside a milestone the cycle log
records. No unticked behavioural task.

## Findings

From the fresh-context smell pass (11 findings), vetted against the cited lines.

| # | Severity | Finding | Evidence | Status |
|---|---|---|---|---|
| 1 | MED (reported HIGH) | The G1 accounting test matches on each event's type to pick its assertion. Every arm asserts (none asserts nothing), and mutants M1–M3 fail it, so not `Conditional logic`; it is an eager test over nine recordings | `crates/micold-core/tests/structured_transcript.rs` `every_event_of_every_recorded_fixture_is_accounted_for` | kept: per-rule tests exist beside it (T004–T012); it is the SC-001 sweep |
| 2 | MED | Wall-clock budget test | `crates/micold-client/tests/transcript_open_budget.rs` | declined: SC-004 is a time budget; release-only and ignored in debug by design (BUG-003) |
| 3 | MED | Two budgets in one test | `transcript_open_budget.rs` | fixed: one test per budget, shared `opened()` |
| 4 | MED→LOW | `a_folded_card_runs_no_block_renderer` asserted only `!painted.is_empty()` beside `unreachable!` renderers | `crates/micold-client/tests/transcript_renderer_seam.rs` | fixed: also asserts the header's tool name and none of the input or result text |
| 5 | MED | Absorbed-only and removal checks in one test | `structured_transcript.rs` `status_and_control_response_events_*` | fixed: split |
| 6 | MED | Empty / omitted / lookup checks in one test, an assertion without message | `structured_transcript.rs` `a_transcript_with_no_event_is_empty`, `an_entry_is_found_by_its_key` | fixed: split, messages |
| 7 | MED | `model.is_some()`, `cost_usd.is_some()` vacuous; unlabelled literals | `structured_transcript.rs` `a_text_turn_is_session_start_reply_and_a_success_footer` | fixed: the recording's model and cost, comment ties them to `text-turn.jsonl` |
| 8 | LOW | Bare asserts; `entry(..).is_some()` vacuous | `features_structured_transcript.rs`, `transcript_renderer_seam.rs`, `src/main_tests.rs` `structured_transcript` | fixed: messages; the entry's key and kind compared |
| 9 | LOW | `rebuilds()` asserted without a rule | `structured_transcript.rs` | fixed: each names FR-021 (rebuilding is the requirement) |
| 10 | LOW | Repeated `events_of(&lines_of(..))` prelude | `structured_transcript.rs` | not fixed: readability only |
| 11 | LOW | One test walks five follow transitions | `src/main_tests.rs` `the_view_messages_scroll_only_when_following_needs_it` | not fixed: each step is labelled |

## Mutation results

No mutation tool. Deliberate mutants, one at a time, each restored with `git checkout` and the
suite re-run green afterwards (the close's full gate):

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| M1 `build.rs` `subagent_messages += 1` → `+= 0` | FR-024 count | No | caught by `sub_agent_messages_are_counted_not_shown` and the G1 total over #944's recordings |
| M2 `build.rs` a parented event treated as top-level | FR-024 not shown | No | 5 tests, G1 among them |
| M3 `stream.rs` `signature_delta` shown as a raw row | absorbed deltas | No | G1, G2 and the thinking-delta test |
| M4 `build.rs` tool error status inverted | FR-007/FR-012 | No | 5 tests |
| C1 `stream.rs` close skips finished (superseded) blocks | M2 fix | No | `a_streamed_block_superseded_by_another_kind_stops_streaming_when_the_stream_closes` |
| C2 `blocks.rs` `plain_text` returns tabs unchanged | M5 fix | No | `a_tab_becomes_spaces_to_the_next_stop_and_a_line_break_restarts_the_count` |
| C3 `features/structured_transcript.rs` toggle of a gone card opens it | M3 fix | No | `a_toggle_of_a_card_not_in_the_transcript_opens_nothing` |
| C4 same file, the bound no longer prunes open cards | M3 fix | No | `a_card_the_bound_dropped_is_forgotten` |
| C5 same file, toggle inverted | FR-010/FR-013 | No | 5 tests |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| SC-001 | `structured_transcript.rs` `every_event_of_every_recorded_fixture_is_accounted_for` (nine real recordings through `Transcript::apply`) | Yes |
| SC-002 | `replaying_a_recorded_fixture_one_event_at_a_time_changes_only_the_streamed_entry`, `a_recording_with_no_raw_row_mid_stream_streams_into_its_turn_without_deltas_exactly` | Yes |
| SC-003 | `features_structured_transcript.rs` follow tests, `transcript_follow_layout.rs`, `main_tests.rs` `structured_transcript` | Yes (window state and real layout) |
| SC-004 | `transcript_open_budget.rs` (release) | Yes |
| SC-005 | `transcript_entry_view.rs` failed card / footer tests; visual pass greyscale evidence `visual/m5-*`, `visual/m6-pane-error-light-grey.png` | Yes, plus manual screenshots |
| SC-006 | the full workspace suite in the gate | Yes |

Untested criteria: none. Tests tracing to nothing: none found.

## What was not audited

- FR-001..FR-024 were not each re-traced to a test; they are covered through tasks.md's per-task
  FR citations, which review B checked per milestone.
- Mutation: 9 hand mutants on high-risk behaviours and every test-after fix, not a scoped tool run.
- Test-first order inside a milestone: not verifiable from history (one commit per milestone).
- quickstart Part B on a live `claude`: run as CPU-rasterised visual passes (M4–M6), not against a
  logged-in CLI; no `claude` login or display in the container.
