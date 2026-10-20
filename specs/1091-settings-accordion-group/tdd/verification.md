---
feature: 1091-settings-accordion-group
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: cf7c64d7d
behaviors: 12 # no test list; tasks T001-T012 used as the behavior list
proven: 0
likely: 0
test_after: 11
no_test: 1 # the stale-press guard added in bd1e87877
high_smells: 0
criteria_total: 9 # FR-001..FR-009
criteria_covered: 9 # FR-005 keyboard half and FR-008 rendering only indirectly, see traceability
mutation_score: 62 # 8 of 13 deliberate mutants caught, sampled, not a tool score
mutants_survived: 5
suite: 149 passed, 0 failed, ~51s # daemons_feature 62, daemons_section_geometry 19, showcase_state 26, showcase_completeness 42; plus 18 accordion lib tests
---

# TDD Verification: Settings accordions form a group

**Verdict: FAIL.** No behavior has a recorded red (no `tdd/test-list.md`, no `cycle-log.md`; every test landed in the same commit as its source), and the sampled mutants show five survivors, one of them a shipped fix with no test.

Independence: this audit read all files cold and did not write the tests.

## Test-first evidence

No cycle log exists, so no behavior can be `PROVEN` or `LIKELY` (Hard Rule 3). History shows test and source changing in one commit each time, which is consistent with a per-cycle commit but proves no red.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T001 group unit tests | TEST_AFTER | `7db0b4f17` adds `accordion_group.rs` source and tests together; no red recorded |
| T002 `in_group` test | TEST_AFTER | same commit, no red |
| T003, T004 impl | TEST_AFTER | same commit |
| T005, T006 reducer tests | TEST_AFTER | `daemons_feature.rs` changed in `7db0b4f17` with `daemons.rs`; no red |
| T007 daemons `State` | TEST_AFTER | same commit |
| T008 geometry helper | NOT_APPLICABLE | characterization update of an existing test, see "Existing tests" |
| T010, T011, T012 showcase | TEST_AFTER | `ca70ba535` adds test and source together; `b91d13a03` changes both again |
| Stale-press guard (no task) | NO_TEST | `bd1e87877` changes `daemons.rs:1901-1909` with no test; mutant F survives |

`tasks.md` ticks T001-T012 `[x]`, all consistent with the code, but no list exists to confirm `DONE`.

### Existing tests changed

No weakening found. Changes follow the spec's behavior change (independent to exclusive):
- `daemons_feature.rs:667-676` `sections_open_by_toggle_and_by_error_...`: expectations changed from several open to one open. Legitimate (FR-001), still value-asserting.
- `daemons_section_geometry.rs:225-229` `MoveUp` count 2 to 1, "opening one closes the other". Legitimate.
- `daemons_section_geometry.rs:430-461, 687-717` `open_form` rebuilt to measure each section separately (T008). Stronger overflow coverage (`bd1e87877` adds a per-section `assert_no_text_overflows`). The `rfind` change at `:765` is explained in a comment. No skips, no filter or threshold changes. (A pre-existing note: `open_form` now stitches a layout from several renders, positions comparable only within a section; acceptable, documented.)

## Findings

| # | Severity | Finding | Evidence | Fix |
| --- | --- | --- | --- | --- |
| 1 | HIGH | No red recorded for any behavior (no test list, no cycle log); ordering unprovable | `tdd/` was absent; commits `7db0b4f17`, `ca70ba535` | Create `tdd/test-list.md` and `cycle-log.md`; for each behavior record a red by applying the matching deliberate mutant above and quoting the failing test output, then restore |
| 2 | MED | Stale-press guard untested: mutant F (`kind.filter(..)` removed, `daemons.rs:1904`) survives. Without it a press for a section outside the open kind sets `Choice::Open(foreign)` and closes every section | `daemons.rs:1901-1909`; `bd1e87877` | Add a reducer test in `daemons_feature.rs`: with a Host form open, send `SectionToggled(Section::Placement)` (a Kubernetes-only section) and assert `open_sections` still equals the first section |
| 3 | MED | Reset in `begin_edit` untested: mutant A (`daemons.rs:862`) survives, masked by `discard_draft` (mutant B, `:890`) also surviving. Editing daemon Y while a form for X has a flipped section is unpinned (FR-007, "two forms in turn") | `daemons_feature.rs:1805-1865` only drives `begin_add`; `:722-735` edits from a fresh state | Add a test: open edit form X, toggle a non-first section, call `begin_edit` for Y without discarding, assert exactly the first section open (kills A). Mutating both A and B together should then fail too |
| 4 | MED | Showcase independent toggle never exercised twice: mutant K (`*open = !*open` to `= true`, `state.rs:~437`) survives. FR-003 "ungrouped sections keep their own state" lacks a close assertion | `showcase_state.rs:131-156` toggles index 0 once, from false | Toggle `IndependentSectionToggled(1)` (starts true) and assert it closes, and toggle `0` twice and assert it returns |
| 5 | MED | Showcase view wiring untested: mutant L (`.in_group(showcase.group(), key, GROUP_DEFAULT)` to `.open(true)`, `surfaces.rs:278`) survives; no test renders the pose or presses a header and checks the emitted `Message` | `surfaces.rs:270-295`; `showcase_completeness.rs` only checks poses exist | Add a render test (client `support::layout`) that paints the grouped pose, counts open bodies (1 by default) and asserts they match `Showcase::group_open` |
| 6 | MED | Tautological comparison: "rapid sequence ends as the same presses one by one" runs the identical sequence on `state` and its clone, so it compares the reducer with itself | `daemons_feature.rs:~1836-1846` (the `other` clone block) | Replace with an expected value: after presses in a known order assert the exact open set, or drop it (the `len() <= 1` line is the real SC-001 check) |
| 7 | MED | Eager test with loops: `the_sections_of_every_kind_form_one_group` asserts add, toggle, close, reopen, rapid, kind change and no-leak in one body | `daemons_feature.rs:1801-1865` | Split per behavior (open replaces, close to none, reset on kind change, no leak); keep the per-kind loop |
| 8 | MED | Expectation computed with logic: `.into_iter().filter(\|s\| Section::of(rt).contains(s))` builds the expected vector, so a section missing from the kind silently shrinks the expectation | `daemons_feature.rs:~1884-1890` | Assert the literal `vec![Section::Resources, Section::Placement]` |
| 9 | MED | `several_held_sections_stay_open_together` only asserts `!held.is_empty()`, so one held section satisfies "several"; the setup (`Saved` on an empty form) is not shown to hold 2 | `daemons_feature.rs:~1920-1944` | Assert `held.len() >= 2` and name the sections, or rename |
| 10 | LOW | Magic section indices `1, 2, Some(1)` in `accordion_group.rs` tests, and `reset_returns_to_the_default` doubles up two scenarios | `accordion_group.rs:75-122` | Name the keys (`FIRST`, `SECOND`) |
| 11 | LOW | Keyboard half of FR-005 (Space or Enter on a focused header) has no test of its own; it rests on the `Button` widget (a framework behavior, existing accordion tests) | spec FR-005, T-list has no keyboard task | Accept as framework-owned, or add one header-focus test through the real update path |

HIGH smells (rubric catalogue): none. Properties: deterministic, no sleeps, fast (the five integration binaries finish in about 2 s of test time). Isolation fine. The geometry helper is coupled to rendering but not flaky.

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, each restored via `git checkout -- <file>` and the suite re-run green (62, 19, 42, 26 passed; tree clean). Sampled 13 mutants over: reducer resets (4), held-section guards (2), stale guard (1), group toggle (1), `in_group` (1), showcase state (2), view wiring (2). Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| A `daemons.rs:862` drop reset in `begin_edit` | T007 / FR-007 | Yes | Real gap, masked by B (finding 3) |
| B `daemons.rs:890` drop reset in `discard_draft` | T007 | Yes | Redundant with the other resets while A holds; low value alone |
| C `daemons.rs:1915` drop reset on `RuntimeChosen` | T007 | No | Caught by 2 tests |
| D `daemons.rs:780` drop reset in `begin_add` | T007 | No | Caught |
| E `daemons.rs:1905` press on held section toggles | T006 / FR-006 | No | Caught by 2 tests |
| F `daemons.rs:1904` drop stale-press guard | none | Yes | Real gap (finding 2) |
| G `daemons.rs:418` drop `held_open` from `section_open` | FR-006 | No | Caught by 10 tests |
| H `accordion_group.rs:61` pressing open key does not close | T001 / FR-002 | No | Caught by unit, reducer, geometry, showcase tests |
| I `accordion_section.rs:69` `in_group` no-op | T002 | No | Caught |
| J `showcase/state.rs:433` group toggle no-op | T010 | No | Caught |
| K `showcase/state.rs` independent toggle stuck at true | T011 / FR-003 | Yes | Real gap (finding 4) |
| L `surfaces.rs:278` pose ignores the group | T012 / FR-008 | Yes | Real gap (finding 5) |
| M `container_fields.rs:583` form ignores `section_open` | FR-004 view | No | Caught by 5 geometry tests |

Score: 8 of 13 caught (62%), sampled, tool none.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-001 / US2 AS1 | `accordion_group::tests::*`, `in_group_sets_open_from_the_group`, `grouped_sections_are_exclusive_and_leave_the_independent_set_alone` | Reducer level (the real `update` path); no rendered press |
| FR-002 | `pressing_the_open_key_closes_it_and_it_can_reopen`, `the_sections_of_every_kind_form_one_group` | Yes (reducer) |
| FR-003 / US2 AS2 | `two_groups_are_independent`, showcase test (weak, finding 4) | Partial |
| FR-004 | `the_sections_of_every_kind_form_one_group`, `sections_open_by_toggle_...`, geometry tests | Yes |
| FR-005 | draft-unchanged assertions in two tests; keyboard: none of its own | Partial (finding 11) |
| FR-006 / US3 | `an_error_held_section_stays_open_beside_the_group`, `several_held_...` (weak), `a_click_on_a_section_an_error_holds_open_is_not_recorded` | Yes |
| FR-007 | `every_kind_opens_on_connection_only_...`, `the_sections_of_every_kind_form_one_group` (add and kind change only; edit unpinned, finding 3) | Partial |
| FR-008 | `showcase_state`, `showcase_completeness` (pose captions) | Partial: no render check (finding 5) |
| FR-009 | `closed_headers_paint_their_summary_and_the_required_marker`, `a_closed_connection_paints_none_of_its_fields` (pre-existing, still green) | Yes |

Untested criteria: none outright; four are partial. Tests tracing to nothing: none. The stale-press behavior (finding 2) traces to no criterion and no test.

## What was not audited

- Quickstart Part B visual pass and both themes: not run (needs a display); not assessed here.
- Full workspace suite and `mise run gate` (clippy, fmt, shell tests): not run. Only the five relevant binaries plus the 18 accordion lib tests.
- Mutation was a 13-mutant deliberate sample, not a tool run; no coverage tool exists in the profile.
- Documentation edits (`docs/user-guide/*.md`) and the 970/1120 commits inside the `f3c827a5f..HEAD` range (other features, not audited).
- Sandbox/acceptance (`sandbox_real_*`) tests: unrelated to this feature.
- Performance and animation behavior (the spec's reveal-animation edge case): no test, not assessed.
