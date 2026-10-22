---
feature: 940-transcript-rich-rendering
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 18e9cf09
behaviors: 20 # no tdd/test-list.md: one batch per milestone, plus each fix or late test that added behaviour
proven: 0
likely: 8
test_after: 10
no_test: 0
not_applicable: 2
high_smells: 0
criteria_total: 7 # SC-001..SC-007; FR-001..FR-022 traced through tasks.md only
criteria_covered: 7
mutation_score: null # no mutation tool (tdd-profile: cargo-mutants absent); 9 deliberate mutants, 8 caught, 1 equivalent
mutants_survived: 1 # K7, judged equivalent
suite: scripts/gate.sh at 18e9cf09 — 9440 passed, 0 failed, 17 ignored (release-only budgets and runtime-gated suites)
---

# TDD Verification: Structured session rich rendering

**Verdict: FAIL — ten behaviours have no red recorded before their code (fence tokens, T019 link
routing, T020 copy arm, CodeBlock and LinkText, T026's contrast cases, M5's two fallbacks, the table
gate, the budget, M5's review A fixes, the two-window assertion): test-after by the rubric.** The
cycle log discloses each of them itself, and for six a red was shown afterwards against a stub or a
mutant. No `HIGH` smell stands, no existing test was weakened, every success criterion has a test
through the real entry point, and of nine deliberate mutants eight were caught and the ninth is
equivalent. The verdict is the rubric's fail closed on ordering, not a gap in what the suite proves.

The audit was run by the close unit of the same autopilot run, so it is not independent of the
authoring context; the smell pass went to a fresh-context reviewer (`autopilot-reviewer`), and every
finding it returned was opened and vetted here.

## Test-first evidence

No `tdd/test-list.md` was planned; the cycle log records one red/green per milestone. Each
milestone landed as one `feat` commit adding tests and source together, so the history cannot show
order inside a milestone: `LIKELY` at best.

| Behavior | Class | Evidence |
|---|---|---|
| M1 markdown model (T006–T012) | LIKELY | cycle log: `markdown_stream` 1 passed 3 failed, `markdown_subset` 2 passed 16 failed on a stub `Document`; `8568ba27` together |
| M1 fence tokens (`fence.rs`) | TEST_AFTER | written beside `fence_token`; red shown afterwards (review B M1) against a stub: 1 passed, 4 failed |
| M1 review A fixes (nested definitions, image links, closed fences, task items, line counts) | LIKELY | cycle log: red → green as regression tests; `503e83e8` together |
| M2 edit diff (T013–T015) | LIKELY | cycle log: `edit_diff` 0/15 on the `todo!()` stub; `91d2b395` together |
| M2 review B: CRLF-only edit's note pinned `None` | NOT_APPLICABLE | `f5e39a6f` test only, pins behaviour already shipped |
| M3 rich cache (T016–T018, T021–T024) | LIKELY | cycle log: 1 passed, 15 failed on `todo!()` stubs; T016 red by compilation; `84cc0685` together |
| M3 T019 web/mail link opens | TEST_AFTER | cycle log: the one test green against the stub, its routing written with the variants |
| M3 T020 checked copy (`interpret` arm) | TEST_AFTER | written after its arm; red shown afterwards by moving the arm (`clipboard_request.rs:247`) |
| M3 review A MINORs (hash in place, one blocking task per batch) | NOT_APPLICABLE | `bcd7efbb`: cost changes under the existing tests, no new behaviour |
| M4 InlineDiff, MarkdownView, `DisclosureCard::meta` | LIKELY | cycle log: `ui::material` 17 failed, `layout_snapshot` 5 failed on stubs; `8011ebed` together |
| M4 CodeBlock, LinkText | TEST_AFTER | cycle log: drafted before their tests; red shown afterwards against `todo!("stub")` |
| M4 T026 `composition_contrast`, `style_outline_discipline` cases | TEST_AFTER | passed against the stubs (they read real style functions) |
| M4 review fixes (one stop per link, list gutter, no-newline caption) | LIKELY | cycle log: each red first with its failure quoted; `23b41057` together |
| M5 `BlockRenderers::RICH`, edit meta, stamp in the lazy key (T035, T037–T041) | LIKELY | cycle log: 5 of 7 failed on a `PLAIN` stub; `b3040d07` together |
| M5 fallbacks to `PLAIN` (streaming/malformed input, cache miss) | TEST_AFTER | green on the `PLAIN` stub by design; red shown afterwards by mutation (empty `Space`) |
| M5 table gate (`transcript_rich_table_wraps`) | TEST_AFTER | cycle log: red not recorded |
| M5 rich budget (T036, SC-004) | TEST_AFTER | a cost guard: passed against the stub, which drew less (disclosed) |
| M5 review A fixes (empty block on a hit, meta from the revision drawn) | TEST_AFTER | `e23cea3a`; no red recorded for these two |
| M5 review B: two windows derive one document | TEST_AFTER | passed on first run (M3's derivation is pure) |
| Red CI fix: pinned monospace face | LIKELY | ledger: guard `the_monospace_face_parses_and_is_the_face_that_measures` red with the pin removed; `18e9cf09` test only |

**Existing tests changed by the feature.** Every removed line in the feature's commits was read:

- `clipboard_request.rs`: `the_shells_translation_decides_nothing` kept; its source read moved into
  `interpret_body()`, shared with the new T020 test. Same assertions.
- `features_structured_transcript.rs`: `structured_events` gained the grammar argument and
  `update_transcript` returns an outcome; each assertion now reads `.scroll`. Same values.
- `icons.rs`: curated set size 79 → 81 for the two task-list marks, with the reason.
- `gates/containment.rs`: the transcript exemption also names the new state
  `session-transcript-rich` (the same exemption, for the state this feature adds).
- `transcript_open_budget.rs`: the two #939 budgets kept with their names and `ignore` reason;
  the history builder generalised over a kind; two rich budgets added.
- `support/covered_states.rs`: the transcript-pane pose moved into a shared builder; `.filter().last()`
  → `.rfind()` (same element).
- `layout_snapshot.txt`: regenerated twice, each a reviewed delta (M5: the new state only; red CI:
  2 lines, 343.3 → 343.2, from the pinned face).

None weakened. **tasks.md**: T001–T042 all ticked, each inside a milestone the cycle log records;
no unticked behavioural task.

## Findings

From the fresh-context smell pass (7 findings), vetted against the cited lines.

| # | Severity | Finding | Evidence | Status |
|---|---|---|---|---|
| 1 | MED | Bypassed test utility: `recorded` hand-rolled fixture loading the `support::transcript` helpers provide | `crates/micold-client/tests/features_transcript_rich.rs:28` | fixed at close: `recorded` calls `support::transcript::{fixture_lines, sequenced}` |
| 2 | MED | Wall-clock budgets, ignored in debug | `crates/micold-client/tests/transcript_open_budget.rs` | declined: SC-004 is a time budget; release-only by design (BUG-003), as #939's |
| 3 | MED (reported as re-implemented expectation) | `assert_nothing_hidden` compares the model's text with a second, hand-written marker stripper | `crates/micold-core/tests/support/markdown.rs:125-209` | kept: an independent oracle for the 10,000-round property test, not the parser's logic (pulldown-cmark); fixed-shape tests (`markdown_subset.rs`, 18) assert exact blocks beside it |
| 4 | MED | Eager gates over a scheme × pose loop; expected header parts branch on the pose | `gates/code_block_fits.rs:30`, `gates/inline_diff_marker_fits.rs:28`, `markdown_subset.rs:409` | kept: every branch asserts (Write still asserts its `+`), every message names scheme and pose; not `Conditional logic` |
| 5 | LOW | `assistant`/`text`/`tool_use` builders defined per file | `features_structured_transcript.rs`, `features_transcript_rich.rs`, `transcript_rich_renderers.rs` | not fixed: readability only |
| 6 | LOW | `links_take_primary_and_muted_text_its_variant` restates the colour mapping | `crates/micold-client/src/ui/material/link_text.rs:288` | not fixed: legibility is checked in `composition_contrast` |
| 7 | LOW | Literal `20.0` for one code line's height | `gates/code_block_fits.rs:90` | fixed at close: `CODE_LINE` |

## Mutation results

No mutation tool. Deliberate mutants, one at a time, each restored with `git checkout` and checked
clean; the close's full gate ran on the restored tree:

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| K1 `markdown/inline.rs` `activatable` → `true` | FR-011 web/mail only | No | `web_and_mail_links_are_activatable_and_others_are_not` |
| K2 `markdown/fence.rs` tag not lower-cased | fence tokens | No | `the_token_is_lower_cased`, `aliases_map_to_their_grammar_token` |
| K3 `markdown/mod.rs` `update` reuses the open slice when the text does not extend | M-4 incremental = fresh | No | `a_text_that_does_not_extend_the_old_one_is_parsed_afresh` |
| K4 `structured/edit_diff.rs` deleted lines as `Context` | SC-003 round trip | No | 7 tests, the recorded round trip among them |
| K5 `features/transcript_rich.rs` copy `Copied` without comparing the read-back | SC-005, FR-012 | No | `the_read_back_decides_copied_or_failed_and_asks_for_the_timer` |
| K6 same file, refresh keeps entries the transcript dropped | prune | No | `keys_the_transcript_no_longer_holds_are_pruned`, `spans_for_a_part_that_is_gone_are_not_stored` |
| K7 same file, `copy_expired` drops its ticket guard | stale timer ignored | Yes | equivalent: `tickets` holds only each block's current ticket (`copy` removes the old one, refresh retains matching ones), so a stale ticket returns at the lookup before the guard; `only_the_current_tickets_expiry_returns_the_control_to_idle` covers the behaviour |
| K8 `ui/transcript/entries.rs` `edit_meta(removed, added)` | FR-017 `+N −M` | No | `rich_keeps_entries_order_folds_statuses_and_headers_but_adds_edit_counts` |
| K9 `ui/material/diff_view.rs` removed marker drawn `+` | SC-006 sign alone | No | `inline_diff::a_multi_edit_shows_its_path_hunks_notes_and_signs` |

M5's review A also mutated the three `PLAIN` fallbacks in `rich.rs` (cycle log): all caught.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| SC-001 | `markdown_subset.rs` (`nothing_received_is_hidden`, `the_recorded_reply_maps_every_element`, `random_cuts_and_random_text_never_fail_or_hide_text`); `markdown_view::the_subset_draws_its_text_without_its_punctuation`; `transcript_rich_renderers.rs` `text_and_thinking_are_drawn_from_the_cached_document` | Yes (window view) |
| SC-002 | `markdown_stream.rs` (4, the recorded turn delta by delta); `a_code_id_names_the_same_block_in_every_longer_prefix`; `features_transcript_rich.rs` `a_refresh_after_a_delta_re_derives_only_the_entry_it_changed`; `the_lazy_key_changes_with_the_stamp_and_only_then` | Yes |
| SC-003 | `edit_diff.rs` `every_recorded_edit_and_write_round_trips`, `malformed_inputs_fall_back_to_the_plain_input`; `edit_inputs_are_diffs_and_results_stay_plain`, `a_streaming_or_malformed_edit_input_is_drawn_as_plain` | Yes |
| SC-004 | `transcript_open_budget.rs` rich variants (release; 118 ms open, 2.7 ms delta in M5) | Yes |
| SC-005 | `a_copy_writes_the_code_byte_for_byte`, `a_copy_of_a_streaming_block_writes_the_code_so_far`; `clipboard_request.rs` `a_checked_write_is_a_write_then_a_read_handed_back` | Yes (state; the OS clipboard is the shell's) |
| SC-006 | `inline_diff` sign tests, `gates/inline_diff_marker_fits.rs`; `composition_contrast` `diff_line_text_is_legible_on_the_added_and_removed_tints`, `syntax_colours_are_legible_on_the_diff_surface_and_both_tints`; `visual-pass/m5-diff-greyscale.png` | Yes, plus screenshots |
| SC-007 | the full workspace suite in the gate; `layout_snapshot`'s #939 records unmoved in M5 | Yes |

Untested criteria: none. Tests tracing to nothing: none found.

## What was not audited

- FR-001..FR-022 were not each re-traced to a test; they are covered through tasks.md's per-task
  FR citations, which review B checked per milestone.
- Mutation: 9 hand mutants on high-risk behaviours, not a scoped tool run.
- Test-first order inside a milestone: not verifiable from history (one commit per milestone).
- The release budget was not re-run at close (no release build in the container's disk budget);
  its figures are M5's.
- quickstart Part B on a live `claude`: run as headless visual passes (M4, M5), not against a
  logged-in CLI; steps 3, 4 and 8 are covered by tests, not driven on screen.
