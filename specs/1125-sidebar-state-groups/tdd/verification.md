---
feature: 1125-sidebar-state-groups
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 42c09604 # short SHA audited (origin/main after M5, PR #1386)
behaviors: 14 # rows of the table below; no tdd/test-list.md exists, tasks.md's test tasks stand in
proven: 0
likely: 0
test_after: 12
no_test: 0
not_applicable: 2
high_smells: 1 # finding 2, fixed in the close PR
high_findings: 2 # finding 1, the test-first breach (12 behaviors), and finding 2
criteria_total: 23 # FR-001..FR-023
criteria_covered: 23 # 20 by tests; FR-018 by pre-existing tests the feature left unchanged; FR-022 docs and FR-023 a grep
mutation_score: unmeasured # no mutation tool in the profile; 6 deliberate mutants, 6 caught
mutants_survived: 0
suite: targeted, green — core lib 1663; client lib 936, features_sidebar 101, sidebar_tree 49, sidebar_state 46, navigation_apply 14, features_navigation 1; full workspace through scripts/gate.sh
---

# TDD Verification: Sidebar grouped by state, with Overview as its first row

**Verdict: FAIL.** No behavior has a recorded red: there is no `tdd/cycle-log.md` and every feature
commit adds its tests and code together, so by the rubric all twelve behaviors are `TEST_AFTER`.
Test strength is good: all six deliberate mutants on the highest-risk behaviors were caught, and
the one `HIGH` smell (a loop that could assert nothing) was fixed in the close PR.

Independence: run inline by the close unit, which did not write the tests (M1–M5 were other units);
the smell pass ran in a fresh-context reviewer (`claude -p --agent autopilot-reviewer`), and each of
its cited lines was re-read here before it went into this report.

## Test-first evidence

There is no `tdd/test-list.md` and no `tdd/cycle-log.md` (the rubric's `BLOCKED` condition; the
skill allows a weaker audit against `spec.md`, which this is). tasks.md marks each test task
"failing tests"; the ledger records for M5 that T052/T053 were "run red first, by compile", but no
red command or output was kept. The history is rebase-merged per milestone, one feature commit plus
review-fix commits each, with tests and source in the same commits.

| Behavior (tasks) | Class | Evidence |
| --- | --- | --- |
| T001/T002 `sidebar_groups` G1–G6 (core) | TEST_AFTER | `ac341944` adds tests and code together; no red kept |
| T003/T004 `location_card_state` extraction | NOT_APPLICABLE | behavior-preserving move under the Overview's existing card tests, plus new direct tests |
| T005/T012 `sidebar_groups()` order, membership | TEST_AFTER | `ac341944` |
| T006/T013 `sidebar_list()`, `GroupToggled`, per-project choices | TEST_AFTER | `ac341944`; `9c94754b` reworked with review A |
| T007/T014 row metrics over `&[ListRow]` | TEST_AFTER | `ac341944`; `774331d5` fold fix with its test |
| T008/T015 chords in grouped order, reveal opens a closed group | TEST_AFTER | `ac341944`, `49fc82d3` (fix and test together, after review B) |
| T009/T011 `TreeItem::heading()` | TEST_AFTER | `ac341944` |
| T010, T024, T034 contrast pairs | NOT_APPLICABLE | characterization of theme tokens, green on first run by nature |
| T020/T025 `sidebar_second_line` | TEST_AFTER | `11732531` |
| T021/T022/T026/T027 two-line rows, row text, chips | TEST_AFTER | `11732531`, `44deeb1e` |
| T032/T033/T035–T038 `Opened`, `row_selected`, Overview row | TEST_AFTER | `944a4c9e`, `916f8f43` |
| T042/T045/T046–T049 chip bar, ⋯ menu, Tab order, icon | TEST_AFTER | `1a94fdba`, `1c183649` |
| T052/T055 keyed `TreeView` rows keep focus and hover | TEST_AFTER | `9884632e`; ledger says red by compile, not recorded |
| T053/T057 `hover_holds` | TEST_AFTER | `9884632e`, `774331d5` |

Tasks against list: T001–T058 are ticked and each names tests that exist and pass; no ticked task
lacks a test. T059–T060 are this close unit's.

Existing tests changed by the feature (every removed or rewritten assertion in `crates/*/tests`):

| File | Before / after | Judgement |
| --- | --- | --- |
| `tests/gates/containment.rs` | `CLIP_REVEALED` two paths to `&[]`; `the_recorded_escapes_are_the_accordion_reveal` removed | legitimate: the filter accordion it attributed is gone (FR-014, T050); the staleness check stands |
| `tests/layout_text_overflow.rs`, `overlay_registry.rs`, `overlay_dismissal_delta.rs`, `overlay_dispatch_ordering.rs`, `sidebar_state.rs` | `filter_open` / `SidebarFilterPanel` assertions moved to `more_menu_open` / `SidebarMoreMenu` | legitimate migration (T043, T044) |
| `tests/jira_status_sidebar.rs` `the_chip_stands_beside_the_key_tag` | `shows("ABC-1")` to `!shows("ABC-1")`, renamed `the_chip_stands_on_the_rows_second_line` | legitimate: FR-013 drops the issue tag chip |
| `tests/features_sidebar.rs`, `overview.rs` | `row_heights`/`current_session_row` counts re-expressed over `ListRow` with headers | legitimate (T014), same rule, new list shape |
| `tests/sidebar_state.rs` hover | `hovered == Some("feat-a")` to `Some(("feat-a", 3))` | legitimate (T057), strengthened |
| `tests/icons.rs` | curated set 81 to 82 | legitimate, new icon |

No assertion was loosened, no test skipped or renamed out of a filter, no threshold lowered.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Twelve behaviors are `TEST_AFTER`: no red kept for any of them. Test strength was shown afterwards (all mutants caught), ordering was not. Accepted: history cannot be rewritten. | table above |
| 2 | HIGH (fixed) | `each_entrys_group_is_its_card_states` skipped every non-worktree entry and never asserted it checked any, so an empty grouping passed. Fixed in the close PR: it counts the worktrees it checks and asserts all five. | `tests/sidebar_state.rs:927` |
| 3 | MED | `the_reveal_offset_counts_headers` ends with `reveal_scroll_offset().is_some_and(\|y\| y > 0)`; the header count is pinned by the row index above it (`2 + 1 + 29`, explained in its message), the offset only smoke-checked. | `tests/sidebar_state.rs:1276` |
| 4 | MED | Assertion roulette and eager tests: `the_groups_come_in_state_order_with_todays_order_inside` has bare `assert_eq!`s without the rule message the profile asks for; `a_choice_holds_for_the_run_per_project` covers moving rows, a re-read and a project switch in one test. | `tests/sidebar_state.rs:898`, `:1144` |
| 5 | MED | Implementation coupled: per-project choice tests read `state.sidebar.group_choices` (`contains_key`, `is_empty`) besides the header's `open`. | `tests/sidebar_state.rs` `state_groups` |
| 6 | LOW | Duplicated setup: `location_of`, `grouped`, `header` in `sidebar_state.rs` and a `rows()` wrapper in `features_sidebar.rs` could live in `tests/support/sidebar_fixture.rs`; `location_of` maps a run group to a made-up `Worktree("group …")` location. | `tests/sidebar_state.rs:865` |
| 7 | LOW | Redundant/unclear: `g2_order_*` pins one order three ways; `g1_`…`g6_` names carry contract ids rather than behaviors; `g6_run_group_state` packs several cases. | `crates/micold-core/src/sidebar_groups.rs` tests |

Declined from the smell pass: "`row_metrics_count_headers_and_skip_closed_groups` asserts only
inside `if matches!(row, Header)`" — the code is a `match` whose both arms assert, so every row is
checked; the header count is pinned by `the_groups_come_in_state_order_with_todays_order_inside`.

No other `HIGH` smell (assertion free, tautology, doubled subject, over-mocking, self-approving
snapshot: the layout fixture was regenerated and its diff read per quickstart, skipped tests): none
found. Isolation and determinism are fine (no clock, network or sleep; fixtures from
`tests/support/sidebar_fixture.rs`); the targeted suites run in seconds.

## Mutation results

No mutation tool in the profile. Six deliberate mutants, one at a time, each restored from the
saved original and confirmed by `git status` clean; the full gate re-ran green afterwards. Sampled:
the open rule, the group mapping, selection under the Overview, the hover hold, keyed rows, and the
reveal into a closed group. Not exhaustive.

| Mutant | Behavior | Survived | Caught by |
| --- | --- | --- | --- |
| `sidebar_groups.rs` `starts_open` returns `true` | FR-005 Done collapsed | No | `g5_open_rule` |
| `sidebar_groups.rs` Needs attention maps to Working | FR-001, US1.2 | No | 3 core tests |
| `features/sidebar.rs` `row_selected` ignores `overview_open` | FR-009 | No | `while_the_overview_is_open_no_location_or_session_row_is_selected` |
| `features/sidebar.rs` `hover_holds` true whenever the row is drawn | US5.5, R14 | No | `hover_holds_only_while_the_row_is_where_it_was_hovered` |
| `tree_view.rs` keyed layout disabled | FR-017, US5.4, R13 | No | `a_keyed_row_keeps_the_keyboard_…`, `…_its_hover_state_…` |
| `features/sidebar.rs` reveal ignores the chord's keyboard row | FR-006 | No | two `navigation_apply` `into_a_closed_group` tests |

## Traceability

| Criterion | Tests | Through the real entry |
| --- | --- | --- |
| FR-001, FR-003, FR-020, SC-001 | core G1–G3; `sidebar_state` `state_groups` order and membership | Yes: `app::State::sidebar_groups` |
| FR-002 | core G4 `header_text`; `a_group_a_filter_empties_is_not_shown` | Yes |
| FR-004, SC-007 | `sidebar_state` `a_state_change_moves_the_row_and_both_counts` | Yes: `State::update` |
| FR-005 | core G5; `sidebar_state` `done_starts_collapsed_and_the_others_open` | Yes |
| FR-006 | `navigation_apply` `into_a_closed_group`, `the_current_sessions_closed_group_opens_on_its_reveal` | Yes: reveal path |
| FR-007, FR-008, SC-002 | `features::overview` `Opened` tests; `ui/sidebar.rs` mounted view tests; badge equality in `ui/overview.rs` | Partly: mounted builders, not the window |
| FR-009, FR-010 | `row_selected` tests; `tree_view` primary selection | Yes / mounted |
| FR-011, FR-012, SC-003 | core T1 `sidebar_second_line`; `features_sidebar` two-line text and tooltip | Yes |
| FR-013 | `jira_status_sidebar`, `features_sidebar` chips | Yes |
| FR-014, FR-015, FR-016 | `features_sidebar` chip bar; `overlay_registry`, `overlay_dispatch_ordering`; `layout_text_overflow` | Yes |
| FR-017, SC-005 | `ui/sidebar.rs` Tab order (T035, T045); `tree_view` keyed rows; `hover_holds` | Mounted |
| FR-018 | pre-existing ripple and context-menu tests (`context_menu_anchor`), unchanged | Yes |
| FR-019, SC-004 | `composition_contrast` pairs, light and dark | n/a (tokens) |
| FR-021 | `showcase_completeness` | Yes |
| FR-022 | user guide sections; visual pass | docs |
| FR-023 | quickstart §A grep, run at close: nothing logged | grep |
| SC-006 | the pre-1125 filter tests (`features_sidebar` `active_filters_combine_with_or_rather_than_and`, `sidebar_state` `default_entry_stays_visible_with_an_active_tag_filter`) unchanged; `the_current_sessions_worktree_survives_a_filter_in_its_group` | Yes |

No test traces to nothing; no criterion lacks a test or check.

## What was not audited

- Ordering cannot be shown for any behavior (no cycle log); the classification is from history.
- Mutation was a sample of six behaviors, not a tool run; the view builders in `ui/sidebar.rs`
  (wiring of keys, chips and the Overview row into the window) were not mutated.
- The smell pass covered the tests in the feature's diff, not every pre-existing sidebar test.
- Performance of the grouped list with very many worktrees was not measured.
- The live visual behaviors no test can see (badge live, chord into Done) are in `visual-pass.md`.
