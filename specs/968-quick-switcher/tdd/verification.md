---
feature: 968-quick-switcher
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset exists)
verified_at: 3d7d3d64 # short SHA audited
behaviors: 13 # derived from cycle-log rows: there is no tdd/test-list.md
proven: 1
likely: 5
test_after: 6
no_test: 1
high_smells: 2
criteria_total: 18 # US1-1..8, US2-1..4, US3-1..5, US4-1 acceptance scenarios
criteria_covered: 14 # 4 more are covered below the real entry point only (see Traceability)
mutation_score: 57 # 12 of 21 deliberate mutants caught; no mutation tool, sampled, changed files only
mutants_survived: 9 # 1 logic survivor inside a DONE behavior, 8 in dialog and shell glue
suite: 8024 passed, 7 failed, 13 ignored, 12m54s (cold build included); the 7 failures are outside this feature
---

# TDD Verification: Quick-switcher (968)

**Verdict: FAIL.** Six behaviors were written test-after, and a deliberate mutant that stops
Query changes from re-seating the highlight (FR-005) survives the whole suite.

The audit is independent: this session did not write the tests. Every file cited was read cold,
and every mutant was run and restored (see Mutation results).
`FEATURE_DIR/tdd/test-list.md` does not exist (the feature was not planned through
`speckit-tdd-plan`). The rubric calls that `BLOCKED`; the audit ran anyway against `spec.md` and the
cycle log, treating each cycle-log row as one behavior, so the per-behavior table is weaker than a
test-list audit would give.

Stage reached: question 1 (test-first) already fails; questions 2, 3 and 4 also have findings.

## Test-first evidence

Git history for the feature is per-milestone: tests and source usually land in one commit, so the
order inside a commit cannot be seen. The cycle log is self-reported. `PROVEN` therefore needs a
test-only commit before the source.

| Behavior (cycle-log row) | Class | Evidence |
| --- | --- | --- |
| T003/T004 core `filter`, `reseat`, `step` | LIKELY | log records red against stubs (`4 passed; 8 failed`); tests and source both in `2d1dfa9f` |
| T005/T006 switcher chord decode, labels, routing | LIKELY | log records compile-fail red; tests and source both in `5e462d79` |
| T009/T013 `candidates` (locations) | LIKELY | log records compile-fail red; one commit `d30849de` |
| T010/T014 reducer | LIKELY | log records compile-fail red; one commit `d30849de` |
| T011 apply a chosen location | TEST_AFTER | log: "no red: the apply path already satisfied every assertion"; `d30849de` |
| T012 registry, showcase pose, chord routing from terminal and `app_chord` | PROVEN | test-only commit `95a342e5` ("failing registry, showcase and chord routing tests"), source in `2a954d94`; red output recorded. The dialog tests `tab_in_the_dialog_dismisses_it` and `the_chord_pressed_in_the_field_still_reaches_the_toggle` arrived with the source in `2a954d94` |
| T020 docs list the chord | LIKELY | `#[ignore = "red until T020"]` committed in `5e462d79`, un-ignored in `2a954d94` with the docs; the ignored test never ran red in a gate |
| visual-pass D2 single-line row with details | TEST_AFTER | log: "written after the fix"; test and fix both in `317b271b` |
| T021/T024 matched location pulls in its sessions | TEST_AFTER | log: "no red observed ... honest gap"; `ed7a17c7` |
| T022/T023/T025 session entries and choosing a session | TEST_AFTER | log: "no red observed"; `ed7a17c7` |
| T027/T028/T029 project entries and choosing a project | TEST_AFTER | red was produced after the fact by deleting the code (M3 review B F1, `52e222f4`); that proves the tests are strong, not that they came first. Tests and source in `d712f5a1`; the restore-session test followed in `f6e85f32` |
| T030 showcase pose has an unavailable project row | NO_TEST | the log cites `showcase_completeness`, which only checks the pose names `Quick-switcher dialog` and `... nothing matches` (`tests/showcase_completeness.rs`), never an unavailable row |
| Highlight follows Up/Down and typing (`838eac54`) | TEST_AFTER | the 4-line fix in `shell/quick_switch.rs` landed first; the only test, `3d7d3d64`, is a text scan of call sites (Finding 3) |

T007, T008 (`location_node`, `apply_step` refactors) and T026, T031 (docs) have no behavior of their
own: refactors covered by the existing #803 tests, and docs checked by the scripts gate.

### Existing tests changed by the feature

No test was weakened. Changes, all legitimate:

- `overlay_registry.rs`: dialog count 19 to 20 and states 40 to 42, in the test-only commit `95a342e5`.
- `popover_displacement.rs`: `DIALOGS` 21 to 22. `root_vocabulary_is_cross_cutting.rs`: variants 24 to 25.
- `picker_highlight_into_view.rs:572-585`: one caller to two, still an exact list.
- `navigation_apply.rs`: helpers `arranged`, `press`, `session_count` moved to `tests/support/sidebar_fixture.rs`, assertions untouched.
- `tests/keymap.rs`: `#[ignore]` added, then removed at HEAD (no ignored test remains).
- The placeholder tests in `tests/quick_switch.rs` (`the_switcher_starts_closed`) were replaced by the real ones.

### tasks.md against the cycle log

T001 to T031 are ticked and each maps to a cycle-log row; no tick without evidence, no `DONE` behavior left unticked. T032 to T034 (visual pass, platform pass, converge) are unticked although `visual-pass/` holds screenshots and `visual-pass.md`: the close unit has not recorded them. Not a TDD finding.

## Findings

Ordered by severity.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Vacuous assertion for FR-005 "the first remaining selectable entry is highlighted after each change to the query". Mutant: drop `open.highlighted = None` in `Msg::QueryChanged`. Every test passes, because the one typing test moves to an entry that is also the first match. Assert instead: highlight a later entry, type a query that still lists it but not first, expect the first match. | `crates/micold-client/src/features/quick_switch.rs:190-196`; `crates/micold-client/tests/quick_switch.rs:272-295` (`Moved(Next)` lands on `feat-login`, which is also the first match for `log`) |
| 2 | HIGH | The dialog glue has no behavior test. Five mutants of `ui/quick_switcher.rs` all survive the quick_switch, layout_snapshot, showcase and overlay suites: Up and Down swapped, click picks the neighbouring entry, unavailable row not disabled, current row unmarked, "Nothing to switch to" shown as "No matches". Acceptance scenarios US1-5, US1-8, US3-3 (click does nothing, row looks unavailable) and FR-014's two messages reach no test through the real entry point. Only Tab and the chord are driven through `press_in_dialog`; Enter, Up, Down and a click are not. | `crates/micold-client/src/ui/quick_switcher.rs:73-98` (`view_of`), `:109-128` (`on_move`, `on_pick`); the only dialog-level tests are `tests/quick_switch.rs:579` and `:591` |
| 3 | MED | Shell behaviors untested and one test coupled to source text. Mutants that remove the focus-on-open task (US1-1 "keyboard focus in its filter field") or the scroll-follow (`follows`, FR-007 "keep it in view") survive. The nearest test, `only_the_issue_highlight_move_chains_the_operation`, scans source files for the call `picker_highlight_into_view()`, so it passes with the call present and `follows` forced to `false`. It asserts where a call is written, not that the list scrolls. | `crates/micold-client/src/shell/quick_switch.rs:20-27`; `crates/micold-client/tests/picker_highlight_into_view.rs:572-585` |
| 4 | MED | No test that an unfocused terminal ignores the chord; the only terminal test is the focused case. Mutant: `if self.focused` made `if true` survives, which would toggle twice (pane and `app_chord`). | `crates/micold-client/src/ui/material/terminal_pane.rs:1743`; test at `:2340` |
| 5 | MED | Framework under test: `two_states_are_independent` asserts two separate `State` values do not share state, which can only fail if `State` used a global. FR-015 (own switcher per window) is not exercised through a window. `opening_filtering_and_closing_start_and_stop_no_session` (LOW as well) only counts sessions across a reducer that cannot touch them; FR-011 is meaningful only on the choose path, which `choosing_a_location_without_a_session_starts_none_and_stops_there` already covers. | `tests/quick_switch.rs:367-372`, `:502-509` |
| 6 | MED | Weak disjunction: `mapped \|\| status == Ignored` passes if the field swallows the chord and a different component happens to map it, and does not check that `app_chord_listener` then toggles. | `tests/quick_switch.rs:605-611` |
| 7 | MED | Foreign style: the profile's convention is a message on every assertion stating the rule. About 60 assertions in `tests/quick_switch.rs` carry none, e.g. lines 44-57, 60-78, 143-168, 199-213, 615-720, 850-1030, so a failure prints values only. The core unit tests (`micold-core/src/quick_switch.rs`) follow the convention. | `crates/micold-client/tests/quick_switch.rs` |
| 8 | MED | Non-deterministic: SC-003 is asserted with `Instant::now()` against a 100 ms wall clock in a debug build. It passed here, but is a flake risk on a loaded runner. SC-003 says "in the app's test harness", yet only the core `filter` is timed; `candidates` and `list` for 500 entries are not. | `crates/micold-core/src/quick_switch.rs:441-459` |
| 9 | LOW | Duplicated setup and bypassed utility: `Project { path, display_name, is_git_repo, availability }` is hand-built three times, and `Session::start_new(...)` twice, instead of one factory in `tests/support/sidebar_fixture.rs`. `with_sessions` is defined in both crates (separate crates; fine). | `tests/quick_switch.rs:832`, `:980`, `:886`, `:998` |
| 10 | LOW | `fifty_locations_are_each_reachable_in_a_short_query` uses the last three characters and only locations; SC-001 allows 4 characters and counts Up/Down presses with sessions in the list. `p < 3` is a magic literal for "at most 2 Up/Down". | `tests/quick_switch.rs:398-427` |
| 11 | LOW | `features_quick_switch.rs` repeats the open/query/dismiss path that `tests/quick_switch.rs` already pins at the same level (the "feature in isolation" test is by design, but its assertions are a subset). | `crates/micold-client/tests/features_quick_switch.rs:8-45` |

Properties: the suite is deterministic apart from Finding 8, fast (quick_switch tests 0.04 s,
core 0.01 s), isolated, and mostly specific about what broke. The client reducer tests drive the
real `State` and the real apply path (`apply_navigation`, `finish_navigation`), not doubles, and use
the repository's fixtures (`sidebar_fixture`), so no over-mocking or tautological assertion was
found in the logic layer. Insensitivity to refactoring is good except Finding 3.

## Mutation results

No mutation tool in the profile (no `cargo-mutants`); deliberate mutants were used, one at a time,
each restored with `git checkout -- <file>` and the suites re-run green afterwards (core lib
`quick_switch` 16 passed; client `quick_switch` 45, `features_quick_switch` 2, `keymap` 45,
`picker_highlight_into_view` 5, `overlay_registry` 19, `showcase_completeness` 41). Changed files
only: `micold-core/src/quick_switch.rs`, `micold-client/src/features/quick_switch.rs`,
`src/ui/quick_switcher.rs`, `src/shell/quick_switch.rs`, `src/keymap.rs`, `terminal_pane.rs`.
21 mutants sampled over about 12 behaviors; not capped (about 25 minutes used of the 30).
Score: 12 caught of 21 = 57%, which is a sample, not a mutation score over every mutant.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| core `found.at == 0` to `== 1` (prefix rank) | T003 | No | 4 tests fail |
| core drop `sort_by_key` | T003 | No | 2 tests fail |
| core `step` Next ignores `selectable` | T003 | No | caught |
| core `reseat` ignores `selectable` | T003 | No | caught |
| core drop the already-listed filter | T021 | No | caught (no duplicates) |
| core pulled-in session takes `Scattered`, not the location's rank | T021 | No | caught |
| client `Chosen` ignores `selectable` | T028 | No | caught |
| client open over any surface (drop `topmost` guard) | T010 | No | caught |
| client dismiss does not restore keyboard row | T010 | No | caught |
| client Overview closes for every step | T011 | No | caught |
| client active project listed as an entry | T027 | No | caught |
| keymap drop the logical fallback | T005 | No | caught (Dvorak and `Other` cases) |
| client `QueryChanged` keeps the old highlight | T010, FR-005 | **Yes** | Real bug-class survivor inside a DONE behavior: Finding 1 |
| ui "Nothing to switch to" shown as "No matches" | FR-014 | **Yes** | not equivalent; Finding 2 |
| ui unavailable row not `disabled()` | US3-3 | **Yes** | not equivalent; Finding 2 |
| ui Up and Down swapped | FR-007 | **Yes** | not equivalent; Finding 2 |
| ui click picks `keys[at + 1]` | US1-8 | **Yes** | not equivalent; Finding 2 |
| ui current row not marked | FR-006 | **Yes** | not equivalent; Finding 2 |
| shell no scroll-follow (`follows = false`) | FR-007 | **Yes** | not equivalent; Finding 3 |
| shell no focus-on-open | US1-1 | **Yes** | not equivalent; Finding 3 |
| terminal pane toggles when unfocused too | FR-001 | **Yes** | double toggle; Finding 4 |

## Traceability

Real entry point here means the reducer plus the state-level apply path for the logic, the
`TerminalPane` widget for the chord, and the built dialog for the glue. Entries marked partial are
covered only below the dialog.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1-1 open from a focused terminal, 0 bytes, field focused | `terminal_pane::the_switcher_chord_is_published_as_a_toggle_and_captured`, `opening_records_the_start...` | Partial: focus into the field untested (Finding 3) |
| US1-2 empty query lists locations, current marked, first highlighted | `locations_come_in_location_order...`, `a_navigation_stop_is_the_current_entry`, `opening_records...` | Partial: the current mark in the view untested (Finding 2) |
| US1-3 `logpg` keeps `Login page`, best match first | core `letters_in_order...`, `prefix_ranks_before_run...`; `typing_reseats_the_highlight...` | Yes, but see Finding 1 |
| US1-4 Enter chooses like a click | `choosing_a_location_with_a_session_shows_it...`, `choose` | Yes at state level (shell `apply_step` is the binary half, untested) |
| US1-5 Up and Down move and scroll | `typing_reseats...`, core `step_...` | Partial: wiring and scroll untested (Findings 2, 3) |
| US1-6 Escape, chord, outside click close and restore | `toggling_or_dismissing...`, `overlay_registry` (cancel by `Dismissed`), `tab_in_the_dialog_dismisses_it` | Yes |
| US1-7 location without a session | `choosing_a_location_without_a_session_starts_none...` | Yes |
| US1-8 click chooses | none beyond the reducer's `Chosen` | No, the click wiring is untested (mutant survived) |
| US2-1..4 sessions listed, chosen, kept with their location, closed absent | `sessions_follow_their_location...`, `choosing_a_session_selects_it...`, `a_locations_name_then_down...`, `a_closed_session_is_absent`, core session tests | Yes |
| US3-1 other projects after the active one, with counts | `other_projects_follow...`, `a_project_shows_its_counts...` | Yes (counts as `context` text) |
| US3-2 Enter switches project | `choosing_an_available_project...`, `applying_a_project_choice...` | Yes |
| US3-3 unavailable: badge, skipped, click does nothing | `a_project_shows_its_counts...`, `the_highlight_skips...`, `an_unavailable_project_cannot_be_chosen...` | Partial: the disabled row and the badge in the view untested (Finding 2) |
| US3-4, US3-5 active project and other projects' contents absent | `the_active_project_and_other_projects_contents_are_not_entries` | Yes |
| US4-1 docs list the chord in both forms | `keymap::the_user_guide_lists_the_switcher_chord_in_both_forms` | Yes for the chord and both pages; the description of the switcher is not checked |

FR coverage not backed by a test through the entry point: FR-010 on dismiss with a terminal that had
focus (only the sidebar row restore is asserted), FR-012 for a context menu and the filter panel
(the registry probes are the only evidence), FR-015 (Finding 5), FR-014 messages (Finding 2).

Untested criteria: none are wholly untested; four scenarios (US1-1, US1-5, US1-8, US3-3) lack a
real-entry test for their glue half. Tests tracing to nothing: none (`features_quick_switch.rs` traces to
SC-004 of feature 021). No `traces` column exists to check, since there is no test list.

## What was not audited

- Mutation was a 21-mutant deliberate sample, not an exhaustive tool run. `features/navigation.rs`
  (refactor for `location_node`), `showcase/sections/surfaces.rs`, `ui/cdk/picker.rs`,
  `ui/material/picker.rs`, `typeahead.rs` and `text_field.rs` were not mutated.
- Coverage: no coverage tool in the profile.
- The `sandbox_real_*` acceptance suite (needs the container image) and the release daemon half
  were not run; the feature does not touch the daemon.
- Seven suite failures were not baselined (the cycle log has no baseline entry). Six are
  permission-denial tests that cannot fail to write as uid 0 (`a_save_over_an_unreadable_file...`,
  `a_refused_write_is_logged...`, `a_directory_that_cannot_be_emptied...`, `the_leftover_report_is_capped`,
  `worktree_delete_blocked_by_an_unremovable_path...`, `a_service_write_over_an_unreadable_settings_file...`);
  the seventh is a feature 938 daemon test `messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal`.
  None touches feature 968 files; the cause was not investigated.
- The visual pass evidence (`visual-pass/`) and quickstart §C (macOS, Windows, non-US layout) were not
  assessed; T032 to T034 are unticked.
- Performance beyond SC-003's core timing; the iced runtime paths in `shell/quick_switch.rs` (the
  binary crate) cannot be unit tested by the current harness and were judged by mutation only.
- The audit did not delegate the smell pass to a subagent; it was done directly from cold reads of
  `tests/quick_switch.rs`, `tests/features_quick_switch.rs`, the keymap and terminal-pane tests and
  the core module tests. The inline test for the D2 row in `ui/material/picker.rs:1032` was not read.

## Remediation (close unit, 2026-10-18)

Findings 1, 2 and 4 to 9 were fixed by T035 to T037 and T039 to T042 (T042 partial), each new test proven red by a mutant (`cycle-log.md`). Finding 3 (T038) is declined: it needs production code moved; the verdict above is as audited at `3d7d3d64`.
