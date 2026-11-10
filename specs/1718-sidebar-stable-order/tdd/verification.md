---
feature: 1718-sidebar-stable-order
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset found
verified_at: 16d873e2e
behaviors: 13 # no tdd/test-list.md; taken from the two tables of tdd/cycle-log.md
proven: 8
likely: 0
test_after: 5
no_test: 0
high_smells: 5
criteria_total: 11 # the acceptance scenarios US1.1-3, US2.1-2, US3.1-2, US4.1-4
criteria_covered: 6 # reached through the real entry point; US2.1, US2.2, US4.1, US4.2, US4.4 are not
mutation_score: unmeasured # no mutation tool in the profile; 9 deliberate mutants on changed files, 4 caught, 5 survived
mutants_survived: 5 # 4 of them in DONE behaviors, none equivalent
suite: "targeted only: 66 lib + 86 main passed, 0 failed (filters below); full workspace not run"
---

# TDD Verification: Sidebar keeps a stable order; state shows on the row (#1718)

**Verdict: FAIL.** The whole of M1 (five behaviors) is `TEST_AFTER`: no red run was recorded and tests and
production landed in one `wip` commit. Separately, the row-level wiring that M2 exists to deliver
(card state to glyph, tint and tooltip in `ui/sidebar.rs`) is covered by no test; deliberate mutants there
all survived.

Independence: this audit was run from a cold context by an agent that did not write the tests. No
subagent smell pass was used; every cited line was opened by the auditor.

## Preconditions and caveats

- **There is no `tdd/test-list.md`.** The feature was not planned through the TDD extension (rubric: the
  feature has no test list, which by the verdict table is `BLOCKED`). The audit was run anyway against
  `spec.md`, `tasks.md` and the cycle log, as the skill allows; behaviors below are the cycle-log rows, so
  "behaviors" and the `traces` check are weaker than for a listed feature. Ordering evidence is therefore
  per cycle-log row, not per list id.
- `tasks.md` has T001-T030 all ticked `[x]`. With no list there is no `DONE` state to cross-check; I found no
  tick that a test or the history contradicts, except that the evidence behind the M1 ticks is `TEST_AFTER`.
- History audited: PR #1758 (M1) and PR #1791 (M2) on `origin/main`. They were rebase-merged, so the
  individual commits survive and ordering is checkable. `a947fd7fc` = PR commit `8331bc895`, `2603dd6af` =
  `4bc70c9b8`, `d75740311` = `0229c7f08`; `22e3054a4` = `e8720ad8e`, `bcf3ae39b` = `e085a674f`,
  `906166c10` = `05cae8c9a`.
- Stack profile read (`.specify/memory/tdd-profile.md`): no coverage, mutation or property tooling; tests
  are libtest, doubles are the `Fake*` types, fixtures are `tests/support/*` (here
  `tests/support/sidebar_fixture.rs`).

## Test-first evidence

| Behavior (cycle-log row) | Class | Evidence |
| --- | --- | --- |
| M1 Card state change never moves a row (SC-001, FR-001) | TEST_AFTER | log: "no per-behaviour red run"; `a947fd7fc` (`wip(1718): flat sidebar list, tests, docs, snapshot (M1, unreviewed)`) changes `features/sidebar.rs` and `tests/sidebar_state.rs` together |
| M1 No header row or toggle (FR-002) | TEST_AFTER | same commit; "compile failure on origin/main" is argued by construction, no command or output recorded |
| M1 Keyboard and reveal walk the flat list (FR-006) | TEST_AFTER | same commit (`navigation.rs` + `navigation_apply.rs`) |
| M1 One row height per location (G4) | TEST_AFTER | same commit (`features_sidebar.rs`) |
| M1 Old state file loads (FR-008) | TEST_AFTER | same commit; no production change was needed, so the test is characterization at best (see finding 9) |
| M2 Five states, five glyphs, one slot width (SC-002) | PROVEN | red command `cargo check -p micold-client --lib --tests`, 35 + 5 errors listed; `22e3054a4` touches only test code, `bcf3ae39b` adds the production after |
| M2 Glyph colour is the tint ink | PROVEN | same red and order |
| M2 Tint by card state (G7) | PROVEN | same |
| M2 Contrast of glyph and text on tints (G9, SC-005) | PROVEN | same |
| M2 Tooltip state line, Done reads "Done (merged)" (G10) | PROVEN | same; verified red is the missing `state_line` |
| M2 Order unchanged while the tint changes | PROVEN | same |
| M2 Selection distinct from tint (G8) | PROVEN | same |
| M2 Showcase poses five states and tints (T020, T028) | PROVEN | same |

Log vs history: the M2 log matches the history (test-only commit first, production after). The M1 log is
candid and agrees with the history (`a947fd7fc` mixes both). The red output for M2 is paraphrased, not the
verbatim output; accepted as a recorded red command with its failure.

Unlogged production change: `906166c10` (message "accept sidebar layout snapshot...") also changed the run
group row's tooltip to carry its state line (`ui/sidebar.rs:1159`). It has no cycle-log row and no test
(mutant 2 below survived). Treat it as `TEST_AFTER` / `NO_TEST` for that behavior; it is not counted in the
13 above because it is not a logged behavior.

### What the change did to tests that already existed

| Where | Before | After | Judgment |
| --- | --- | --- | --- |
| `crates/micold-client/src/ui/sidebar.rs:1828` (`every_sidebar_row_has_a_distinct_key`, commit `d75740311`, titled `test(1718)`) | `assert!(keys.len() >= 6, ...)` | `assert!(keys.len() >= 5, ...)` | **Loosened.** Justified by one fewer header row, but a lower bound stays vacuous: the count is deterministic and should be asserted exactly. Counts as a weakened existing test (rubric: FAIL on its own). |
| `tests/feature_registration_cost.rs:123` (`2603dd6af`) | allow-list entry `sidebar` reads `overview` `location_card_state` | entry removed | Tightening, not weakening. |
| `tests/sidebar_tree.rs` (`2603dd6af`) `hover_holds_across_a_state_change`, `an_open_row_menu_stays_on_its_location_across_a_state_change` | asserted the row **moved** and the hover was lost | assert the row **stays** and the hover holds | Behavior reversal required by FR-001/FR-003; assertions are as strong as before. Legitimate. |
| `tests/sidebar_state_icon.rs:488-512` (`22e3054a4`) | asserted the `State unknown` tooltip line and `Blocked: failed` | `LocationState` replaced by `CardState`; `Unknown` has no counterpart | Behavior removed with the type; new test pins all five words. Legitimate. |
| `tests/sidebar_state.rs`, `tests/navigation_apply.rs` (`a947fd7fc`) | 14 group tests (`done_starts_collapsed...`, `a_header_press_flips_its_group`, `a_toggle_for_a_group_not_shown_records_nothing`, etc.) | deleted or rewritten as flat-list tests | Group behavior removed by FR-002/FR-007; each deletion maps to a removed API. Legitimate. |
| `ui/material/composition_contrast.rs` (`a_sidebar_group_header_is_legible_on_the_sidebar`), `micold-material` `a_sidebar_group_header_is_twelve_px_bold` | group header contrast and size | deleted | Removed with `TypeRole::SidebarGroup`. Legitimate. |
| `tests/fixtures/layout_snapshot.txt` | regenerated in `a947fd7fc` (+/- 19082 lines) and `906166c10` (472/445) | accepted in the same commit as the production change | See finding 6. |

No test was skipped, `#[ignore]`d, renamed out of a filter, or excluded through config; no coverage or
mutation threshold exists to lower.

## Findings

Ordered by severity. `HIGH` smells are counted in `high_smells`.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (ordering) | M1 is `TEST_AFTER`: five behaviors with no recorded red; tests and production in one `wip ... unreviewed` commit. Rubric makes any `TEST_AFTER` a FAIL. | `tdd/cycle-log.md:3-19`; `a947fd7fc` |
| 2 | HIGH (coverage) | **The real sidebar draw path is untested.** The wiring that turns a row's card state into its glyph, tint and tooltip (`ui/sidebar.rs` `tinted` at :1107, `.state_icon(StateIcon::new(card_state, r))` at :958 and :1418, `with_state_line` at :961, :1159, :1425) has no test: no `ui/sidebar.rs` test mentions tint, state icon or tooltip, and `layout_snapshot` records geometry only. US2.1, US4.1, US4.2 and SC-005 are verified for the pure functions (`row_tint`, `glyph`, `state_line`) and for the showcase, never for the sidebar a user sees. Mutants 1-4 below all survived. | `crates/micold-client/src/ui/sidebar.rs:958,961,1107-1115,1159,1418,1425`; `specs/1718-sidebar-stable-order/autopilot.md:46` ("real client ... not driven") |
| 3 | HIGH (smell: tautological / doubled subject) | `sidebar_list()` is a one-line alias of `sidebar_entries()` (`features/sidebar.rs:1802-1804`), so `assert_eq!(list, entries, "the list is the stable order")` and `assert_eq!(list, order)` compare a function with itself. The new "stable order" tests never pin the order (Default first, then worktrees in listing order, run groups arranged). Mutant 7 (worktrees reversed in `sidebar_entries`) was caught only by older tests (`row_index_finds_location_rows...`, `the_reveal_scroll_reaches_the_keyboard_row...`) and `the_reveal_offset_counts_rows_only`, not by any of the tests that claim SC-001/SC-003. Should assert the expected `Vec<SessionLocation>` literal for the fixture. | `tests/sidebar_state.rs:1102-1105, 1131-1135` |
| 4 | HIGH (smell: re-implemented expectation) | The "tint followed its state" check computes the expected value with the function under test: `tints` holds `row_tint(state)`, then the loop asserts `tint == row_tint(*state)`. It cannot fail. The remaining checks (`any none`, `any some`, `distinct.len()==3`) are weaker than `row_tint_by_card_state` already is. | `tests/sidebar_state.rs:1100, 1113-1116` |
| 5 | HIGH (smell: vacuous) | `the_list_holds_only_location_entries` asserts every entry `matches!` the three variants of `SidebarEntry`, which has exactly those three variants (`features/sidebar.rs:190-197`); it is true for any list and is enforced by the compiler. FR-002 (no header row) is carried by the type, which is fine, but this test adds nothing. | `tests/sidebar_state.rs:1140-1146` |
| 6 | HIGH (smell: tautological, doubled subject) | `glyph(state)` is defined as `super::card_state_icon(state)` (`state_icon.rs:22-24`), so the loop `assert_eq!(glyph(state), card_state_icon(state))` is the function compared with itself. The explicit `Icon` array above it already pins the glyphs. | `crates/micold-client/src/ui/material/state_icon.rs:163-165` |
| 7 | HIGH (existing test loosened) | `keys.len() >= 6` became `>= 5` (see table above). | `crates/micold-client/src/ui/sidebar.rs:1828` |
| 8 | MED (self-approving snapshot) | `layout_snapshot.txt` was regenerated and accepted in the same commit as the production change both times (M1 +/- 19082 lines, M2 `906166c10`, whose message says "accept"). The M2 diff shifts a row subtree by a wrapper (`x` 12.0 to 4.0, width 228 to 244, then children renumbered); the cycle log never records a review of the diff. Two review rounds are recorded as CLEAN in `autopilot.md:46`, which is some mitigation. | `crates/micold-client/tests/fixtures/layout_snapshot.txt`; `906166c10`, `a947fd7fc` |
| 9 | MED (framework under test / vacuous) | `settings_load_unchanged_and_store_no_group_choice` writes `{"filter_open": true}`, loads and saves it, and asserts `group_choices` is absent. No settings field ever had that name (it was in-memory only, spec Assumptions), so the second assertion cannot fail; the first tests `JsonFileSettingsStore`. It evidences FR-008 only as a characterization of the store. | `tests/sidebar_state.rs:1278-1289` |
| 10 | MED (eager test) | `a_location_cycling_through_every_card_state_never_moves` asserts order identity, list equality, tint presence, tint equality, tint count, and card-state coverage in one test (~70 lines), with loose labels. A failure does not say which behavior broke. | `tests/sidebar_state.rs:1055-1124` |
| 11 | MED (foreign style) | The profile requires every assertion to carry a message explaining the rule. Most assertions in `flat_list` and `over_the_flat_list` have none (e.g. `sidebar_state.rs:1133, 1134, 1142, 1161, 1188, 1199-1200, 1217, 1233-1242, 1272`; `navigation_apply.rs:189, 211-212`). | files cited |
| 12 | MED (implementation coupled / weak) | `the_state_icon_has_an_entry_posing_its_five_states_and_tints` asserts that string names appear in a catalogue table (`entry.posed`, `entry.live`), not that the poses render. It is the repository's pattern for showcase completeness, but it proves registration, not a hover-over-tint pose. | `tests/showcase_completeness.rs:493-528` |
| 13 | LOW | Several tests have no doc comment or a non-behavioral name (`no_project_no_rows`); `Magic value` `distinct.len(), 3`, `list.len(), 6` have a message but no named constant. | `tests/sidebar_state.rs:1135, 1120, 1198` |

Properties: the targeted run is fast (0.1 to 1 s per binary) and deterministic; the new tests share
`support::sidebar_fixture` (`five_states`, `session_in`, `pr_read`, `worktree`, `listed`) and do not
hand-roll doubles, so no `Bypassed test utility` finding. No sleeps, clock or randomness. The
`tinted_row` helper in `tree_view.rs` tests contains a `match` with an untinted branch; it builds the
fixture and asserts nothing, so it is not a conditional-assertion smell.

## Mutation results

Scope: no mutation tool exists in the profile, so these are **deliberate mutants**, 9 in total, on files
the feature changed, run one at a time against `cargo test -p micold-client --lib --test main -- state_icon
tree_view composition_contrast ui::sidebar sidebar_state navigation_apply features_sidebar
sidebar_state_icon showcase_completeness layout_snapshot` (66 + 86 tests, baseline green). Every mutant was
restored from a byte copy and `git diff` confirmed clean after each; the final run on the restored tree
(`HEAD 16d873e2e`, `git status` clean) was green again. This is a sample of 9 mutants over 5 files, not an
exhaustive run, and it is not a score.

| # | Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- | --- |
| 1 | `ui/sidebar.rs:1112` `tinted()` no longer applies the tint (`=> item`) | row tints (FR-010/011, US4.1) | **Yes** | Real gap: no test sees the tint on a drawn sidebar row. Finding 2. |
| 2 | `ui/sidebar.rs:1159` run-group tooltip drops its state line | tooltip names state (G10, FR-011) | **Yes** | Real gap, and the behavior has no cycle-log row. Finding 2. |
| 3 | `ui/sidebar.rs:958` worktree rows get `StateIcon::new(CardState::Idle, r)` | per-row indicator (FR-004, SC-002) | **Yes** | Every worktree row could show the Idle glyph and the suite stays green. Finding 2. |
| 4 | `ui/sidebar.rs:1418` Default row gets the Idle icon | same | **Yes** | Same. |
| 5 | `features/sidebar.rs:1391` Done state line `"Done (merged)"` to `"Done"` | G10, FR-011 | No | Caught by `sidebar_state_icon::the_state_line_is_the_card_states_word_and_done_reads_merged`. |
| 6 | `state_icon.rs:42` Needs attention tint to `error_container` | G7, FR-010 | No | Caught by `row_tint_by_card_state`. |
| 7 | `tree_view.rs:466` `resting_fill` `if selected` inverted | G8, FR-012 | No | Caught by 11 tests. |
| 8 | `features/sidebar.rs:1949` worktrees listed in reverse in `sidebar_entries` | stable order, FR-001/003 | No (but see finding 3) | Caught by 3 older/reveal tests, **not** by the 1718 tests that name the order. |
| 9 | `features/navigation.rs:58` expanded run groups dropped from `location_nodes` (`if false && ...`) | FR-006 over run groups | **Yes** | The run-group arm was touched by T009 and has no test: chords never visit a run inside an expanded group. Not equivalent. |

No survivor is an equivalent mutant. Survivors 1-4 sit in behaviors the cycle log marks green
(M2 rows "Five states, five glyphs", "Tint by card state", "Tooltip state line"), so per the rubric they are
`HIGH`; the tested units are right, the wiring that uses them is not tested.

Coverage: not available (no tool).

## Traceability

No `traces` column exists; mapping is spec criterion to the tests that claim it by name or doc comment
(all the tests named below exist and ran in the baseline). "End to end" means a test reaches the criterion
through `State` or the drawn sidebar, not only through a unit with the collaborators faked.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1 / FR-001 / FR-003 / SC-001 (rows keep position) | `flat_list::a_location_cycling_through_every_card_state_never_moves` | Yes, through `State.sidebar_list()`; order itself not pinned (finding 3) |
| US1.2 / FR-002 / SC-004 (no header) | `the_list_holds_only_location_entries` (vacuous), `row_metrics_have_one_entry_per_location_row`, `every_sidebar_row_has_a_distinct_key`; header type deleted | Type-level; docs/showcase grep (SC-004) has no test |
| US1.3 hover/selection/keyboard stay | `sidebar_tree::hover_holds_across_a_state_change`, `..._menu_stays_...`, `flat_list::a_rows_open_menu_stays_on_it...` | Yes |
| US2.1 / FR-004 / SC-002 (indicator per state) | `state_icon::tests::five_card_states_draw_five_different_glyphs`, `every_state_takes_the_same_width`, `idle_is_drawn_not_blank` | **No**: unit only; sidebar wiring mutants 3-4 survived |
| US2.2 (indicator updates in place) | cycling test (model only) | No for the drawn row |
| US3.1 / SC-003 (Done listed) | `a_done_location_is_listed_in_stable_order` (tautological order) | Yes (membership and count) |
| US3.2 / FR-006 (nav and reveal) | `navigation_apply::over_the_flat_list::*`, `the_reveal_offset_counts_rows_only` | Yes, except run-group runs (mutant 9) |
| US4.1 / FR-010 / SC-005 (loud tint) | `row_tint_by_card_state`, `a_tinted_rows_text_is_legible_on_its_tint`, `every_state_icon_is_legible_on_every_fill_of_a_sidebar_row` | **No**: unit and contrast on the function's output; mutant 1 survived |
| US4.2 / FR-011 (Done quieter, "Done (merged)") | `row_tint_by_card_state`, `the_state_line_is_the_card_states_word_and_done_reads_merged` | **No** for the drawn tooltip (mutant 2 survived; worktree-row tooltip unmutated and untested) |
| US4.3 / FR-012 (hover/selection over tint) | `a_selected_row_is_distinct_from_a_merely_tinted_one`, `a_tint_changes_no_geometry` | Yes at the `TreeView` level (not the sidebar) |
| US4.4 (tint changes in place) | cycling test | Model only |
| FR-005 (hover, ripple, menu, tag filter, run groups, badge as before) | `a_tag_filter_removes_without_reordering`, `a_run_group_is_one_entry_that_stays_put`, older suites | Yes |
| FR-007 (no group control reachable) | type removal; `showcase_completeness` rows | Type-level |
| FR-008 (state files load) | `settings_load_unchanged_and_store_no_group_choice` | Weak (finding 9) |
| FR-009 / SC-004 (docs, snapshot, showcase describe flat list) | layout snapshot (finding 8); docs not tested | Partial |

Criteria with no test at all: FR-009 documentation (T013, T021, T029 are prose edits; no docs gate exercised).
Tests tracing to nothing: `the_list_holds_only_location_entries` and the `glyph == card_state_icon` loop (they
test the type system and a delegation, not a requirement).

## What was not audited

- The full workspace suite (`mise run test` / `mise run gate`) was **not run**: 591 test binaries, and the
  baseline in the profile is 5071 passed in 343 s. Only the 66 lib + 86 `main` tests selected by the filters
  above were run (this also bounded disk and I/O use). The lib filter covers `state_icon`, `tree_view`,
  `composition_contrast` and `ui::sidebar`; the `main` filter covers the sidebar, navigation, showcase and
  layout modules. `micold-core` tests (`sidebar_groups` deletion, `run_group_state`) were not run or read.
- Files not read in full: `docs/*` edits, `showcase/catalogue.rs`, `showcase/sections/atoms.rs`,
  `type_role_mapping.rs`, `micold-material/src/text.rs`. Only their test-facing effects were checked.
- No fresh-context subagent smell pass; all findings are the auditor's own.
- Mutation: nine hand-picked mutants only. `micold-core` and `micold-material` production files, and the
  upstream `location_card_state`, were not mutated. No score is claimed.
- Coverage: unavailable.
- Layout snapshot: the regenerated fixture was sampled, not reviewed line by line (19k lines).
- Visual pass (`specs/1718-sidebar-stable-order/visual-pass/`) was not re-run; the log says the real client
  was not driven, only the showcase.
- Performance, concurrency (Principle II, several windows) and platform behavior: no test, no criterion
  beyond "none", not assessed.
- Remediation tasks were **not** appended to `tasks.md`: the audit request limited writes to this file.
  Suggested order for whoever adds them: (1) a test that builds the real sidebar items for a state matrix and
  asserts glyph, tint and tooltip (kills mutants 1-4); (2) pin the stable order literally and delete or
  rewrite findings 3-6; (3) make the key-count assertion exact; (4) add a run-group nav test (mutant 9); (5)
  record a verbatim red for any future cycle.

## Resolution (close unit)

- Row wiring (mutants 1-4): `sidebar::tests::each_sidebar_row_wears_its_card_states_icon_tint_and_tooltip_word` and `a_run_group_row_wears_the_worst_state_of_its_runs` assert icon, tint and tooltip on the real `build_items` rows (test-only seams `TreeItem::drawn_state`/`drawn_tint`, `StateIcon::card_state`).
- Loosened key-count assertion: now `assert_eq!(keys.len(), 5)`.
- Tautological tests: stable order pinned to a literal list, tints to literal roles, the vacuous variant `matches!` and the glyph self-compare removed.
- Survivor 9: `navigation::the_chords_visit_the_runs_of_an_expanded_group_and_not_a_collapsed_ones`.
- M1 TEST_AFTER (5 behaviours): history, not fixable now. Accepted; the behaviours are now covered by tests that pass against mutants.
- Not re-run: mutants (no tool), so the survivors are expected, not measured, to be killed.
- Cycle-log gap for the run-group tooltip (906166c10) is filled in `cycle-log.md`.
- Findings 8-13 (MEDIUM/LOW: the accepted layout snapshot, the FR-008 settings test, one multi-behaviour test, assertion messages) are accepted unchanged.
