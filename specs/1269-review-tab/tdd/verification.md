---
feature: 1269-review-tab
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 41105c69 # M4 tip (PR #1394); M1–M3 on main
behaviors: 26
proven: 12
likely: 8
test_after: 0
no_test: 0
not_applicable: 6
high_smells: 0
criteria_total: 23 # FR-001–FR-017, SC-001–SC-006
criteria_covered: 23
mutation_score: null # no mutation tool (profile: mutation null); 5 deliberate mutants, 5 caught
mutants_survived: 0
suite: 10809 passed, 0 failed, 18 ignored, 656 binaries (scripts/gate.sh, green)
---

# TDD Verification: Review as a per-worktree tab that keeps its state

**Verdict: PASS_WITH_GAPS.** No HIGH smell, no weakened existing test, every FR and SC has a test
through the real entry point, and all five deliberate mutants were caught; the gaps are eight
behaviours whose only recorded red is a build error, a test-construction failure, or a mutation red
recorded after the fact, and no mutation tool to measure strength beyond the sample.

The audit ran inline in the close unit (cloud run: forked skills run inline), a fresh context that
wrote none of the tests; the smell pass went to a fresh `autopilot-reviewer` (sonnet) and every cited
line was opened before it reached this report. The feature has no `tdd/test-list.md`; tasks.md's
test tasks and `tdd/cycle-log.md`'s rows are the list (as features 482, 837, 946, 972).

## Test-first evidence

History: M1–M3 reached `main` by rebase-merge, so per-commit order survives; each `feat` commit adds
tests and source together, so order rests on the cycle log's reds.

| Behaviour (cycle-log row) | Class | Evidence |
|---|---|---|
| M1 T001 types, stubs, `Rng` move | NOT_APPLICABLE | structural |
| M1 T002–T003 T1, T2, T3, T7 | PROVEN | 10 failed on the stubs (`assertion left == right`), `67fe13f1` |
| M1 T004–T005 T4 drops, no-other-drop property | PROVEN | 3 failed; property red by mutation (`seed 0x1, step 63`) |
| M1 T006–T007 T5 freshness, T6 `session_for` | PROVEN | 11 failed, quoted |
| M1 T008–T009 T8 faces | PROVEN | 4 failed, quoted |
| M1 review A F1/F2 | PROVEN | 2 failed, `7db060b3` |
| M2 T012 `open` → `tabs` migration | NOT_APPLICABLE | structural; every existing client test green after |
| M2 T010/T011/T013 U1, U2 | PROVEN | old `selecting_a_session_closes_the_view` and two PR-choice tests failed, quoted |
| M2 T014–T016 U4, U9 | LIKELY | red was a build error only (`Msg::TabClosed`, `WorktreesListed {..}`) |
| M2 T017–T019 U3, U12 | LIKELY | old tests asserting the close were rewritten; no new red quoted |
| M2 T020–T022 U5, U6, U7, U11 | PROVEN | 3 failed by mutation, quoted |
| M2 T023–T024 U10 focus, chords, attention | NOT_APPLICABLE | characterization: FR-013/FR-014 require behaviour unchanged |
| M3 T026/T028 U3 other location, U7 remember | PROVEN | 2 failed against the `shown_location()` stub, quoted |
| M3 T027/T029 U8 | PROVEN | 1 failed, quoted; rewritten through `CloseRequested` after review A, failed again |
| M3 T030 `over_review` glue | NOT_APPLICABLE | view glue; covered by layout states and the M4 visual pass |
| M3 review A (every way the session goes) | PROVEN | T027 rewritten failed (`left: None`) |
| M3 review A r3/r4 gone session's location | PROVEN | red by mutation (read reverted to `header.entry`) |
| M4 T032/T035 `CountBadge` | LIKELY | build error only |
| M4 T033/T036 U13 members | LIKELY | build error, then a failure of the test's own fixture |
| M4 T034/T036 U14, U15 | LIKELY | build error, then the test's × lookup was wrong |
| M4 T037/T038 covered states, gates | LIKELY | failures were the test's layout descent and the fixture re-record |
| M4 review A F1 (strip press hides the review) | LIKELY | written with the fix; red recorded by this audit, mutant K1 below |
| M4 T041 finding 2 (location change reveals from start) | PROVEN | red by mutation, quoted |
| M4 T041 finding 1 (oversized member at leading edge) | LIKELY | written with the fix; red recorded by this audit, mutant K2 below |
| M4 gate: `feature_registration_cost` allowlist | NOT_APPLICABLE | an existing guard caught it; allowlist entries with reasons |
| M4 gate: material boundary, builder API, captions | NOT_APPLICABLE | existing guards caught them |

(The two gate rows count as one in `not_applicable`; 26 rows in all.)

**Existing tests.** The removed assertions across the feature's commits (`git show -U0` over every
`(1269)` commit) are the `state.open` → `state.tabs` migration (same predicate on the new API), fmt
re-wraps, and three deliberate flips each named in the cycle log and the spec: the session
selection now hides instead of closing (D1/FR-004), a Changes press no longer closes (D5/FR-012),
and the header follows the opened location (M3, R5). `the_header_stands_over_the_open_changes_view`
became `the_header_and_the_strip_stand_over_the_shown_review` with more assertions (three children,
both review states). Nothing skipped, ignored or filtered.

**tasks.md vs evidence.** T001–T041 ticked, each with a cycle-log row or a doc/visual-pass
deliverable; T042–T043 ticked by the close PR.

## Findings

| # | Severity | Finding | Evidence | State |
|---|---|---|---|---|
| 1 | MED | About 100 assertions in the core tests carried no message (the profile's convention: every assertion names its rule); `t3_close_…` and `t5_files_changed_…` pack several cases with unlabelled asserts, so a failure did not say which case broke | `crates/micold-core/src/review_tabs.rs` tests, e.g. `assert_eq!(tabs.close(&key, &[], None).unwrap().show, None);` | fixed in the close PR: a rule message on each (case named in t3/t5) |
| 2 | LOW | The Review tab gate fell back to the mark *slot* when the slot had no child, so a missing changed mark passed as the slot's own rectangle | `crates/micold-client/tests/gates/session_area_fits.rs:467` `mark.first().map_or(rect(slots[0]), …)` | fixed: the mark child is required |
| 3 | LOW | Two test names still said the old behaviour (closes) after M2 made it hide/drop | `crates/micold-client/tests/features_changes.rs:279,297` | fixed: `selecting_a_session_hides_the_review_and_keeps_it`, `the_entry_leaving_the_catalog_drops_its_review_tab` |
| 4 | LOW | The badge cap is implemented twice: `ReviewTabFace::badge_text` in core (used only by its own tests) and `count_badge::text` in material (what is drawn); both are tested | `crates/micold-core/src/review_tabs.rs:386`, `crates/micold-material/src/count_badge.rs:34` | follow-up (code, not a test fix): drop the core copy or have material call it |

Declined smell-pass findings (vetted against the cited lines):

- *Re-implemented expectation* in `the_review_member_has_one_width_and_the_offsets_and_reveal_count_it`
  (`session_strip.rs`): it asserts the Review member at `before + spacing::SM` and `REVIEW_WIDTH`
  wide. Fixed widths plus an SM gap is the specified layout rule (S6, R10), stated from constants,
  not computed through `member_offsets`.
- *Tautology* in `the_badge_is_the_section_lists_chip_around_a_number` (`count_badge.rs:132`): the
  equality is R11 itself — the badge is the section list's chip — and it fails if `CountBadge` draws
  its own pill.

Properties: all feature tests are headless, seeded (`Rng` with fixed seeds) and run in well under
a second per target; none sleeps or touches the network.

## Mutation results

No mutation tool (profile `mutation: null`). Deliberate mutants, one at a time, each restored
byte-for-byte (`cmp`) and the target re-run green:

| Mutant | Behaviour | Caught by | Survived |
|---|---|---|---|
| C1 `review_tabs.rs` badge cap `>` → `>=` | T8 | `t8_badge_text_caps_at_99_plus_and_keeps_the_count` | No |
| C2 `review_tabs.rs` `drop_gone_worktrees` keeps the missing, drops the listed | T4, FR-008 | both `t4_drop_gone_worktrees_…` tests | No |
| C3 `review_tabs.rs` `hide` leaves the tab shown | T2, FR-004 | 6 tests (t1, t2, t3, t5) | No |
| K1 `session.rs` `shows_the_terminal` drops `Outcome::ReviewHidden` | M4 review A F1, US1.2 | `a_session_shell_or_plus_press_hides_a_shown_review` | No |
| K2 `session_strip.rs` `scroll_into_view` without the oversized-member arm | T041 finding 1 | `a_member_beyond_either_edge_scrolls_just_into_view` | No |

Sample: 5 behaviours of 26, chosen where an acceptance scenario rests on one guard and where the
cycle log had no red (K1, K2). Not exhaustive.

## Traceability

Every FR in plan.md's requirement map checked against its test (T043):

| Criterion | Tests (crate, file) | End to end |
|---|---|---|
| FR-001 tab in the strip, header and strip above | `session_strip` `a_hidden_review_tab_is_the_last_member_and_marks_nothing`, `a_sessionless_locations_strip_is_its_review_alone`; `session_area_fits` `the_header_and_the_strip_stand_over_the_shown_review`; covered states | Yes (layout) |
| FR-002 at most one per location | core t1; `features_changes` `repeated_opens_keep_one_tab_per_location` | Yes |
| FR-003 button and Review changes create or show | `features_changes` `review_changes_on_another_location_shows_it_with_its_last_shown_else_main_session`; `session_area` `the_changes_press_opens_the_shown_location_or_shows_its_review` | Yes |
| FR-004 session/shell press keeps the tab | `features_changes` `selecting_a_session_hides_the_review_and_keeps_it`; `session_strip` `a_session_shell_or_plus_press_hides_a_shown_review` | Yes |
| FR-005 switches, Overview, Settings keep tabs | core property test; `features_changes` `switching_location_and_back_…`, `covering_the_review_…`, `switching_project_and_back_…`; `navigation` `a_session_chord_with_the_review_shown_…` | Yes |
| FR-006 kept view as left | `features_changes` `a_hidden_review_comes_back_exactly_as_left`, `two_reviews_keep_their_own_views` | Yes |
| FR-007 × removes, shows previous else main | core t3; `features_changes` `closing_the_review_tab_shows_the_session_and_starts_fresh_next_time`; `main_tests` `closing_the_review_tab_through_the_shell_…` | Yes |
| FR-008 removal drops | core t4; `features_changes` `removal_drops_only_the_removed_locations_tabs`, `the_entry_leaving_the_catalog_drops_its_review_tab` | Yes |
| FR-009 draft badge | `session_strip` `the_badge_counts_the_locations_drafts_and_none_while_unknown`; `count_badge` tests; core t8 | Yes (strip model from state) |
| FR-010 no read while hidden; changed mark | `changes` `a_hidden_view_reads_nothing_and_is_read_once_when_shown`, `a_change_marks_the_hidden_tab_of_the_headers_location`; `session_strip` `only_a_hidden_review_tab_carries_the_changed_mark`; `idle_subscriptions` | Yes |
| FR-011 re-read on show | core t5; `changes` `a_failed_reread_on_show_keeps_the_tab`; `features_changes` `covering_the_review_keeps_it_and_rereads_it_once` | Yes |
| FR-012 button selected; press changes nothing | core t8 `ChangesButton`; `session_area_stability` `pressing_changes_with_the_review_shown_changes_nothing` | Yes |
| FR-013 chords and focus unchanged | `terminal_focus`, `keyboard` `escape_with_the_review_shown_keeps_it`, `navigation`, `app_chord_tests` | Yes |
| FR-014 in-view as today | `attention_view_report` `view_facts_has_the_session_in_view_while_its_review_is_hidden` | Yes |
| FR-015 first-try presses, no overlap | `session_area_stability` (U14 cases), `session_area_fits` `the_review_tab_holds_its_label_badge_mark_and_close_whole`, `tab_reveal` | Yes |
| FR-016 rules without a window | `micold_core::review_tabs` tests (28) | Yes (`test-core`) |
| FR-017 docs, snapshot, visual pass | `layout_snapshot` fixture; docs gate; `visual-pass.md` | Yes |
| SC-001 every switch kind keeps the view whole | `a_hidden_review_comes_back_exactly_as_left`, `switching_location_and_back_…`, `switching_project_and_back_…` | Yes |
| SC-002 nothing but × or removal drops | core property test (5 seeds × 200 steps) | Yes |
| SC-003 no read while hidden | `a_hidden_view_reads_nothing_and_is_read_once_when_shown` (effect counts) | Yes |
| SC-004 every pane chord | `navigation`, `keyboard`, `app_chord_tests` | Yes |
| SC-005 never a second tab | `repeated_opens_keep_one_tab_per_location` | Yes |
| SC-006 strip fits at both sizes and themes | `session_area_fits`, `layout_snapshot` | Yes |

Untested criteria: none. Tests tracing to nothing: none found among the feature's tests.

## What was not audited

- Mutation strength beyond the five deliberate mutants: no `cargo-mutants`; coverage unavailable
  (`cargo-llvm-cov` absent), so untested branches in the changed files are unknown.
- `ui/session_area.rs`, `ui/mod.rs` and `ui/changes.rs` drawing glue: checked by layout states and
  the M4 visual pass (`visual-pass.md`), not by assertion-level tests.
- Quickstart B11 (twelve tabs) and B12 (chords) by hand, B9's Settings half: not run by hand (their
  tests cover them; recorded in the ledger).
- The macOS and Windows CI legs and the sandbox suite: not run here; no feature code is
  platform- or container-specific.
- Performance: no criterion, not assessed.
