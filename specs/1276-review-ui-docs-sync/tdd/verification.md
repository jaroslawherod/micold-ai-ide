---
feature: 1276-review-ui-docs-sync
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 09ee4cb1
behaviors: 7
proven: 0
likely: 6
not_applicable: 1
test_after: 0
no_test: 0
high_smells: 0
criteria_total: 14
criteria_covered: 14
mutation_score: null # no cargo-mutants; 5 deliberate mutants, 5 caught
mutants_survived: 0
suite: "feature tests: core 3+3+2, client layout_snapshot 162 + layout_text_overflow 3 + layout_coverage_registry 3, client lib 37 (filter `features::changes:: ui::changes:: pull_request_row:: review_bar:: review_panel_row::`); 0 failed"
---

# TDD Verification: Review UI — docs, layout snapshot, visual pass and mockup sync

**Verdict: PASS_WITH_GAPS.** Every code behaviour has a recorded red and a test that fails on a
deliberate mutant, but no red can be ordered in git history (M2 and M3 were squash-merged, M1's red
was reconstructed after review), so the six code behaviours are `LIKELY` (B2 is a refactor,
`NOT_APPLICABLE`); mutation is sampled by hand, not measured.

Not independent: run by hand by the close unit (the forked skill and subagents were not available
to it), from the files and history as they stand, not by a fresh-context auditor. No
`tdd/test-list.md` exists: behaviours below are taken from `tdd/cycle-log.md` and tasks.md.

## Test-first evidence

| Behavior | Class | Evidence |
| --- | --- | --- |
| B1 `review_keys_doc`: guide table = `REVIEW_KEYS` (T003–T005, FR-004) | LIKELY | cycle 1 red (0 passed, 3 failed against `origin/main`'s guide) recorded after the fact (M1 review B F1); test and guide land together in `3856e17b` |
| B2 `terminal_only_doc` moved onto `marked_table.rs` (refactor) | NOT_APPLICABLE | refactor: passes on `origin/main`'s guide and on the branch |
| B3 fixture covers every review state with its anchors (T015, FR-005/006) | LIKELY | T015 red (149 passed, 1 failed); squashed in `ba048262` |
| B4 `review_tab_fits` (T018, FR-007) | LIKELY | T018 red (20 state/theme pairs, filed #1474); wrong-anchor check recorded; squashed in `ba048262` |
| B5 M3 one-line title/name/actions (T029: checks 02, 08, 11) | LIKELY | red 0 passed 4 failed; squashed in `3054321a` |
| B6 M3 cut error reads whole (T029: check 07) | LIKELY | same red; squashed |
| B7 cut name/title reads whole, whole has no tooltip (M3 review A F1) | LIKELY | red 0 passed 2 failed (`left: []`); squashed |

Existing tests: `terminal_only_doc.rs` lost its private helpers to `support/marked_table.rs` with
the same parsing and the same assertions (diff `30f41325..HEAD`); no assertion removed, loosened,
skipped or renamed out of a filter. `layout_snapshot.txt` was re-recorded four times; each re-record
was reviewed by group in the ledger (M2, M2-fix, M3 rows), so it is not self-approving.

tasks.md: T025 was unticked with its behaviour done (CI green on #1495); ticked in the close PR.

## Findings

| # | Severity | Finding | Evidence | Status |
| --- | --- | --- | --- | --- |
| F1 | HIGH (conditional logic) | `findings` checked the review bar and the drawer only `if let Some(..)` they were laid out at their path, so a state that anchors them could pass with neither check run | `crates/micold-client/tests/gates/review_tab_fits.rs:431,446` (before) | Fixed in the close PR: a premise asserts every state anchoring `review.bar` / `review.drawer` lays it out at `REVIEW_BAR` / `REVIEW_DRAWER`. With the bar's path pointed at `[0,0,1,1,2,9]` the gate fails (`session-area-review-shown … premise: the state anchors review.bar`); restored, 2 passed |
| F2 | MED (eager test) | `a_long_entry_name_is_one_ellipsized_line` asserted the one-line cut and the tooltip (review A F1 extended it) | `crates/micold-client/src/ui/changes.rs:1629` (before) | Fixed in the close PR: split into `a_long_entry_name_is_one_ellipsized_line` and `a_cut_entry_name_reads_whole_in_its_tooltip` over one `narrow_heading` helper; both pass, and the tooltip test fails on mutant D |
| F3 | MED (implementation coupled) | `review_tab_fits` and the fixture locate the bar, drawer and diff pane by child-index paths (`REVIEW_BAR = [0,0,1,1,2,2]`), so a wrapper added above them moves every path | `crates/micold-client/tests/support/covered_states.rs:4274` | Kept: the repo's layout gates all address nodes by path and anchors are how the fixture names them (`docs/development/layout-snapshot.md`); a path that moves fails the fixture naming the anchor, it does not go silent |

Properties: the feature's client tests are deterministic (headless renderer, no clock or network);
`layout_snapshot` takes ~33 s warm for 162 tests, measured in T022.

## Mutation results

No mutation tool (profile: `mutation: null`). Deliberate mutants, one at a time, each restored with
`git checkout` and the suite re-run green (`layout_snapshot` 162 passed; client lib 13 passed, filter
`pull_request_row:: review_bar:: changes::a_long_entry_name`):

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| A guide row `` `j` `` "Next file" → "Next changed file" | B1 | No | `the_guides_table_is_the_review_key_table` fails (`review_keys_doc.rs:58`) |
| B `review_bar.rs:205` strip padding `[0.0, MD]` → `[1.0, MD]` | B3 | No | `the_layout_matches_the_committed_fixture` fails naming `path 0/0/0/1/1/2/2/0/1/0, inside review.bar` (quickstart §A7) |
| C `review_bar.rs:149` `whole = …` → `true` (no tooltip on a cut error) | B6 | No | `an_ellipsized_error_reads_whole_in_its_tooltip` fails, `left: []` |
| D `cdk/tooltip.rs:395` `open_if` ignored (tooltip on a whole label) | B7 | No | `a_cut_title_reads_whole_in_its_tooltip` and `a_cut_entry_name_reads_whole_in_its_tooltip` fail, `left: ["Scheduler fix"]` / `["Idle"]` |
| E (test-side, F1) bar path in the premise → `[0,0,1,1,2,9]` | B4 | No | `every_review_state_fits_at_both_windows_in_both_themes` fails on the premise |

Sampled: B1, B3, B4, B6, B7. Not sampled: B5's three one-line fixes and `ReviewPanelRow`'s
actions line.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-001, FR-002, FR-003, SC-001, SC-005 | quickstart §A2–A3 greps and reads (docs); FR-004 below | Docs: no code entry point |
| FR-004 | `review_keys_doc` (3) | Yes: reads the guide and `REVIEW_KEYS` as shipped |
| FR-005, FR-006, SC-002 | `layout_snapshot::the_layout_matches_the_committed_fixture`, `review_tab_fits::every_review_state_of_the_contract_is_covered_with_its_anchors` | Yes: the real view laid out headless |
| FR-007 | `review_tab_fits::every_review_state_fits_at_both_windows_in_both_themes`, `layout_text_overflow`, `layout_coverage_registry` | Yes |
| FR-008 | quickstart §A4 (every covered state named in `layout-snapshot.md`) | Docs |
| FR-009, FR-010, SC-003 | `visual-pass/results.md` (34 crops); T029 unit tests B5–B7 | Recorded pass; fixes unit-tested |
| FR-011, FR-012, SC-004 | `visual-pass/mockup-sync.md`; M4 review B round 2 | Recorded |
| FR-013 | quickstart §A5; M5 review B | Docs |
| FR-014 | CI `docs check` on #1495, #1545; `documentation-set.test.sh`; `documentation_is_not_read` | CI |

Untested criteria: none (docs criteria are checked by quickstart greps and reviews, as the plan
specifies). Tests tracing to nothing: none.

## What was not audited

- History order: M2–M5 were squash-merged and M1's red was reconstructed, so no behaviour is
  `PROVEN`.
- Mutation is a hand sample of five mutants, not a measured score; B5 was not sampled.
- The full workspace suite was not run in this audit (the close PR's `mise run gate` runs it).
- macOS and Windows: CI only; the visual pass and fixture were not re-run on them here.
- The visual pass (§B) and the mockup comparison (§C) were not re-run; their records were read.
- Independence: the audit ran in the close unit's own context, not a fresh-context subagent.
