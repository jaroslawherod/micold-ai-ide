---
feature: 1024-overview-larger-cards
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 1a68b7b6 # short SHA audited; M1 diff 384f526b..1a68b7b6 (4ceca365, c9b0a4a5)
behaviors: 11
proven: 5
likely: 0
test_after: 1
no_test: 0
not_applicable: 4
pending_visual: 1
high_smells: 0
criteria_total: 11
criteria_covered: 11
mutation_score: 100 # scope: card_grid.rs only, 6 hand-made mutants (no cargo-mutants), 6 killed
mutants_survived: 0
suite: scoped only, card_grid 9 passed, 0 failed, 0.25 s test time (full workspace suite not run)
---

# TDD Verification: Overview with larger cards, five per row

**Verdict: FAIL.** The behavior is well pinned (every mutant tried was killed, no weakened existing test, no HIGH smell), but the `c9b0a4a5` follow-up commit added a new behavioral assertion (30 cards at 2400 px lay out in six rows of five) after the source change with no recorded red, and the rubric fails closed on any `TEST_AFTER`.

Independence: this audit was run by a fresh session that did not write the tests; every test and source line cited was re-read from the working tree.

## Test-first evidence

History shape: `4ceca365` (feat) lands `card_grid.rs` source and tests in one commit; `c9b0a4a5` (test) touches only the test module afterwards. The cycle log names commit `a383c665`, which exists in the object store but is not an ancestor of HEAD (rewritten to `4ceca365`), so ordering inside the first commit cannot be corroborated from history; the log's red output is the only evidence, and same-commit is consistent with it.

| Behavior | Class | Evidence |
| --- | --- | --- |
| A1 | PROVEN | outcome of U2, U3 |
| A2 | PROVEN | outcome of U5 |
| A3 | NOT_APPLICABLE | visual pass only; test list still says PENDING though T012 is ticked and `visual-pass.md` exists (finding 4) |
| A4 | NOT_APPLICABLE | outcome of U6, U7, characterization |
| U1 | PROVEN | cycle 1 red 1 (compile) and red 2 (`left: 8 / right: 5`); test and source in `4ceca365` |
| U2 | PROVEN | red 2 (`left: [8, 4] / right: [5, 5, 2]`) |
| U3 | PROVEN | red 2 (`3 cards: 286 != 467.2`) |
| U4 | PROVEN | red 2 (`left: 6 / right: 3`) |
| U5 | PROVEN | red 2 (`286 < 1.5 x 286`) |
| U6 | NOT_APPLICABLE | log: passed before the rule, pins unchanged below-threshold behavior; covered by a mutant, not a red. 1463 -> 4 and 1464..3840 no-overlap hold on the old code too |
| U7 | TEST_AFTER | the 2400 x 400 scroll/no-sideways assertions pass on old code (characterization), but `c9b0a4a5` added `row_counts == vec![5; 6]` and a right-edge check after the source, and rewrote U2's last-row-left-aligned check; neither has a recorded red |

Existing tests changed: `columns_follow_the_width_and_never_drop_below_one` (only a fourth argument added, expected values unchanged), `the_laid_out_grid_has_the_columns_its_width_gives` (case added), `no_card_overlaps_another` (widths and a right-edge assertion added), `thirty_cards_scroll_vertically` (looped over 600 and 2400; every original assertion kept at 600). None weakened, none skipped, no config/threshold change. No weakened existing test.

tasks.md vs test list: all T001-T013 ticked; every behavioral task maps to a DONE behavior. A3 is PENDING on the list while T012 is `[X]` (stale list, mild inverse).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | MED (FAIL condition) | Row-shape assertion `vec![5; 6]` and the reworked last-row alignment check were added/changed after the source with no recorded red (`TEST_AFTER`). Mitigated: mutants M1 and M5 fail `thirty_cards_scroll_vertically`/`full_rows_...`, so they have teeth, but that is not test-first | `card_grid.rs:229-236`, `card_grid.rs:283-293` (`c9b0a4a5`) |
| 2 | MED | Cycle log cites commit `a383c665`, not on the branch; test/source order unverifiable from history, so U1-U5 are PROVEN on the log plus same-commit only | `tdd/cycle-log.md` "commit: a383c665" vs `4ceca365` |
| 3 | MED | Redundant test: `thirty_cards_scroll_vertically` row-shape `vec![5; 6]` pins the same cap rule as `full_rows_hold_five_cards_from_the_threshold_up`; also an eager test now (scroll, no sideways growth, row shape) | `card_grid.rs:219-236` |
| 4 | MED | Test list stale: A3 `PENDING` and `updated_at` unchanged although T012 is done; the visual pass did not cover a ~2400 px client grid (its own "Not covered"), so FR-007/FR-008 text-overlap at the widest card widths has no automated or visual evidence | `tdd/test-list.md` A3; `visual-pass.md` "Not covered" |
| 5 | MED | FR-011 (Overview and showcase share the grid) is not tested at the entry point: nothing asserts `ui/overview.rs:45` builds `CardGrid` with the default cap (it does today, by not calling `max_columns`); a later `.max_columns(8)` there would break no test | `crates/micold-client/src/ui/overview.rs:45` |
| 6 | LOW | Duplicate ids in doc comments: feature 813's `U4` and this feature's `U4 (1024)` sit in one module | `card_grid.rs:163,190,214,311` |
| 7 | LOW | Magic values: gap `16.0` and min `280.0` repeated in `row_width`, `grid`, and the `columns` calls; tolerance `0.01` repeated ~12 times | `card_grid.rs:134-136,230,242,...` |
| 8 | LOW | Loops over width lists in `full_rows_...`, `no_card_overlaps_another`, `thirty_cards_...` always assert (not conditional), but failures rely on the `{width}` message for specificity | `card_grid.rs:196,263,282` |

HIGH smells: none. Convention check: tests live in `mod tests` in the source file (matches `unit_src` exemplar), use `test_support::{laid_out, leaves, roles}`; no hand-rolled fakes duplicating `helpers`; assertions carry messages. `row_width` computes the spec's `(w - (n-1)*gap)/n`, independent of the code (which uses `FillPortion`), so not a re-implemented expectation. Not asserted: a vacuous, tautological or doubled-subject test (none found).

## Mutation results

Tool: none in profile (no `cargo-mutants`); deliberate mutants, hand-applied one at a time to `crates/micold-client/src/ui/material/card_grid.rs` (the only non-doc file the feature changed), each restored with `git checkout` and verified. Command: `scripts/build-lock.sh cargo test -p micold-client --lib card_grid`; baseline 9 passed. Sampled 6 of 10 planned (build time about 15 min per mutant from a cold target; M7-M10 were not run).

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| M1 `clamp(1, max.max(1))` to `max.max(1) + 1` (cap six) | U1, U2 | No | 6 failed (U1, U2, U3, U4, U5, 30 cards) |
| M2 `floor` to `ceil` | U1, U6 | No | 3 failed |
| M3 divisor `(min + gap)` to `min` | U1, U6 | No | 3 failed |
| M4 drop last-row `Space` padding | U2, U3 | No | 2 failed |
| M5 `new()` default cap 5 to 4 | U4 | No | 4 failed |
| M6 `From` impl ignores the builder (`MAX_COLUMNS` passed) | U4 | No | 1 failed (`max_columns_sets_the_cap`) |

Not run: cap-of-zero (`max.max(1)` to `max.max(2)`), `MAX_COLUMNS` 6, dropped `.clamp` lower bound (U1's `max = 0` and the `1` floor are only reasoned, not mutated), `chunks` size. Score 6/6 killed on the sample; this is a sample, not exhaustive. Final run after restore: 9 passed, working tree clean.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 1-3, 5, FR-001, FR-002, SC-001 | U1, U2 | Yes (laid-out `CardGrid` element), not via `ui/overview.rs` |
| US1 4, FR-004 | U3 | Yes (same) |
| FR-011 | U4 (default and builder) | Partial: finding 5 |
| US2 1-2, FR-005, FR-006, SC-002 | U5 | Yes (same) |
| US2 3, FR-007-FR-009 | A3 visual pass | No automated test; visual pass at 1544 px only |
| US3 1-3, FR-003, SC-003 | U6 | Yes (same) |
| US3 3, SC-004 | U7 (scrolling), visual pass (keyboard) | Scrolling yes; keyboard visual only |
| FR-010 | feature 813's unchanged tests | Not re-verified |
| FR-012 | docs only (T008), no test | n/a |

Untested criteria: none claimed without a test or a recorded visual pass; FR-007-FR-009 and keyboard part of SC-004 rest on the visual pass alone. Every `traces` test name in the list exists and runs (9 tests found in `card_grid.rs`). Tests tracing to nothing: none.

## What was not audited

- The full workspace suite and `mise run gate` were not run (cold shared target dir, over an hour to build); the log's "gate green at 8046c5f6" is unverified here. Only `card_grid` (9 tests) ran.
- Mutation was a 6-mutant hand sample on `card_grid.rs`, not a tool run; 4 planned mutants not run.
- Coverage: no tool in the profile.
- Visual pass contents (`visual-pass.md`, screenshots) were read but not re-run; the layout snapshot fixture and showcase completeness gate were not inspected for this feature.
- Docs change (`docs/user-guide/overview.md`, T008), FR-010 regression tests of feature 813, performance, and cross-platform behavior: not assessed.

## Resolution (close unit, 2026-10-18)

Finding 1: retroactive red recorded in `cycle-log.md` (T014); the late `vec![5; 6]` assertion was then dropped as a duplicate (T015, finding 3). The FAIL verdict stands for the history as it was (test-after cannot be undone); the remediation is complete. Finding 4: test list A3 DONE, 2400 px not checked by eye (T016). Finding 5: stated in the test list (T017). Findings 6-7: ids and constants fixed (T018).

