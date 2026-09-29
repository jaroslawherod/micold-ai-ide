---
feature: 035-report-missing-include-script
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 10 # US1 AS1–AS5, US2 AS1–AS3, US3 AS1–AS2
planned_at: 0f7e0c8f
updated_at: 0f7e0c8f
suite_baseline: green # 3662 passed, 0 failed, 8 ignored, 341 binaries at 0f7e0c8f
---

# Test List: Report a Missing Environment-Include Script

Derived from `spec.md` (the acceptance scenarios, FR-001 to FR-014, Edge Cases), `plan.md` (test
strategy), `research.md` R1–R9, and the two contracts. The code was read only to place behaviours
and to find existing tests. None of these behaviours has an existing test.

Trace ids: `US<n>-AS<m>` is acceptance scenario *m* of user story *n* in spec.md. `FR-0xx` and
`SC-00x` are spec.md's. Contract row ids (`C*`, `P*`, `S*`, `N*`, `T*`) are from
`contracts/script-path-check.md` and `contracts/settings-indication.md`. They are cited beside the
spec trace for placement, never instead of it.

## Outer loop: acceptance behaviors

**Entry point.** No end-to-end GUI runner exists (the profile's `acceptance` runner is the sandbox
suite, which has nothing to do with this feature). The highest level a cargo test reaches is the
binary's `App` in `crates/micold-client/src/main_tests.rs`, with `Capabilities` narrowed to a
`FakeScriptPathProbe`. Each test runs the real shell handler (`on_settings_opened`, `apply_save`,
the `DaemonMsg::SettingsChanged` arm, or a session launch), runs the check job that handler
prepared synchronously (see T017), feeds its message back through `app.core.update`, and then
reads the page's lines with `script_path_notice(&app.core.settings.script_check,
&app.env_include_last_outcome)` and the notification queue. What is actually rendered is the
quickstart §B visual pass (T021, T042, T027, T029).

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| A1  | Feature off, stored absolute path missing: after Settings opens and its check lands, the page's first line is the caution `Script not found: <path>`, followed by the OFF note | US1-AS1, FR-001, FR-002, SC-001 | example | PENDING | |
| A2  | Feature off, stored path an existing regular file: after the check lands, the page has no lines below the timeout field | US1-AS2, SC-002 | example | PENDING | |
| A3  | Feature off, stored path blank: the page has no lines, and the probe is never called | US1-AS3, FR-011 | example | PENDING | |
| A4  | Feature off with a missing stored path: a session launch makes no probe call and no resolver call, and the launch proceeds as before | US1-AS4, FR-003, FR-006, SC-004 | example | PENDING | |
| A5  | A save with a missing stored path, the feature off and then on: the settings are written, and exactly one Info notification names the path and says it was not found, for each save | US1-AS5, FR-004 | example | PENDING | |
| A6  | Feature on, stored path missing, last outcome `MissingScript`: the page shows exactly one `Script not found: <path>` caution, then the ON note | US2-AS1, FR-005 | example | PENDING | |
| A7  | From A6, the user unticks the feature and saves, then reopens Settings: the same `Script not found: <path>` caution, now followed by the OFF note | US2-AS2, FR-005, SC-003 | example | PENDING | |
| A8  | A missing path is reported, then the file is created and Settings is reopened. With the feature on (last outcome still `MissingScript`), the page shows FR-014's caution and the note that the file exists now and saving or restarting a session will source it. With it off, the page has no lines | US2-AS3, FR-009, FR-014 | example | PENDING | |
| A9  | From a reported missing path, the user types an existing absolute path and saves: the check lands `Present`, the page has no path lines, and no notification is posted | US3-AS1, FR-009, SC-006 | example | PENDING | |
| A10 | From a reported missing path, the user clears the path and saves: `script_check` is `Idle`, the page has no lines, and no notification is posted | US3-AS2, FR-011, SC-006 | example | PENDING | |

## Inner loop: unit behaviors

### `crates/micold-core/src/script_path_check.rs`: `classify`

Tests in `crates/micold-core/tests/script_path_check.rs`, with `FakeScriptPathProbe`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U1  | `""` gives `None`, and the probe is not called | FR-011 (C1) | example | DONE | `crates/micold-core/tests/script_path_check.rs::an_empty_path_has_no_check_and_examines_nothing` |
| U2  | Whitespace-only `"   "` gives `None`, and the probe is not called | FR-011 (C1) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_whitespace_only_path_has_no_check_and_examines_nothing` |
| U3  | `"~"` and `"~/env.sh"` give `NotFound { tilde: true }` without a probe call | FR-001, Edge Cases `~` (C2) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_tilde_path_is_not_found_without_a_probe` |
| U4  | `"~\\env.ps1"` gives `NotFound { tilde: true }` on every OS (a string test) | FR-001, FR-012 (C2) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_backslash_tilde_path_is_not_found_on_every_os` |
| U5  | `"~env.sh"` (a name that starts with `~` but is not `~/` or `~\`) is `Relative`, not tilde: the other side of U3's boundary | FR-001 (C2/C3) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_name_that_merely_starts_with_a_tilde_is_relative` |
| U6  | `"env.sh"`, `"./env.sh"` and `"scripts/env.sh"` give `Relative` without a probe call | FR-001, Edge Cases relative (C3) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_relative_path_is_not_checked` |
| U7  | An absolute path whose probe answers `File` gives `Present`. The probe is called once, with the path exactly as stored (no trim, no expansion) | FR-001 (C4) | example | DONE | `crates/micold-core/tests/script_path_check.rs::an_absolute_path_that_is_a_file_is_present_and_probed_once_as_stored` |
| U8  | An absolute path whose probe answers `Missing` gives `NotFound { tilde: false }` | FR-001 (C5) | example | DONE | `crates/micold-core/tests/script_path_check.rs::an_absolute_path_with_nothing_there_is_not_found` |
| U9  | An absolute path whose probe answers `NotAFile` gives `NotReadable` | FR-001, Edge Cases directory (C6) | example | DONE | `crates/micold-core/tests/script_path_check.rs::an_absolute_path_that_is_not_a_file_is_not_readable` |
| U10 | An absolute path whose probe answers `Unreadable` gives `NotReadable` | FR-001, Edge Cases check error (C6) | example | DONE | `crates/micold-core/tests/script_path_check.rs::an_absolute_path_that_cannot_be_opened_is_not_readable` |

### `crates/micold-core/src/script_path_check.rs`: `StdScriptPathProbe`

Tests in `crates/micold-core/tests/script_path_check.rs`, on `tempfile` directories.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U11 | A regular readable file answers `File` | FR-001 (P1) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_regular_file_answers_file` |
| U12 | Nothing at the path answers `Missing` | FR-001 (P2) | example | DONE | `crates/micold-core/tests/script_path_check.rs::nothing_at_the_path_answers_missing` |
| U13 | A directory answers `NotAFile` | FR-001, Edge Cases directory (P3) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_directory_answers_not_a_file` |
| U14 | `cfg(unix)`: a mode-000 file answers `Unreadable` (skipped as root) | FR-001 (P4) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_file_the_user_cannot_open_answers_unreadable` |
| U15 | `cfg(unix)`: a file under a parent without search permission answers `Unreadable`, not `Missing` | Edge Cases check error (P5) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_file_under_a_directory_without_search_permission_answers_unreadable` |
| U16 | Probing a script that would create a marker file if run leaves no marker | FR-003 (P6) | example | DONE | `crates/micold-core/tests/script_path_check.rs::probing_a_script_never_runs_it` |
| U17 | `cfg(unix)`: a symlink to a regular file answers `File` | FR-001 (P7) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_symlink_to_a_regular_file_answers_file` |
| U18 | `cfg(unix)`: a dangling symlink answers `Missing` | FR-001 (P8) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_dangling_symlink_answers_missing` |

### `crates/micold-core/src/script_path_check.rs`: `check_bounded`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U19 | A probe that never answers gives `Some(Unchecked)`, and the call returns within the bound plus slack (100 ms bound, under 1 s) | FR-006, Edge Cases hang (C7) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_probe_that_never_answers_is_reported_as_unchecked_within_the_bound` |
| U20 | A probe that answers at once gives the same result as `classify`: the side of the bound where an answer arrives | FR-006 (C8) | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_probe_that_answers_at_once_gives_the_same_result_as_classify` |
| U21 | A blank path gives `None` without calling the probe | FR-011 | example | DONE | `crates/micold-core/tests/script_path_check.rs::a_bounded_check_of_a_blank_path_is_none_without_a_probe` |
| U22 | `SCRIPT_PATH_CHECK_BOUND` is 2 s | FR-006, SC-004 | example | DONE | `crates/micold-core/tests/script_path_check.rs::the_bound_is_two_seconds` |

### Capability guards: `crates/micold-client/tests/inventory/mod.rs`, `no_concrete_implementations.rs`, `service_capability_fakes.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U23 | `ScriptPathProbe` is a registered port: it has a core fake that a test exercises, and `StdScriptPathProbe` is named only at its definition and in `Capabilities::real()` | FR-006, FR-003 (the test seam every trigger test relies on); plan R9 | example | DONE | `crates/micold-client/tests/no_concrete_implementations.rs::each_implementation_is_chosen_in_exactly_one_place`, `service_capability_fakes.rs` (existing guards, port registered by T005) |

### `crates/micold-client/src/features/settings.rs`: reducer

Tests in `crates/micold-client/tests/features_settings.rs`, through `update` and its returned
`Outcome`s.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U24 | `ScriptPathCheckStarted` raises `script_check_seq` by one and sets `Pending { seq, last }`, with `last` the previous `Done` answer (or `None`) | FR-009 (S1) | example | DONE | `crates/micold-client/tests/features_settings.rs::script_path_check::starting_a_check_raises_the_sequence_and_keeps_the_previous_answer_showing` |
| U25 | `Saved` origin sets `script_check_save_seq = Some(seq)`, and `Opened` leaves it unchanged | FR-004 (S1) | example | DONE | `crates/micold-client/tests/features_settings.rs::script_path_check::a_save_marks_its_check_as_one_to_report_and_an_open_does_not` |
| U26 | A `ScriptPathChecked` whose `seq` matches sets `Done(c)` | FR-009 (S2) | example | DONE | `crates/micold-client/tests/features_settings.rs::script_path_check::the_current_checks_answer_is_shown` |
| U27 | A matching `ScriptPathChecked` with `result: None` sets `Idle` | FR-011 (S3) | example | DONE | `crates/micold-client/tests/features_settings.rs::script_path_check::a_blank_paths_answer_leaves_nothing_to_show` |
| U28 | A `ScriptPathChecked` with an older `seq` leaves `script_check` unchanged | FR-009, Edge Cases multi-window (S4) | example | DONE | `crates/micold-client/tests/features_settings.rs::script_path_check::an_older_checks_answer_is_dropped` |
| U29 | `Saved` + `NotFound { tilde: false }` returns exactly one Info `NotificationRaised` reading `The environment-include script was not found: <path>` | FR-004 (S5) | example | PENDING | |
| U30 | `Saved` + `NotFound { tilde: true }` appends ` (~ is not expanded; use a full path)` | FR-004, Edge Cases `~` (S5) | example | PENDING | |
| U31 | `Saved` + `NotReadable` reads `The environment-include script is not a readable file: <path>` | FR-004 (S5) | example | PENDING | |
| U32 | `Saved` + `Present`, `Relative`, `Unchecked` or `result: None` returns no notification | FR-004 (S6) | example | PENDING | |
| U33 | `Opened` + `NotFound` returns no notification | FR-007 (S7) | example | PENDING | |
| U34 | A `Saved` result whose display was superseded by a newer `Opened` check still notifies | FR-004 (S5) | example | PENDING | |
| U35 | An older save's result, after a newer save started, does not notify | FR-004 (S5) | example | PENDING | |
| U36 | The same `Saved` result delivered twice notifies once | FR-004 (S5) | example | PENDING | |
| U37 | Neither message changes `settings_draft` or any setting | FR-008, FR-010 (S8) | example | DONE | `crates/micold-client/tests/features_settings.rs::script_path_check::neither_check_message_touches_the_draft_or_a_setting` |

### `crates/micold-client/src/features/settings.rs`: `script_path_notice` (feature off, and shared rows)

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U38 | `Idle` passes 011's lines through unchanged: `NonZeroExit` → `Exited with an error` caution plus its diagnostic note; `TimedOut` → `Timed out` plus diagnostic; `Success` or `Disabled` → nothing; `MissingScript` → `Script not found` | FR-005 (N1); 011 FR-013 | example | PENDING | |
| U39 | `Pending { last: Some(c) }` gives the same lines as `Done(c)`, and `Pending { last: None }` is N1 | FR-009 (N1) | example | PENDING | |
| U40 | Off + `NotFound { tilde: false }` → `Caution("Script not found: P")`, `Note(OFF)` | FR-002 (N2) | example | PENDING | |
| U41 | Off + `NotFound { tilde: true }` → the not-found caution, `Note(TILDE)`, `Note(OFF)` | FR-002, Edge Cases `~` (N5) | example | PENDING | |
| U42 | Off + `NotReadable` → `Caution("Not a readable file: P")`, `Note(OFF)` | FR-002 (N6) | example | PENDING | |
| U43 | `Relative` → `Note(REL)`, then 011's lines | Edge Cases relative (N8) | example | PENDING | |
| U44 | `Unchecked` → `Caution("Couldn't check the script path: P")`, `Note(HUNG)`, then 011's lines | FR-006 (N9) | example | PENDING | |
| U45 | Off + `Present` → no lines | SC-002 (N11) | example | PENDING | |
| U63 | M1–M2 interim: with `enabled` on, the lines are exactly 011(last) for every check state, so the on-state page is today's until M3. Throughout the interim, 011(last) keeps 011's wording (`Script not found`, no path), also after N8/N9's lines | FR-005 (no regression between merges); contracts/settings-indication.md §2 "Interim, M1–M2" | example | PENDING | superseded in M3 (T024): mark `DROPPED` then |

### `crates/micold-client/src/features/settings.rs`: `script_path_notice` (feature on)

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U46 | On + `NotFound` + `MissingScript` → exactly `Caution("Script not found: P")`, `Note(ON)`, and 011's line is not repeated | FR-005 (N3) | example | PENDING | |
| U47 | On + `NotFound` + `NonZeroExit` → the not-found caution, `Note(ON)`, then 011's category caution and diagnostic | FR-005 (N4) | example | PENDING | |
| U48 | On + `NotFound { tilde: true }` → the caution, `Note(TILDE)`, `Note(ON)` | FR-002 (N5) | example | PENDING | |
| U49 | On + `NotReadable` + `NonZeroExit` → `Caution("Not a readable file: P")`, `Note(ON)`, then 011's non-zero-exit caution and diagnostic | FR-002, Edge Cases directory (N7) | example | PENDING | |
| U50 | On + `Present` + `MissingScript` → `Caution("The last attempt could not find the script")`, `Note("P exists now. Save Settings or restart a session to source it.")` | FR-014 (N10) | example | PENDING | |
| U51 | On + `Present` + `Success` → no lines: the other side of U50 | FR-014, SC-002 (N11) | example | PENDING | |
| U52 | `Relative` + `MissingScript` → `Note(REL)`, then `Caution("Script not found: P")` (011's line, path-aware) | FR-005 (N8) | example | PENDING | |
| U53 | For every state, the first caution is the same with `enabled` on and off | SC-003 | example | PENDING | |
| U54 | No combination of state, `enabled` and outcome yields "Script not found" twice | FR-005 | example | PENDING | |

### `crates/micold-client/src/shell/{env_include,persist,daemon_sync}.rs`: triggers

Tests in `crates/micold-client/src/main_tests.rs`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U55 | `on_settings_opened` leaves the draft seeded and `script_check` `Pending`, and its job carries the stored path and stored enabled flag with origin `Opened` | FR-006, FR-009 (T1); research R8 | example | PENDING | |
| U56 | `apply_save` prepares a `Saved` job for the saved path, including when the path did not change | FR-004, FR-009 (T2) | example | PENDING | |
| U57 | A terminal restart (`TerminalRestartRequested`) makes no probe call | FR-006, SC-004 | example | PENDING | |
| U58 | After a save with a missing path, the written `Settings` holds the enabled flag, path and timeout as drafted, and nothing else is added for environment-include | FR-010, SC-005 | example | PENDING | |
| U59 | `DaemonMsg::SettingsChanged` with a new path while `settings_draft` is `Some` prepares an `Opened` job for the new path | FR-009, Edge Cases multi-window (T3) | example | PENDING | |
| U60 | `DaemonMsg::SettingsChanged` while Settings is closed prepares no job: the other side of U59 | FR-006 (T3) | example | PENDING | |
| U61 | `on_settings_opened` makes no env-include resolver call (`FakeEnvIncludeResolver::calls()` empty) | FR-014 | example | PENDING | |
| U62 | Across open, check and save from a missing path, no environment-include setting changes other than what the user drafted | FR-008 | example | PENDING | |

## Invariants and edge cases still to place

None. Every Edge Case in spec.md has a line above. "Several sessions and windows" is U28 and U59. U59–U60 trace the Edge Case directly: FR-009
names Settings being shown and saves, and the Edge Case adds that every window showing Settings
agrees on the same stored path.
"The default path is missing" is not a separate behaviour: the check never special-cases the
default, and U7–U10 cover any absolute path.

## Out of scope

- Rendering (the order of cautions and notes on screen, theming): GUI glue under Constitution I's
  exception, validated by quickstart §B (T021, T042, T027, T029, T031), not by a cargo test.
- The spawn_blocking `JoinError` → `Unchecked` mapping in `start_script_path_check`: glue. The job
  it wraps is tested (U55–U60), and a panic in the probe is not a spec behaviour.
- Windows-specific unreadable files (ACL denial): no portable way to create one in a test. U13
  (directory) covers "not a readable file" on every OS.
- The daemon's per-directory resolution failure (#454): spec Out of Scope.
- Changes to 011's sourcing, cache or timeout: spec Out of Scope.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`, matches CI)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Coverage: none (no cargo-llvm-cov, no cargo-tarpaulin)
- Mutation: none (no cargo-mutants)

Tests inside `crates/micold-client/src/main_tests.rs` are a `#[cfg(test)]` module of the client
binary. Run one with `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide {name}`.
