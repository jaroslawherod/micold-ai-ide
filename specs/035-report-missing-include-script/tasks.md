---

description: "Task list for feature 035: report a missing environment-include script"
---

# Tasks: Report a Missing Environment-Include Script

**Input**: Design documents from `/specs/035-report-missing-include-script/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/script-path-check.md,
contracts/settings-indication.md, quickstart.md

**Tests**: MANDATORY (Constitution Principle I). Each test task is written first and observed
failing for the stated reason before the implementation task it covers.

**Organization**: grouped by user story. Contract row IDs (C*, P*, S*, N*, T*) refer to
`contracts/script-path-check.md` and `contracts/settings-indication.md`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1, US2 or US3 from spec.md
- **[A#] / [U#]**: the behaviour ids from `tdd/test-list.md` that the task covers. `/speckit.tdd.run` ticks a task only when it can read these ids. They are not story labels.

## Path Conventions

Cargo workspace: `crates/micold-core/` (render-free core), `crates/micold-client/` (iced client:
`src/features/` render-free reducers, `src/shell/` I/O glue, `src/ui/` rendering), `docs/user-guide/`.
Run tests with `mise run test-core` (core) and `mise run gate` (everything, in CI's order).

---

## Phase 1: Setup

**Purpose**: the new core module exists and is compiled, with no behaviour yet.

- [X] T001 Create `crates/micold-core/src/script_path_check.rs` with a module doc comment (spec 035, research R1–R3) and the type declarations from data-model.md, with derives `Debug, Clone, PartialEq, Eq`: `ScriptPathState { Present, NotFound { tilde: bool }, NotReadable, Relative, Unchecked }`, `ProbeAnswer { File, NotAFile, Missing, Unreadable }`, `CheckedScriptPath { path: String, enabled: bool, state: ScriptPathState }`, trait `ScriptPathProbe { fn probe(&self, path: &Path) -> ProbeAnswer; }`, and `pub const SCRIPT_PATH_CHECK_BOUND: Duration = Duration::from_secs(2);`. Add `pub mod script_path_check;` to `crates/micold-core/src/lib.rs`, and create an empty `crates/micold-core/tests/script_path_check.rs`.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the core check, the capability and the reducer state that every story renders from.

**⚠️ CRITICAL**: no user-story work starts until this phase is complete.

### Tests (write first, observe red)

- [X] T002 [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] [U9] [U10] Write `classify` tests for contract rows C1–C6 in `crates/micold-core/tests/script_path_check.rs`, using `FakeScriptPathProbe::answering(..)`. Assert both the result and `calls()`: no call for C1 (blank `""`, `"   "` → `None`), C2 (`"~"`, `"~/env.sh"`, `"~\\env.ps1"` → `Some(NotFound { tilde: true })`, "a string test, asserted on every OS") or C3 (`"env.sh"`, `"./env.sh"`, `"scripts/env.sh"` → `Some(Relative)`). `"~env.sh"` (no separator after `~`) is `Relative`, not tilde (U5). One call, with the path unchanged, for C4–C6 (absolute: `File` → `Present`, `Missing` → `NotFound { tilde: false }`, `NotAFile`/`Unreadable` → `NotReadable`). Build the absolute test path from `std::env::temp_dir()` so it is absolute on every OS.
- [X] T003 [U11] [U12] [U13] [U14] [U15] [U16] [U17] [U18] Write `StdScriptPathProbe` tests for contract rows P1–P7 in `crates/micold-core/tests/script_path_check.rs` using `tempfile`: P1 regular file → `File`; P2 missing → `Missing`; P3 directory → `NotAFile`; P4 `#[cfg(unix)]` mode-000 file → `Unreadable` (skip when the effective uid is 0); P5 `#[cfg(unix)]` file under a mode-000 parent directory → `Unreadable`, restoring the mode before the tempdir drops; P6 a script whose execution would create a marker file → `File`, and the marker does not exist afterwards (FR-003); P7 `#[cfg(unix)]` symlink to a regular file → `File`; `#[cfg(unix)]` a dangling symlink → `Missing` (U18).
- [X] T004 [U19] [U20] [U21] [U22] Write `check_bounded` tests for C7–C8 in `crates/micold-core/tests/script_path_check.rs`: a `FakeScriptPathProbe::blocking()` probe with a 100 ms bound returns `Some(Unchecked)` and returns in under 1 s (C7); an answering probe gives the same result as `classify` (C8); a blank path gives `None` without calling the probe; `SCRIPT_PATH_CHECK_BOUND == Duration::from_secs(2)` (U22).
- [X] T005 [U23] Register the port in the capability guards before it exists in the client: add `"ScriptPathProbe"` to `PORTS` in `crates/micold-client/tests/inventory/mod.rs`, and `"FakeScriptPathProbe"` to the closed `known` fakes list in `crates/micold-client/tests/no_concrete_implementations.rs`. Observe `service_capability_fakes.rs` / `no_concrete_implementations.rs` red until T010 lands (T007 supplies the fake).
- [X] T006 [U24] [U25] [U26] [U27] [U28] [U37] Write reducer tests for contract rows S1–S4 and S8 in `crates/micold-client/tests/features_settings.rs`: `ScriptPathCheckStarted` bumps `script_check_seq` and sets `Pending { seq, last }`, with `last` the previous `Done` answer; `Saved` also sets `script_check_save_seq`; a matching `ScriptPathChecked` sets `Done(c)` (or `Idle` for `result: None`); a non-matching `seq` leaves `script_check` unchanged; neither message changes `settings_draft` or any setting.

### Implementation

- [X] T007 [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] [U9] [U10] Implement `FakeScriptPathProbe` (`answering(ProbeAnswer)`, `blocking()` which never answers until dropped, `calls() -> Vec<PathBuf>`; `Send + Sync`) and `classify(path: &str, probe: &dyn ScriptPathProbe) -> Option<ScriptPathState>` in `crates/micold-core/src/script_path_check.rs`, in the order of research R2: blank → `None`; `== "~"` or starts with `~/` or `~\` → `NotFound { tilde: true }`; `!Path::is_absolute()` → `Relative`; otherwise probe (makes T002 green).
- [X] T008 [U11] [U12] [U13] [U14] [U15] [U16] [U17] [U18] Implement `StdScriptPathProbe` in `crates/micold-core/src/script_path_check.rs` per research R1: `std::fs::metadata` (follows symlinks), where `ErrorKind::NotFound` → `Missing`, any other error → `Unreadable`, and not `is_file()` → `NotAFile`; then `File::open`, where an error → `Unreadable`, otherwise `File`. It never reads or executes the file (makes T003 green).
- [X] T009 [U19] [U20] [U21] [U22] Implement `check_bounded(probe: Arc<dyn ScriptPathProbe + Send + Sync>, path: String, bound: Duration) -> Option<ScriptPathState>` in `crates/micold-core/src/script_path_check.rs` per research R3: `classify` on a detached `std::thread` and `mpsc::Receiver::recv_timeout(bound)`, with a timeout → `Some(Unchecked)` (makes T004 green).
- [X] T010 [U23] Add the capability to `crates/micold-client/src/shell/capabilities.rs`: the field `script_path_probe: Arc<dyn ScriptPathProbe + Send + Sync>`, constructed as `StdScriptPathProbe` in `Capabilities::real()` only; the accessor `script_path_probe(&self) -> Arc<dyn ScriptPathProbe + Send + Sync>` (owned, because the consumer is a blocking task); and `#[cfg(test)] pub(crate) fn with_script_path_probe(self, probe) -> Self`. Make `base_app()` in `crates/micold-client/src/main_tests.rs` hand every test a `FakeScriptPathProbe::answering(ProbeAnswer::File)` (makes T005 green).
- [X] T011 [U24] [U25] [U26] [U27] [U28] [U37] Add `CheckOrigin { Opened, Saved }`, `ScriptCheck { Idle, Pending { seq: u64, last: Option<CheckedScriptPath> }, Done(CheckedScriptPath) }` (default `Idle`), the `State` fields `script_check`, `script_check_seq: u64` and `script_check_save_seq: Option<u64>`, and the `Msg` variants `ScriptPathCheckStarted { origin }` and `ScriptPathChecked { seq, origin, result: Option<CheckedScriptPath> }` to `crates/micold-client/src/features/settings.rs`. Implement S1–S4 in `update`, with doc comments citing spec 035 (makes T006 green). The existing catch-all in `src/shell/settings.rs` routes both variants, so no routing change is needed.

**Checkpoint**: `mise run test-core` passes the new core tests, and the reducer holds check results. Nothing is visible yet.

---

## Phase 3: User Story 1 — See that the stored script path does not exist while the feature is off (Priority: P1) 🎯 MVP

**Goal**: with the feature off, Settings reports a stored path that names no readable file (scenarios 1–4, M1), and a save that leaves such a path posts a notification (scenario 5, M2).

**Independent Test**: store `env_include_enabled: false` and `/tmp/does-not-exist.sh`, then open Settings → Environment: `Script not found: /tmp/does-not-exist.sh` plus the OFF note. With an existing file or a blank path there is no indication (quickstart §B B1–B3).

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these first and observe each fail for the stated reason before its implementation task.

- [X] T032 [US1] [A1] [A2] [A3] [A4] Write the outer-loop acceptance tests A1–A4 in `crates/micold-client/src/main_tests.rs` at the entry point described in `tdd/test-list.md` (real shell handler → the check job run synchronously with a `FakeScriptPathProbe` → its message through `app.core.update` → `script_path_notice` and the notification queue). A1: off + missing absolute path → first line `Script not found: <path>`, then OFF. A2: off + existing file → no lines. A3: off + blank → no lines and no probe call. A4: off + missing, a session launch (`view_and_start`) → no probe call and no resolver call. They stay red until T017–T019 land. T033 confirms them green.
- [X] T012 [P] [US1] [U38] [U39] [U40] [U41] [U42] [U43] [U44] [U45] [U63] Write `script_path_notice` tests for the feature-off rows N1, N2, N5 (off), N6, N8, N9 and N11, plus the interim (U63, M1–M2): with `enabled` on, the lines are exactly 011(last) for every check state (contracts/settings-indication.md §2, "Interim"; 011(last) keeps 011's wording, `Script not found` with no path, also after N8/N9) in `crates/micold-client/tests/features_settings.rs`, with the exact wording keys of contracts/settings-indication.md §2 (OFF, TILDE, REL, HUNG). Also assert the invariants: every not-found or not-readable row names `P` and contains the OFF note; no row carries an action; `Pending { last: Some(c) }` renders as `Done(c)`; and 011's lines (`NonZeroExit`/`TimedOut` caution plus diagnostic note) still appear for N1 with the outcomes that produce them.
- [X] T014 [US1] [U55] [U57] Write shell tests in `crates/micold-client/src/main_tests.rs` with a `FakeScriptPathProbe` via `with_script_path_probe`: T1, where `on_settings_opened` leaves the draft seeded and `script_check` `Pending` (the check does not block the open, FR-006), and running the prepared job gives the probe exactly the stored path, with origin `Opened` and the stored enabled flag; and a terminal restart (`TerminalRestartRequested`) makes no probe call (SC-004; the launch side is A4).

### Implementation for User Story 1

- [X] T015 [US1] [U38] [U39] [U40] [U41] [U42] [U43] [U44] [U45] [U63] Implement `NoticeLine { Caution(String), Note(String) }`, `script_path_notice(check: &ScriptCheck, last: &EnvIncludeOutcome) -> Vec<NoticeLine>` in `crates/micold-client/src/features/settings.rs`, beside `missing_cli_notice`, covering rows N1, N2, N5, N6, N8, N9 and N11, the 011(last) passthrough, and the interim gate (feature on → exactly 011(last), in 011's wording). Move the logic of `ui/settings/environment.rs`'s private `failure()` in here (makes T012 green).
- [X] T017 [US1] [U55] Implement the check job in `crates/micold-client/src/shell/env_include.rs` per contracts/settings-indication.md §3. `prepare_script_path_check(app: &mut App, origin: CheckOrigin) -> ScriptPathCheckJob` dispatches `ScriptPathCheckStarted`, reads `script_check_seq`, and captures `app.env_include_script_path`, `app.env_include_enabled` and `app.caps.script_path_probe()`. `ScriptPathCheckJob::run(self) -> Message` runs `check_bounded(probe, path, SCRIPT_PATH_CHECK_BOUND)` synchronously and returns `Message::Settings(Msg::ScriptPathChecked { seq, origin, result })`; tests call it. `run_script_path_check(job) -> Task<Message>` wraps the job in `Task::perform` over `tokio::task::spawn_blocking(move || job.run())`, with a `JoinError` → `ScriptPathChecked { seq, origin, result: Some(CheckedScriptPath { path, enabled, state: Unchecked }) }`.
- [X] T018 [US1] [U55] [A1] [A2] [A3] Run the check from `on_settings_opened` (origin `Opened`, prepared in `open_settings` and returned as its task through `run_script_path_check`) in `crates/micold-client/src/shell/persist.rs` (makes T014 green).
- [X] T019 [US1] Render the notice: thread `&app.core.settings.script_check` from `render` in `crates/micold-client/src/main.rs` through `crates/micold-client/src/ui/mod.rs` and `crates/micold-client/src/ui/settings_view.rs` to `environment::view`. In `crates/micold-client/src/ui/settings/environment.rs`, delete `failure()` and push `script_path_notice(check, outcome)` lines after the timeout field: `Caution` through `ui::settings::caution`, `Note` through `ui::settings::note` (Principle VIII; GUI-glue exception, validated by T021).
- [X] T020 [US1] Update `docs/user-guide/settings.md` (FR-013). Replace the "Script path" bullet's "whether it resolves to a usable script is only discovered when it's actually used" with the check. Add a subsection under Environment on the missing-path indication with the feature off: the not-found and not-readable wording, `~` not expanded, a relative path not checked, "could not be checked" after 2 s, and what to do (edit the path to a full path of a readable file, or clear it).
- [X] T033 [US1] [A1] [A2] [A3] [A4] Confirm the outer-loop tests A1–A4 from T032 are green with the full suite (`mise run gate`), and record the result in `specs/035-report-missing-include-script/tdd/cycle-log.md`. A2–A4 and U57 assert an absence and may pass once they compile. For each one that does, record a deliberate mutant that turns it red (for example, start the check from the launch or restart path, or render a line for a present file), then revert it.
- [ ] T021 [US1] Run quickstart §B steps B1, B2, B3, B7, B8 and B12 with the `visual-pass` skill. Save the evidence under `specs/035-report-missing-include-script/visual-pass/`.

**Checkpoint (M1)**: US1 acceptance scenarios 1–4 hold with the feature off. Scenario 4 (launch unaffected) is covered by A4 (T032) and the terminal-restart test U57 (T014).

### Tests for User Story 1, scenario 5: the save-time notification (MANDATORY — Constitution Principle I) ⚠️

> Write these first and observe each fail for the stated reason before its implementation task.

- [ ] T039 [US1] [A5] Write the outer-loop acceptance test A5 in `crates/micold-client/src/main_tests.rs` at the same entry point as T032: a save with a missing path, the feature off and then on, writes the settings and posts exactly one Info notification per save naming the path. It stays red until T016 and T038 land.
- [ ] T013 [US1] [U29] [U30] [U31] [U32] [U33] [U34] [U35] [U36] Write reducer notification tests S5–S7 in `crates/micold-client/tests/features_settings.rs`, asserting on the `Outcome`s `update` returns. `Saved` + `NotFound { tilde: false }` → exactly one `Outcome::NotificationRaised` (Info) reading `The environment-include script was not found: <path>`. `tilde: true` adds ` (~ is not expanded; use a full path)`. `NotReadable` reads `The environment-include script is not a readable file: <path>`. `Present`, `Relative`, `Unchecked` or `result: None` → no notification. `Opened` origin → never (FR-007). A `Saved` result whose display was superseded (S4) still notifies. An older save's result, after a newer save started, does not notify. A save notifies once even if its result is delivered twice.
- [ ] T037 [US1] [U56] [U58] Write shell tests in `crates/micold-client/src/main_tests.rs` with a `FakeScriptPathProbe`: T2, where `apply_save` prepares a `Saved` job for the saved path even when the path is unchanged (FR-004, "every save"); and after a save with a missing path, the settings written hold only the enabled flag, path and timeout for environment-include (FR-010, SC-005).

### Implementation for User Story 1, scenario 5

- [ ] T016 [US1] [U29] [U30] [U31] [U32] [U33] [U34] [U35] [U36] Implement `save_notice(&CheckedScriptPath) -> String` and the notification step S5–S7 in `update` in `crates/micold-client/src/features/settings.rs`, returning `crate::features::notifications::info(save_notice(&c))`, never calling `State::notify_info` (makes T013 green).
- [ ] T038 [US1] [U56] [A5] Start a check (`run_script_path_check(prepare_script_path_check(app, CheckOrigin::Saved))`) from `apply_save` (origin `Saved`, after the write, 011's `refresh_env_include` and `Msg::Saved`, batched with the survival task through `Task::batch`) in `crates/micold-client/src/shell/persist.rs` (makes T037 green).
- [ ] T041 [US1] Update `docs/user-guide/settings.md` (FR-013): the save-time notification. Saving always goes through, and a save that leaves a missing or unreadable path posts a notice naming it.
- [ ] T040 [US1] [A5] Confirm A5 from T039 is green with the full suite (`mise run gate`), and record the result in `specs/035-report-missing-include-script/tdd/cycle-log.md`.
- [ ] T042 [US1] Run quickstart §B step B11 with the `visual-pass` skill. Save the evidence under `specs/035-report-missing-include-script/visual-pass/`.

**Checkpoint (M2)**: US1 acceptance scenarios 1–5 hold.

---

## Phase 4: User Story 2 — The same report whether the feature is on or off (Priority: P2)

**Goal**: with the feature on, the page says the same thing about the path as with it off, merges it with 011's "Script not found" instead of saying it twice, and explains FR-014's "the file exists now" state. Another window's save refreshes an open page.

**Independent Test**: store the feature on with a missing path and open Settings: one `Script not found: <path>` caution plus the ON note. Switch it off and save, then reopen: the same caution plus the OFF note. Create the file and reopen with the feature on: FR-014's caution and note (quickstart §B B4–B6).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> Write these first and observe each fail for the stated reason before its implementation task.

- [ ] T034 [US2] [A6] [A7] [A8] Write the outer-loop acceptance tests A6–A8 in `crates/micold-client/src/main_tests.rs` at the same entry point as T032. A6: on + missing + last outcome `MissingScript` → exactly one `Script not found: <path>` caution, then ON. A7: from A6, untick, save, reopen → the same caution, then OFF. A8: create the file and reopen: on (last outcome still `MissingScript`) → FR-014's caution and note; off → no lines.
- [ ] T022 [P] [US2] [U46] [U47] [U48] [U49] [U50] [U51] [U52] [U53] [U54] Write `script_path_notice` tests for the feature-on rows N3, N4, N5 (on), N7 and N10 in `crates/micold-client/tests/features_settings.rs`. Also assert: 011's `MissingScript` line reads `Script not found: <path>` when the check's path is known and `Script not found` otherwise; no row shows "Script not found" twice (FR-005); the same `P` gives the same first caution whether `enabled` is on or off (SC-003); N7 with a `NonZeroExit` outcome shows the path caution, the ON note, then 011's category caution and diagnostic (Edge Cases: both notes); N10 names `P` and says to save Settings or restart a session (FR-014).
- [ ] T023 [P] [US2] [U59] [U60] [U61] Write shell tests for T3 in `crates/micold-client/src/main_tests.rs`: a `DaemonMsg::SettingsChanged` carrying a new path while `settings_draft` is `Some` starts an `Opened` check for the new path, and `script_check` becomes `Pending`; with Settings closed it starts none. Showing Settings (T1) never calls the env-include resolver (FR-014 "MUST NOT re-source"), asserted with `FakeEnvIncludeResolver::calls()` staying empty across `on_settings_opened`.

### Implementation for User Story 2

- [ ] T024 [US2] [U46] [U47] [U48] [U49] [U50] [U51] [U52] [U53] [U54] [A6] [A7] [A8] Extend `script_path_notice` in `crates/micold-client/src/features/settings.rs`: remove the interim gate (M1–M2) (then delete U63's test and mark U63 `DROPPED`, superseded by U46–U54, in `specs/035-report-missing-include-script/tdd/test-list.md`), and add rows N3, N4, N5 (on), N7 and N10, the ON wording key, and the path-aware 011 `MissingScript` caution (makes T022 green).
- [ ] T025 [US2] [U59] [U60] Start a check (`run_script_path_check(prepare_script_path_check(app, CheckOrigin::Opened))`) in the `DaemonMsg::SettingsChanged` arm of `crates/micold-client/src/shell/daemon_sync.rs`, after the `app.env_include_*` fields are updated and only while `app.core.settings.settings_draft.is_some()` (spec Edge Cases, "Several sessions and several open windows": every window showing Settings shows the same result for the same stored path), and fold its task into the arm's `follow_up` (makes T023 green).
- [ ] T026 [US2] Update `docs/user-guide/settings.md` "If the script fails" (FR-013): **Script not found** now names the path and reads the same with the feature on or off, and the note when the file exists now but the last attempt did not find it (save Settings or restart the session to source it).
- [ ] T035 [US2] [A6] [A7] [A8] Confirm the outer-loop tests A6–A8 from T034 are green with the full suite (`mise run gate`), and record the result in `specs/035-report-missing-include-script/tdd/cycle-log.md`.
- [ ] T027 [US2] Run quickstart §B steps B4, B5 and B6 with the `visual-pass` skill. Save the evidence under `specs/035-report-missing-include-script/visual-pass/`.

**Checkpoint**: US2 acceptance scenarios 1–3 hold, and US1 still passes.

---

## Phase 5: User Story 3 — Fix a stored path that has gone missing (Priority: P3)

**Goal**: from the indication, the user reaches a configuration with no missing path by editing or clearing it in Settings, with no automatic recovery (FR-008).

**Independent Test**: store a missing path, then in Settings type an existing path (or clear it) and save, then reopen: no indication and no notification (quickstart §B B9, B10).

No production code: the behaviour is FR-009's check on open and after every save, shipped in M1 (T018) and M2 (T038). These are acceptance tests for that behaviour and the FR-008 guard, and the ledger records why they are not expected to go red.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T028 [US3] [A9] [A10] [U62] Write shell tests in `crates/micold-client/src/main_tests.rs` with a `FakeScriptPathProbe`, starting from a stored missing path whose check is `Done(NotFound)`. US3 scenario 1: saving an existing absolute path gives a `Saved` check that resolves to `Done(Present)` and posts no notification. US3 scenario 2: saving a blank path gives `script_check` `Idle` and posts no notification (FR-011). FR-008: across open, check and save, no environment-include setting is changed except by the user's own draft (enabled flag, path and timeout equal the saved draft).
- [ ] T036 [US3] [A9] [A10] Confirm A9–A10 from T028 are green with the full suite (`mise run gate`). If a test passes on arrival, record in `specs/035-report-missing-include-script/tdd/cycle-log.md` a deliberate mutant that turns it red (for example, remove the `Saved` trigger T038 added to `apply_save`), then revert it.
- [ ] T029 [US3] Run quickstart §B steps B9 and B10 with the `visual-pass` skill. Save the evidence under `specs/035-report-missing-include-script/visual-pass/`.

**Checkpoint**: all three stories hold.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T030 [P] Add `ScriptPathProbe` to the capability examples in `docs/development/architecture.md` (the "Declare the trait in the core" list: `script_path_check.rs` holds `ScriptPathProbe`).
- [ ] T031 Run quickstart Part A (`mise run test-core`, `mise run gate`) and the whole of Part B (B1–B11; B12 was M1's interim, light and dark themes, no line overflowing the column) with the `visual-pass` skill, and record the result in `specs/035-report-missing-include-script/visual-pass/README.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: after Setup. Blocks every story.
- **US1 (Phase 3)**: after Foundational.
- **US2 (Phase 4)**: after US1, including scenario 5 (it extends `script_path_notice`, reuses `prepare_script_path_check` and `run_script_path_check`, and A7 saves).
- **US3 (Phase 5)**: after US1 (it tests US1's save trigger).
- **Polish (Phase 6)**: after all stories.

### Within Each Phase

- T002–T004 → T007–T009 (core tests before core code, all in one test file, so sequential).
- T005 → T010. T006 → T011.
- T032 first in US1. T012 → T015. T014 → T017/T018. T019 after T015. T033 after T018/T019. T021 after T033.
- US1 scenario 5: T039 first. T013 → T016. T037 → T038. T040 after T016/T038. T042 after T040.
- T034 first in US2. T022 → T024. T023 → T025. T035 after T024/T025.
- T028 → T036.

### Parallel Opportunities

- T012 [P] with T014 (different files).
- T022 [P] with T023 (different files).
- T030 [P] with T031's Part A.

## Parallel Example: User Story 1

```text
Task: "T012 notice tests (feature off) in crates/micold-client/tests/features_settings.rs"
Task: "T014 shell trigger tests in crates/micold-client/src/main_tests.rs"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 + Phase 2 (core check, capability, reducer state).
2. Phase 3 (US1): the reported case, feature off, is fixed and documented.
3. Stop and validate with quickstart §B B1–B3, B7, B8, B12.

### Incremental Delivery

1. M1: Setup + Foundational + US1 scenarios 1–4 → the off-state report ships.
2. M2: US1 scenario 5 → the save-time notification ships.
3. M3: US2 → on and off read the same, and FR-014 ships.
4. M4: US3 + Polish → recovery tests, architecture doc, the full quickstart pass.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Report a missing script path while the feature is off 🎯 MVP

- **Tasks**: T001–T012, T014, T015, T017–T021, T032–T033
- **Deliverable**: with environment-include off and a stored path that names no readable file, Settings → Environment shows `Script not found: <path>` and says the feature is off. An existing file or a blank path shows nothing. The user guide describes it. With the feature on, the page stays as it is today (011's note alone, in 011's wording) until M3, the interim of U63.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001 (off), FR-002 (off), FR-003, FR-006 (off), FR-009 (on open, off), FR-010, FR-011, FR-012 (for the M1 rows), FR-013 (off state); SC-001, SC-002 (off), SC-004
- **Verify**: `mise run test-core` (`crates/micold-core/tests/script_path_check.rs`); `mise run gate` (`tests/features_settings.rs` rows N1/N2/N5/N6/N8/N9/N11, U63 and S1–S4/S8, `src/main_tests.rs` A1–A4, T1 and no-probe-on-restart); quickstart §B B1–B3, B7, B8, B12 via `visual-pass`
- **Depends on**: —

### M2 — Notify when a save leaves a missing script path

- **Tasks**: T013, T016, T037–T042
- **Deliverable**: saving Settings with a stored path that names no readable file writes the settings and posts one notification naming the path, with the feature on or off. The user guide describes it.
- **Satisfies**: US1 acceptance scenario 5; FR-004, FR-007, FR-009 (after every save), FR-010 (save side), FR-013 (save-time notice); SC-005
- **Verify**: `mise run gate` (`tests/features_settings.rs` S5–S7, `src/main_tests.rs` A5, T2 and the written-settings test); quickstart §B B11 via `visual-pass`
- **Depends on**: M1

### M3 — The same report with the feature on or off

- **Tasks**: T022–T027, T034–T035
- **Deliverable**: with environment-include on and a missing path, Settings shows one `Script not found: <path>` caution (merged with 011's note) and says the feature is on. Switching the feature off keeps the same caution. With the file created since the last attempt, the page says it exists now and that saving or restarting a session will source it. Another window's save refreshes an open Settings page.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-001 (on), FR-002 (on), FR-005, FR-006 (on), FR-009 (on), FR-012 (all rows), FR-013 (on state), FR-014; SC-002 (on), SC-003
- **Verify**: `mise run gate` (`tests/features_settings.rs` rows N3/N4/N5/N7/N10 and the no-duplicate invariant, `src/main_tests.rs` A6–A8, T3 and no-resolver-call-on-open); quickstart §B B4–B6 via `visual-pass`
- **Depends on**: M1, M2

### M4 — Recovery by editing the path, and polish

- **Tasks**: T028–T031, T036
- **Deliverable**: tests on `main` show that fixing or clearing a missing path in Settings removes the indication without a notification, and that nothing recovers on its own. The architecture doc lists the new capability. The full quickstart §B pass is recorded.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-008; SC-006
- **Verify**: `mise run gate` (`src/main_tests.rs` A9–A10 and U62); `specs/035-report-missing-include-script/visual-pass/README.md` records B1–B11
- **Depends on**: M1, M2, M3

## Notes

- [P] tasks touch different files and have no dependency on an incomplete task.
- Commit after each red → green step (the `tdd.run` hook records it in `tdd/cycle-log.md`).
- #454 (the daemon's per-directory silent failure) is out of scope. No task touches `crates/micold-daemon`.
