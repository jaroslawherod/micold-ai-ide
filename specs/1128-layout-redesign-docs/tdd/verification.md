---
feature: 1128-layout-redesign-docs
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 357d48d5b
behaviors: 6
proven: 0
likely: 2
test_after: 0
no_test: 0
not_applicable: 4
high_smells: 0
criteria_total: 4 # the criteria a test can hold: FR-012, FR-013, SC-004, SC-005; the other 23 are doc criteria
criteria_covered: 4
mutation_score: null # no mutation tool (tdd-profile `mutation: null`); 7 deliberate mutants, all caught
mutants_survived: 0
suite: 11263 passed, 0 failed, 18 ignored (scripts/gate.sh, 583 test binaries and doc-test runs) + scripts/tests/*.test.sh green # counts from the close unit's gate run on 357d48d5; log not committed
---

# TDD Verification: layout redesign docs, layout snapshot and visual pass

**Verdict: PASS_WITH_GAPS.** No HIGH smell, no weakened existing test, and every criterion a test
can hold has one through the layout snapshot binary; the gap is evidence: the feature has no
`tdd/test-list.md` or cycle log (`tdd.plan` was skipped at the tasks stage), so ordering rests on
the ledger's self-reported reds and same-commit history, and strength on deliberate mutants only.

This is mostly a docs feature: FR-020 forbids any `crates/*/src` change, and the feature's
commits (`git log --grep='(1128)' origin/main`) touch none. Its tests are milestone M3's: two
whole-window covered states, gate W, and a fixture-difference report that names late states.

The audit ran in the close unit of the autopilot run that wrote the tests, without a subagent for
the smell pass (none available); every file below was re-read as it stands at `357d48d5b`.

## Test-first evidence

No test list, so the behaviours are taken from tasks T021–T027 and decisions D6–D8.

| Behavior | Class | Evidence |
| -------- | ----- | -------- |
| B1 `window-session-view`, `window-overview` registered and in the fixture (FR-012, FR-013; T022–T024) | LIKELY | Ledger *M3 implementation evidence*: registering the states before regenerating failed `the_layout_matches_the_committed_fixture`; `384da9dfc` adds states and fixture together |
| B2 `describe_difference` names every differing state, past the 20 detailed lines (SC-005; D8) | LIKELY | D8 records the red of `a_difference_past_the_detailed_ones_still_names_its_state`; test and change in `384da9dfc` |
| B3 Gate W1–W4: regions fit against each other (SC-004; T025) | NOT_APPLICABLE | Characterization of untouched layout code (FR-020), green by definition; strength from the T027 mutation proof and the mutants below |
| B4 Gate W5: `window-overview` has the Overview open beside the sidebar | NOT_APPLICABLE | Characterization over `State` fields (D6) |
| B5 `strip_at` generalises `strip_with`, its states byte-identical (T021) | NOT_APPLICABLE | Refactor of a fixture builder; the committed fixture held unchanged for the existing states |
| B6 `settings_sections` allowlist names the M2 pages (`228cd4d54`) | NOT_APPLICABLE | Guard allowlist entry for a menu item the pages now name |

**Existing tests.** `aa2df9ac9` moves the `at()` lookup of the three `strip_controls_*` gates into
`support::layout::anchored`, body unchanged (no assertion lost); `layout_snapshot.rs`'s describe
test now builds its input with `emit_fixture` instead of reading the fixture file. `228cd4d54`
widens `settings_sections`' `ALLOWED` list from 11 to 14 entries: each new entry is the live
*Session service diagnostics* menu item on a page M2 rewrote, not the removed page title the test
guards against. No assertion removed or loosened, nothing skipped.

**Tasks.** Every task T001–T034 is ticked and its milestone merged; T035 is ticked by this close.

## Findings

| # | Severity | Finding | Evidence |
| - | -------- | ------- | -------- |
| 1 | LOW | `w5_window_overview_opens_the_overview_and_selects_its_row` says "selects its row" but reads `overview.open`, `!overview.full_screen` and `workspace.active`; the comment explains these fields are the selection as shipped, since `ui::sidebar::row_selection` is private and FR-020 forbids a `src` accessor (D6). A future selection accessor should replace them | `crates/micold-client/tests/gates/window_regions_fit.rs:171,180-189` |
| 2 | LOW | Gate W's dark pass repeats the light pass's geometry, which `the_other_scheme_lays_out_identically` already pins; kept because the contract asks for both schemes (declined in M3 review A) | `crates/micold-client/tests/gates/window_regions_fit.rs:74` |

Loops in gate W iterate the fixed `WINDOW_STATES` × two schemes, and `windows()` panics on a
missing state, so no run asserts nothing. Every assertion names the state, the scheme, what is
measured and both numbers (`Window::expect`). Tests are deterministic (no clock, network or sleep)
and read the binary's record cache; the `layout_snapshot` target runs 146 tests in ~10 s warm.

## Mutation results

No mutation tool. The deliberate mutants touch only the feature's test files: Hard Rule 4 forbids
mutating `src`, which this feature did not change. The five `src` mutants of the T027 mutation
proof were run by M3 with the M3 commit and are recorded in the ledger; they are listed for
completeness, not re-run.

| Mutant | Behavior | Survived | Judgment |
| ------ | -------- | -------- | -------- |
| T027: title bar +1 px (`Toolbar` bar height) | B1, B3 | No | Fixture names both window states; W1 44 vs 43, W3 45 vs 44 |
| T027: status bar +1 px (`Lane::new(HEIGHT + 1.0)`) | B1, B3 | No | Fixture; W2 29 vs 28 |
| T027: sidebar 1 px narrower | B1, B2 | No | Fixture names both window states (via B2); W1–W5 hold, as the sidebar's width is not a W rule |
| T027: 1 px spacer under the title bar | B1, B3 | No | Fixture; W2 and W4 |
| T027: Overview closed in `window-overview` | B1, B4 | No | Fixture; W5 |
| This audit: `WINDOW_ANCHORS` `status_bar` path `[0,0,2]` → `[0,0,1]` | B3 | No | W2 fails: `window-session-view (Light): status_bar height is 728, want 28` |
| This audit: `strip_at` drops `session.lifecycle = replaced.lifecycle` | B1, B5 | No | Fixture fails, naming 10 differing states (both window states, eight title-bar states) |

Both audit mutants were restored with `git checkout`, and the full `layout_snapshot` target
re-run green (146 passed).

## Traceability

| Criterion | Tests | End to end |
| --------- | ----- | ---------- |
| FR-012 | `the_layout_matches_the_committed_fixture` (both window states in `tests/fixtures/layout_snapshot.txt`) | Yes: `ui::view` laid out by the real renderer |
| FR-013 | `the_layout_matches_the_committed_fixture`; text overflow: `no_text_is_drawn_wider_than_its_clip` (`tests/layout_text_overflow.rs`); containment: `no_layout_node_escapes_its_parent` (`tests/gates/containment.rs`) — both over every covered state in the light scheme, with dark held to the same geometry by `the_other_scheme_lays_out_identically` (`tests/layout_snapshot.rs`) | Yes |
| SC-004 | `w1_…`, `w2_…`, `w3_…`, `w4_…` (both schemes) | Yes |
| SC-005 | `the_layout_matches_the_committed_fixture`, `a_difference_past_the_detailed_ones_still_names_its_state`, gate W | Yes |

The other criteria (FR-001–FR-011, FR-014–FR-020, SC-001–SC-003, SC-006, SC-007) are about the
user-guide text, the visual-pass record or the absence of a `src` change; they are held by
quickstart A1–A2, the specs-only and docs gates, reviews A and B of each milestone, and
`git diff --stat origin/main -- 'crates/*/src'`, not by tests. Tests tracing to nothing: none.

## What was not audited

- No `tdd/test-list.md` or `tdd/cycle-log.md` exists: test-first order cannot be `PROVEN` for
  any behaviour.
- No mutation or coverage tool: strength rests on 7 deliberate mutants (5 from M3, 2 here), not
  a measured score.
- The smell pass was not delegated to a fresh-context reviewer (no Agent tool in this run).
- The doc criteria were not graded here; they are not tests.
- Rendering on macOS and Windows (CI legs) and the sandbox suite were not run; the feature
  changes neither.
