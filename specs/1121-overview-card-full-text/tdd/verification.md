---
feature: 1121-overview-card-full-text
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 77d8cfa1
behaviors: 37 # cycle-log rows (no tdd/test-list.md; tasks.md test tasks are the list)
proven: 34 # incl. 2 after-the-fact reds and 1 not-applicable fixture row counted separately below
likely: 0
test_after: 0
no_test: 0
high_smells: 0
criteria_total: 24 # FR-001..FR-018 (18) + SC-001..SC-006 (6)
criteria_covered: 24
mutation_score: 10 of 11 distinct mutants caught (91%) # scope: 6 changed source files, deliberate mutants only, no cargo-mutants
mutants_survived: 1 # EXPAND_REST floor, judged equivalent; M7 survived the unit tests but the layout fixture killed it
suite: scoped feature run 1131 passed, 0 failed, 107s (full workspace not run)
---

# TDD Verification: Overview cards, full text without truncation

**Verdict: PASS_WITH_GAPS.** No HIGH smell, no weakened existing test, every FR/SC has a test, and 10 of 11 distinct deliberate mutants were killed; the gaps are weak ordering evidence on 5 guard/after-the-fact rows, one unit test that does not pin the text width it names (M7), and the full suite not being run here.

This audit is independent: the auditor did not write these tests.

## Test-first evidence

No `tdd/test-list.md` exists (cycle log header: tasks.md's test tasks are the list). History is one commit per cycle with test and source in the same commit (inline `#[cfg(test)]`), so order inside a commit cannot be shown by git; the red is the cycle log's recorded command and failure output. Class `PROVEN` below means a red command with failure output is recorded and the test is in the same commit as, or before, its source. Nothing is `TEST_AFTER` or `NO_TEST`.

| Behaviour (cycle-log row) | Class | Evidence |
|---|---|---|
| M1 T001/T004 tint, T002/T005 clamps, T003, T007, T006 (all) | PROVEN | red output per row; `87e24cb8`, `af6564bf`; three T001 rows passed at red and were shown to fail by deliberate mutants |
| M1 review A F1 (slot reuse at another role) | PROVEN | red recorded |
| M2 T011/T012 K6, T016, T009/T017 K5, T010/T018 K3 | PROVEN | red recorded; `84946927` |
| M2 T013/T019 (part), T014/T015 grid, review r1 F1-F4 | PROVEN | red recorded; `dafbd062`, `5e6efd07`, `894e0fde` |
| M2 T013/T019 (rest): header/title outside scroll, shadow, edges | LIKELY-WEAK, counted PROVEN | implementation landed first; tests written against it and shown red with one combined mutant (`14625598`). Ordering is after-the-fact, evidence is a mutant, not a red |
| M3 T024/T025/T028, T026/T029, T027/T030/T032, review A F1-F4, r2 F1-F2 | PROVEN | red recorded; `4c8e6066`, `17ec27a1`, `b28fbd39`, `f48a1f7d`, `7dd8a2f1` |
| M3 T031 (FR-004 no tooltips) | after-the-fact red | log: "written with the removal (no separate red run)"; red shown by three one-off mutants (`the_title_branch_and_activity_have_no_tooltip`); each failed |
| M3 T033 (`overview-expanded` covered state) | NOT_APPLICABLE | fixture-feeding gates over every state; no red run possible, red is the state's absence |
| M4 K4, T036/T040, T042, T038/T043, T039/T044, review A F1-F3 | PROVEN | red recorded; `1354d981`, `f23d233c`, `41542127`, `1b5e6d5d`, `a744df78` |
| M4 r2 (`2f96f10e`, `0c183775`): removed opening card drops its opening focus; inner-scroll inset snapshot | no cycle-log row | see finding 2 |

Existing tests: the diff `1952d53a..HEAD` removes assertions and tests only in `crates/micold-core/src/overview_text.rs` and `crates/micold-client/src/ui/material/overview_card.rs`. Both are the tooltip tests (`title_tooltip`, `branch_line`/`activity_line` tooltips, `the_full_title_is_in_the_tooltip`) deleted with the tooltips by FR-004 / T031, replaced by `the_title_branch_and_activity_have_no_tooltip` and `ExpandedText` tests. Not weakening. Removed assertions seen in `daemons_section_geometry.rs`, `features_pr_changes.rs` and `review/comment.rs` belong to other features' interleaved commits. No `#[ignore]`, skip, filter rename, or threshold change in the feature's files.

`tasks.md` vs evidence: every behavioural task is ticked; open are T047 and T048 (close unit, quickstart run and visual-pass record), correct as of now.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | MED | The inner-scroll inset test pins the bar but not the text width. Dropping `- clear` from the body's right padding (text loses `BAR_FOOTPRINT` of width) passes `an_expanded_cards_inner_scrollbar_clears_the_grids_scrollbar`; only the `layout_snapshot` fixture catches it. The test's text check is an upper bound only (`b.x + b.width <= width - MD`). | `crates/micold-client/src/ui/material/overview_card.rs:544` (mutant M7), test at `:1128-1145` |
| 2 | MED | Two M4 review-round-2 behaviours have no cycle-log row: `a_removed_opening_card_drops_its_opening_focus` (test exists, `overview_expansion.rs:485`) and the inner-scroll inset (`0c183775` regenerated the fixture; the unit test was added in `2f96f10e`). Red evidence cannot be read from the log; mutant M4 did kill the first test, so it is real, but ordering is unproven. | `tdd/cycle-log.md` ends at "M4 review A F3"; `2f96f10e`, `0c183775` |
| 3 | MED | Sleepy test: the wheel-latching test sleeps a real 120 ms for iced's wall-clock scroll transaction. Justified in a comment, but a fixed sleep. | `crates/micold-client/src/ui/material/card_grid.rs:2441-2442` |
| 4 | LOW | Implementation coupling: the Escape test reads the private `focused` field. | `crates/micold-core/src/overview_expansion.rs:558` |
| 5 | LOW | `EXPAND_REST` has no spec'd floor: shrinking it to 1 ms survives in core and the grid (the grid test derives its times from the constant). Equivalent with respect to the spec (SC-006 gives only the 0.5 s ceiling, K2 pins it and non-zero). | `crates/micold-core/src/overview_expansion.rs:12`, mutants M11/M11b |
| 6 | LOW | Several K1 tests are eager and use unlabelled `assert_eq!` chains (e.g. `an_open_menu_freezes_the_form`, `a_window_blur_collapses_and_nothing_re_expands`); a failure does not say which bullet broke. | `overview_expansion.rs:535-613` |

## Mutation results

Tool: none installed (profile `mutation: null`). Deliberate mutants only: 13 runs (11 distinct; M7 and M11 were each run against two test sets), one at a time, each restored with `git checkout -- <file>` and the tree checked clean after each. Sample of high-risk behaviours: FR-001 clamps, D1 Escape order, FR-013 opening focus, FR-005 focus loss under the pointer, FR-008 placement, the inner scrollable's inset, FR-012 menu freeze, SC-006 rest delay.

| Mutant | Behaviour | Survived | Judgment |
|---|---|---|---|
| M1 `overview_card.rs` title `LineClamped` 3 to 4 lines | FR-001 | No | killed by `the_title_wraps_onto_up_to_three_lines_then_ends_in_an_ellipsis`, `a_long_title_branch_and_activity_end_in_an_ellipsis_at_every_width` |
| M2 activity 2 to 3 lines | FR-001 | No | killed by `the_activity_wraps_onto_up_to_two_lines_…` and the same width test |
| M3 `app.rs::on_escape` expanded-card arm disabled | D1 | No | killed by `--test overview` `escape_collapses_an_expanded_card_before_it_closes_the_overview` |
| M4 `overview_expansion.rs` opening-focus early return disabled | FR-013 | No | killed by `the_opening_focus_expands_nothing`, `a_removed_opening_card_drops_its_opening_focus` |
| M5 focus loss under the resting pointer always collapses | FR-005 | No | killed by `a_focus_loss_under_the_resting_pointer_keeps_the_card_by_hover` |
| M6a `place` height not capped at the view | FR-008 | No | killed by 3 `overview_expansion` tests |
| M6b `place` never shifts up | FR-008 | No | killed by 2 tests (`…moves_up_to_its_bottom`, `…capped_and_scrolls`) |
| M7 body right padding without `- clear` | inset (M4 r2) | Yes in `--lib overview_card` (52 passed) | killed by `layout_snapshot` `the_layout_matches_the_committed_fixture` (M7b); finding 1 |
| M8 pointer leave ignores an open menu | FR-012 | No | killed by 3 `overview_expansion` tests (`an_open_menu_freezes_the_form`, `a_leave_during_…`, `a_rest_held_by_the_menu_…`) |
| M9 inner scrollable not narrowed by `BAR_FOOTPRINT` | inset (M4 r2) | No | killed by `an_expanded_cards_inner_scrollbar_clears_the_grids_scrollbar` |
| M11 `EXPAND_REST` 400 ms to 1 ms (core) | SC-006 | Yes | equivalent w.r.t. spec (finding 5) |
| M11b same (grid rest timer) | SC-006 | Yes | equivalent w.r.t. spec (finding 5) |

Of 11 distinct mutants: 9 killed by targeted tests, M7 killed by the fixture only, M11 judged equivalent. No survivor sits inside a behaviour with a missing test.

## Traceability

| Criterion | Tests (real entry point first) | End to end |
|---|---|---|
| FR-001, SC-004 | `overview_card::tests::the_title_wraps_…`, `the_activity_wraps_…`, `line_clamp` tests; `card_grid::a_wrapped_title_sets_its_rows_height_…` | Yes (mounted card) |
| FR-002, SC-001 | `--test overview` `a_rested_card_is_expanded_in_the_view`, `keyboard_focus_expands_a_card_…`; `ui::overview::tab_moves_report_card_focus_to_the_expansion` | Yes |
| FR-003 | `an_expanded_card_casts_the_menu_shadow`, `an_expanded_card_is_drawn_after_every_other_card` | Yes (mounted) |
| FR-004 | `an_expanded_card_shows_every_text_in_full_inside_it`, `the_expanded_text_holds_every_text_in_full`, `the_title_branch_and_activity_have_no_tooltip` | Yes |
| FR-005 | core K1 tests; `--test overview` focus/Escape tests; `expansion_window_blurred_…`; `the_window_losing_focus_publishes_the_expansion_blur` | Yes |
| FR-006 | `a_rest_on_another_card_expands_only_that_card`, `focus_expands_the_focused_card` | Core only (single `Option` by type) |
| FR-007, SC-002 | `no_wrapped_text_leaves_the_card`, `a_card_with_every_row_fits_at_every_card_width`, `overview-expanded` layout_snapshot gates | Yes |
| FR-008 | K3 `place` tests, `an_expanded_card_in_the_last_row_is_moved_up_into_the_view`, `a_wheel_over_a_tall_expanded_card_…`, `page_down_on_a_focus_expanded_card_pages_it`, `expansion_card_paged_scrolls`, M4 r1 F2 | Yes |
| FR-009, SC-003 | `an_expanded_card_keeps_every_slot_and_takes_its_own`, `an_expanded_card_takes_new_text_in_place` | Yes |
| FR-010, FR-011, SC-005 | `the_part_over_a_neighbour_belongs_to_the_expanded_card`, `an_expanded_card_activates_as_the_collapsed_one`, `an_expanded_cards_ring_mark_and_ripple_cover_its_bounds`, `the_twin_takes_no_focus_traversal`, `tab_order_is_the_same_with_a_card_expanded` | Yes |
| FR-012 | `a_open_menu_freezes_the_form` family (core), `an_open_card_menu_freezes_the_expansion`, `a_chosen_menu_entry_unfreezes_the_expansion`, `escape_with_a_card_menu_open_only_closes_the_menu` | Yes |
| FR-013 | `the_opening_focus_expands_nothing`, `the_opening_focus_expands_no_card`, `expansion_none_after_the_opening_focus` | Yes |
| FR-014 | `a_directory_inside_the_home_folder_starts_with_a_tilde`, `a_directory_outside_…`, `each_card_carries_its_directory` | Yes (D2 `~` form) |
| FR-015 | `an_expanded_cards_texts_keep_the_collapsed_cards_colours`, `the_title_is_17_px_the_body_14_…`, `overview-expanded` contrast gates | Yes |
| FR-016, FR-018 | core module is pure (`overview_expansion` tests need no I/O); write-isolation scan `state_scan.rs` classifies `Expansion::apply`/`expanded` | Gate-level |
| FR-017 | docs gate (user guide) | Not a behaviour test |
| SC-006 | `the_rest_is_under_half_a_second`, `a_resting_pointer_publishes_the_cards_rest_then_its_leave` | Core + grid |

Untested criteria: none. Tests tracing to nothing: none found among the feature's tests.

## What was not audited

- The full workspace suite and the `scripts/tests/*.sh` / sandbox suites: only the feature's scope was run (`-p micold-core --lib overview_`, `-p micold-client --lib ui::`, `--test overview`, `--test idle_requests_no_frames`, `--test layout_snapshot`, `--bin micold-ai-ide expansion_`): 45 + 833 + 14 + 72 + 62 + 3 passed, 0 failed, 107 s, run before and after all mutants.
- Mutation: 11 distinct deliberate mutants, not an exhaustive tool run; `card_grid.rs` internals (rest timer, twin measurement, wheel chaining), `keyboard_focus.rs`, `line_clamp.rs` beyond the clamp counts and `main.rs` wiring were not mutated beyond `app.rs::on_escape`.
- Coverage: no tool installed.
- Test-first ordering inside a commit: git cannot show it; only the self-reported cycle log.
- Smell pass: full catalogue read on `overview_expansion.rs` tests and sampled via diff greps (ignore, sleep, conditionals) on the rest; the 900+ line card/grid test modules were not read line by line.
- Performance: no criterion; the scoped run took 107 s of which almost all is build.
- Cross-platform (Windows/macOS) legs: not runnable on this host.
