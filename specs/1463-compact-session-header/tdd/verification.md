---
feature: 1463-compact-session-header
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 2ac41d5a0
behaviors: 8 # no tdd/test-list.md; behaviors from cycle-log.md rows and tasks.md (M1, M2) plus the M3 remediation tasks T021-T028
proven: 0
likely: 0
test_after: 8
no_test: 0
high_smells: 1
criteria_total: 11
criteria_covered: 11 # FR-010 docs and FR-011 only indirectly/numerically
mutation_score: "3 caught of 4 run (touch-target mutant survived); 4 of the 7 planned mutants not run (build lock starvation)" # no mutation tool; hand-made mutants
mutants_survived: 1
suite: "layout_snapshot 163 passed, session_area_stability 16 passed, 0 failed, ~19 s run (3m13s wall incl. build); micold-material lib not re-run as a whole; mise run gate not run"
---

# TDD Verification: Compact session header (re-audit after M3)

**Verdict: FAIL (test strength).** The new FR-012 touch-target check added in M3 never runs: it compares part names that cannot match, and a mutant raising its threshold to 100000 passes all 163 layout tests.
Audit is independent (the auditor did not write the tests). Supersedes the report at `e727ac2`.

Still no `tdd/test-list.md` (rubric says BLOCKED; audited anyway against `spec.md`/`tasks.md`).

## Test-first evidence

| Behavior | Class | Evidence |
| --- | --- | --- |
| M1 one-line header + compact button; title shortens first; H10 fit gates | TEST_AFTER | `tdd/cycle-log.md:3-4` admits gates and code were written in one pass; unchanged since the first report |
| M2 strip gives way by rank; New session/splits whole; restart position; `switch_offer_of`; tiled row | TEST_AFTER | `tdd/cycle-log.md:19-20` "implementation came first" |
| M3 T021-T028 (gates pinned against mutants) | NOT_APPLICABLE / unrecorded | These are characterization tests on existing code; the red is a mutant failing. `tdd/cycle-log.md` has no M3 section; proof is only in `autopilot.md:50-51` (reviewers ran mutants T023, T026). The order in git: one commit `0b8207078` changes tests only (no `src` change besides test modules in `terminal.rs`), so no source/test ordering issue |

The missing red for M1/M2 is a recorded limitation that cannot be redone; it is carried over, not remediation work.

## Existing tests weakened

The previous HIGH (F2, `assert_button_unsqueezed` loosened to `0 < w <= wanted`) is fixed: `session_area_fits.rs:231-248` now asserts the cut width equals the room left by the line (`expected = wanted.min(line_right - left)`, line 240). `a_tiled_pane_still_has_its_switch_row` (1458) was strengthened from a height heuristic to path lookup with the recorded 487/48/34.2/4/8/40 numbers. No assertion was removed, no test ignored or renamed, no threshold lowered in `main...HEAD`. The `T024` redundant restart conjunct was dropped (`:1297`), which is sound (implied by the first).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (vacuous assertion / conditional logic in test) | The M3 touch-target gate in `assert_fits_under` is guarded by `what == "New session" \|\| what == "split buttons"`, but `strip_parts` names the parts `"strip New session"` and `"strip split buttons"` (`session_area_fits.rs:149-159`, `format!("strip {name}")`). The `if` is never true, so nothing is asserted. Mutant: threshold `MIN_TOUCH_TARGET - TOLERANCE` -> `100000.0` leaves `cargo test -p micold-client --test layout_snapshot` at 163 passed. Task T024 ("natural-width check of New session/split buttons") is ticked but its assertion is dead. Should match the real names (or iterate the two by index) and assert at least one part was checked | `crates/micold-client/tests/gates/session_area_fits.rs:266-273` (names at 149-159) |
| 2 | MED | `a_button_is_as_tall_as_its_density_step` computes its expected heights with `density::height(...)`, the function the code under test calls (re-implemented expectation); only the `DENSE < STANDARD` line is independent. `density::STANDARD` -> `DENSE` mutants still fail, but a wrong `density::height` would pass the two `assert_eq!`. Prefer literal 40 / 28 | `crates/micold-material/src/button.rs:799-819` |
| 3 | MED | Recorded literal numbers in `a_tiled_pane_still_has_its_switch_row` (487.0, 48.0, 34.2) are tied to the 1280x800 fixture and the pane header's current height; provenance is a doc comment only (the pre-change numbers cannot be re-derived from the tree). Acceptable, but brittle to any pane-header change | `session_area_fits.rs:1456-1500` |
| 4 | MED | No M3 section in `tdd/cycle-log.md`; the mutant proofs for T021-T027 exist only as reviewer notes in `autopilot.md:50-51` (only T023 and T026 named). Survivor-kill evidence for T021, T022, T025, T027 is not recorded | `tdd/cycle-log.md` |
| 5 | LOW | `CAPTION_CAP = 120.0` is a restated literal (intended; see commit rationale) but plus `SM` slack makes the assertion `<= 128 + TOL`, so a cap of 124 survives | `session_area_fits.rs:1350,1378` |

No assertion-free, doubled-subject, over-mocked, self-approving snapshot, skipped or sleepy tests found in the M3 additions. `the_offer_carries_the_held_queue_caption_only_while_messages_wait` follows the unit-in-source exemplar (`terminal.rs` test mod, `shown_session`); `GiveWayRow` test reads the private `slots` of the type in its own module (implementation coupled, but in the exemplar style and gives the rank order directly).

## Mutation results

No mutation tool in the profile. Hand-made mutants, one at a time, each restored with `git checkout` (tree verified clean). Seven were planned; the run was starved by other worktrees' build locks (`flock` queue held > 40 min) and stopped after three post-baseline results, so four were not run.

| Mutant | Behavior | Caught | Judgment |
| --- | --- | --- | --- |
| `session_area_fits.rs` touch-target threshold -> 100000.0 | FR-012 New session/splits whole (T024) | **No** (163 passed) | HIGH, F1: dead assertion |
| `terminal.rs:719` `caption: None` | FR-004/FR-012 held-queue caption (T025) | Yes: `the_offer_carries_the_held_queue_caption_only_while_messages_wait` | pinned |
| `terminal.rs:761` tiled row padding `[XS,SM]` -> `[SM,SM]` | FR-007/SC-004 (T022) | Yes: `a_tiled_pane_still_has_its_switch_row` | pinned |
| `SWITCH_CAPTION_MAX_WIDTH` 120 -> 400 (T023) | FR-012 | not run to completion | reviewer B recorded it failing (`autopilot.md:51`); not independently confirmed |
| `.shorten(1, words)` -> `.shorten(0, words)` (T027) | FR-012 | not run | unverified |
| `button.rs:433` density ignored (T026) | FR-003 | not run | unverified (reviewer B recorded failing) |
| `give_way.rs:704` `Shorten(rank)` -> `Keep` (T026) | FR-012 | not run | unverified (reviewer B recorded failing) |

Score on what ran: 2 caught, 1 survived (one survivor is non-equivalent and inside a ticked task). Not comparable to anything; sample only.

Restore verified: `git status --short` empty after the run; baseline suites green before mutation (layout_snapshot 163, session_area_stability 16).

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 sc.1-3, FR-001/002/003 | `the_header_is_one_line_with_a_compact_changes_button`, `the_title_shortens_before_the_chips_and_the_button`, `assert_button_unsqueezed` (exact again) | Yes |
| US2 sc.1/2, FR-004/005/009 | `a_claude_terminal_session_has_two_rows_and_its_switch_in_the_strip` | Yes |
| US2 sc.3, FR-012, SC-006 | `a_long_caption_is_cut_...`, `the_switch_slot_gives_way_before_the_status_words`; touch-target part **dead (F1)** | Yes, with F1 gap |
| US2 sc.4, FR-006 | `restart_keeps_its_place_when_the_review_is_shown_and_closed` | Yes |
| US2 sc.5, FR-009 | `a_session_without_a_switch_has_two_rows_and_an_empty_slot` | Yes |
| US2 sc.6, FR-007, SC-004 | `a_tiled_pane_still_has_its_switch_row` (now pinned by numbers) | Yes |
| US3 sc.1, FR-008, SC-005 | `session_area_stability` | Yes |
| US3 sc.2, SC-003 | `assert_fits_under` family | Yes |
| FR-010 | layout fixture test; docs | docs untested |
| FR-011 | `the_session_tab_width_is_its_recorded_value` (`tab.rs:643`) | Unit, direct |

Untested criteria: none. Tests tracing to nothing: none.

## What was not audited

- `mise run gate` and the full workspace suite were not run; only `layout_snapshot`, `session_area_stability` and three mutant runs.
- Four of the seven planned mutants were not run (build lock starvation), including `SWITCH_CAPTION_MAX_WIDTH`, rank tie, density and `GiveWayRow`; their status is taken from the reviewer ledger, unverified.
- The Changes-button cut-width exact check (T021) was not mutated.
- Visual quality, user-guide wording, performance beyond observed run time.
- The pre-feature numbers behind the recorded tiled-row values (487/48/34.2) were not re-derived.

## Close-unit addendum

Findings 1 (touch-target gate matched no part), 2 (button test used the code under test) and 4 (cycle-log M3 section) were fixed in the close PR as T029-T031; the T029 mutant now fails `layout_snapshot`. Mutants T021, T024, T027 and the rank tie were not re-run at close. The missing red phase (all M1/M2 behaviors test-after) stays a recorded limitation.
