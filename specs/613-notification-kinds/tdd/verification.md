---
feature: 613-notification-kinds
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no preset or override present)
verified_at: 0c2b4a1d # short SHA audited (plus uncommitted tasks.md ticks and a cycle-log entry), branch claude/project-thread-8kdqkn; round 2 re-audit after Phase 9 T077-T079 (working tree: the gate test, settings_sections.rs, tasks.md and tdd/ files uncommitted); round 1 report was FAIL on F14
behaviors: 48 # test-list.md: A1-A29 outer, U1-U19 inner
proven: 14 # weakest-link: a behavior takes the class of its weakest test task
likely: 34 # includes the behaviors of the seven formerly TEST_AFTER tasks, now carrying a retroactive red
test_after: 0
no_test: 0
high_smells: 0 # F14 cleared in round 2; the seven earlier TEST_AFTER and both earlier survivors are cleared
criteria_total: 29 # acceptance scenarios US1.1-11, US2.1-13, US3.1-5
criteria_covered: 29 # US2.5 now has a behavioral test (F14 cleared); 2 partial (US3.4 Windows/macOS arms compile-only, US3.5)
mutation_score: unmeasured # cargo-mutants not installed; scoped deliberate mutants on the Phase 8 files: 6 valid, 5 caught, 1 survived (equivalent)
mutants_survived: 1 # D7, judged equivalent; the cycle-12 `if master_on` survivor is now killed (round 2: 2 of 2 caught)
suite: 5342 passed, 6 failed (root-only permission tests, ignored per instruction), 9 ignored, 441 result lines, 254 s
---

# TDD Verification: Notification kinds, per-kind settings and icons (round 2, after Phase 9)

**Verdict: PASS_WITH_GAPS.** F14 (the master-switch-off state of the Settings kind rows and threshold field had no behavioral test) is cleared: `if master_on` to `if true` at `environment.rs:185` and at `:202` each now fail the new test; the remaining gaps are the permanent history gaps (F5, F15), unmeasured mutation, and unaudited files.

Round 2 scope, as instructed: findings 14, 16 and 18 only; the full suite and the earlier mutants were not re-run, and round 1's results stand. Independence: the auditor did not write these tests; the gate test and `settings_sections.rs` diff were re-read cold at the working tree. No fresh-context subagent was available.

**Round 2 mutants** (one at a time, `git checkout` after each, `git status` clean for `environment.rs`; output in the scratchpad `m185.txt`, `m202.txt`): baseline `CARGO_INCREMENTAL=0 cargo test -p micold-client --test layout_snapshot notification_kind` 2 passed. M-A `environment.rs:185` `if true`: FAILED `the_kind_rows_and_the_threshold_take_input_only_while_the_master_switch_is_on` with `with the master switch off a click on NeedsPermission publishes nothing, got [Settings(NotificationKindToggled(NeedsPermission, false)), ...]`. M-B `:202` `if true`: FAILED, `typing into the threshold field edits it only with the master switch on (on: false)`, left 1, right 0. After restore: 2 passed. The sibling geometry test stayed green under both, so the new test, not the old gate, carries the behavior.

| # | Round 2 | Evidence |
| --- | --- | --- |
| 14 | **Resolved** (HIGH cleared) | The test renders the Environment page with the master switch on and off, clicks each kind row's box and types a digit into the threshold field through real `update` calls, and asserts the published messages; both mutants above fail it. The vacuous scan `the_kind_rows_are_disabled_while_the_master_switch_is_off` is deleted; the `||` in the neighbour is now `!view.contains("NotificationKindToggled(cli")` (`settings_sections.rs:623-626`) |
| 16 | **Resolved** (MED cleared) | `tdd/test-list.md:88-100` lists the two remaining scans (`the_desktop_notifications_section_lists_the_four_kind_rows`, `the_threshold_field_sits_under_the_long_task_row`) and the rendered test that stands for each behavior; they are wiring checks, not evidence |
| 18 | **Resolved by reading** (LOW cleared) | The new test zips the four kind rows with `NotificationKind::ALL`, so each click must yield that row's kind, which a swap of any two rows fails. The swap mutant (cycle 15, `environment.rs:177`) was taken from the log, not re-run, as scoped. Limit: the test shares `ALL` with the view, so a reordering of `ALL` itself is invisible to it |

Open: F5 and F15 (permanent), F17, F19, F20, F21 (LOW; T080, T081 open). No tasks added for F5 or F15.

## Suite

`CARGO_INCREMENTAL=0 scripts/build-lock.sh cargo test --workspace --no-fail-fast`: 5342 passed, 6 failed, 9 ignored, 254 s (previous run: 5321 passed, so 21 tests added by Phase 8). The 6 failures are the same root-only permission tests as before (`a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched`, `a_refused_write_is_logged_as_a_failure_with_the_reason`, `a_service_write_over_an_unreadable_settings_file_is_refused`, `a_directory_that_cannot_be_emptied_names_what_survived`, `the_leftover_report_is_capped`, `worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports`). After the mutants were restored: `daemon_sync::tests::` 42 passed, core `attention` 66 passed, the layout gate 1 passed, and five consecutive runs of `attention_claims`, `settings_notification_kinds`, `settings_long_task_threshold` and `attention_error_notice` pinned to one CPU with `taskset -c 0` all exited 0. `git status` shows no source change from the audit.

## Previous findings

| # | Was | Now | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Resolved** | Three bin tests in `daemon_sync.rs:4349-4392` drive `on_daemon_event` and run the returned task. Mutants D1 (grant arm passes a fixed `TurnFinished`), D3 (grant arm drops the notification) and D2 (error arm drops it) each fail 1-2 of those tests |
| 2 | HIGH | **Resolved** | The C15 kind snapshot is deleted (`state.rs:1637-1655`), contract reworded, and `turning_a_kind_on_keeps_a_pending_event_of_a_kind_that_was_already_on` (`settings_notification_kinds.rs:631`) pins the one observable difference; its red is recorded in cycle 14. The master switch's snapshot stays and its test is unchanged |
| 3 | HIGH | **Cleared, discipline gap remains** | Seven behaviors now carry a recorded red (cycle 14 lines 216-220), made by breaking finished code, so none is test-first. Classed `LIKELY` as the previous audit classed the same kind of red for T008-T011. See F15 |
| 4 | MED | **Resolved** | `tdd/test-list.md` exists: 29 outer and 19 inner behaviors, every one `DONE`, every `traces` id a real criterion. The `tests` column names task ids, not test names (F19) |
| 5 | MED | Open, permanent | History still cannot order the original cycles; commits `6a2e9081` and `0c2b4a1d` add the Phase 8 tests with their source |
| 6 | MED | **Resolved** | `DaemonState::advance_turn_clock` and `Uptime::saturating_add`; the 613 tests use a 30 s threshold and a 31 s advance. D4 and D6 (the seam ignored or bypassed) fail 13 tests. Only 20 ms supervision-tick loops remain |
| 7 | MED | **Resolved** | Reading recorded in `research.md` R7 (lines 201-206) with the gate left on the glyph at 16 px. Windows and macOS arms still compile-only (see not audited) |
| 8 | MED | **Resolved, with a limit** | `tests/gates/notification_kind_rows_sit_under_the_switch.rs` states the arrangement (five indented rows, stacked, threshold the fourth and taller); its red is recorded (reversed order). It cannot see two non-adjacent kind rows swapped (F18) |
| 9 | MED | **Resolved** | `the_sc_001_sequence_gives_eleven_notifications_of_their_kinds` (`attention_claims.rs:584-660`) asserts 5 + 5 grants, 0 `TurnFinished`, 1 `SessionErrorNotice`, 11 in all, and `attention_seq == 35` |
| 10 | MED | **Resolved** | Three tests; `a_user_stop_sends_nothing` ends through `stop_session` and a supervision tick alone, no `Ended` event |
| 11 | LOW | **Resolved, residue** | `attention_error_notice.rs` and `settings_notification_kinds.rs` use `attention_support`; `kinds()` is imported. `attention_claims.rs:143,176,206` still defines its own `idle_process`, `connect` and `next_frame` (F20) |
| 12 | LOW | **Resolved** | `tempfile::tempdir()` at `attention_claims.rs:527` |
| 13 | LOW | **Partly resolved** | `linux.rs` fixtures now say "needs permission". `the_threshold_is_one_minute` still pins a constant to its literal (`core/src/attention.rs:1076`, F21) |

## Test-first evidence

Classified per test task, then per `test-list.md` behavior by its weakest task (a behavior is `PROVEN` only when every task in its `tests` column is). Commits `6a2e9081` and `0c2b4a1d` change tests and source together, as every earlier implementation commit did, so history corroborates no order for any behavior.

| Behavior | Class | Evidence |
| --- | --- | --- |
| T001, T005, T006, T007 (U1, U3-U5) | PROVEN | Cycles 1 and 3: reds recorded before the code, same commit `63554884` |
| T021-T025 (U8, U9, U5's wire half) | PROVEN | Cycles 9-10: stubbed reds and a review-fix red; `dc6ac923`, `4cfe5bcc`; T074's split adds no behavior |
| T032, T056, T057 (A24, U11, U15) | PROVEN | Cycle 11: E0432 and E0559 reds before the code; `1b6f7eb7` |
| T046, T047 (A28, A29, U17, U18) | PROVEN | Cycle 13: compile reds, and the first green honestly failed the distinctness gate; `2cb52cc7` |
| A18, A19, U2 (T002) | LIKELY | Cycle 2 core red is a compile error; the daemon half was red by stubbing after the code |
| A6, A8, A9, A11, U6, U7 (T008-T011) | LIKELY | Cycles 5-7: reds by stubbing after the code. T068's change is test-first (cycle 14 red, then the deletion), so U7's C15 behavior is `PROVEN` |
| A13-A15, A17, A20 (T033) | LIKELY, retroactive | Cycle 14 line 217: a four-test red from `Catalog::notify` ignoring the kind switch, made after the tests and code landed together |
| A18, A22, A23 (T058) | LIKELY, retroactive | Cycle 14 line 218: 5 failures from the `SettingsSet` arm dropping the threshold |
| A22, U16 (T059) | LIKELY, retroactive | Cycle 14 line 216: 2 failures from `TurnClock::change` ignoring the threshold passed |
| A12, A25, A26, U12 (T034) | LIKELY, retroactive | Cycle 14 line 220: icons, icons_font and notification_icon failures from a changed codepoint |
| A16, A20, A21, A23, U13 (T035, T060) | LIKELY, retroactive | Cycle 14 line 220: 3 `features_settings` failures. **No test fails for the disabled state itself** (F14) |
| A12, A16, A26, U14 (T036) | LIKELY, retroactive | Cycle 12 mutants (`Checkbox::icon`) and one genuine red (`a_disabled_checked_checkbox_keeps_a_visible_mark`); the cycle-14 attempt did not turn a test red, which is what T075's gate then closed for ordering. The disabled-rows scan test stays vacuous (F14) |
| U10 (T067) | LIKELY, retroactive | Cycle 14 line 221: red by M15 after the three tests and the `with_notifier` seam landed together |
| A21, U19 (T075) | LIKELY, retroactive | The gate's red came from reversing the source after the gate existed |

Counts by weakest task: 14 behaviors `PROVEN` (A5, A24, A28, A29, U1, U3-U5, U8, U9, U11, U15, U17, U18), 34 `LIKELY`, 0 `TEST_AFTER`, 0 `NO_TEST`. The retroactive class is a decision to read, not a pass: a red made by breaking finished code proves the test can fail, not that it came first.

### Existing tests changed by Phase 8

The diff `f8f07f58..HEAD` was read for removed or loosened assertions in tests that predate the branch. None: the 200 ms and 300 ms constants moved to 30 s and 31 s with a seam that makes the long case reachable without waiting (the short case is stricter, not weaker); `endings_without_an_error_send_nothing` was split, each ending keeping its assertion and the user-stop case now asserting more (`live_session` is `None`). `schema_hash.rs`, `version.rs` and `linux.rs` changes beyond 613 come from the merge of main (BUG-442, BUG-566). No skip, filter rename, exclusion or threshold change.

`tasks.md` against `test-list.md`: all behaviors `DONE`; every task ticked `[X]` or `[x]` (T001-T052, T056-T076) maps to a `DONE` behavior; the three open boxes (T053-T055) are the close unit and map to none. No `DONE` behavior has an unticked behavioral task.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 14 | ~~HIGH~~ **Resolved in round 2** | (round 1 text) The disabled state of the Settings kind rows and threshold field (US2.5, FR-012, A16; also T060's "not editable") has no behavioral test. `the_kind_rows_are_disabled_while_the_master_switch_is_off` asserts only that the view's source text contains `draft.environment.desktop_notifications`, a string present on the unconditional `let master_on = ...` line, so `if master_on` to `if true` at `environment.rs:185` and `:202` passes it (the same mutant cycle 12 disclosed as surviving). Its two siblings scan the same way: `the_desktop_notifications_section_lists_the_four_kind_rows` (`!view.contains("AiCli::ALL") \|\| !view.contains(...)` is true when either string is absent) and `the_threshold_field_sits_under_the_long_task_row`. `features_settings.rs` has no test of a draft with the master switch off. Fix: render the Environment page with the master switch off and assert the kind rows and the threshold field carry no toggle and no input handler (a view-model or layout-state assertion), and with it on that they do; keep the scans or drop them as redundant | `crates/micold-client/tests/settings_sections.rs:603-690`, `crates/micold-client/src/ui/settings/environment.rs:185,202`, `tdd/cycle-log.md:176-179` |
| 15 | MED | Seven behaviors (T033, T034, T035, T036, T058, T059, T060) and the T067 and T075 gates have only a retroactive red. Nothing in Phase 8 can change that; it is a permanent gap, recorded so the next feature does not treat the cycle-14 reds as test-first | `tdd/cycle-log.md:216-227` |
| 5 | MED | Carried over: order unprovable from history for cycles 2 and 4-7 | `tdd/cycle-log.md:46-95` |
| 16 | **Resolved in round 2** | (round 1 text) Source-text scans as behavior tests are a pattern, not three accidents: `settings_sections.rs` has 17 `contains` checks. They are the repository's recorded guard style for architecture rules, but here they are used for rendered behavior (order, indent, disabled). T075 closed the order and indent half; the disabled half is F14 | `crates/micold-client/tests/settings_sections.rs` |
| 17 | LOW | `DaemonState::advance_turn_clock` is `pub` and compiled into production builds (integration tests need it), as `set_long_task_threshold` is. Its only callers are tests. Mutant D7 (the spinner path reading `clock::now()` directly) survives because no test advances the clock before a spinner starts a turn; judged equivalent today, unpinned if the offset ever precedes the start | `crates/micold-daemon/src/state.rs:654,3677` |
| 18 | **Resolved in round 2** | (round 1 text) The T075 gate pins that the fourth row is taller and the run is indented and stacked, not which kind is which. Swapping Needs permission and Session error leaves it green; order is otherwise pinned only by `settings_sections` scanning for `NotificationKind::ALL` | `crates/micold-client/tests/gates/notification_kind_rows_sit_under_the_switch.rs:122-139` |
| 19 | LOW | `test-list.md` `tests` column names task ids, so `traces` cannot be checked to a test function mechanically; the audit did it through `tasks.md` text | `tdd/test-list.md:27-81` |
| 20 | LOW | `attention_claims.rs` keeps its own `idle_process`, `connect` and `next_frame` beside `mod attention_support`, so the duplication T076 removed from two files persists in the third | `crates/micold-daemon/tests/attention_claims.rs:143,176,206` |
| 21 | LOW | `the_threshold_is_one_minute` pins `LONG_TASK_THRESHOLD` to its own literal | `crates/micold-core/src/attention.rs:1076` |

No other `HIGH` smell in the files read: no assertion-free test, tautology, doubled subject, conditional assertion or ignored test. `RecordingNotifier` in `daemon_sync.rs` is a hand-written double of `DesktopNotifier`, a trait seam the profile's `helpers` do not cover; the new dispatch tests assert what reached it, with the real `attention_notification` and `session_error_notification` unmocked. The three Checkbox tests read cold: `a_disabled_row_with_an_icon_builds` asserts `on_toggle.is_none()` and construction, enough for what it claims.

## Mutation results

`cargo-mutants` is not installed (profile `mutation: null`), so this is deliberate mutants only, on the files Phase 8 touched: `crates/micold-client/src/shell/daemon_sync.rs` (dispatch arms) and `crates/micold-daemon/src/state.rs` plus `crates/micold-core/src/clock.rs` (the turn clock seam). One mutant at a time, restored with `git checkout`, tree verified clean, targeted suites re-run green afterwards. A first batch mis-passed `--no-fail-fast` after `--`, giving a harness error for D1 and for D2; both were re-run correctly and the results below are from the re-run.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| D1 `daemon_sync.rs:705` grant arm passes `NotificationKind::TurnFinished` | U10 (T067) | No | Caught by 2 of the 3 dispatch tests (`a_needs_permission_grant_...`, `each_granted_kind_...`); finding 1 closed |
| D2 `daemon_sync.rs:713` error arm drops the notification | U10 | No | Caught by `a_session_error_notice_reaches_the_notifier_as_a_session_error` |
| D3 `daemon_sync.rs:706` grant arm drops the notification | U10 | No | Caught by 2 tests |
| D4 `state.rs:257` `turn_now` ignores the offset | U3, A2, A3, A22 | No | Caught by 13 tests across 5 binaries |
| D6 `state.rs:3483` `note_activity` reads `clock::now()` instead of `turn_now` | A2, A3, A7 | No | Caught by the same 13 |
| D7 `state.rs:3677` spinner path reads `clock::now()` directly | A2 (spinner) | **Yes** | Equivalent today: the offset is zero when the spinner starts a turn in every test (F17) |
| D5 `clock.rs:55` `saturating_add` returns `self` | | n/a | Not applied (the edit did not match); D4 exercises the same effect through `turn_now` |

Score: 5 of 6 valid mutants caught (83 %), one equivalent survivor. Scope: 3 files, 6 mutants; not comparable to a tool score. The cycle-12 survivors (label `disabled_tint`; kind rows' `if master_on`) were not re-run: outside the scope given, and F14 explains why the second cannot be caught.

Coverage: none (no tool).

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1-1.3, 1.7, 1.8, FR-002, FR-003, SC-001 | `attention_claims.rs` 613 block, `turn_clock_tests`, `the_sc_001_sequence_gives_eleven_notifications_of_their_kinds` | Yes: `note_activity` through `serve_connection` and a framed window |
| US1.4-1.6, 1.10 (error), FR-004, FR-007 | `attention_error_notice.rs` (12 tests), `daemon_sync` dispatch tests, `attention_notify.rs` | Yes, now through the client dispatch too (finding 1 closed) |
| US1.9, US1.11, FR-014, FR-024 | `copilot_activity.rs`, `pi_activity.rs`, `a_helper_agent_finishing_changes_nothing` | Yes |
| US1.10 (awaiting-input kinds) | inherited 039 tests, `daemon_sync` dispatch of the three grant kinds | Yes at the dispatch; the claim path is 039's |
| US2.1, 2.9, 2.10, FR-009, S5 | `settings_sections.rs` scans, `features_settings.rs`, the T075 gate | Partial: geometry and order by the gate, wording by scans |
| **US2.5, FR-012** | `gates/notification_kind_rows_sit_under_the_switch.rs` `the_kind_rows_and_the_threshold_take_input_only_while_the_master_switch_is_on` | Yes (round 2: fails under `if true` at `environment.rs:185` and `:202`) |
| US2.2-2.4, 2.6, 2.7, SC-002, SC-003, SC-006 | `settings_notification_kinds.rs` (10 tests) | Yes |
| US2.8, 2.13, FR-010, FR-025, FR-026, SC-008 | `settings.rs` units, `settings_long_task_threshold.rs` (core and daemon), `features_settings.rs` | Yes |
| US2.11, 2.12 | daemon and core threshold tests, `a_bad_threshold_refuses_the_save_with_the_s6_message` | Yes |
| US3.1-3.3, FR-015, FR-017 | `notification_icon.rs`, `icons*.rs`, `attention_notify.rs`, `research.md` R7 | Partial: gate measures the glyph at 16 px by decision (T072) |
| US3.4, FR-016 | `desktop_notify` units | Partial: Windows and macOS compile-only |
| US3.5 | tile luminance gate, visual pass | Partial |
| FR-022, FR-023 | showcase section, user guide | Not automatically tested |
| SC-004, SC-005, SC-007 | human trials, `sandbox_real_*` | Not covered (T054, T055 open) |

Untested criteria: none. Tests tracing to nothing: none found (`the_threshold_is_one_minute` pins D4's default, F21).

## What was not audited

- Round 2 re-read only the gate test, the `settings_sections.rs` diff, `environment.rs:170-215`, cycle 15 and the test-list scan section; everything else is round 1's evidence, and the suite was not re-run (the round 1 counts in the frontmatter stand).
- Mutation: no tool, 6 deliberate mutants on 3 files, as scoped. The remaining feature files (client UI, rasteriser, `windows.rs`, `macos.rs`, wire encoding, `settings.rs` persistence, `catalog.rs`) were not mutated in this audit. The cycle-12 `if master_on` survivor was re-run in round 2 and is killed; the `disabled_tint` survivor was not re-run.
- Smell pass: one reader, not a fresh-context subagent. Re-read this round: the Phase 8 diffs of every test file the remediation touched, `daemon_sync.rs` new tests, the T075 gate, `settings_sections.rs` 590-690, `checkbox.rs` tests. **Not re-read cold this round** and unchanged since the previous report's coverage: `core/tests/settings_long_task_threshold.rs`, `core/src/settings.rs` and `catalog.rs` test modules, `daemon/src/{attention,activity,hooks}.rs` test modules, `features_settings.rs` beyond the threshold and kind tests, `icons_font.rs`, `main_tests.rs` beyond the merged BUG-442 test, `pi_activity.rs`, `window` and `showcase` code. HIGH smells in those remain unassessed.
- The "20 consecutive green runs" claimed for T071 was spot-checked with 5 runs on one CPU, not repeated 20 times; flakiness under a loaded runner is not measured.
- Windows and macOS code paths: cannot run on this host.
- The acceptance suite (`sandbox_real_*`, needs a container runtime and image) and SC-007: not run.
- `cargo fmt --check`, clippy, the 35-target gate run and `scripts/tests/*.test.sh` were not run.
- Docs (FR-023), the showcase (FR-022), the visual-pass PNGs and `quickstart.md` Part A (T054) were not audited. T053-T055 are open by design (the close unit).
- Performance beyond wall time: the suite took 254 s against a profile baseline of 343 s; no per-test timing was taken.
