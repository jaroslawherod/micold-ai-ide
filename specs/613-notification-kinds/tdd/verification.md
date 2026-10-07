---
feature: 613-notification-kinds
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no preset or override present)
verified_at: f8f07f58 # short SHA audited, branch claude/project-thread-8kdqkn, base origin/main
behaviors: 26 # no tdd/test-list.md: the 26 test tasks of tasks.md stand in for behavior ids
proven: 14
likely: 5
test_after: 7
no_test: 0
high_smells: 0 # in the files read; see "What was not audited" for the unread remainder
criteria_total: 29 # acceptance scenarios US1.1-11, US2.1-13, US3.1-5
criteria_covered: 27 # 2 partial: US1.10 (awaiting-input kinds), US3.4 (Windows/macOS arms compile-only)
mutation_score: unmeasured # no cargo-mutants; 14 valid deliberate mutants: 12 caught, 2 survived (see below)
mutants_survived: 2 # both inside DONE behaviors, neither equivalent
suite: 5321 passed, 6 failed (root-only permission tests, ignored per instruction), 9 ignored, 440 result lines, 154 s
---

# TDD Verification: Notification kinds, per-kind settings and icons

**Verdict: FAIL.** Seven of the 26 test tasks (the M3 daemon-integration, client and UI tests) have no recorded red and landed with their code, and two deliberate mutants inside finished behaviors survived.

Independence: the auditor did not write these tests. Every file cited was re-read at HEAD. The feature has no `tdd/test-list.md` (cycle-log lines 3-8 admit it), so by the rubric alone this is `BLOCKED`; it was audited against `tasks.md` anyway and the findings stand.

## Suite

`CARGO_INCREMENTAL=0 cargo test --workspace --no-fail-fast`: 5321 passed, 6 failed, 9 ignored, 154 s (baseline in the profile: 5071 passed, 9 ignored, so about 250 tests added). The 6 failures are the root-only permission tests (`a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched`, `a_refused_write_is_logged_as_a_failure_with_the_reason`, `a_service_write_over_an_unreadable_settings_file_is_refused`, `a_directory_that_cannot_be_emptied_names_what_survived`, `the_leftover_report_is_capped`, `worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports`), none in a file the feature touched. A second full run after all mutants were restored gave the identical 5321 / 6 / 9, and `git status` was clean.

## Test-first evidence

No commit on the branch is test-only. Every implementation commit (`63554884`, `6a9d2397`, `b67fb7b7`, `dc6ac923`, `1b6f7eb7`, `d85e6c2d`, `905e9a64`, `2cb52cc7`) adds tests and source together, so history can corroborate neither order nor absence of order; the cycle log is the only evidence of a red. Behavior ids are task ids.

| Behavior (test task) | Class | Evidence |
| --- | --- | --- |
| T001, T005, T006 | PROVEN | Cycle 1: red 40 passed / 24 failed from `todo!()` stubs and a title mismatch; same commit `63554884` |
| T007 | PROVEN | Cycle 3: E0559 no field `kind`; `63554884` |
| T021-T025 | PROVEN | Cycle 9: stubbed reds recorded for daemon, client and core; cycle 10 adds a red for the review fix; `dc6ac923`, `4cfe5bcc` |
| T032, T056, T057 | PROVEN | Cycle 11: E0432 and E0559 recorded before the code; `1b6f7eb7` |
| T046, T047 | PROVEN | Cycle 13: E0432/E0422/E0425 recorded; the first green failed the distinctness gate honestly; `2cb52cc7`. Windows and macOS arms compile-only (see finding 7) |
| T002 | LIKELY | Cycle 2 core red is a compile error; the daemon half (cycle 4) was "written with the code", red added afterwards by stubbing |
| T008, T009, T010, T011 | LIKELY | Cycles 5-7: "recorded in unit 3 by stubbing", i.e. after the code. Proves the tests can fail, not that they came first. Cycle 8 (spinner clock, `left: []` red) is genuine test-first for one T010 case |
| T033 | TEST_AFTER | Cycle 12 records no red for `settings_notification_kinds.rs` |
| T058 | TEST_AFTER | Same: `settings_long_task_threshold.rs` (daemon), no red |
| T034, T035, T060 | TEST_AFTER | Cycle 12: "written by delegated workers in the same pass as their code: no separate red run recorded (honest gap)" |
| T036 | TEST_AFTER | Same; only `a_disabled_checked_checkbox_keeps_a_visible_mark` has a red |
| T059 | TEST_AFTER | Cycle 11: the SC-008 and FR-013 clock tests "passed at once"; only the description text had a red. Mutant `>=` to `>` run afterwards |

Retroactive mutation in cycle 12 (lines 171-185) is welcome evidence of strength but is not a red: the rubric classes "no red recorded" as `TEST_AFTER`.

### Existing tests changed by the feature

Reviewed every removed or changed assertion in tests that predate the branch. None is weakened, skipped, loosened, renamed out of a filter, or excluded, and no threshold was lowered (`ci.yml` gains `--test notification_icon`; `Cargo.toml` gains one dependency).

- `crates/micold-client/tests/attention_notify.rs`: `attention_granted(b, &notifier)` gains the kind argument, and the title expectation moves from "waiting for input" to the long-task title. A legitimate behavior change (FR-008).
- `crates/micold-client/tests/icons.rs` and `icon_roles.rs`: `Icon::ALL.len()` 35 to 39 and `IconSurface::ALL.len()` 5 to 6, each with a new value pinned beside it.
- `crates/micold-daemon/tests/{attention_claims,settings_desktop_notifications,unread_state}.rs`: threshold set to zero so 039's claim rules still see a grantable kind, with a comment saying why. The 039 assertions are unchanged.
- `crates/micold-core/tests/schema_hash.rs`: version constant moves with each wire bump.
- Snapshots `style_snapshot.txt` (8 checkbox lines) and `layout_snapshot.txt` (148 changed lines) were regenerated in `1dbbb112` and `8872c8f3`. The style change is pinned by a test that went red first (cycle 12); the layout change is pinned only by `settings_sections.rs` and the visual pass. See finding 8.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Mutant M15 survived: the window's handler of `DaemonMsg::AttentionGranted` can ignore the grant's kind and title every notification "finished its turn", and all 400 bin tests plus the 16 `attention_notify` tests stay green. `attention_notify.rs` calls `State::attention_granted(b, kind, ..)` directly, so the dispatch arm in the bin-only `daemon_sync.rs` is never exercised. T019 and T030 are ticked. Fix: a test that feeds `DaemonMsg::AttentionGranted` / `SessionErrorNotice` through the daemon_sync dispatch and asserts the notifier received the right kind | `crates/micold-client/src/shell/daemon_sync.rs:704-716` |
| 2 | HIGH | Mutant M6 survived: `if turned_on` in `set_notification_kinds` changed to `if false && turned_on` and all of `settings_notification_kinds` (9/9) passes. The C15 snapshot is unobservable in every specified scenario, because `note_event(.., notify=false)` already uses up an event whose kind was off. It is observable in one unspecified case: a pending, unclaimed event of an enabled kind is dropped when another kind is switched on. Either the block is dead code or that behaviour is wanted and unpinned. T038 is ticked | `crates/micold-daemon/src/state.rs:1603-1611` |
| 3 | HIGH | Seven behaviors are `TEST_AFTER` (T033, T034, T035, T036, T058, T059, T060): written with their code, no red, and the log calls it an honest gap. A `FAIL` condition on its own | `tdd/cycle-log.md:158-170`, `:146-151` |
| 4 | MED | No `tdd/test-list.md`: no behavior ids, no `traces` column, no per-behavior state, so traceability and the `[X]` against `DONE` check could not be done mechanically. All ticked tasks T001-T052 and T056-T066 were checked against the cycle log by hand; none lacks a cycle, but the log has none for T020, T031, T039, T040 and T044 as separate entries, which were folded into cycles 12-13 | `tdd/cycle-log.md:3-8` |
| 5 | MED | Order is unprovable from history, and cycles 4-7 (T002 daemon half, T008-T011) have reds made by stubbing code that already existed, so they are `LIKELY` at best | `tdd/cycle-log.md:46-95` |
| 6 | MED | Real-clock timing with a 200 ms threshold and 300 ms sleeps. The SC-001 test's permission turns await a claim round trip between the prompt and the `Stop` (`attention_claims.rs:613-618`), so a stall over 200 ms on a loaded runner turns a `NeedsPermission` expectation into an extra `LongTaskFinished` grant and fails the test. Same pattern in `settings_notification_kinds.rs:145-149` and `settings_long_task_threshold.rs:128`. Make the clock injectable or widen the margin by an order of magnitude | `crates/micold-daemon/tests/attention_claims.rs:383-386,449,472,507,570,609,675,777` |
| 7 | MED | The FR-017 distinctness gate measures `glyph_mask(kind, 16)`, the glyph at span 1.0, which is a production function that exists only for the gate. The shipped tile render uses the 62.5 % span, where cycle 13 recorded SessionError and LongTaskFinished differing in only 13 of 256 pixels. The gate was redefined after its first red; FR-017's wording supports the reading, but a reviewer should confirm it. Windows and macOS request tests (`toast_icon`, `ICON_CROP`, `banner_image`) are compile-only here | `crates/micold-client/tests/notification_icon.rs:132-151`, `crates/micold-client/src/notification_icon.rs:86-93`, `tdd/cycle-log.md:202-210` |
| 8 | MED | Self-regenerated layout snapshot: 148 lines rewritten in the same commit as the Settings change, accepted by a visual pass rather than a test that states the expected geometry. `settings_sections.rs` covers order and indentation only | `crates/micold-client/tests/fixtures/layout_snapshot.txt`, commit `1dbbb112` |
| 9 | MED | SC-001 asks for 11 notifications including the error ending; the sequence test is named "ten grants" and stops at the 10 awaiting-input grants. The error ending is covered only separately | `crates/micold-daemon/tests/attention_claims.rs:588-626` |
| 10 | MED | Eager test with a weak middle case: three endings in one test, and the "user stop" case sends `Ended { error: true }` after `stop_session`, so it pins "a stopped session is not live" rather than "a user stop is no error" | `crates/micold-daemon/tests/attention_error_notice.rs:390-433` |
| 11 | LOW | Duplicated helpers: `kinds()` at `attention_claims.rs:396` repeats `attention_support/mod.rs:174`; `connect`, `next_frame`, `idle_process` are copied in `attention_error_notice.rs:186-264` and `settings_notification_kinds.rs:239-308` although `attention_support/mod.rs` provides them | files cited |
| 12 | LOW | Fixed shared temp path `micold-hooks-test-613` in a shared directory | `crates/micold-daemon/tests/attention_claims.rs:534` |
| 13 | LOW | `the_threshold_is_one_minute` pins a constant to its own literal; the Linux request fixture still carries the pre-613 title "waiting for input" | `crates/micold-core/src/attention.rs:1076`, `crates/micold-client/src/shell/desktop_notify/linux.rs:455` |

No `HIGH` smell from the catalogue (assertion free, tautological, doubled subject, vacuous, conditional) in the files read. The Linux image-data tests compare against `render(kind, 64)`, which would be a re-implemented expectation if it were the only check on the pixels, but `notification_icon.rs` pins the render separately.

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, each restored with `git checkout` and the tree verified clean; the full suite was re-run after the last one. 15 ids attempted: M11 and M12 did not compile, M8 was mis-aimed and redone as M8a/b, leaving 14 valid runs.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| M1 `turn_clock` `>= threshold` to `>` (`core/attention.rs:311`) | T005, T059 | No | Caught by 4 tests, boundary pinned |
| M2 `Paused`+`Working` restarts the start (`core/attention.rs:297`) | T005 (FR-003) | No | Caught by 2 tests |
| M3 `Catalog::notify` drops the master switch (`catalog.rs:423`) | T002, T004 | No | Caught |
| M4 `error_notice` drops the in-view guard (`state.rs:3399`) | T024 | No | Caught by `a_session_in_view_sends_nothing` |
| M5 `error_notice_target` `.last()` to `.first()` (`daemon/attention.rs:81`) | T022 | No | Caught by unit and integration tests |
| M6 `if turned_on` to `if false && turned_on` (`state.rs:1603`) | T033, T038 (C15) | **Yes** | Finding 2 |
| M7 `MAX_LONG_TASK_THRESHOLD_SECS` 3600 to 3601 (`core/settings.rs:94`) | T056 | No | Caught by 3 tests |
| M8a/b client `long_task_threshold()` upper bound exclusive, lower bound plus one (`client/features/settings.rs:806`) | T060, T064 | No | Caught by `a_bad_threshold_refuses_the_save_with_the_s6_message`. A first attempt (M8) hit the older `timeout()` function with the same text and is discarded |
| M9 `SubagentStop` classified as `Stop` (`hooks.rs:245`) | T008 (FR-024) | No | Caught by the unit test and the integration test |
| M10 `grant` ignores `notify` (`daemon/attention.rs:122`) | T009 | No | Caught by 2 unit tests |
| M13 effective threshold ignores the override (`state.rs:247`) | T058, T063 | No | Caught by 4 tests |
| M14 daemon setter drops the clamp (`catalog.rs:447`) | T056, T063 | No | Caught |
| M15 `daemon_sync` grant arm titles every grant `TurnFinished` (`daemon_sync.rs:704`) | T019, T030 | **Yes** | Finding 1 |
| M11, M12 | | n/a | Did not compile (a match arm made non-exhaustive; a missing import). Not counted |

Score: 12 of 14 valid mutants caught (86 %). Sample of 14, over the turn clock, the master and kind switches, the error notice routing, the threshold range, the hook classification and the client dispatch. Not comparable to a tool score, and the sample is not exhaustive. Cycle 12's own mutants (save writes kinds, toggle, threshold range, icon mapping, `Checkbox::icon`) were not repeated; its two disclosed survivors (label `disabled_tint`, `if master_on` gate) are visual-pass only.

Coverage: none (no tool).

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1, 1.2, 1.3, 1.7, 1.8, FR-002, FR-003 | `attention_claims.rs` 613 block, `turn_clock_tests` | Yes: `DaemonState::note_activity` through `serve_connection` and a framed window |
| US1.4, 1.5, 1.6, 1.10 (error), FR-004, FR-007 | `attention_error_notice.rs` (10 tests), `attention_notify.rs` | Yes, daemon side. Client dispatch not (finding 1) |
| US1.9 | `copilot_activity.rs`, `pi_activity.rs` | Yes |
| US1.11, FR-024 | `a_helper_agent_finishing_changes_nothing` via the hook receiver, `hooks.rs` units | Yes |
| US1.10 (awaiting-input kinds) | inherited 039 tests only | Partial: no 613 test |
| US2.1, 2.9, 2.10, FR-009 | `settings_sections.rs`, `features_settings.rs` | Partial: view model, not a rendered window (layout gate only) |
| US2.2-2.7, SC-002, SC-003, SC-006 | `settings_notification_kinds.rs` | Yes |
| US2.8, 2.13, FR-010, FR-026 | `settings.rs` units, `settings_long_task_threshold.rs` (core) | Yes |
| US2.11, 2.12, FR-025, SC-008 | daemon and core threshold tests, `features_settings.rs` | Yes |
| US3.1-3.3, FR-015, FR-017 | `notification_icon.rs`, `icons*.rs`, `attention_notify.rs` | Partial: gate measures a proxy (finding 7) |
| US3.4, FR-016 | `desktop_notify` units | Partial: Windows and macOS compile-only |
| US3.5 | tile luminance gate, visual pass | Partial |
| FR-022, FR-023 | showcase section, user guide | Not automatically tested (visual pass, review) |
| SC-001 | `the_sc_001_sequence_gives_ten_grants_of_their_kinds` | Partial (finding 9) |
| SC-004, SC-005 | human trials | Not testable; proxies only (T054, still open) |
| SC-007 | `sandbox_real_*` | Not covered: no 613 test runs in a container |

Untested criteria: none outright. Tests tracing to nothing: none found (`the_threshold_is_one_minute` pins D4's default, `icons.rs` pins codepoints).

## What was not audited

- No fresh-context subagent was available, so the smell pass was one reader. Read in full: `core/src/attention.rs` test modules, `daemon/tests/attention_support/mod.rs`, `attention_claims.rs` (613 block), `attention_error_notice.rs`, `settings_notification_kinds.rs` (to line 686), `client/tests/notification_icon.rs`, the Linux and `desktop_notify/mod.rs` test modules, and the diffs of `attention_notify.rs`, `copilot_activity.rs`, `icons.rs`, `icon_roles.rs`. **Not read cold:** `daemon/tests/settings_long_task_threshold.rs`, `core/tests/settings_long_task_threshold.rs`, `core/src/settings.rs` and `catalog.rs` test modules, `daemon/src/{attention,activity,hooks}.rs` test modules, `client/tests/{features_settings,settings_sections,icons_font}.rs` beyond one test, `checkbox.rs` tests, the Windows and macOS test modules, `main_tests.rs`, `pi_activity.rs`, and the showcase. HIGH smells in those are unassessed.
- Mutation: 14 valid deliberate mutants, no tool, no whole-feature score. Client UI, rasteriser internals, `windows.rs`, `macos.rs`, wire encoding and `settings.rs` persistence were not mutated beyond cycle 12's own list.
- Coverage: no tool.
- Windows and macOS code paths: cannot run on this host; only the cycle log's `cargo check`/clippy claims exist for them.
- The acceptance suite (`sandbox_real_*`, needs a container runtime and a built image) and SC-007 were not run.
- Isolation, flakiness and speed were assessed by reading only: no repeated runs, no load test. Finding 6 is a reasoned risk, not an observed flake.
- `cargo fmt --check`, clippy, the 35-target architecture gates and `scripts/tests/*.test.sh` were not run.
- Docs (FR-023), the showcase (FR-022), the visual-pass PNGs and `quickstart.md` Part A (T054) were not audited.
- Phase 7 tasks T053-T055 are open by design (the close unit).
