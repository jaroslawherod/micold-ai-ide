---
feature: 1126-session-context-header
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no override, no preset; extension copy
verified_at: 9c1f758f
behaviors: 35 # test-bearing rows of tdd/cycle-log.md (no tdd/test-list.md exists); 12 structural/move/docs rows are NOT_APPLICABLE and not counted
proven: 25
likely: 3
test_after: 6
no_test: 1
high_smells: 2 # both `Re-implemented expectation`, severity fixed by the rubric; both mitigated, see findings 5
criteria_total: 22 # US1 sc1-6, US2 sc1-8, US3 sc1-5, US4 sc1-3 (SC-001..006 and FR-001..015 traced through them)
criteria_covered: 22 # every criterion has a test; 2 are unit-level only (finding 7)
mutation_score: unmeasured # no cargo-mutants; 6 deliberate mutants: 5 caught, 1 survived
mutants_survived: 1
suite: feature-scoped only (whole suite not run, by instruction). 1109 passed, 0 failed in client lib + 9 client test binaries (~64 s warm), 17 core lib, 2 bin; plus the 6 mutant runs
---

# TDD Verification: Worktree context header and session tabs above the terminal (#1126)

**Verdict: FAIL.** An existing test was loosened to fit the feature (`terminal_size_reporting.rs:133`,
`rows > 30` became `rows > 1`), six behaviors were written after their code (`TEST_AFTER`), and one
(the `ChangesRequested` queueing, F3 of M3 review A) has no test at all. The tests that exist are
mostly strong: 5 of 6 deliberate mutants were caught and the surviving one sits in a defensive guard.

Independence: this audit did not write the tests. The smell pass over the client files was done by a
fresh-context `autopilot-reviewer`; its findings were vetted by opening the cited lines (the ones kept
below). Nothing was run in the whole-workspace sense (instruction); the audited commits are on `main`
(`29622f59`, `8c9f4a43`, `af825f57`, `e7ec795e`, `127a8178`, `2deba4ad`, `064d13e0`, `0dae0103`,
`ce40a800`, `fc1c9107`, `832db49f`, `bfe21637`). The unmerged pre-rebase copies of M3 (`4a281f47`,
`e3646f9b`, `bae63f13`, `4a54f7fd`, `5a0366da`) are on `origin/claude/project-thread-y28cqn` only and
were not graded.

## Evidence sources and where they disagree

- **Cycle log** (`tdd/cycle-log.md`, 118 lines): reds quoted per row, with a stub or a left-out
  implementation. Rows that say "structural" carry no red (scaffolding, moves, docs); they are
  `NOT_APPLICABLE`.
- **Git history**: each milestone landed as one feature commit with tests and source together
  (`29622f59` M1, `2deba4ad` M2, `0dae0103` M3) plus review-fix commits. History therefore cannot show
  test-before-source *within* a milestone; the ordering evidence is the log, and "same commit" is what
  the rubric accepts for `PROVEN` when the log has a red.
- **Files as they stand**: read. No test is skipped or ignored by the feature (`grep '#\[ignore'` over
  the feature's diff: none). `keyboard.rs` and `navigation.rs` are untouched (T041 holds).
- **Discrepancy**: none where the log claims a red the history contradicts. Two the other way: the log
  itself admits T039, T045 (first run), T046, T050 and M3 review A F1 had no red taken first. Those
  are classed from the log's own words.

## Test-first evidence

No `test-list.md`: behaviors are the test-bearing rows of the cycle log, named by task.

| Behavior (task) | Class | Evidence |
|---|---|---|
| H1–H4 `HeaderText::of`, `StatusPhrase::of_status` (T006, T007, T012, T013) | PROVEN | stub run, 5 FAILED with values quoted (log:13) |
| Header follow entry (T005) | PROVEN | stub `Effect::None`, `a new entry is followed left: None` (log:14) |
| `location_card` equals the Overview card (T008, T014) | PROVEN | stub `None` (log:15) |
| Issue title read once on follow (T010, T015) | PROVEN | hook left out, `left: 0 right: 1` (log:18) |
| `Tag::max_width`, `Tag::leading` (T016) | PROVEN | builders as no-ops, widths quoted (log:19); the short-label case passes on the no-op by design |
| `pr_chip_input` (T009, T017) | PROVEN | stub `None` (log:20) |
| US1 sc. 5, header and sidebar agree (T010) | PROVEN | log:21 |
| Root follow step (T005, T010 H7) | PROVEN | step removed, `left: None` (log:22) |
| H10 geometry, header drawn (T011) | PROVEN | header left out, 2 FAILED (log:23) |
| M1 review A F1, titles become readable | PROVEN | log:31-35 |
| S1–S4 `StripModel::of` incl. labels (T021, T032) | PROVEN | stub, 12 FAILED (log:83-84) |
| S5 press/close/menu per member (T022) | PROVEN | log:85 |
| S6 `member_offsets`, overflow, `scroll_into_view` (T023) | PROVEN | log:86 |
| `SESSION_WIDTH` derived (T030) | PROVEN | constants `0.0`, `left: 0.0 right: 120.0` (log:87) |
| `new_terminal_chord_label` (T031) | LIKELY | log names a stub `""` but quotes no failure output (log:88) |
| Keyed slot follows its key (T024) | PROVEN | positional Row, `left: [0,7,0] right: [0,0,7]` (log:89) |
| Retargeted `terminal_tabs`, `tab_reveal` (T026) | PROVEN | log:90 |
| `session_area_stability` gates (T025) | TEST_AFTER | "the new gates pass on arrival"; the red quoted is the *old* file failing against the new view (log:91) |
| Strip geometry gates (T027) | PROVEN | mutant, 12 FAILED then 3 FAILED (log:93) |
| `session_area_fits` strip (T028) | PROVEN | same mutant (log:94) |
| Focus guard (T039) | TEST_AFTER | "passes on arrival; written after T033, not before T034" (log:96) |
| Press in flight (T040) | PROVEN | shell-opens case red against a positional Row; the other two "hold positionally by design" (log:97), see finding 9 |
| M2 review A F1, `Started` arms reveal | PROVEN | log:100 |
| M2 review A F2, nothing under the terminal | LIKELY | "would fail" over `origin/main`'s pane, not run (log:101) |
| H5, H6 label and `ChangeSummary::of` (T043, T047) | PROVEN | 2 FAILED (log:107) |
| `HeaderCounts` transitions (T044, T048) | PROVEN | 4 FAILED; `the_counts_are_unknown_unless_a_read_finished` passed on arrival (log:108) |
| H7 through `State::update` (T045, T049) | TEST_AFTER | "first run against the implementation"; the only failure was a defect in the test itself (log:109) |
| Header watch (T050) | TEST_AFTER | "written with the code (no red taken)"; a stub red was taken later in M3 review B F4 (log:110, 117) |
| Changes press H8 (T046, T051) | TEST_AFTER | "written before T051 but first run after it"; late stub red (log:111, 117) |
| M3 review A F1, recount after the Overview | TEST_AFTER | "written after the fix (no red taken)", the red is hypothetical (log:114) |
| M3 review A F2, queued read behind a failing one | LIKELY | extended test; "against d2f82b17 its `Err` arm returned `Effect::None`" is described, not run (log:115) |
| M3 review A F3, `queue_changes_effect` on `ChangesRequested` | NO_TEST | "structural"; no test names it (log:116); finding 3 |
| Gate and snapshot, Changes button width (T052, T053) | PROVEN | gate against H9's first layout, quoted (log:112) |
| M3 review B F2, one-line label | PROVEN | wrapping `Text`, quoted (log:118) |

`NOT_APPLICABLE` (12 rows): T001/T002 scaffolding, T003 and T004 moves with their tests, T018/T029/T036
snapshot regeneration, T033/T034 view rewrite, T035/T016-showcase, and the docs and visual-pass rows.

### Existing tests changed by the feature

| File:line | Before | After | Reading |
|---|---|---|---|
| `tests/terminal_size_reporting.rs:133` | `cols > 100 && rows > 30` | `cols > 100 && rows > 1` | **Weakened** (finding 1). The rows bound fell from "larger than the daemon's 30-row seed" to "not degenerate". |
| `tests/gates/composer_fits.rs:~193` | transcript `>= WINDOW.height / 3.0` | `>= WINDOW.height / 3.0 - header` | Relaxed by the header's measured height; justified in a comment, bound still tied to a measurement (finding 13) |
| `tests/gates/composer_fits.rs:~356` | interrupted-turn text found in the shared window | found in a window twice as tall | Scope change: the test checks words, not geometry (finding 13) |
| `tests/gates/containment.rs` | `the_recorded_tab_overflow_is_the_instance_strip` asserted a single child, then equal tab widths within `TOLERANCE` | asserts each child's width equals its model member's `width()` | Equivalent or stronger; the "exactly one child" assertion has no counterpart because the padding container is gone |
| `containment.rs` path constants | positional paths | renumbered paths | Structural, forced by the new tree |
| Bar gates retired: `the_ai_tab_is_the_last_thing_in_the_bar`, `the_terminal_tabs_meet_the_trailing_controls`, `no_control_in_the_bar_is_squeezed_by_the_strip`, `the_add_instance_control_measures_the_same_at_every_instance_count`, `the_tabs_share_a_midline_with_the_controls_beside_them`, `the_bar_does_not_branch_on_*`, `the_bar_has_no_release_focus_control`, `the_pinned_ai_tab_names_the_sessions_cli`, `the_ai_tab_names_the_cli_from_the_provider_and_never_a_literal`, `the_ai_tab_is_unclosable_and_marked_from_one_source`, `the_strip_holds_every_instance_plus_the_ai_tab_last`, `the_bars_restart_control_asks_which_process_it_is_restarting` | | | All but the midline gate have a named successor in `strip_controls_*`, `session_area_stability`, `session_area_fits`; the midline gate does not (finding 12) |

No test was skipped, ignored, renamed out of a filter, or excluded through config; no coverage or
mutation threshold exists to lower.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | **Weakened existing test.** The terminal-size test no longer rules out "no larger than the seed" on rows; any grid of 2+ rows passes. It should assert the exact rows the geometry gives (header and status bar subtracted) or a bound derived from their measured heights, as `composer_fits` now does. | `crates/micold-client/tests/terminal_size_reporting.rs:133` (commits `e7ec795e`, `127a8178`; log:57-74) |
| 2 | HIGH | **Six `TEST_AFTER` behaviors**: the new source-level stability gates (T025), the focus guard (T039), H7 through `State::update` (T045/T049), header watch (T050), Changes press H8 (T046/T051), the recount after the Overview (M3 review A F1). Late stub reds exist for T046 and T050 only (log:117); T025, T039, T045 and the recount have none. Each needs a recorded red: run the test against a stub or against the pre-change source and record the failure. | log:91, 96, 109, 110, 111, 114 |
| 3 | HIGH | **`NO_TEST`: `Outcome::ChangesRequested` queues its read beside a waiting counts read.** `queue_changes_effect` replaced a plain `state.changes.pending = Some(effect)`. The failure it fixes (the header's counts read dropped, the button stuck on `Changes`) has no test; the cycle log marks it "structural". Not mutation-checked (6-mutant cap). | `crates/micold-client/src/app.rs:1336` and `:1553`; log:116 |
| 4 | MED | **Surviving mutant: `counts_of`'s guard.** Forcing the project check true (`(true \|\| active == Some(project))`) passes `--lib session_area changes::tests session_strip` and `--test features_changes --test session_area_stability --test layout_snapshot`. No test asks `counts_of` for a location the header does not follow, or for the right location under another active project; US3 sc. 4 ("never another location's figure") is held only by the follow step. Judged defensive but unpinned, not equivalent. | `crates/micold-client/src/ui/session_area.rs:104-113` |
| 5 | HIGH (rubric-fixed; mitigated) | **Re-implemented expectation, two places.** (a) `model.status == session_status(&state, id)` compares the model to the function that built it; the literal words are pinned independently at `:1291`, so S4 is covered. (b) `the_bar_and_the_strip_read_one_predicate` rebuilds `marked_tab` with its own `match` and compares `attached_process_restartable` to `process_stopped`, but the former is literally `process_stopped(.., marked_tab(..))` (`:178`); it was moved by T003, not written new, and its name still says "the bar". Assert literal restart outcomes per (mode, shell, lifecycle) row. | `crates/micold-client/src/features/session_strip.rs:841`; `:1193-1216` |
| 6 | MED | **Delegating state assertion.** `after.state == card.state` compares two products of one `location_card`, and `assert_ne!(after.state, before.state)` only says "not Idle"; a wrong non-Idle state passes. Assert `CardState::Working`. The test is also eager (state, pull request, sidebar parity, follow entry). | `crates/micold-client/src/ui/session_area.rs:589-590` (test at `:563-605`) |
| 7 | MED | **No test presses the rendered Changes button.** US3 sc. 2 / FR-007 / SC-003 are covered at message level by `changes_press` (`:609-629`) and the sidebar's own Review changes route, but nothing proves `changes_button` wires `.on_press(press)`, `.selected(open)` and the tooltip text. A button wired to the wrong message passes every test. Add a click through the laid-out header (the `clicks_at` helper). The same gap holds for US1 sc. 6's tooltips (FR-013): the chips' tooltip text is unit-tested in core, the `Tooltip` wrapping is not. | `crates/micold-client/src/ui/session_area.rs:155-180` |
| 8 | MED | **Negative-only source-text guards.** `the_pane_area_draws_no_bar_under_the_terminal` asserts `!contains("row![" ..)`; a bar built with `column!`, `container(`, or a renamed helper passes, and the file's own comment says the geometry gate cannot see it. `no_child_is_pushed_under_an_if_on_session_shape` catches six literal question strings only. `body_from` ends at the first `\n}` and no guard checks the extracted body is non-trivial, so a truncated body makes every `!contains` pass. `the_strips_restart_control_asks_which_process_it_is_restarting` re-greps what `session_strip.rs:765` already tests by value. | `crates/micold-client/tests/session_area_stability.rs:51, 119, 149, 223, 247` |
| 9 | MED | **`a_press_survives_a_header_chip_appearing` cannot fail.** It never checks a chip appeared, and the header is not a sibling of the strip's keyed row, so the scenario cannot shift the strip's children; the log concedes two of T040's three scenarios "hold positionally by design". FR-011's coverage is the shell-opens case only. Assert the header gained a child, or state in the test that it is a no-regression pin. | `crates/micold-client/tests/session_area_stability.rs:445-469`; log:97 |
| 10 | MED | **Conditional or vacuous measure in the width gate.** If `inside_button` is empty, `inner` is `0` and `inner <= one_line + TOLERANCE` passes; the baseline is whichever state comes first. `if window == WINDOW` skips `assert_button_unsqueezed` at 640×480 by design. `header_height` returns `0.0` when the header record is missing (`map_or(0.0, ..)`), which `composer_fits` then subtracts silently. Add `assert!(!inside_button.is_empty())`, take the baseline from `WINDOW` explicitly, and `expect` the header record. | `crates/micold-client/tests/gates/session_area_fits.rs:77, 407-448` |
| 11 | MED | **Redundant, eager and bypassed-utility tests.** Redundant pairs: `terminal_tabs.rs:224` and `session_area_stability.rs:247` (both `command_caption` present, `display_name()` absent); `terminal_tabs.rs:155` and `session_area_stability.rs:119`; `changes.rs:3890` `the_header_follows_the_shown_location` and `:3941` `following_a_new_entry_reads_its_counts`. Eager: `changes.rs:4003` (about six behaviors), `session_strip.rs:649` and `:765`, `features_changes.rs:3387`. Bypassed: `terminal_tabs.rs:44-100` hand-rolls `source_of` and `body_from` though `tests/support/source_scan.rs` has `strip_comments`; `body_from` is duplicated; `Project {..}` is hand-built at `session_strip.rs:511,539`, `tab_reveal.rs:36`, `terminal_tabs.rs:247`, `terminal_focus.rs:902` where `Project::new` exists. | as listed |
| 12 | LOW | **Retired gate with no successor**: the strip's vertical alignment against New session and the split buttons (`the_tabs_share_a_midline_with_the_controls_beside_them`). `tab_children_fit::a_tabs_content_sits_on_its_tabs_midline` is within a tab only. | removed in `2deba4ad` |
| 13 | LOW | **Relaxed composer gates**, recorded for the maintainers: the transcript floor is now less the header's height and the interrupted-turn test paints at twice the window. Both justified in comments; neither is a skip. | `crates/micold-client/tests/gates/composer_fits.rs` |
| 14 | LOW | Style: bare `assert_eq!` with no message (`keyed.rs:273-281`, `features_session_strip.rs:17-19`); `assert!(Known(..) == titles[..])` shows no values (`main_tests.rs:7618`); float `assert_eq!` depending on accumulation order (`session_strip.rs:1011-1017`); magic literals (`session_area_fits.rs:303-305, 399-401`, `session_strip.rs:1014`); duplicated overlap loops in `session_area_fits.rs:231-245, 353-366, 412-425`; `tab_reveal.rs:67-81` sends events for `ShellInstanceId(1)` that the fixture never opens; `keyed.rs:326` covers only the row axis; `session_strip.rs:598-602` repeats `:596`. | as listed |
| 15 | LOW | `core/src/context_header.rs:281` `assert_eq!(text.state.label(), state.label())` follows `:280` `assert_eq!(text.state, state)` and adds nothing; the test never pins a state word as a literal (it trusts `CardState::label`, owned and tested in `overview`). | `crates/micold-core/src/context_header.rs:280-286` |

Properties: the feature-scoped tests are deterministic and fast (client lib 1007 tests in 11 s; the
layout gates 6.6 s); no real clock, network or sleep was found by the smell pass. Failure messages
mostly state the rule. The suite is coupled to tree paths in `containment.rs` (renumbered by hand
in this feature), which the repo accepts by design.

## Mutation results

Tool: none (`cargo-mutants` absent). Six deliberate mutants on the highest-risk behaviors, one at a
time, each restored with `git checkout -- <file>` and `git status` confirmed clean before the next;
the final feature-scoped run on the restored tree was green (1109 passed).

| Mutant | Behavior | Caught | Judgment |
|---|---|---|---|
| `context_header.rs` `files == 1` → `files == 2` (plural) | H5 label | Yes, `the_changes_label_names_files_and_lines_or_says_there_are_none` | Boundary pinned |
| `changes.rs:1692` `if current != seq` → `if false && ..` | `HeaderCounts` drops an old answer | Yes, `the_current_counts_answer_is_kept_and_an_old_one_dropped` | Pinned |
| `session_area.rs:107` project check forced true | `counts_of` never shows another location's figure | **No** (lib filters and `features_changes`, `session_area_stability`, `layout_snapshot`) | Finding 4; not equivalent, defensive and unpinned |
| `session_area.rs:120` `&view.entry == location` → `true` | H8 press: open on another location opens, not closes | Yes, `the_changes_press_opens_the_shown_location_or_closes_its_view` | Pinned |
| `changes.rs:1669` same-entry short-circuit disabled | Header follow leaves the same entry alone | Yes, 2 tests | Pinned |
| `session_strip.rs:443` `trailing - viewport` → `trailing` | `scroll_into_view` brings a member just into view | Yes, `a_member_beyond_either_edge_scrolls_just_into_view` | Pinned |

Sampled: H5, `HeaderCounts` (two transitions), `counts_of`, H8, strip scrolling. Not sampled: the
strip's member order and `restart`, `pr_chip_input`'s staleness, the keyed row's key mapping
(FR-011), the Changes label's ellipsis, the `ChangesRequested` queueing (finding 3). Score: 5/6 =
83% of this sample; not comparable to a tool score.

## Traceability

Criteria are the spec's acceptance scenarios; requirement and success-criterion ids are traced through
them. "E2E" means exercised through the real entry point (`State::update`, the laid-out view, or the
shell's own `update`).

| Criterion | Tests | E2E |
|---|---|---|
| US1 sc. 1, 2, 4 title rule, Default entry (FR-002, SC-001) | core `the_title_is_the_cards…`; `a_locations_card_is_the_overviews_card_for_that_place`; `the_header_shows_a_change_on_the_refresh…` | Yes (via `header_text` on a `State`) |
| US1 sc. 3, 5 state, branch, PR chip, refresh (FR-003–005, FR-014) | core H2–H4; `pr_chip_input` tests; `session_area` test; `the_header_follows_the_shown_location` | Yes, with finding 6 |
| US1 sc. 6 cut text with full tooltip (FR-013) | `Tag::max_width` tests; `session_area_fits::a_long_title_and_a_pull_request_chip_still_fit` | Layout yes; tooltip wiring no (finding 7) |
| US2 sc. 1 order header, strip, panes; nothing below (FR-001, SC-005) | `session_area_fits`, `session_area_stability` | Yes (geometry); source guards weak (finding 8) |
| US2 sc. 2–4 press tab, New session, splits (FR-008, FR-009) | `terminal_tabs`, `session_strip` S5, `strip_controls_*`, `terminal_focus` | Yes |
| US2 sc. 5, 7 stopped mark, restart, AI tab as session tab (FR-009) | `session_strip` S1–S4 tests | Model level; no rendered-press test of restart |
| US2 sc. 6, 8 one-pane strip, scrolling (FR-010) | S6 tests, `tab_reveal`, `containment`, `strip_controls_hold_their_size` | Yes |
| US3 sc. 1, 3, 4 counts, no changes, unknown (FR-006) | core H5/H6; `HeaderCounts` tests; `the_header_reads_the_counts_of_the_location_it_follows`; width gate | Yes (state), `counts_of` guard unpinned (finding 4) |
| US3 sc. 2 press opens, again closes (FR-007, SC-003) | `changes_press` rows | **Message level only** (finding 7) |
| US3 sc. 5 view open shown on the button | `changes_press` returns `open`; `the_header_stands_over_the_open_changes_view` | Layout yes; `.selected` wiring no (finding 7) |
| US4 sc. 1 every chord unchanged (FR-012, SC-002) | `keyboard.rs`, `navigation.rs`, `app_chord_tests` untouched and green | Yes (regression suite, no new test) |
| US4 sc. 2 focus after a strip press (FR-011) | `terminal_focus::a_strip_press…` | Yes, but TEST_AFTER (finding 2) |
| US4 sc. 3 press in flight (FR-011, SC-006) | `session_area_stability::press_in_flight` (3 tests) | Yes for shells; 2 of 3 cannot fail (finding 9) |
| SC-004 no overlap at 640 px and 1280×800, light and dark | `session_area_fits` | Yes; per-platform run is CI's, not run here |

Untested criteria: none outright. Unit-level only: US3 sc. 2 and sc. 5 wiring, tooltips. Tests tracing
to nothing: none found; `session_area_stability` guards trace to FR-011/FR-012 by their comments.

## What was not audited

- The whole workspace suite, `scripts/gate.sh` and the CI-only legs (macOS, Windows, the sandbox jobs)
  were not run (instruction). SC-002 and SC-004 per platform are CI's claim, not verified here.
- Coverage: no tool in the profile. Mutation: no tool; six deliberate mutants only, so test strength
  is *sampled*, not measured. The strip's keyed-row key mapping, `pr_chip_input` staleness and the
  label ellipsis were not mutated.
- The smell pass over the client files was delegated to a fresh-context reviewer and vetted on the
  cited lines that drive findings 5, 6, 8 (partly), 9, 10; the LOW items in finding 14 and the
  redundancy pairs in finding 11 are the reviewer's, with the lines opened for 11's `changes.rs` pair
  only in outline.
- `main_tests.rs` was read for the two `the_header_reads_*` tests only; the other 15k lines are not
  this feature's. The visual pass (`visual-pass.md`) and the screenshots were not re-examined.
- Polish tasks T056 and T057 are unticked in `tasks.md` and were not audited as TDD evidence.
- Performance beyond the wall times above, and the Windows-only paths (`cfg(windows)`), were not
  assessed.
