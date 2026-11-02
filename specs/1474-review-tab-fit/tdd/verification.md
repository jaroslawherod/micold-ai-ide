---
feature: 1474-review-tab-fit
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: cb3910fbb # short SHA audited
behaviors: 14
proven: 0
likely: 8
test_after: 3
no_test: 1
not_applicable: 2
high_smells: 2
criteria_total: 27 # 14 acceptance scenarios + 13 FRs
criteria_covered: 26
mutation_score: unmeasured # no mutation tool in the profile; 7 deliberate mutants, 6 caught, 1 survived
mutants_survived: 1 # not equivalent
suite: 12380 passed, 0 failed, 20 ignored (cargo test --workspace, 613 result lines incl. doctests), wall 28m40s of which most was waiting on another worktree's build lock
---

# TDD Verification: Review tab fits the smallest window (#1474)

**Verdict: FAIL.** An existing test (`review_bar_fixed.rs`) was loosened and now takes its expectation
from the code under test; a deliberate mutant in `freed_width` (the effective-sidebar reader at compact
width) survives the whole `micold-client` suite; and no red is recorded for any behavior.

Scope: milestones M1 (`9589c7841`, PR #1637) and M2 (`34effec55`, PR #1641), both merged. Ignored per
instruction: `specs/1474-review-tab-fit/visual-pass.md`. Nothing was fixed or committed.

Independence: this audit was run from a fresh context; the author session is not this one. The smell
pass was done by the auditor directly, every cited line opened.

## Preconditions

- `specs/1474-review-tab-fit/tdd/test-list.md` and `cycle-log.md` do **not** exist: the feature was
  not planned through the TDD extension. The rubric makes "no test list" a `BLOCKED` condition; the
  command text says to audit against `spec.md` anyway with a weaker verdict on ordering. This report
  does that. Behaviors below are derived from `tasks.md` and the diffs, not from a test list, so the
  ids (B1..B14) are the auditor's.
- Ordering evidence is the ledger (`specs/1474-review-tab-fit/autopilot.md`), self-reported, and git
  history. Each milestone landed as one commit holding tests and source together (`git show --stat`
  of `9589c7841` and `34effec55`), so history cannot show test-before-source. Per Hard Rule 3 that is
  `LIKELY` at best, and `TEST_AFTER` where not even a red is claimed.
- Baseline: the suite is green at HEAD, so there are no pre-existing failures to separate.

## Test-first evidence

| Behavior | Class | Evidence |
| --- | --- | --- |
| B1 compact below 720, unreported not compact (FR-001) | LIKELY | Ledger M1 notes: "with `src/` reverted the three suites fail to compile (`COMPACT_WIDTH`, `compact`, `sidebar_hidden` missing)". A compile error is not a behavior red; no output recorded; one commit `9589c7841` |
| B2 entering compact shows strip, stored state untouched (FR-002) | LIKELY | same claim, `sidebar_state.rs` |
| B3 leaving compact restores, clears override (FR-003) | LIKELY | same claim |
| B4 show/hide/toggle at compact (FR-004) | LIKELY | same claim |
| B5 hidden stays hidden through a narrow window | LIKELY | same claim |
| B6 full screen with compact width (FR-005, US4) | LIKELY | same claim, `overview.rs` |
| B7 `freed_width` follows the effective state at compact width (T005, FR-005) | NO_TEST | `ui/drawer_flags_tests.rs:38` only toggles `hidden` at the default (0,0) size; see finding 1 and mutant B |
| B8 gate premise: strip 32px, main area 608px at 640x480, expanded at 1280 (FR-011, SC-004) | LIKELY | T007 in `9589c7841`; no red recorded |
| B9 diff header, placeholder, drawer rows fit at 640x480 (FR-007..009, FR-012) | LIKELY | Entries deleted in `9589c7841`/`34effec55`; the gate's stale-entry check is the red/green signal, but no failing run is recorded |
| B10 `capped_with_floor` (T011) | TEST_AFTER | Tasks say "write red unit tests"; no red recorded; test and source in `34effec55` |
| B11 `drawer::width` uses the anatomy floor, `MIN_WIDTH` pinned (T012) | TEST_AFTER | same |
| B12 `SideSheet::min_width` wired into layout and `comments_drawer` (T013) | TEST_AFTER | Only the layout gates cover it (mutant F); no red recorded |
| B13 tree filter and rows fit at 640 (FR-010, T017) | NOT_APPLICABLE | Characterization: already held after M1; nothing was changed for it |
| B14 1280x800 unchanged (FR-013, SC-005) | NOT_APPLICABLE | Snapshot fixture `layout_snapshot.txt` untouched by both commits |

Docs (FR-006, `docs/user-guide/*`) are not testable behavior and are not in the table.

### Existing tests changed by the feature

| Test | Before | After | Reading |
| --- | --- | --- | --- |
| `crates/micold-client/tests/gates/review_bar_fixed.rs:309-331` `in_a_narrow_window_the_drawer_is_half_the_diff_area` | `assert_drawer_covers_the_right_of(&records, area.width / 2.0)` | `assert_drawer_covers_the_right_of(&records, drawer::width(area.width))` | Expected value now computed by the function the feature changed (finding 2) |
| `review_bar_fixed.rs:349-351` `assert_drawer_covers_the_right_of` | `assert!(drawer.x >= area.x + area.width / 2.0 - TOLERANCE, ...)` | `assert!(wanted > area.width / 2.0 + TOLERANCE \|\| drawer.x >= ..., ...)` | The "starts at or right of the middle" check is switched off whenever the floor applies, i.e. in exactly the narrow case this test exists for (finding 2) |
| `crates/micold-client/tests/gates/review_tab_fits.rs` `SMALLEST_WINDOW` (13 `#1474` entries in M1, 1 in M2) | known-defect entries | deleted | Legitimate: FR-012 requires each fixed defect's entry to go, and the gate fails on a stale entry. Not a weakening. The stale-notice entry was deleted rather than re-filed because it stopped firing; consistent with the spec |
| `crates/micold-client/tests/root_state_is_shared.rs:221` | `COMPONENT_LOCAL` entry for `sidebar.hidden` | deleted | Justified in the ledger (`sidebar.hidden` is now read by `app::State::sidebar_hidden`); the guard still runs. Not a weakening, but worth one line of confirmation that the scan genuinely finds the new reader |
| `tests/overlay_registration.rs` `INLINE_FLAGS` | one entry | adds `sidebar.compact_open` | Additive |
| `tests/tokens_anatomy.rs:55-59` | `ALL.len() == 55` | `== 56` | Tracks the added constant; additive |

No test was skipped, ignored, renamed out of a filter, and no threshold was lowered.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Surviving mutant in a `DONE` behavior: `freed_width` reading `state.sidebar.hidden` instead of `state.sidebar_hidden()` passes the entire `micold-client` suite (`cargo test -p micold-client --no-fail-fast`, exit 0). At compact width, full screen would size the grid with the wrong freed width, with nothing failing. The only test, `ui/drawer_flags_tests.rs:38-48`, runs at the unreported (0,0) size where compact is false, so it cannot tell the two readers apart. T005 and the spec's Key Entities ("every reader uses it") make this reader part of the feature. Should assert: at 640 wide with `compact_open` false, `freed_width` is `STRIP_WIDTH`; with `compact_open` true it is the expanded width plus the handle | `crates/micold-client/src/ui/mod.rs:1752-1758`, `crates/micold-client/src/ui/drawer_flags_tests.rs:38` |
| 2 | HIGH | Weakened existing test. `in_a_narrow_window_the_drawer_is_half_the_diff_area` now derives its expected width from `drawer::width` (the function changed by the feature, a re-implemented expectation), keeps a name and doc that still say "half", and `assert_drawer_covers_the_right_of` skips the `drawer.x >= middle` check whenever `wanted > area.width / 2`. The behavior change (a 240px floor) is legitimate and has unit tests (`review_drawer.rs`), but this gate no longer pins the width independently. Should assert literal expectations: `MIN_WIDTH` (240) when half the area is under it, half the area otherwise, and a separate bound that the drawer does not exceed the area | `crates/micold-client/tests/gates/review_bar_fixed.rs:309-331`, `:349-351` |
| 3 | MED | No red-phase evidence for any behavior; no `tdd/` artifacts. The only red claim is "fail to compile" for M1 and "entry removed first (red)" for M2, neither with command or output. Both milestones are one test-plus-source commit each | `specs/1474-review-tab-fit/autopilot.md` (M1 notes, M2 notes), commits `9589c7841`, `34effec55` |
| 4 | MED | Tasks T001 lists "toggle/resize ordering" and the spec's Edge Cases say a toggle after a resize acts on the post-resize effective state. No test pins the order (a toggle then a resize across 720 is not asserted) | `tasks.md` T001; `crates/micold-client/tests/sidebar_state.rs:46-152` |
| 5 | MED | SC-003 and FR-002 promise the persisted sidebar state is unchanged. Tests assert the in-memory `hidden` and `width`, never the persisted form. The spec scopes persistence out ("whether they survive a restart is not part of this feature"), so this is a gap in wording against the test, not a defect | `crates/micold-client/tests/sidebar_state.rs:60-71` |
| 6 | MED | `two_states_are_independent` builds two default `State` values and resizes each; independence is a property of Rust ownership, so the test pins nothing the code could get wrong unless a global is introduced. Closest smell: Framework under test / Redundant test | `crates/micold-client/tests/sidebar_state.rs:145-153` |
| 7 | LOW | `closing_full_screen_*` tests assert `sidebar_hidden()` and the stored fields but not the view (the drawer's open flag or the strip), so the claim "the sidebar returns as the strip" is reached through the state accessor only. The layout gate covers the strip at 640 without full screen | `crates/micold-client/tests/overview.rs:512-546` |
| 8 | LOW | New assertions in `sidebar_state.rs` and `overview.rs` are bare `assert!` with no message about the rule, against the profile's convention ("every assertion carries a message"). The surrounding file is also bare, so this matches the file but not the profile | `crates/micold-client/tests/sidebar_state.rs:46-152` |
| 9 | LOW | Magic literals `1000`, `700`, `640`, `480`, `650`, `719` appear in many tests instead of `COMPACT_WIDTH` and `MIN_WINDOW_SIZE`; the boundary test is the only place the 719/720 pair is named | `crates/micold-client/tests/sidebar_state.rs:46-152`, `features_window.rs:116-145` |

Smell catalogue results for the new and changed tests, otherwise: no assertion-free, empty, skipped,
vacuous or tautological tests found; no sleeps, no real clock or network; the loop in
`the_tree_and_its_filter_lie_inside_the_tree_pane_at_the_smallest_window` iterates a fixed theme pair
and cannot skip its assertions; the `e.edge != "bottom"` filter in that test is a declared scroll
exemption (the tree's `VirtualRows`), consistent with the gate's other entries. The layout snapshot
fixture was not regenerated in either commit. The doubles used are the repository's own helpers
(`support::*`, `State::default()`); no hand-rolled fakes were added. Isolation, determinism and speed
hold: the new tests are reducer-level and gate-level and ran in well under a second each, with the
layout gate (`layout_snapshot`, 173 tests) at about 11s.

## Mutation results

No mutation tool in the profile (`mutation: null`), so this is a sample of 7 deliberate mutants over 5
source files, one at a time, each restored with `git checkout --` and verified by `git diff --quiet`
(clean tree, apart from the pre-existing untracked `visual-pass*`). Test scope per mutant: `-p
micold-client --lib --test overview --test sidebar_state --test features_window --test layout_snapshot`
unless stated. Not exhaustive; mutation score is unmeasured.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `features/window.rs` `width < COMPACT_WIDTH` to `<=` | B1 | No | Caught by `compact_is_below_720_and_not_for_an_unreported_size` |
| `features/window.rs` `width >= COMPACT_WIDTH` to `>` (leave-compact) | B3 | No | Caught by `leaving_compact_width_clears_the_override_only` |
| `features/sidebar.rs` `compact_open = !compact_open` to `= true` | B4 | No | Caught by `hide_after_show_at_compact_width_returns_to_the_strip` |
| `app.rs:2031` `CompactWidthLeft` route to no-op | B3 | No | Caught by `show_at_compact_width_holds_until_the_window_is_widened` |
| `ui/mod.rs:1753` `freed_width` reads `sidebar.hidden` | B7 | **Yes** | Not equivalent: different result at compact width. Also survives all of `cargo test -p micold-client --no-fail-fast`. Finding 1 |
| `core/review/drawer.rs` drop `.max(floor)` in `capped_with_floor` | B10 | No | Caught by the review gate `every_review_state_fits_at_both_windows_in_both_themes`; the cargo run stopped at that target before reaching `review_drawer.rs`, where `a_floor_keeps_the_drawer_from_shrinking_below_what_its_rows_need` also pins it (not observed) |
| `ui/changes.rs` remove `.min_width(MIN_WIDTH)` | B12 | No | Caught by `review_tab_fits` and by `review_bar_fixed::in_a_narrow_window_the_drawer_is_half_the_diff_area` |

Coverage: not available (`coverage: null`).

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1-AC1 header controls inside row inside pane | `review_tab_fits::every_review_state_fits_at_both_windows_in_both_themes` (states `changes-review-*`) | Yes, real view laid out |
| US1-AC2 placeholder inside its box | same gate (`changes-tree-filter`, `review-two-worktrees`) | Yes |
| US1-AC3 drawer header, tabs, rows inside | same gate (`drawer-drafts`, `drawer-replies`, `review-empty`); `review_bar_fixed` narrow test (weakened, finding 2) | Yes |
| US1-AC4 filter and tree rows inside tree pane | `the_tree_and_its_filter_lie_inside_the_tree_pane_at_the_smallest_window` | Yes |
| US1-AC5 1280x800 unchanged | `layout_snapshot` fixture (untouched), gate at `WINDOW` | Yes |
| US2-AC1 strip and freed width at 640 | `the_sidebar_collapses_below_720_and_returns_as_left`, gate `premise_sidebar` | Yes (reducer plus layout) |
| US2-AC2 restore at 720 | same | Yes (reducer) |
| US2-AC3 hidden stays hidden | `a_hidden_sidebar_stays_hidden_through_a_narrow_window` | Yes (reducer) |
| US2-AC4 start at 640, stored state unchanged | `starting_narrow_shows_the_strip_without_touching_the_stored_state` | Yes (reducer; in-memory only, finding 5) |
| US3-AC1..3 | `show_at_compact_width_holds_until_the_window_is_widened`, `hide_after_show_at_compact_width_returns_to_the_strip`, `a_toggle_at_720_or_wider_flips_the_stored_state` | Yes (reducer via `State::update`) |
| US4-AC1 full screen at 640 closes to strip | `closing_full_screen_at_a_narrow_width_shows_the_strip` | Partial: accessor only, `freed_width` unpinned (findings 1, 7) |
| US4-AC2 full screen at 1000 returns expanded | `closing_full_screen_at_a_wide_width_returns_the_sidebar_expanded` | Partial: same |
| FR-001..FR-004 | sidebar_state / features_window tests above | Yes |
| FR-005 | overview tests above | Partial |
| FR-006 docs | none (docs) | No test; unticked in nothing, text not audited |
| FR-007..FR-010, FR-012 | review gate; `review_tab_fits` stale-entry check | Yes |
| FR-011 | `premise_sidebar` in `review_tab_fits.rs` | Yes |
| FR-013 | `layout_snapshot` fixture unchanged | Yes |

Untested criteria: FR-006 (documentation; no test possible); `freed_width` under FR-005 (finding 1);
ordering rule in Edge Cases (finding 4); animation and reduced-motion edge cases (not tested and not
in any task). Tests tracing to nothing: none; there is no test list to mislead, and every new test
names the feature in its doc comment or maps to a task.

### tasks.md against the evidence

T001-T018 are ticked `[x]`; T019 (visual pass, the close unit) is unticked and its record is outside
this audit's scope. There is no test list, so no `DONE` states to compare. The ticks for T001, T002,
T011 and T012 claim "written red" with no recorded red (finding 3).

## What was not audited

- `specs/1474-review-tab-fit/visual-pass.md` and its directory, per instruction, and tasks T019.
- The text of `docs/user-guide/overview.md` and `worktrees-and-sessions.md` (FR-006).
- Mutation: no tool; 7 hand-picked mutants only (5 files). The mutants did not cover `side_sheet.rs`
  directly (F covers its call site), `anatomy.rs`, `features/changes` keys, or the sidebar view code
  in `ui/sidebar.rs`. The core `review_drawer` target was not reached by mutant E.
- Coverage: not available.
- `sandbox-real-runtime` acceptance suite, the release daemon half, and the visual look of the
  result: not run, no criterion needs them.
- Performance and load: no criterion; wall time of the suite is dominated by the shared build lock
  and cannot be used as a speed measurement. The layout gate ran in about 11s.
- Whether the PR squash merge discarded finer-grained commits: the branch history shows one
  test-plus-source commit per milestone, so the intermediate cycles, if any existed, are gone.

## Remediation (close unit, 2026-11-02)

T020-T024 cleared the test-strength findings 1, 2, 4, 6, 7-9 in the close PR; the worker saw the freed_width mutant fail the new test. Findings 3 (process) and 5 are left standing. Process finding (no red-first log) stands: both milestones shipped as single commits. Verdict above is the audit as run, before remediation.
