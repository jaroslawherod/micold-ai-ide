---
feature: 1463-compact-session-header
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: e727ac2
behaviors: 8 # no tdd/test-list.md; behaviors taken from cycle-log.md rows and tasks.md test tasks
proven: 0
likely: 0
test_after: 8
no_test: 0
high_smells: 0
criteria_total: 11 # US1 sc.1-3, US2 sc.1-6, US3 sc.1-2
criteria_covered: 11 # FR-010 docs and FR-011 only indirectly
mutation_score: "7/11 = 64% (12 hand-made mutants, 1 equivalent excluded)" # no mutation tool in the profile
mutants_survived: 5 # 4 test gaps (caption cap, held-queue caption, tiled row padding, rank tie), 1 equivalent
suite: "cargo test --workspace --no-fail-fast: 11541 passed, 0 failed (one earlier run hit a flaky micold-core streaming_child test, green on rerun); mise run gate not run"
---

# TDD Verification: Compact session header

**Verdict: FAIL (test strength).** Four non-equivalent mutants survive and an existing H10 assertion was loosened. The missing red phase (all behaviors `TEST_AFTER`, cannot be redone) is a recorded limitation, not remediation work.
The auditor did not write these tests, so the audit is independent.

No `tdd/test-list.md` exists, so the feature was not planned through this extension. Rubric strictly says BLOCKED for that;
the audit was run anyway against `spec.md` and `tasks.md` and the verdict is FAIL on the evidence found.

## Test-first evidence

| Behavior (cycle-log row / task) | Class | Evidence |
| --- | --- | --- |
| M1 one-line header + compact button (T003, FR-001/003) | TEST_AFTER | log: "no commit shows a gate failing"; `f210aace5`/`78a45d077` change `session_area.rs` and `session_area_fits.rs` together |
| M1 title shortens first (T004, FR-002) | TEST_AFTER | same; red described as observed mid-edit, not recorded as command + output |
| M1 H10 fit gates at 640 px | TEST_AFTER | existing gate loosened to match the new layout (see F2) |
| M2 strip gives way by rank (FR-012) | TEST_AFTER | log: "implementation came first"; `e4f4e3326` adds `session_area.rs`, `terminal.rs` and gates in one commit |
| M2 new session / splits stay whole | TEST_AFTER | same |
| M2 restart position (stability path `[1,0,2]`) | TEST_AFTER | stability test edited to follow the moved control |
| M2 `strip_switch_offer` unit (T009) | TEST_AFTER | `terminal.rs` source and its test land in `46fbd7663`/`e4f4e3326` together; log says "written with the helper" |
| M2 tiled row kept (T011) | TEST_AFTER | log: "accepted as a known weak gate" |

Tasks.md: T003-T018 ticked and each has a log row or gate; T019 unticked (visual pass, deliberately out of milestone). No `[X]` without evidence found. tasks.md says tests are "seen red first" (Constitution I); the log contradicts that, which is a discrepancy between the plan and the record.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | LIMITATION (not blocking; cannot be redone) | No behavior has a recorded red command/output; log admits gates were written after the code in both milestones | `tdd/cycle-log.md:3-4,19-20`; `git show --stat e4f4e3326 46fbd7663 78a45d077 f210aace5` |
| 2 | HIGH (rubric: weakened existing test) | `assert_button_unsqueezed` was an exact check "button laid out at the width it asks for (H10)"; it now only requires `0 < width <= wanted` whenever the title is under `TITLE_MINIMUM` (40 px). Justified by the spec amendment (user Q1), but the weakening is reported regardless | `crates/micold-client/tests/gates/session_area_fits.rs` (fn `assert_button_unsqueezed`), diff in `78a45d077` |
| 3 | MED | Another existing assertion loosened: parts may now have zero width if named `"strip tab region"` (before only `"title"`). Backed by FR-012 ("tab region may shrink to nothing") | `session_area_fits.rs:269` |
| 4 | MED | `a_tiled_pane_still_has_its_switch_row` does not check FR-007/SC-004 as T011 specifies (size and position relative to the pane-area origin equal to pre-change values). It finds tiles by a height heuristic (`36 +/- 12`, `width > 100`) and asserts only `y` within 80 px of the pane area; wrong row size would pass. Log admits it | `session_area_fits.rs:1434-1475` (heuristic ~1455) |
| 5 | MED | `SWITCH_CAPTION_MAX_WIDTH` 120 -> 400 survives: the caption gate compares against the same constant the code uses, so the value of the cap is pinned by nothing (tautological on the constant). The assertion "button whole / caption within cap" still checks the rule | `session_area_fits.rs:1330-1375` |
| 6 | MED | Magic bounds: `TWO_ROWS_MAX = 100` against T010/spec "about 90" (comment explains: strip is 64 px); `window` loops; `80.0`, `36.0`, `12.0` literals in the tiled gate | `session_area_fits.rs:1210,1455-1470` |
| 7 | LOW | Redundant conjunct: `restart.x >= switch.x + switch.width - TOL && restart.x + restart.width >= switch.x + switch.width` (second is implied by the first) | `session_area_fits.rs` (restart check in the two-rows test) |
| 8 | LOW | Assertion roulette: long multi-assertion gates (`a_claude_terminal_session_has_two_rows...`) loop over states and windows; messages do name state, so low severity | `session_area_fits.rs:1235-1285` |
| 10 | MED | Held-queue caption branch of `switch_offer_of` (`caption: state.held_queue_line(..)`) is untested: replacing it with `None` passes every test | `terminal.rs:709`; mutant A |
| 11 | MED | `GiveWayRow::{keep,shorten}` and `Button::density` have no unit test in micold-material; mutants are caught only by client layout gates | `give_way.rs` (tests cover `plan` only), `button.rs:193` |
| 12 | MED | Rank tie between the cluster's words and switch slot (`shorten(1, words)` -> `shorten(0, ..)`) is not pinned; order of giving way between words and switch is untested | `session_area.rs:399-401`; mutant I |
| 13 | LOW | Switch-row heuristic, FR-012 natural width of New session/split buttons and "title width > 0 waived" in `session_area_fits.rs` weaken the gate; FR-011 (`SESSION_WIDTH`) has no direct test | known from first static pass |
| 9 | LOW | Layout fixture `layout_snapshot.txt` re-recorded wholesale (14,231 and 1,681 line diffs, M1 and M2) in the same commit as the code; self-approval risk mitigated by the visual pass evidence in `specs/1463-compact-session-header/visual-pass/` and by the geometry gates | `78a45d077`, `46fbd7663` |

No assertion-free, doubled-subject, over-mocked, vacuous or skipped tests found in the new tests. No coverage or mutation threshold exists in the repo to lower. No test marked ignored, renamed out of a filter, or excluded by config in the range.

Style and helper use: new gates reuse `covered_states`, `StateUnderTest`, `records_of`, `with_review_of` (made `pub` rather than copied), matching the profile's conventions; `claude_terminal` fixture is new but builds on `session_pane`. No foreign style found.

Suite properties: layout target ~10 s, stability ~1 s, deterministic (no clock, no sleep); failures name state, theme and window.

## Mutation results

No mutation tool in the profile. Twelve hand-made mutants, one at a time, each restored with `git checkout` (tree verified clean). Round 1 ran the lib, `layout_snapshot`, stability and size-reporting targets; round 2 ran `cargo test -p micold-client -p micold-material` in full.

| Mutant | Behavior | Caught | Judgment |
| --- | --- | --- | --- |
| `terminal.rs` `strip_switch_offer`: drop `review_shown` guard | FR-006 | Yes: `restart_keeps_its_place_when_the_review_is_shown_and_closed` | pinned |
| `terminal.rs`: drop `several` guard | FR-007 | Yes, only by the unit test (`--lib`); no gate fails | pinned, not through layout (F4) |
| `session_area.rs`: remove `.density(DENSE)` on Changes button | FR-003 | Yes, 5 tests | pinned |
| `session_area.rs`: `SWITCH_CAPTION_MAX_WIDTH` 120 -> 400 | FR-012 | No | gap, F5 |
| `terminal.rs`: drop `shows_transcript` guard | FR-004 | No | equivalent (`switch_offer_of` rejects non-`Terminal` kinds) |
| `terminal.rs` `switch_offer_of`: `caption: None` (held-queue caption branch) | FR-004/012 | No | gap, F10 |
| `give_way.rs` `shorten` builds `GiveWay::Keep` | FR-012 | Yes, 4 client gates; no micold-material test | pinned only via client (F11) |
| `button.rs`: height ignores `b.density` | FR-003 | Yes, 5 client gates; no micold-material test | pinned only via client (F11) |
| `terminal.rs` tiled row padding `[XS, SM]` -> `[SM, SM]` | FR-007/SC-004 | No | gap, F4 |
| `session_area.rs`: caption slot `.shorten(0, caption)` -> `.keep(caption)` | FR-012 | Yes: `a_long_caption_is_cut_before_the_switch_or_restart_is_clipped` | pinned |
| `session_area.rs`: `.shorten(1, words)` -> `.shorten(0, words)` (rank tie in the cluster) | FR-012 | No | gap, F12 |
| `session_area.rs`: status cluster `.shorten(0, ..)` -> `.keep(..)` | FR-012 | Yes, 4 gates | pinned |

Score: 7 caught of 11 non-equivalent = 64%. Not exhaustive. `mise run gate` and the micold-core streaming_child flake (unrelated, passed on rerun) are noted under the suite.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 sc.1 (order on one line), FR-001 | `the_header_is_one_line_with_a_compact_changes_button` | Yes (real `view`, layout walk) |
| US1 sc.2 (button no taller), FR-003 | same | Yes |
| US1 sc.3 (title first), FR-002 | `the_title_shortens_before_the_chips_and_the_button` | Yes |
| US2 sc.1/2, FR-004/005/009 | `a_claude_terminal_session_has_two_rows_and_its_switch_in_the_strip` | Yes |
| US2 sc.3 (caption beside switch), FR-012, SC-006 | `a_long_caption_is_cut_before_the_switch_or_restart_is_clipped` | Yes, cap value unpinned (F5) |
| US2 sc.4 (Review hides), FR-006 | `restart_keeps_its_place_when_the_review_is_shown_and_closed` | Yes |
| US2 sc.5 (non-terminal two rows), FR-009 | `a_session_without_a_switch_has_two_rows_and_an_empty_slot` | Yes |
| US2 sc.6 (tiled unchanged), FR-007, SC-004 | `a_tiled_pane_still_has_its_switch_row`; unit test | Weak (F4) |
| US3 sc.1 (stable child list), FR-008, SC-005 | `session_area_stability` (source-shape tests, `rendered_presses`) | Yes |
| US3 sc.2 (no overlap at 640x480), SC-003 | `the_filled_strip_and_the_header_fit_at_the_smallest_window`, `assert_fits_under` family | Yes |
| FR-010 (snapshots, H9, user guide) | `the_layout_matches_the_committed_fixture`; docs edited, no test | Docs untested |
| FR-011 (SESSION_WIDTH unchanged) | none; guarded only by tab snapshots | Indirect |

Untested criteria: none outright; FR-011 and the doc part of FR-010 have no direct test. Tests tracing to nothing: none.

## What was not audited

- `mise run gate` (fmt, clippy, scripts tests) was not run; the workspace `cargo test` was.
- Mutation tooling absent; twelve hand-made mutants only. `give_way.rs` was mutated only through `shorten`; its own unit tests were not graded beyond noting they cover `plan` only.
- `catalog_two_daemons.rs` and `scripts/tests/settings-hooks.test.sh` changed in the range via merged main commits (#1467, #1440), not feature 1463; not graded.
- Visual quality (screenshots), user-guide wording, performance beyond observed run times.
- Whether the M1 history before the merge is squashed on `main` was checked only via the feature commits listed.
