---
feature: 1185-jira-selector-filters
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: df6c4089f
behaviors: 45 # tasks T002-T045 (no tdd/test-list.md; the tasks.md test tasks are the list)
proven: 0
likely: 45 # cycle-log red is compile red / mutation red on the committed tree, never a per-cycle recorded run
test_after: 0
no_test: 0
high_smells: 0
criteria_total: 17 FR + 5 SC + 3 stories
criteria_covered: all
mutation_score: n/a # no mutation tool; 8 deliberate mutants, 8 killed
mutants_survived: 0
suite: 11185 passed, 0 failed, 18 ignored (workspace gate, GATE_EXIT=0)
---

# TDD Verification: Jira selector filters (#1185)

**Verdict: PASS_WITH_GAPS.** No HIGH smell, no weakened test, every criterion has a test and all 8 deliberate mutants
were killed; the evidence of test-first is weaker than the rubric's `PROVEN` (red is reconstructed after the fact, and
the M4 panel Tab fix has no log row at all).

Scope: M1 (#1381) and M2 (#1396) are merged on main (`3b7ae99fd`, `1888ee79f`); M3 and M4 are the 12 unmerged commits of
`git log origin/main..HEAD` (`65b6c98cc` .. `df6c4089f`). `--no-tasks`: tasks.md untouched. The audit was run by a
worker that did not write these tests, but it read the cycle log, the spec and the files cold.

## Test-first evidence

The cycle log says so itself (cycle-log.md:3-11): tests and code were written task by task, red proven afterwards by
compile red against the previous milestone's `src/` and by mutation. History corroborates the shape: every feat commit
carries its tests in the same commit (`65b6c98cc`, `a66433912`), none lands source ahead of tests.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| M1-M4 rows of cycle-log.md (T002-T045) | LIKELY | red recorded as compile red / mutation red, reproduced on the tree; commit carries test and source together, so order is not provable |
| Filters panel Tab cycle and Escape (`dfe512ab4`, `picker.rs:1059`, `:1113`) | LIKELY (weak) | no cycle-log row; tests and fix in one commit; the red is the visual pass's finding in the commit doc comment (`picker.rs:1054-1058`) |
| Panel nested-overlay gate (`b380c0405`, D16) | LIKELY | no cycle-log row; the commit adds the gate and the harness change together |

Existing tests: none weakened. `git diff origin/main..HEAD` shows removed lines in `jira_filters_fit.rs`, `jira_filters_picker.rs`
and `support/layout.rs`; each is the inline filter row's gate rewritten for the Filters panel (ledger D15), the 4-interaction SC-001
test renamed `five_interactions_reach_the_two_clause_listing` (`jira_filters_picker.rs:1104`, 4 → 5, matching spec SC-001's amendment),
or a diff alignment artefact (`advanced_mode_neither_applies_nor_drops_the_chosen_filters`, `:801`, is intact with every assertion). No `#[ignore]` in the feature's
tests, no filter or threshold change. tasks.md: every task T001-T045 ticked with a cycle-log row; T046 (close unit) is correctly open.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | MED | The Filters panel's Tab/Shift+Tab/Escape cycle (FR-015) is tested only in the widget's own fixture (trigger and two fields); no test drives the real four-field panel of the New worktree dialog, and `jira_filters_picker.rs` has no Tab test. Quickstart step 9 (manual/visual) is the only end-to-end check. | `crates/micold-material/src/cdk/picker.rs:1059,1113`; no `Tab` in `crates/micold-client/tests/jira_filters_picker.rs` |
| 2 | MED | No cycle-log row for the M4 panel keyboard fix (`dfe512ab4`) or the D16 gate (`b380c0405`): test-first is unrecorded for the two latest changes. | `tdd/cycle-log.md` ends at T044/T045 |
| 3 | MED | SC-002's 16-combination test is partly self-referential: the fake returns the one scripted page whatever the clauses, and the recorded clauses are compared with `filters.clauses()` (the code's own output), not with literal JQL. The literal clause text is pinned elsewhere (`jira_filter_clauses.rs`, adapter tests), so the combination test only proves clauses are forwarded. | `crates/micold-core/tests/jira_filter_choices.rs:187-212` |
| 4 | LOW | Cycle-log "red" for most rows is compile red by construction ("by construction" rows: T024, T025, T031-T037), which proves only that the API did not exist, not that the behaviour failed first. | `tdd/cycle-log.md:39,40,52-54` |
| 5 | LOW | No `tdd/test-list.md`: no `traces` column, so traceability below is derived from tasks.md and test names. | `specs/1185-jira-selector-filters/tdd/` |

## Mutation results

No mutation tool in the profile. 8 deliberate mutants, one at a time, on high-risk behaviour, each against the narrowest target and restored with `git checkout` (re-run green after: `jira_filter_clauses` 11, `jira_filter_choices` 26, `jira_filters_picker` 49, `cdk::picker` 2). A sample, not exhaustive (not sampled: adapter request building, the debounce, the failure/Retry rows, redaction).

| Mutant | Behavior | Survived | Caught by |
| --- | --- | --- | --- |
| M1 `jira_filter.rs` `jql_string`: escape `"` before `\` | FR-010 escaping | No | `a_value_is_quoted_with_backslash_then_quote_escaped`, `every_value_of_the_corpus_is_exactly_one_quoted_string`, `narrowed_inserts_the_clauses_before_order_by` |
| M2 `label_choices`: drop `labels.dedup()` | FR-005 no duplicates | No | `label_choices_hold_every_label_once_sorted_case_insensitively_ties_by_exact_text` |
| M3 `label_choices`: case-sensitive sort | FR-005 alphabetical | No | same test |
| M4 `others`: `!=` → `==` (keep only the own user) | FR-004 own user once | No | `the_own_user_is_removed_from_the_people_results` |
| M5 `reporter_choices`: people before the own user | FR-004 own user first | No | `a_people_search_failure_keeps_the_own_user_offered`, `two_people_sharing_a_name_stay_distinct`, `typing_waits_for_the_debounce_and_a_newer_text_discards_an_older_result` |
| M6 `ClearedAll`: `reload = had` → `false` | FR-002/FR-008 Clear filters | No | `any_clears_the_component_and_clear_filters_clears_all_each_reloading` (+2) |
| M7 panel cycle: forward step `+1` → `+2` | FR-015 Tab order | No | `tab_and_shift_tab_cycle_through_the_trigger_and_the_open_panel`, `escape_in_the_panel_…` |
| M8 panel: Tab guard `trigger.is_some()` → `false` | FR-015 panel keeps Tab | No | same two |

Score: 8 of 8 killed; 0 survivors to triage.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 AS1-AS5, FR-003, FR-007, FR-008 | `focusing_component_asks_once…`, `picking_a_component_reloads…`, `a_component_and_a_fix_version_load_with_both_clauses_anded`, `any_clears_the_component_and_clear_filters…`, `rapid_changes_with_answers_out_of_order…` (`jira_filters_picker.rs`) | Yes (reducer through the app `State`, `FakeJiraSource` shell in `main_tests.rs::jira_filter_shell`) |
| US2, FR-004, FR-006 | `focusing_reporter_asks_for_the_own_user_once…`, `two_people_sharing_a_name_stay_distinct`, `jira_token_never_leaks` (3), `jira_is_requested_only_on_named_events` (8) | Yes |
| US3, FR-005 | `the_label_filter_offers_the_loaded_labels…`, `a_filtered_load_never_replaces_the_label_choices`, `a_filter_chosen_before_the_unfiltered_load_returns…`, `label_choices_*` | Yes |
| FR-009, FR-017 | `search_text_narrows_within_the_filtered_list…`, `a_key_typed_with_filters…`, `a_pick_from_a_filtered_list_fills_the_form…` | Yes |
| FR-010, SC-003 | `jira_filter_clauses` (11, corpus ≥ 20 asserted at `:84`), adapter tests | Yes (adapter POSTs the narrowed JQL) |
| FR-011-FR-013, SC-004 | `a_filtered_list_with_no_issue_says_so…`, `each_choice_failure_shows_its_cause…`, `a_versions_failure…`, `a_myself_failure…` | Yes |
| FR-014 | statement text in `jira_filters_picker.rs`, `jira_token_never_leaks` | Yes |
| FR-015 | `the_keyboard_reaches_every_row_picks_with_enter…` (lists), widget Tab tests (panel) | Partial: panel Tab only at the widget level (finding 1) |
| FR-016, edge cases | `leaving_jira_or_the_source…`, `a_filter_chosen_in_one_dialog_leaves_another_unchanged`, `advanced_mode_…` | Yes |
| SC-001 (five interactions) | `five_interactions_reach_the_two_clause_listing` | Yes |
| SC-002 | `every_combination_lists_exactly_the_issues_scripted…` | Yes, weak (finding 3) |
| SC-005 | `rapid_changes_with_answers_out_of_order…` | Yes |

Untested criteria: none. Tests tracing to nothing: none found.

## What was not audited

- Mutation was a sample of 8 deliberate mutants, not a tool run; the adapter (`jira_http.rs`), the debounce, Retry rows, the redacting `Debug` and the layout gates were not mutated.
- Smell pass read the core tests in full and the client/picker tests by sampling; no rubric item was found, but `jira_filters_picker.rs` (about 1,700 lines) and `layout.rs` were not read line by line.
- Visual and geometry behaviour (the layout fixture, `jira_filters_fit`) was taken from the green gate, not re-judged; `layout_snapshot.txt` re-recordings (`df6c4089f`) were not reviewed for self-approval beyond the cycle log's account.
- Coverage: no tool in the profile. Performance and flakiness: only the single green gate run was observed.
- M1/M2 history was read from the cycle log and the merged commits' subjects, not diffed line by line.
- During the run `specs/1185-jira-selector-filters/autopilot.md` was modified by the orchestrator, not by this audit.
