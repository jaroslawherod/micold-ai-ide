---
feature: 1793-structured-view-header-one-row
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: fa0e41a73 # short SHA audited (all four milestones merged)
behaviors: 13 # no test list exists: the 13 test-first tasks in tasks.md stand in (T001-T003, T007, T009, T013, T017, T021, T022, T028, T029, T034; T035 retires gates)
proven: 0
likely: 0
test_after: 12 # 2 contradicted by history (T021, T029), 10 with no red recorded and history that cannot show order
no_test: 0
not_applicable: 1 # T035 (retires or re-points existing gates)
high_smells: 0
high_findings: 3 # existing coverage removed (2), tasks ticked without evidence (1)
criteria_total: 14 # FR-001..FR-014 (the 18 acceptance scenarios were traced through them)
criteria_covered: 12 # FR-014 has no test; FR-011's tabs-shrink clause has none
mutation_score: unmeasured # no cargo-mutants (profile: mutation null); 4 deliberate mutants only, 3 of 4 survived
mutants_survived: 3
suite: 25 passed, 0 failed (main, filtered to features_header_bar and features_session_strip), 176 passed, 0 failed (layout_snapshot); targeted run only, 4m22s cold build + 0.2s / 28s
---

# TDD Verification: Structured view header in one row (#1793)

**Verdict: FAIL.** The feature has no `tdd/test-list.md` and no `tdd/cycle-log.md`, so no behavior has a recorded red; history shows the M4 gate (`header_row_fits`) landing after the code it covers; and the M4 diff removed existing coverage of the Changes button without replacing it.

The targeted tests pass, and the new tests are mostly sound. The failure is about evidence and about coverage the change dropped, not about broken tests.

Independence: this audit read every cited file cold in a session that did not write the tests. The rubric lists "no test list" under `BLOCKED`; the invocation asked for an audit against `spec.md` anyway, so the audit ran and a verdict is reported (`FAIL` conditions were found, so `BLOCKED` would understate it).

## Test-first evidence

History is shallow: `git rev-parse --is-shallow-repository` is `true`, and the oldest reachable commit is `08028266f` (M3 head), which contains the whole repository. M1, M2 and M3 therefore cannot be ordered from history. The ledger (`autopilot.md`) records no red either. Per Hard Rule 3 these are not assumed compliant.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T001 ContextRing tests | TEST_AFTER | no red recorded; M1 squashed into graft root `08028266f`, order unverifiable |
| T002 SegmentedControl tests | TEST_AFTER | same |
| T003 HeaderBarModel tests | TEST_AFTER | same |
| T007 HeaderMenu reducer tests | TEST_AFTER | same |
| T009 top_bar_items gate | TEST_AFTER | same |
| T028 top_bar_collapse gate | TEST_AFTER | same |
| T013 worktree menu tests | TEST_AFTER | same (M2) |
| T017 PlusMenuModel tests | TEST_AFTER | same (M3 up to `08028266f`) |
| T022 activity dot and chip tests | TEST_AFTER | tests and code both first appear in `34e0d624b`; no red recorded, order inside the commit unknowable |
| T034 ring tooltip, running count | TEST_AFTER | test added to `features_header_bar.rs` in `34e0d624b` with the code; no red recorded |
| T021 `header_row_fits` gate | TEST_AFTER (history contradicts test-first) | code in `34e0d624b` ("code, gates and docs still to do"); the gate file first appears in `954b83e5d`, a later commit. Ledger row "M4 Review B" also says "gate written after code" |
| T029 tab scroll gate | TEST_AFTER (same) | same file and commit as T021; shrink-before-scroll part was never written (see finding 5) |
| T035 retire or re-point `session_bar_fits`, `session_area_fits` | NOT_APPLICABLE | edits to existing gates; see findings 1 and 2 for what it dropped |

`tasks.md` header says "every test task precedes the implementation it covers and is written failing first". Nothing in the repository evidences that, and for T021 and T029 history contradicts it.

### Existing tests changed by the feature (range `747d325b5..36b210e7d` for M4; M1 to M3 not diffable)

| Change | Where | Judgment |
| --- | --- | --- |
| Removed `the_widest_counts_fit_at_the_layout_window` (Changes label with the widest counts laid out whole beside the title; `assert_button_unsqueezed`) | pre-image `747d325b5:crates/micold-client/tests/gates/session_area_fits.rs:843`; `THE_WIDEST_COUNTS` is now referenced by no test | Weakened: finding 1 |
| Removed `pressing_changes_with_the_view_closed_opens_the_shown_location` and `pressing_changes_with_the_review_shown_changes_nothing` (feature 1269 FR-012) | `crates/micold-client/tests/session_area_stability.rs` (pre-image) | Weakened: finding 2 |
| Removed `pressing_the_header_toggle_asks_for_the_opposite`, `pressing_the_hidden_notice_show_button_turns_raw_events_on`, `a_transcript_of_only_hidden_entries_shows_the_notice_not_empty`; `Row::Hidden` assertions turned into absence checks | `crates/micold-client/tests/transcript_entry_view.rs` | Justified by FR-010 (the notice moved to the chip); the chip and menu are covered at model level only (finding 2) |
| `the_view_draws_the_session_bar_above_the_entries` inverted to "no longer draws"; the `No data yet` assertion dropped | `crates/micold-client/tests/transcript_insights_view.rs:~485` | Justified (FR-012 and R6); empty state is now `features_header_bar.rs:73` |
| Removed `a_long_title_and_a_pull_request_chip_still_fit`, `the_title_shortens_before_the_chips_and_the_button`, `a_press_survives_a_header_chip_appearing` | `session_area_fits.rs`, `session_area_stability.rs` | Justified: the title row and header chips no longer exist |
| `SMALLEST_WINDOW` allow-list: states and one `#1466` sideways-scroll entry removed; `KNOWN_OVERFLOWS`, `containment.rs` paths re-pointed | `crates/micold-client/tests/gates/review_tab_fits.rs:~128`, `layout_text_overflow.rs:58`, `containment.rs` | Checked: the gate's own rule says entries must be dropped once their node fits, so this tightens the allow-list. Not a weakening, and no thresholds or skips were changed |
| `terminal_tabs.rs` counts `ActivityBadge::for_emphasis(` beside `mark(` | `crates/micold-client/tests/terminal_tabs.rs:~203` | Source-text count widened to a second leading form; value check kept |
| Fixture `layout_snapshot.txt` regenerated (about 18k changed lines) | `9ef03ad45` | Regeneration is expected for a layout change; no review evidence beyond the ledger's "CLEAN" rows |

No test was skipped or marked ignored, no exclusion was added, and no tolerance or threshold was loosened.

### tasks.md against the evidence

- T029 and T031 are `[x]` but their own text says tabs shrinking before they scroll (FR-011) is "a recorded follow-up": the box overstates completion.
- T030 is unticked and "Removed"; harmless.
- T032 (quickstart Part B visual pass, FR-014) and T033 are unticked: no visual-pass evidence exists in the feature directory. The user-guide tasks T012, T016, T020, T027, T031 are ticked; docs were not audited.
- All other test tasks are `[x]` with no per-behavior evidence available: HIGH (finding 3).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Existing coverage dropped: the widest-counts gate for the Changes button was deleted with the header and not re-created for the top bar's button. A long `Changes` label with three- or four-digit counts is no longer checked against overlap or squeeze anywhere (`top_bar_collapse` and `top_bar_items` use a fixture with small counts) | `session_area_fits.rs` at `747d325b5:...:843` (removed); `grep THE_WIDEST_COUNTS crates/micold-client/tests` finds nothing |
| 2 | HIGH | No test drives the real controls. The Changes press (US2 scenario 4) is only checked through the pure `changes_press` function (`crates/micold-client/src/ui/session_area.rs:572`); the removed click tests were the only ones that pressed the button through the view. The new top-bar `changes_button` (`crates/micold-client/src/ui/toolbar.rs:352`, `.on_press(press)`), the `+` and worktree `⋮` triggers, the `N hidden` chip and the menu rows are asserted at model level only, so a miswired `on_press` or a dropped trigger passes | removed tests above; `rg "clicks_at|click\(" tests` finds no press of any of them |
| 3 | HIGH | Completion claims with no evidence: 12 of 13 test tasks are `[x]` with no cycle log, test list or red; T021 and T029 are contradicted by history | `tdd/` did not exist; `34e0d624b` vs `954b83e5d` |
| 4 | MED | Surviving deliberate mutants in `DONE` behaviors (see Mutation): the shell-tab Working status, the meter ratio clamp and the chip's zero boundary are not pinned | `session_strip.rs:75`, `header_bar.rs:94`, `session_strip.rs:437` |
| 5 | MED | US5 scenario 3 and FR-011 "tabs shrink then scroll" have no test; only scroll and reachability are gated. Deferred in the ledger but left `[x]` | `header_row_fits.rs:166-198`; `tasks.md` T029, T031 |
| 6 | MED | The activity dot is tested with one session. The spec's edge case (every working session's tab shows its own dot; one session's state never leaks) is not exercised with two sessions where one is working | `features_session_strip.rs:166-187` |
| 7 | MED | Meter assertions are `contains` checks (`%`, `$`, ` · `), so wrong percent or cost digits pass. The only exact `8% · $1.28` check is on a hand-built ring in `crates/micold-material/src/context_ring.rs` (unit), not on `HeaderBarModel` output. The `TokensOnly`, `(percent, no cost)` and `Near compaction` branches are unasserted | `features_header_bar.rs:36-58`; `header_bar.rs:79-120` |
| 8 | MED | Tautological constants: `the_ring_text_hides_before_the_changes_label` asserts `RING_TEXT_RANK == 0` and `CHANGES_LABEL_RANK == 1`, which restates the constants. The behavior (order of collapse) is covered by the gate, so this is redundant, and it fails on a harmless renumbering | `features_header_bar.rs:135-139` |
| 9 | MED | Disjunctive assertion `text.starts_with("12") \|\| text.len() >= 10` accepts nearly any chip text; the `if extra > 1_000` guards it. The exact `N hidden` text is pinned at `features_session_strip.rs:206`, so this only weakens the fit premise | `header_row_fits.rs:144-150` |
| 10 | LOW | `top_bar_collapse` decides asserts with `if !d.ring_text` inside a width loop; the trailing `seen_*` asserts keep it from being vacuous, but a failure says only the width | `top_bar_collapse.rs:84-106` |

Foreign style and Bypassed utility were checked: the new tests use `support::covered_states` and stock assertions with messages, as the exemplar does. Smell catalogue: no assertion-free, doubled-subject, over-mocked, self-approving-snapshot, skipped or non-deterministic tests were found in the new files. Isolation, determinism and speed are good: the 25 feature tests run in 0.07 to 0.3 s and the 176 layout tests in 19 s, with no clock, network or order dependence seen. The fixture `layout_snapshot.txt` was regenerated twice (`bbb2404b1`, `9ef03ad45`) with the ledger's CLEAN review rows as the only review record; I did not read the 18k-line diff.

## Mutation results

No mutation tool (`mutation: null` in the profile; `cargo-mutants` not installed). Score unmeasured. Four hand-picked deliberate mutants in the changed source files, run one at a time with `cargo test -p micold-client --test main -- features_header_bar:: features_session_strip::` (25 tests), each restored with `git checkout -- <file>`; the working tree was confirmed clean afterwards. This is a sample, not an exhaustive set.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `session_strip.rs:75` shell-tab Working: `== Some(ShellLifecycle::Running)` to `.is_none()` | T022 / FR-006 (dot per tab, terminal tabs) | Yes | Real gap: no test builds a Regular-mode session. Finding 4 |
| `header_bar.rs:94` drop `.min(1.0)` on the ring ratio | T003 / FR-005 | Yes | Real gap, though only used above 100% of the window; the test never exceeds 1.0 and only range-checks. Finding 4 and 7 |
| `session_strip.rs:437` `(n > 0)` to `(n > 1)` (chip appears from 2 hidden) | T022 / FR-010 | Yes | Real gap: the zero boundary is tested only at the raw-events-on side, not with exactly one hidden event. Finding 4 |
| `session_strip.rs:440` chip press `ShowRawEvents(true)` to `false` | T022 / FR-010 | No | Caught by `the_hidden_chip_shows_the_count_only_while_events_are_hidden` |

Mutants A to C each pass all 25 tests; with no equivalence argument available, they count as survivors inside `DONE` behaviors, which is a `FAIL` condition under the rubric. After the last restore the same 25 tests were rerun: 25 passed, 0 failed.

## Traceability

Requirements are traced from `spec.md` (FR-001 to FR-014). There is no test list, so the mapping is by reading the tests.

| Criterion | Tests | End to end (real entry point) |
| --- | --- | --- |
| FR-001, SC-001 one 40 px row | `gates/header_row_fits.rs::one_forty_px_row_precedes_the_first_event` | Yes (laid-out view) |
| FR-002 no title row | `header_row_fits.rs` (exactly two children under the area); `transcript_entry_view.rs` | Yes |
| FR-003 shown only with a selection | `features_header_bar::nothing_shows_on_the_overview...`, `gates/top_bar_items::the_overview_shows_none_of_them` | Yes |
| FR-004 top-bar order | `gates/top_bar_items::the_meter_status_and_changes_come_before_settings_and_the_menu` | Yes |
| FR-005 ring text and tooltip | `features_header_bar` meter tests, `context_ring.rs` units | Partly: contains-only (finding 7) |
| FR-006 activity dot, no `running` | `features_session_strip::only_a_working_session_tab_shows_the_activity_dot` | Model only; one session; no Regular mode |
| FR-007, FR-008 single `+` and menu | `features_session_strip` plus-menu tests, `header_bar` Escape tests | Model and message level; no press through the view (finding 2) |
| FR-009 worktree menu | `features_header_bar` worktree tests, `the_worktree_trigger_is_named_and_the_app_menu_gains_no_worktree_entries` | Model level |
| FR-010 chip and Show action | `features_session_strip::the_hidden_chip_...`, `header_row_fits::the_hidden_chip_fits_...` | Fit: yes. Activation: model only |
| FR-011 collapse order | `gates/top_bar_collapse.rs` | Yes for ring text and label; tabs shrink: untested (finding 5) |
| FR-012 nothing lost | covered by the rows above | Partly (finding 2) |
| FR-013 accessible names, Escape, outside click | `features_header_bar` and `features_session_strip` name and Escape tests | Model level; the registry's `escape` is the real hook and is called |
| FR-014 visual pass | none | No: T032 unticked, no evidence file |
| US4 scenario 2 (menu reflects new state) | raw-events and view segments reflect state at model level | Partly |
| US5 scenario 3 | scroll and reachability only | Partly |

Untested criteria: FR-014 (no visual pass recorded); FR-011 tabs-shrink clause. Tests tracing to nothing: none found; `the_ring_text_hides_before_the_changes_label` traces to FR-011 but proves nothing the gate does not.

## What was not audited

- Mutation testing across the changed files: no `cargo-mutants`, and a full-workspace build was avoided as instructed. Only 4 hand-picked mutants were run, all in `session_strip.rs` and `header_bar.rs`. `ui/toolbar.rs`, `ui/session_area.rs` and the transcript view files were not mutated.
- The full suite was not run (`mise run gate`, `cargo test --workspace`): only `--test main` filtered to the two feature modules (25 tests) and `--test layout_snapshot` (176 tests). No baseline-vs-feature failure separation was possible, and coverage is unavailable (`coverage: null`).
- `crates/micold-material` unit tests (`context_ring.rs`, `segmented.rs`) were read, not run.
- M1 to M3 history and any PR-level evidence: the clone is shallow, with one graft commit holding all earlier work. Review comments on PRs #1798, #1827, #1847, #1869 and CI results were not consulted.
- Source-text tests (`terminal_tabs.rs`, `settings_sections.rs`, `session_area_stability.rs` body searches) were read only at the changed lines.
- The regenerated `layout_snapshot.txt` fixture content, user-guide docs (T012, T016, T020, T027, T031) and the quickstart Part B visual pass.
- Performance and load behavior: no criterion, no test.
