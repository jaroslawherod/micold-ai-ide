---
feature: 1273-review-file-tree
verdict: FAIL # TEST_AFTER history, accepted at close; the HIGH smell F1 fixed (see the last section)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: c9a21a6a1 # short SHA audited (main after #1430); close-unit fix on top, see the last section
behaviors: 19 # rows of the table below; no tdd/test-list.md exists, the cycle log's task rows stand in
proven: 6
likely: 0
test_after: 5
no_test: 0
not_applicable: 8
high_smells: 1 # F1, re-implemented expectation; fixed in the close unit
high_findings: 2 # F1, and the test-first breach (5 behaviors)
criteria_total: 24 # FR-001..FR-018, SC-001..SC-006
criteria_covered: 24 # FR-015 and SC-006 by construction and the visual pass only, see Traceability
mutation_score: unmeasured # no mutation tool in the profile; 10 deliberate mutants, 10 caught
mutants_survived: 0
suite: scoped before the fix (micold-core lib 1763 + review_tree 42; client tree_/reviewed_marks/source_menu filters 38 + 29), all passed; full gate at the close commit, see the ledger
---

# TDD Verification: Review UI, changed-file tree, filter and sort

**Verdict: FAIL.** Five behaviors were written test-and-code in one pass, their red obtained
afterwards by mutation (`TEST_AFTER`), and one core test computed its expectation with the matcher
and rule it checks (F1, fixed in the close unit). Test strength is good: ten deliberate mutants
across the core layout and the client reducer all failed a test.

Independence: this audit ran in the close unit of the autopilot run that wrote the feature. Every
cited file was re-read; the smell pass ran in a fresh-context reviewer (sonnet) and each of its
cited lines was opened here before it reached this report.

## Test-first evidence

There is no `tdd/test-list.md` (the cycle log says so: tasks.md's test tasks are the list). The
milestones were merged by rebase, so each feature commit keeps its order on `main`.

| Behavior (cycle-log row) | Class | Evidence |
| --- | --- | --- |
| T001 module and test file | NOT_APPLICABLE | structural |
| T002/T003/T005 → T004/T006 layout, chaining, order, `TreeChoices` | PROVEN | compile red logged (`unresolved imports …tree::layout`); `6e4118b48` adds tests and source together |
| T007 → T010 folder totals | PROVEN | red logged: 4 failed (totals were 0); `d2b92019b` |
| T008/T009 → T010 bar, indent cap, hover text | PROVEN | compile red logged; `d2b92019b` |
| T011/T013/T015/T016 → T012/T014/T017 filter, spans, sorts, offer | PROVEN | compile red then 12 failed logged; `d2b92019b` |
| T018 SC-003 timing | TEST_AFTER | "written after T012/T017", no red |
| T019 list pane geometry | NOT_APPLICABLE | structural |
| T020/T021 → T022 reducer folds, reveal, re-read, source switch | PROVEN | 15 compile errors logged; tests commit `fcb6a5045` precedes code commit `44f0315bc` |
| T023 Files tree on the core layout | NOT_APPLICABLE | behaviour-preserving refactor; existing Files tree tests unchanged in `44f0315bc` |
| T024/T025 `ChangeBar` | TEST_AFTER | "written with the component in one step"; red by later mutation (review B M4 F1) |
| T026 showcase entry | NOT_APPLICABLE | existing guard `showcase_completeness`, no new test |
| T027 SC-006 room, kind tag, no bar | TEST_AFTER | red by later mutation (review B M4 F1) |
| T028 layout snapshot states | NOT_APPLICABLE | approval fixture; reviewed by the M4 visual pass |
| T031/T032 `TextField::on_escape` | PROVEN | red logged (`E0599 no method named on_escape`); `9bb6d0632` |
| T033–T035 filter, sort, marks in the reducer | TEST_AFTER | "not run before the arms were written"; red by later mutation (review B M5 F1) |
| T036 filter box, chips, no-match line | TEST_AFTER | "drawn first"; red by mutation after T037 |
| T037 snapshot states | NOT_APPLICABLE | approval fixture; reviewed by the M5 visual pass |
| T038 user guide | NOT_APPLICABLE | docs |
| T039 visual pass | NOT_APPLICABLE | evidence in `visual-pass/m5.md` |

Tasks against list: T001–T039 are ticked and each has a cycle-log row; T040 is this close unit.

Existing tests changed by the feature:

| File:line | Before / after | Judgement |
| --- | --- | --- |
| `crates/micold-core/tests/review_tree.rs` (`4d982120b`) | a file row's label checked as a path suffix → checked against its parent folder's path | strengthened (the feature's own test) |
| `crates/micold-client/src/showcase/catalogue.rs` `EXEMPTIONS` (`c2f5af141`) | `EmphasisedLabel` (made public for FR-009) exempted from posing, with its reason | legitimate: one named component, posed through the type-ahead's entries; not a prefix or pattern |
| `crates/micold-client/tests/fixtures/layout_snapshot.txt` | Changes states re-recorded, three states added | approval fixture, reviewed in the M4 and M5 visual passes |
| `crates/micold-client/tests/support/covered_states.rs` | three tree states added | additive |

No assertion was removed or loosened, no test skipped, no threshold lowered.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | `a_folder_rows_spans_are_the_union_of_its_files_spans` built its expectation by calling `typeahead::match_one` and re-doing the clip, union and dedup in a loop with `if let`, so a wrong folder-span rule moved both sides. **Fixed in the close unit**: literal spans for both files and the folder; mutant M10 (folder spans never pushed) now fails it. | `crates/micold-core/tests/review_tree.rs:503` |
| 2 | HIGH | Five behaviors are `TEST_AFTER` (table above). Test strength for them was shown (mutants fail the tests), ordering was not. | `cycle-log.md` rows T018, T024/T025, T027, T033–T035, T036 |
| 3 | MED | SC-003 timing test uses the real clock (slowest of 15 layouts) with a 5× debug allowance, and its only functional assertion is `!rows.is_empty()`. | `crates/micold-core/tests/review_tree.rs:870-921` |
| 4 | MED | `size_puts_the_largest_file_and_folder_first` asserts inside a loop on a generated fixture, guarded by `assert!(runs > 3)`; it overlaps `size_orders_each_level_largest_first` (literal shape). | `crates/micold-core/tests/review_tree.rs:727-778` |
| 5 | MED | The SC-006 room check computes the room with the renderer's own `name_room` and `TREE_INDENT_STEP`; only the cap's lower bound is independent. A cap of 0 is caught (cycle log M4). | `crates/micold-client/src/ui/changes.rs:1480-1493` |
| 6 | MED | Most `assert_eq!` in `review_tree.rs` carry no rule message, against the profile's convention; some tests chain two scenarios. | `crates/micold-core/tests/review_tree.rs:103, 485, 715, 812` |
| 7 | LOW | `assert_eq!(MIN_BAR, 2.0)` pins a constant; tolerance literals unnamed. | `crates/micold-core/src/review/tree.rs:585` |
| 8 | LOW | `rows()` helper copied in `features_changes.rs` and `features_pr_changes.rs`. | `features_changes.rs:5891`, `features_pr_changes.rs:3012` |

No other HIGH smell from the catalogue (assertion free, tautology, vacuous, doubled subject,
skipped tests). Isolation and determinism are fine apart from finding 3; speed is fine
(`review_tree` 42 tests in 0.3 s).

## Mutation results

No mutation tool in the profile. Ten deliberate mutants, one at a time, each restored with
`git checkout` and the touched suites re-run green. Sampled: totals, sort, openness, filter-time
folds, reveal, bar split, folder spans (core); reveal on select and on a reference, re-layout on a
mark change (client). Not exhaustive; previously recorded mutants (`ChangeBar::segments`,
`tree_indent_cap`, `kind_tag`, `counted`, `TreeFilterCleared`, the no-match arm) are in the cycle log.

| Mutant | Behavior | Survived | Caught by |
| --- | --- | --- | --- |
| M1 `tree.rs:160` folder `removed` never summed | FR-007 totals | No | `a_folder_row_sums_its_files_counts` and 4 more |
| M2 `tree.rs:215` Size folders ascending | FR-012, SC-004 | No | `size_orders_each_level_largest_first` and 2 more |
| M3 `tree.rs:241` chained row closed only when all its folders are | FR-002/FR-003 | No | `a_chained_row_is_closed_when_any_of_its_folders_is_collapsed` and 2 more |
| M4 `tree.rs:430` filter-time folds cleared on every keystroke | FR-010 | No | `starting_a_filter_clears_filter_folds_and_keeps_the_text_as_typed` |
| M5 `tree.rs:476` reveal keeps the reviewer's folds | FR-008 | No | `reveal_opens_every_ancestor_of_the_path`, `reveal_clears_both_sets` |
| M6 `tree.rs:328` added part sized by `removed` | FR-006 | No | `a_bar_splits_added_and_removed_in_proportion`, `a_one_line_change_is_at_least_the_minimum_bar` |
| M7 `features/changes.rs:2514` `FileSelected` reveals nothing | FR-008 | No | `tree_state::selecting_a_file_reveals_it`, `open_diff_from_a_browsed_file_reveals_it` |
| M8 `features/changes.rs:3933` a reference reveals nothing | FR-008 | No | `tree_state::a_reference_to_a_changed_file_reveals_it` |
| M9 `features/changes.rs:759` layout key ignores the marks | FR-012 (Unreviewed first) | No | `reviewed_marks::unreviewed_first_follows_the_marks` |
| M10 `tree.rs:166` folder spans never pushed (after the F1 fix) | FR-009 | No | `a_folder_rows_spans_are_the_union_of_its_files_spans`, `spans_are_rebased_to_each_rows_label` |

## Traceability

Contract cases L1–L12 (`contracts/review-tree.md` §1) are the core tests' unit of claim.

| Criterion | Tests | Through the real entry |
| --- | --- | --- |
| FR-001, FR-002 | `review_tree` L1–L4, L12; Files tree tests (T023) | Yes (`layout`, reducer `change_rows`) |
| FR-003, FR-004 | `TreeChoices` toggle tests; `tree_state` fold tests | Yes |
| FR-005 | `hover_text`; `modified_files_carry_no_kind_tag`; SC-002 test | Yes |
| FR-006 | `bar` unit tests; `change_bar` pixel tests; `binary_not_text_and_mode_rows_have_no_bar` | Yes |
| FR-007 | totals tests (L5) | Yes |
| FR-008 | reveal tests (core, `tree_state`) | Yes |
| FR-009 | filter tests (L8–L10), span tests; `changes-tree-filter` snapshot | Yes |
| FR-010 | filter-fold tests; `text_field` `on_escape`; `clearing_the_filter_restores_the_folds_before_it` | Yes |
| FR-011 | `tree_state` no-match test; `changes-tree-no-match` snapshot | Yes |
| FR-012 | `sort_options`, `set_sort`, sort tests (L6, L7, L11); `unreviewed_first_follows_the_marks` | Yes |
| FR-013, FR-014, SC-005 | `tree_state` re-read and source-switch tests; `tree_source` | Yes |
| FR-015 | none: the tree is built in `VirtualRows` (`ui/changes.rs:701`), as the flat list was | Construction and visual pass only |
| FR-016 | `tree_state` reading/failed/empty test; snapshot states | Yes |
| FR-017 | core tests over `layout` (all of the above) | Yes |
| FR-018 | `layout_snapshot`, `showcase_completeness`; guide checked by review | Yes |
| SC-001, SC-002, SC-004 | `review_tree.rs:591`, `:203`, size tests | Yes |
| SC-003 | `laying_out_five_thousand_files_stays_under_100_ms` (finding 3) | Yes, timing only |
| SC-006 | `a_24_character_name_keeps_its_room_at_every_depth` (finding 5); visual pass | Partly |

Untested criteria: none outright; FR-015 by construction only. Tests tracing to nothing: none.

## What was not audited

- The full workspace suite was not run inside the audit; the close unit's full gate runs it (see
  the ledger). The audit's scoped runs are in the frontmatter.
- Whole-feature mutation: no tool; 10 deliberate mutants. `ui/changes.rs` drawing (row layout,
  chips, filter box) was not mutated here beyond the cycle log's recorded mutants.
- Coverage: no tool in the profile.
- The visual-pass evidence (`visual-pass/*.md`, `*.png`): not re-run or judged.
- `docs/user-guide/reviewing-changes.md`: not graded here (the close review reads it).

## Close unit follow-up

Finding 1 is fixed with a test-only change (literal expectations; mutant M10 caught). Accepted, not
fixed: finding 2 (history cannot be redone; the cycle log records each red obtained by mutation),
findings 3–4 (the timing budget and the generated SC-004 check are deliberate, research R9; the
literal-shape tests pin the rules), finding 5 (the test's subject is the cap; it shares the list's
geometry by design, and the M4 visual pass checked the drawn result), findings 6–8 (MED/LOW, style).
No remediation phase is appended to tasks.md: nothing left is unbuilt behaviour or a fixable gap
the close unit did not take.
