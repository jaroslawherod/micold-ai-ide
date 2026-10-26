---
feature: 1275-review-stale-quick-comment
verdict: FAIL # TEST_AFTER history in M1–M3 (and three later rows), accepted at close; no weakened test, no HIGH smell found, 10/10 deliberate mutants caught
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 02f5abe50 # short SHA audited (M6 tip, PR #1450)
behaviors: 30 # rows of the table below; no tdd/test-list.md exists, milestones and the cycle log's rows stand in
proven: 14
likely: 3
test_after: 7
no_test: 0
not_applicable: 6
high_smells: 0 # smell pass by hand, not by a fresh-context reviewer (see What was not audited)
high_findings: 1 # the test-first breach (7 behaviors)
criteria_total: 36 # FR-001..FR-029 with FR-001a (30), SC-001..SC-006
criteria_covered: 36 # FR-029, SC-003 and SC-006 by docs, snapshot and the visual pass, see Traceability
mutation_score: unmeasured # no mutation tool in the profile; 10 deliberate mutants, 10 caught
mutants_survived: 0
suite: scoped — micold-core lib review:: 337 passed (1 ignored, 0.33 s); micold-client --test features_changes 224 passed (0.06 s); full gate per milestone, see the ledger
---

# TDD Verification: Review UI — stale-awareness and quick comment

**Verdict: FAIL.** M1 and M2 (core) were written test-and-code in one commit with red shown only by
later mutation, and M3's T015–T020 tests were written after the code (stated in the M3 PR); three
later rows have the same shape. No existing test was weakened, and test strength is good: ten
deliberate mutants across the core logic and the client reducer all failed a named test. The
breach is in the history, not in the tests as they stand, and is accepted at close as #1273's was.

Independence: this audit ran in the close unit of the autopilot run that wrote the feature, and
this unit had no Agent tool, so the smell pass was not delegated to a fresh-context reviewer. Every
file cited here was re-read for this report.

## Test-first evidence

There is no `tdd/test-list.md`; the milestones' task ranges and the cycle log's rows (M4–M6) stand
in. Milestones were merged by rebase, so each feature commit keeps its order on `main`.

| Behavior | Class | Evidence |
| --- | --- | --- |
| M1 T001–T008 change identity, `review::stale` (core) | TEST_AFTER | one commit `3a06669ce`; ledger: "TDD red shown by mutation (no `identify`, no `--no-abbrev`: I1a, I3–I6 fail)" |
| M1 review A F1–F6 (adopt, identify fallback) | LIKELY | PR body "each with a red test first"; tests and fix in one commit `38ebac0e3`, no red output logged |
| M2 T009–T014 `review::drag`, `review::chips`, setting (core) | TEST_AFTER | one commit `2b5f32bec`; no red recorded |
| M3 T015–T020 hold, notice, dots, apply (client) | TEST_AFTER | M3 PR body: "written after the code (red not observed)" |
| M3 review A F1–F6 (`chose_gone`, …) | LIKELY | PR body "seen red first"; no output logged |
| M3 T021–T025 docs, snapshot, visual pass | NOT_APPLICABLE | docs, approval fixture, screenshots |
| M4 `Drag::span` with line 0 | PROVEN | red logged (panic at drag.rs:67) |
| M4 T026 number cell press/over | PROVEN | compile red logged |
| M4 Q1–Q9, file change mid-drag (T027–T029) | PROVEN | compile red logged (`OpenView.drag`, `Msg::GutterDragOver`) |
| M4 T030 release listener | PROVEN | red logged with the Esc arm put back (review B B1) |
| M4 forget "chose a gone file" (M3 MINOR a) | PROVEN | assertion red logged |
| M4 slots stay under the pre-press pick | TEST_AFTER | "fix written first"; red by later mutation (review B B1) |
| M4 click takes an open box to its line (C4) | PROVEN | old test red after the slot fix, logged |
| M4 **Add comment** ends a dangling drag | PROVEN | 3 failures logged |
| M4 review A F1 (release in the press's batch) | PROVEN | assertion red logged (diff_view.rs:1545) |
| M4 review A F2 (Esc during a drag) | PROVEN | assertion red logged |
| M4 review A F3 (tab switch, focus loss) | PROVEN | assertion red logged; listener test red with the arm removed |
| M5 T037 keyboard chip | PROVEN | compile red logged |
| M5 T035 the switch on the draft | PROVEN | compile red logged |
| M5 T038 save keeps every other setting | TEST_AFTER | red by mutation only (no `set!`) |
| M5 T038/T039 save applies, cancel discards | PROVEN | compile red, then mutation, logged |
| M5 Q10–Q12, H14 chips in the reducer | PROVEN | compile red logged |
| M5 T042 snapshot state | NOT_APPLICABLE | approval fixture |
| M5 T040/T041 view, docs | NOT_APPLICABLE | drawing glue covered by the snapshot; docs |
| M6 T044 H16 | PROVEN | assertion red logged (features_changes.rs:5923) |
| M6 T045 core `held_file_review` | LIKELY | test written before the function (compile error, not run); red shown by mutation |
| M6 review A F1, F2, F3 | PROVEN | assertion red logged for each |
| M6 review A F4 | TEST_AFTER | test-only tightening, passes on correct code (logged as such) |
| M6 T046 docs and visual pass | NOT_APPLICABLE | docs, screenshots |
| T047 close (this unit) | NOT_APPLICABLE | quickstart results |

**Existing tests.** Removed or changed assertions across the feature's commits were searched
(`git show -U0` over every `1275` commit, `^-\s*assert`). Two hits, both the feature's own tests:
`94467d743` removed the app-level focus-loss test added two commits earlier in `68acd6d1a`, replaced
by the listener test `changes_drag_message_ends_on_release_and_cancels_on_blur` (logged);
`eac7ed877` changed H16's `c.rs` row from `(HasDrafts, true)` to `(HasDrafts, false)`, review A F3's
spec decision (a mark on the current id is not *changed since its mark*), with an older-id case
added. No pre-existing test was loosened, skipped or filtered.

**Tasks against list.** T001–T046 are ticked and each milestone's tests exist; T047 is ticked by this
unit.

## Findings

1. **HIGH — test-first breach (7 behaviors).** M1 T001–T008, M2 T009–T014, M3 T015–T020, M4 slot
   pick, M5 T038 keep-other-settings, M6 F4 (test-only). Each now has a test that fails on a
   mutant (ledger, cycle log, and the mutants below), so the gap is ordering, not coverage. No
   remediation task: the code is merged and the tests are as strong as test-first ones would be.
2. **LOW — red claimed but not logged** for the M1 and M3 review-A fix tests (LIKELY). The cycle log
   starts at M4.
3. **LOW — glue without its own test** (left at tasks time): T030's subscription wiring is covered by
   `subscriptions::tests`; the view functions of T022/T031/T040 by the layout snapshot only. A
   restored `suggestion_chips: true` at boot has no test (`startup.rs`, review A M5 MINOR); the
   M5 visual pass's restart covered it.

Smell pass (by hand, over `review/stale.rs`, `review/drag.rs`, `review/chips.rs`,
`review/marks.rs` tests and the `hold::`, `drag::`, `chips::`, `reviewed_marks::` modules of
`tests/features_changes.rs`): tests assert values through the public reducer and core functions,
use the file's existing helpers (`reading_a`, `background`, `list`, `view`), and name the
behaviour; no tautological or re-implemented expectation found. Not graded item by item against the
rubric's catalogue (see below).

## Test strength — deliberate mutants

One change at a time, the scoped suite run, the file restored with `git checkout`, tree checked
clean afterwards.

| Mutant | Caught by |
| --- | --- |
| `same_change` ignores the identity | `stale::s2_kind_counts_and_identity_count_…` |
| a rename's old path counts as changed too | `stale::s4_a_rename_counts_once_under_its_new_path` |
| `Held::adopt` keeps the old set | `stale::s8`, `s11`, `s12` |
| a chip after a trailing newline adds another | `chips::k4_a_trailing_newline_adds_no_blank_line` |
| an old-side drag reads the new numbers | `drag::d6_an_old_side_drag_follows_context_rows_old_numbers` |
| `held_file_review` ignores the hold | `marks::a_held_change_to_a_marked_file_is_not_reviewed` |
| a held background read replaces the shown list (client) | 8 `hold::` tests |
| chips shown with the setting off (client) | `chips::q10`, `q11`, `q12` |
| applying keeps a selection no longer listed (client) | `a_refresh_that_drops_the_selected_file_closes_a_new_comment`, `back_to_a_diff_whose_file_left_the_list_starts_at_the_top` |
| the mark control not blocked while held (client) | `reviewed_marks::the_mark_control_is_blocked_while_the_selected_file_is_held` |

10 sampled, 10 caught. Sampled for the acceptance criteria they carry (FR-001a/003, FR-001
renames, FR-009, FR-024, FR-017, FR-013, FR-005/006, FR-022/023, FR-010, FR-013); not exhaustive.

## Traceability

| Criteria | Tests (real entry point: the reducer's `update` or the core function) |
| --- | --- |
| FR-001, FR-001a, FR-002, FR-003, FR-004 | core `review::stale::tests::s1`–`s12`, `review::git` I1–I8 (`tests/review_git.rs`, real git) |
| FR-005, FR-006, FR-007, FR-008, FR-010, FR-011, FR-012, FR-014 | `features_changes` `hold::` (H1–H10a, H14, H15); `features_pr_changes` H11–H13 (pull request hold) |
| FR-009 | `hold::` H3 `a_read_the_user_causes_applies_even_queued_behind_a_watch_read`, H10a `DiffRow::from_read`, adopt tests |
| FR-013 | `reviewed_marks::` H16 and the review-A F1/F2 tests; core `marks::a_held_change_…` |
| FR-015–FR-021 | core `review::drag` D1–D9; `features_changes` `drag::` Q1–Q9; `diff_view::tests` gutter cell; `subscriptions::tests`; `overlay_dispatch_ordering` |
| FR-022, FR-026 | client lib `features::settings::suggestion_chips_tests`; bin `persist` and `suggestion_chips_are_applied_by_save_and_discarded_by_cancel`; core K5 |
| FR-023, FR-024, FR-025 | core `review::chips` K1–K4; `chips::` Q10–Q12, H14 |
| FR-027 | material `toggle_chip::tests::a_keyboard_chip_is_one_stop_…`; M5 visual pass (Tab order) |
| FR-028 | the core modules above (pure, unit-tested) |
| FR-029, SC-006 | docs diff; `layout_snapshot` states `changes-review-held`, `changes-review-held-chips`; `visual-pass/` |
| SC-001, SC-002 | H-tests above; quickstart §A steps 2 and 8 (close unit: 276 frames, none moved) |
| SC-003 | Q5 (one drag opens the box focused); M4 visual pass |
| SC-004 | Q10 (off by default), settings default test; M5 visual pass |
| SC-005 | H13, Q6, Q7; quickstart §A step 6 |

Every criterion has at least one test or recorded run; none traces to a missing test (the names
above were taken from the test files as they stand).

## What was not audited

- The smell pass was by hand in the unit that wrote the feature, not by a fresh-context reviewer,
  and not item by item against the rubric's catalogue.
- Mutation testing: no tool; 10 deliberate mutants only. `review::git` (real-git identity) and the
  view functions were not mutated.
- The full workspace suite was not re-run here beyond the scoped suites; each milestone's full gate
  is in the ledger.
- Performance (SC-002's 2 s) was observed once on a debug build, not measured under load.
- macOS and Windows: not run (quickstart §C).
