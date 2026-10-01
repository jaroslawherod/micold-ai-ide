---

description: "Task list for feature 037: explain why an AI CLI is not offered"
---

# Tasks: Explain Why an AI CLI Is Not Offered

**Input**: Design documents from `/specs/037-explain-hidden-cli/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/availability-answer.md,
contracts/reason-wording.md, quickstart.md

**Tests**: MANDATORY (Constitution Principle I). Each test task is written first and observed
failing for the stated reason before the implementation task it covers.

**Organization**: grouped by user story. Contract row IDs refer to
`contracts/availability-answer.md` (A1–A5, S1–S7, C1–C4) and `contracts/reason-wording.md`
(W1–W7 and the surfaces, written here as **surface U1** to **surface U6**).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1, US2 or US3 from spec.md
- **[A#] / [U#]**: the behaviour ids from `tdd/test-list.md` that the task covers. `/speckit.tdd.run` ticks a task only when it can read these ids. They are not story labels, and they are not the contract's surface rows.

## Path Conventions

Cargo workspace: `crates/micold-core/` (render-free core, shared by the client and the session
service), `crates/micold-daemon/` (the session service), `crates/micold-client/` (iced client:
`src/features/` render-free reducers, `src/shell/` I/O glue, `src/ui/` rendering),
`docs/user-guide/`. Run tests with `mise run test-core` (core) and `mise run gate` (everything, in
CI's order).

---

## Phase 1: Setup

**Purpose**: the new core module exists and is compiled, with no behaviour yet.

- [X] T001 Create `crates/micold-core/src/cli_reason.rs` with a module doc comment (spec 037, research R2 and R4) and the declarations from data-model.md and contracts/reason-wording.md: `SpawnEnv { IncludeOff, NoScriptPath, ScriptNotFound, ScriptFailed, ScriptTimedOut, Applied }` with derives `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize` and "No payload: the script's output is never carried (FR-007)"; `Place<'a> { ThisComputer, Image(&'a str) }`; `AttemptDir<'a> { Home, Dir(&'a Path) }`; `Explanation { reason: String, action: String }`; and `pub const LABEL_ENABLED: &str = "Source a script before each session"`, `LABEL_SCRIPT_PATH: &str = "Script path"`, `LABEL_TIMEOUT: &str = "Timeout"`. Add `pub mod cli_reason;` to `crates/micold-core/src/lib.rs`, and create an empty `crates/micold-core/tests/cli_reason.rs`.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the state is classified, carried by the availability answer, and held by the client with the directory it was asked for. Every story's sentence is read from it.

**⚠️ CRITICAL**: no user-story work starts until this phase is complete.

### Tests (write first, observe red)

- [X] T002 [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] Write `SpawnEnv::classify` tests for every row of research R2's table in `crates/micold-core/tests/cli_reason.rs`: `enabled == false` → `Some(IncludeOff)` whatever the path and attempt; enabled with `""` or `"   "` → `Some(NoScriptPath)`; attempt `MissingScript` → `ScriptNotFound`; `NonZeroExit { .. }` → `ScriptFailed`; `TimedOut { .. }` → `ScriptTimedOut`; `Success` → `Applied`; enabled, path set and attempt `None` or `Disabled` → `None`. Assert `script_applied()` is true only for `Applied`.
- [X] T003 [U9] [U10] [U11] [U12] [U13] [U14] [U15] [U16] [U17] [U18] [U19] [U20] [U21] [U22] [U23] [U24] [U25] Write `explain` and `name_list` tests in `crates/micold-core/tests/cli_reason.rs` for contracts/reason-wording.md W1 and W2: the exact `reason` and `action` strings of each of the seven W2 rows, for `Place::ThisComputer` and `Place::Image("img:tag")`, for one, two and three CLIs ("it"/"them", "was"/"were", "isn't"/"aren't", "its directory"/"their directories"), and for `AttemptDir::Home` ("your home directory") and `Dir(p)`. `explain(&[], ..)` is `None`. `name_list` gives "A", "A and B", "A, B and C" and `None` for an empty slice. Assert the rules over all states × both places: W2a (where `!env.script_applied()` neither string contains `isn't installed`, `not installed`, `isn't in` or `aren't in`, and the action is never only an instruction to install), W2b (every quoted label is one of the three constants), W2c (`{dir}` appears exactly in `ScriptNotFound`, `ScriptFailed`, `ScriptTimedOut` and `Applied` on the host), W2d (the `Applied`/image reason and action joined by a space are today's sentence byte for byte), W2e (no sentence contains `.bashrc`, `.zshrc`, `.profile` or `$PROFILE`), W2f (no sentence contains "see below").
- [X] T004 [P] [U26] [U27] Write the wire tests for contract A1: in `crates/micold-core/tests/schema_hash.rs` the pinned `PROTOCOL_VERSION` is 18 with the new schema hash, and in `crates/micold-core/tests/protocol_roundtrip.rs` `DaemonMsg::AiCliAvailability` round-trips with `env: Some(SpawnEnv::ScriptTimedOut)` and with `env: None`.
- [ ] T005 [P] [U28] [U29] [U30] [U31] [U32] [U33] [U34] [U35] [U36] [U38] [U39] Write the service tests for contract A2 and A3 in `crates/micold-daemon/tests/ai_cli_availability.rs`, with the bash and PowerShell script fixtures the file already has: S1 (off → `Some(IncludeOff)`), S2 (on, blank path → `Some(NoScriptPath)`), S3 (a script path that names no file → `Some(ScriptNotFound)`), S4 (a script that exits 3, and on unix a script path that names a directory → `Some(ScriptFailed)`, FR-001's note on an unreadable path), S5 (a script that sleeps past a 1 s timeout → `Some(ScriptTimedOut)`), S6 (a script that adds a directory holding a fake CLI → `Some(Applied)` and the CLI is in `available`; a script that succeeds and adds nothing → `Some(Applied)` and the CLI is absent), S7 (no `cwd` and no home → `SpawnEnv::classify(enabled, path, None)`; S7's other half, a failed resolving task, takes the same expression and has no test seam). In every row `available` equals what the test asserted before this feature. `a_second_answer_for_a_directory_does_not_run_the_script_again` also reads `env` from both answers and still counts one run (FR-014, SC-006). After `set_env_include` turns the feature on, the next answer for the same directory carries the new state with no restart (FR-013).
- [ ] T006 [P] [U40] [U41] [U42] [U43] [U44] [U45] Write the client tests for contract A4 in `crates/micold-client/tests/directory_availability.rs`: `AvailabilityAnswers::answered` stamps `asked_for` with the key the request named (`Home` for a request without a directory, `Dir(path)` otherwise) (C2); `for_dir` for a row without its own answer returns the home answer, whose `asked_for` is `Home`, and returns the row's own with `asked_for == Dir(path)` once it is filed (C3, FR-004a); a newer answer for a key replaces `env` together with `available`, and an older one is dropped (FR-012, FR-013). In `crates/micold-client/src/main_tests.rs`: a `DaemonMsg::AiCliAvailability { env: Some(SpawnEnv::IncludeOff), .. }` through the shell arm files a `CliAvailability` whose `env` is that state and whose `source` is the boot plan's (C1).

### Implementation

- [X] T007 [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] Implement `SpawnEnv::classify(enabled: bool, script_path: &str, attempt: Option<&EnvIncludeOutcome>) -> Option<SpawnEnv>` and `SpawnEnv::script_applied(self) -> bool` in `crates/micold-core/src/cli_reason.rs`, in the order of research R2: `enabled` first, then the blank path, then the attempt (makes T002 green).
- [ ] T008 [U9] [U10] [U11] [U12] [U13] [U14] [U15] [U16] [U17] [U18] [U19] [U20] [U21] [U22] [U23] [U24] [U25] Implement `name_list(clis: &[AiCli]) -> Option<String>` and `explain(missing: &[AiCli], env: SpawnEnv, place: Place<'_>, dir: AttemptDir<'_>) -> Option<Explanation>` in `crates/micold-core/src/cli_reason.rs` per contracts/reason-wording.md W1 and W2. Move `name_list` out of `crates/micold-client/src/features/settings.rs` and make that file call the core function (makes T003 green).
- [X] T009 [U26] [U27] Add `env: Option<SpawnEnv>` to `DaemonMsg::AiCliAvailability` in `crates/micold-core/src/protocol/messages.rs`, and raise `PROTOCOL_VERSION` 17 → 18 with its paragraph in `crates/micold-core/src/protocol/version.rs` (research R3: no `#[serde(default)]`, no compatibility shim). Bring the stale sentence in `docs/daemon.md`'s protocol paragraph ("version 10 today") up to version 18 and what it added. So the workspace compiles, `crates/micold-daemon/src/server.rs` sends `env: None` and `crates/micold-client/src/shell/daemon_sync.rs` ignores the field until T010 and T011 (makes T004 green).
- [ ] T010 [U28] [U29] [U30] [U31] [U32] [U33] [U34] [U35] [U36] [U38] [U39] In `crates/micold-daemon/src/state.rs`, add `ResolvedEnv { vars: Vec<(String, String)>, env: SpawnEnv }` and change `EnvIncludeCell` to `Arc<OnceLock<ResolvedEnv>>` (research R1). Add `DaemonState::spawn_env_for(cwd) -> ResolvedEnv`, which does what `env_include_vars_for` does today and keeps the attempt's outcome through `SpawnEnv::classify`; for the two settings states it takes no cell and returns `ResolvedEnv { vars: merge_with_term(&[]), env: IncludeOff | NoScriptPath }`. `env_include_vars_for` becomes `spawn_env_for(cwd).vars`. Add `availability_in(cwd) -> (Vec<AiCli>, SpawnEnv)` and keep `ai_clis_available_in` as its first half. Invariant from data-model.md: "`vars` and `env` come from one call of `env_include::resolve`. Nothing writes one without the other." In `crates/micold-daemon/src/server.rs`, the `AiCliAvailabilityRequest` arm sends `env` per contract A2, S1–S7 (makes T005 green).
- [ ] T011 [U40] [U41] [U42] [U43] [U44] [U45] In `crates/micold-client/src/features/session.rs`, add `env: Option<SpawnEnv>` and `asked_for: AvailabilityKey` to `CliAvailability`, and make `AvailabilityAnswers::answered(req, answer)` stamp `asked_for` with the key the request named before filing it (contract A4 C2). In `crates/micold-client/src/shell/daemon_sync.rs`, the `DaemonMsg::AiCliAvailability` arm passes `env` (C1). Update the fixtures that build a `CliAvailability` in `crates/micold-client/tests/` and `crates/micold-client/src/main_tests.rs`. No new request is sent: `tests/availability_is_asked_only_on_named_events.rs` keeps passing (C4) (makes T006 green).

**Checkpoint**: `mise run test-core` passes the classification and wording tests. Every availability answer carries its state, and the client holds it with the directory it was asked for. No sentence a user reads has changed yet.

---

## Phase 3: User Story 1 — Settings says why a CLI is missing and what would make it appear (Priority: P1) 🎯 MVP

**Goal**: the note under **Default AI CLI** (and the one under *Image reference* in container placement) gives the reason for the state of the home directory's environment and the action that would change it, in each of the six states.

**Independent Test**: with `pi` only on a `PATH` the include script adds, open Settings → Environment in each state: off; on with a blank path; script not found; exited with an error; timed out; succeeded with the CLI still missing. Each note names Pi Coding Agent, the reason and the action (quickstart §B B1–B8). Turning the setting on, saving and reopening Settings lists the CLI with no restart.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these first and observe each fail for the stated reason before its implementation task.

- [ ] T038 [US1] [A1] [A3] [A4] [A5] [A6] [A7] Write the outer-loop acceptance tests A1 and A3–A7 in `crates/micold-client/src/main_tests.rs` at the client entry point described in `tdd/test-list.md` (a `DaemonMsg::AiCliAvailability { req, available, env }` for the home request through the shell arm, then `missing_cli_notice(app.core.session.availability.home())`). A1: Pi missing + `IncludeOff` → the `IncludeOff` host sentence. A3: `ScriptTimedOut` → "timed out for your home directory", naming "Script path" and "Timeout", the same whatever `app.env_include_last_outcome` holds. A4: `ScriptFailed`, then `ScriptNotFound`. A5: `NoScriptPath`, with no "Turn it on". A6: `Applied` on this computer. A7: every CLI available → no note in any state. They stay red until T011 and T014 land. T039 confirms them green.
- [ ] T012 [US1] [U47] [U48] [U50] [U53] [U54] [U55] Write `missing_cli_notice` tests for surfaces U1 and U2 and for W5 in `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs`: for each `SpawnEnv` state, with `source` this computer and with `source` an image, the note is `explain`'s `{reason} {action}` for the missing CLIs, with `AttemptDir::Home` (FR-006, FR-005). The note under *Image reference* is the same string as the note under **Default AI CLI**. The existing image assertions for the `Applied` state keep passing unchanged (W2d). The note is `None` with no answer, with nothing missing, and with `env: None` (W5, FR-011, Story 1 scenarios 6 and 7). The function takes the state by shared reference and changes no setting (FR-015). Rewrite the existing tests the new sentence breaks: in `crates/micold-client/tests/a_field_note_shares_its_fields_column.rs`, `settings_showing` builds its `CliAvailability` with `env: Some(SpawnEnv::Applied)` (with `env: None` there is no note, W5), and `the_missing_cli_notice_lines_up_with_the_select_it_is_about` finds the note by a phrase of the new host sentence ("was not found on the PATH") in place of "installed on this computer"; `and_so_does_the_one_under_the_image_reference` keeps its needle. `crates/micold-client/tests/features_settings.rs` `a_failure_message_names_the_cli_the_human_readable_way` (line 291) builds its own `"{} isn't installed."` string and is not broken by M1: make it assert that `explain(&[cli], ..)`'s reason names the display name.
- [ ] T013 [US1] [A2] [A8] Write a shell test in `crates/micold-client/src/main_tests.rs` for Story 1 scenarios 2 and 7: with no answer filed the note is `None`; after `DaemonMsg::AiCliAvailability { available: [claude], env: Some(IncludeOff), .. }` through the shell arm the note names the other CLIs with the `IncludeOff` reason; after a second answer with every CLI available the note is `None`, with no other message sent (FR-013, SC-002).

### Implementation for User Story 1

- [ ] T014 [US1] [A1] [A2] [A3] [A4] [A5] [A6] [A7] [A8] [U47] [U48] [U50] [U53] [U54] [U55] Make `missing_cli_notice(availability)` in `crates/micold-client/src/features/settings.rs` return `explain(missing, env, place, AttemptDir::Home)`'s `{reason} {action}`, with `place` from `CliAvailability::source` and `None` when `env` is `None` (contracts/reason-wording.md W4 surfaces U1 and U2, W5). `crates/micold-client/src/ui/settings/daemon.rs` keeps calling the same function (makes T012 and T013 green).
- [X] T015 [US1] In `crates/micold-client/src/ui/settings/environment.rs`, render the checkbox label and the two field labels from `cli_reason::LABEL_ENABLED`, `LABEL_SCRIPT_PATH` and `LABEL_TIMEOUT` instead of string literals, so a sentence cannot name a label the page does not show (FR-003, W2b; GUI-glue exception, validated by T017).
- [X] T016 [US1] Update the user guide (FR-017): in `docs/user-guide/settings.md`, the Default AI CLI section describes the note's six reasons and the action each names, and says the note answers for the home directory; in `docs/user-guide/sandboxed-daemon.md`, the note under *Image reference* says the image lacks a CLI only when the environment was applied, and otherwise gives the environment-include reason.
- [ ] T039 [US1] [A1] [A2] [A3] [A4] [A5] [A6] [A7] [A8] Confirm the outer-loop tests A1–A8 from T038 and T013 are green with the full suite (`mise run gate`), and record the result in `specs/037-explain-hidden-cli/tdd/cycle-log.md`. A7 asserts an absence and may pass once it compiles. If it does, record a deliberate mutant that turns it red (for example, return a note when nothing is missing), then revert it.
- [ ] T017 [US1] Run quickstart §B steps B1–B8 and B14 with the `visual-pass` skill, in the light and dark themes, and check the note wraps inside the field's column. Save the screenshots under `specs/037-explain-hidden-cli/evidence/`.

**Checkpoint (M1)**: US1 acceptance scenarios 1–7 and 4a hold. A start failure and the missing-default message keep today's text until M2.

---

## Phase 4: User Story 2 — A failed or refused start gives the same reason (Priority: P2)

**Goal**: the missing-default message, a start or restart failure, and the reply an AI session gets for `create_session` give the reason and action for the directory's state, and no longer say "isn't installed" when no script was applied.

**Independent Test**: with a stored default that resolves only through the script, switch environment-include off and press start on a row: the notification gives the `IncludeOff` reason and action and the list of available CLIs opens (quickstart §B B9). Restart a session of that CLI after the script starts timing out: the pane and the banner give the `ScriptTimedOut` reason and do not say "install" (B10).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T040 [US2] [A9] Write the outer-loop acceptance test A9 in `crates/micold-client/src/main_tests.rs` at the same entry point as T038: a row whose directory's answer lacks the stored default with `env: Some(SpawnEnv::IncludeOff)`; the primary press (`SessionMsg::StartMenuOpened` with `unavailable_default`) posts one notification equal to `start_refusal(cli, IncludeOff, Place::ThisComputer, .., Fresh)`, the list is open, no start is sent to the service and the stored default is unchanged. It stays red until T022 and T025 land.
- [ ] T018 [US2] [U56] [U57] [U58] [U59] [U60] [U61] [U62] [U63] Write `start_refusal` tests for contracts/reason-wording.md W3 in `crates/micold-core/tests/cli_reason.rs`: for every state × place, `LaunchMode::Fresh` is `{reason} {action} Or start this session on another AI CLI.` and `LaunchMode::Resume` is `{reason} {action} Then restart this session: its conversation can only continue in {name}.`, where `{reason}` and `{action}` are `explain(&[cli], ..)`'s (FR-012), except in the `Applied`/image row; that row is today's `missing_cli_reason` text byte for byte in both forms (W3b); the `Resume` form never contains "another AI CLI" (W3a); in `ScriptNotFound`, `ScriptFailed` and `ScriptTimedOut` no form contains "install" (W3c, FR-009); in every state and place except `Applied`/image both forms begin with `explain`'s `{reason} {action}` (W3d); `start_refusal_unknown(cli)` is `{name} would not be found by a session here. Start this session on another AI CLI.`; rule W2a holds for both forms and for `start_refusal_unknown` (W3e, FR-002).
- [ ] T019 [P] [US2] [A11] [A12] [A13] [U64] [U65] [U66] Write the start-failure tests for surface U4 in `crates/micold-daemon/tests/session_start.rs`: a `Fresh` and a `Resume` start of a CLI the directory's environment lacks fail with `start_refusal(cli, env, place, Dir(cwd), launch)` when environment-include is off, when the script exits with an error, and when it succeeds without the CLI; with `MICOLD_IMAGE_REFERENCE` set and the script applied the text is today's (Story 2 scenario 4); with it set and environment-include off the text gives the `IncludeOff` image row and does not contain "isn't in" (Story 2 scenario 5). The failure and an availability answer for the same directory name the same state (contract A3). Replace the file's own `missing_cli_reason` test helper with calls to `cli_reason::start_refusal`.
- [ ] T041 [US2] [A10] Write the outer-loop acceptance test A10 in `crates/micold-daemon/tests/session_start.rs` (after T019, the same file): with a script that sleeps past a 1 s timeout, a `Resume` start of a CLI the directory's environment lacks fails with `start_refusal(cli, ScriptTimedOut, place, Dir(cwd), Resume)`; the reason names the CLI and the directory, says to restart this session, and contains neither "install" nor "another AI CLI" (Story 2 scenario 2).
- [ ] T020 [P] [US2] [A14] Write the test for surface U5 in `crates/micold-daemon/tests/mcp_create_session.rs` (a `#![cfg(unix)]` file: the wording is held on every platform by T003), by rewriting `a_cli_that_is_not_installed_fails_naming_it_and_leaves_no_record`, which asserts the binary name `pi` today: `create_session` for a CLI the target directory's environment lacks is refused with `explain(&[cli], env, place, Dir(cwd))`'s `{reason} {action}`; with environment-include off the reply does not contain "is not installed" (FR-009a, Story 2 scenario 6).
- [ ] T021 [P] [US2] [U69] [U71] [U72] [U73] [U74] [U75] Write the missing-default tests for surface U3 in `crates/micold-client/tests/unavailable_default_says_so.rs`: for each state, pressing start on a row whose stored default is missing posts one notification with `start_refusal(cli, env, place, asked_for, Fresh)`, opens the list of available CLIs, starts nothing and leaves the stored default unchanged (FR-008, FR-015); with `env: None` the notification is `start_refusal_unknown(cli)` (research R8); a row reading the home answer says "your home directory" (FR-004a). In `crates/micold-client/tests/directory_availability.rs`: a newer answer filed after the notification posts no second one and does not change the first (FR-012, W6). Update the fixtures in `crates/micold-client/tests/start_failure_notice.rs` that quote the old text.

### Implementation for User Story 2

- [ ] T022 [US2] [U56] [U57] [U58] [U59] [U60] [U61] [U62] [U63] Implement `start_refusal(cli: AiCli, env: SpawnEnv, place: Place<'_>, dir: AttemptDir<'_>, launch: LaunchMode) -> String` and `start_refusal_unknown(cli: AiCli) -> String` in `crates/micold-core/src/cli_reason.rs` per W3 and W5 (makes T018 green).
- [ ] T023 [US2] [A10] [A11] [A12] [A13] [U64] [U65] [U66] In `crates/micold-daemon/src/state.rs`, make the launch gate (the `plan.mode == TerminalMode::AiCli` check in the start path) read the directory's `ResolvedEnv` from `spawn_env_for(cwd)` and fail with `start_refusal(cli, env, place, AttemptDir::Dir(&plan.cwd), launch)`, with `place` from `MICOLD_IMAGE_REFERENCE`. Remove `missing_cli_reason` (makes T019 green).
- [ ] T024 [US2] [A14] In `crates/micold-daemon/src/mcp/tools.rs`, make `create_session`'s refusal for a missing CLI read the same `ResolvedEnv` and reply with `explain`'s `{reason} {action}` for `Dir(cwd)` (makes T020 green).
- [ ] T025 [US2] [A9] [U69] [U71] [U72] [U73] [U74] [U75] In `crates/micold-client/src/features/session.rs`, make `start_menu_toggled` post `start_refusal(cli, env, place, dir, Fresh)` from the answer in use for the row (`place` from `source`, `dir` from `asked_for`), and `start_refusal_unknown(cli)` when `env` is `None` (research R8) (makes T021 green).
- [ ] T026 [US2] Update the user guide (FR-017): in `docs/user-guide/worktrees-and-sessions.md`, the "When a CLI isn't installed" section describes the reasons a start or restart failure and the missing-default message now give, and that a restart failure says to fix the cause and restart; in `docs/user-guide/agent-tools.md`, the `create_session` refusal carries the same reason and action.
- [ ] T042 [US2] [A9] [A10] [A11] [A12] [A13] [A14] Confirm the outer-loop tests A9–A14 from T040, T041, T019 and T020 are green with the full suite (`mise run gate`), and record the result in `specs/037-explain-hidden-cli/tdd/cycle-log.md`. A12 is a characterization test (Story 2 scenario 4, "unchanged from today"): record that it was green before T023 changed the gate.
- [ ] T027 [US2] Run quickstart §B steps B9 and B10 with the `visual-pass` skill and save the screenshots under `specs/037-explain-hidden-cli/evidence/`.

**Checkpoint (M2)**: US2 acceptance scenarios 1–6 hold, and US1 still passes. No surface says "isn't installed" where no script was applied (SC-003).

---

## Phase 5: User Story 3 — A row's CLI list tells me a CLI is hidden (Priority: P3)

**Goal**: a row's CLI list that already opens (two or more available CLIs) names the CLIs not offered there, with the reason and action for that row's directory, in a note that cannot be pressed. A row with fewer than two available CLIs is unchanged (D5, 026 FR-006).

**Independent Test**: with two available CLIs and Pi missing for a project's directory, press the row's chevron: the list shows the two CLIs, a divider, and the note (quickstart §B B11). A row with one available CLI has no chevron (B12). A row with every CLI available has no note (B13).

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T043 [US3] [A15] [A16] [A17] [A18] [A19] [A20] Write the outer-loop acceptance tests A15–A20 in `crates/micold-client/src/main_tests.rs` at the same entry point as T038, reading `app.core.session.start_menu_note(dir)` and `offered_providers(Some(dir))` after each row's own answer is filed. A15: two CLIs offered, Pi missing, `ScriptFailed` → the note names Pi Coding Agent, the reason, the action and the row's directory. A16: one CLI → no chevron and no note. A17: one CLI that is not the stored default → the primary press opens the list with T040's message and there is no note. A18: every CLI → no note. A19: two rows with different answers keep their own notes when the other's list is opened, answered and closed. A20: with the list open the offered CLIs do not include the one the note names, and nothing was started. They stay red until T030 lands. T044 confirms them green.
- [ ] T028 [P] [US3] [U78] [U79] [U80] [U81] [U82] [U83] [U84] [U85] Write `start_menu_note` tests for surface U6 in `crates/micold-client/tests/directory_availability.rs`: with three supported CLIs, an answer offering two with `env: Some(..)` gives `Some` of `explain`'s `{reason} {action}` for the missing one and the answer's `asked_for`; offering three gives `None` (Story 3 scenario 2); offering one or none gives `None` (scenarios 1a and 1b, D6); `env: None` gives `None`; no answer in use gives `None` (W5). A row reading the home answer says "your home directory", and switches to its own directory when its own answer is filed (D7). Two rows with different answers each get their own note, and filing one row's answer does not change the other's (scenario 3).
- [ ] T029 [P] [US3] [U87] [U88] [U89] [U90] Write the anatomy tests for W7 in `crates/micold-client/src/ui/material/menu_anatomy.rs`: a `MenuOverlay` with a note lays out the items, then a `Divider`, then the note; the note wraps at the panel's width less the item padding at both sides; the note has no pressable region (Story 3 scenario 4); `menu_panel_size_with_note(items, note)` equals the laid-out panel's size for a one-line and a three-line note; a menu without a note is laid out exactly as `menu_panel_size` says today.

### Implementation for User Story 3

- [ ] T030 [US3] [A15] [A16] [A17] [A18] [A19] [A20] [U78] [U79] [U80] [U81] [U82] [U83] [U84] [U85] Implement `State::start_menu_note(dir) -> Option<String>` in `crates/micold-client/src/features/session.rs` per research R6: `Some` only when the answer in use for the row offers two or more CLIs, misses at least one and carries an `env` (makes T028 green).
- [ ] T031 [US3] [U87] [U88] [U89] [U90] Implement `MenuOverlay::note(text: impl Into<String>) -> Self` and `menu_panel_size_with_note(items, note) -> (u16, u16)` in `crates/micold-client/src/ui/material/menu.rs` per W7 (a `material::Divider`, then wrapped `TypeRole::Label` text in the muted role with the item padding; no `on_press`, no ripple, no state layer), and re-export `menu_panel_size_with_note` from `crates/micold-client/src/ui/material/mod.rs` (makes T029 green).
- [ ] T032 [US3] In `crates/micold-client/src/ui/mod.rs`, pass `start_menu_note(dir)` to the start list's `MenuOverlay` and clamp its anchor with `menu_panel_size_with_note` (Principle VIII; GUI-glue exception, validated by T035).
- [ ] T033 [US3] Add a `MenuOverlay` with a note to the component showcase in `crates/micold-client/src/showcase/sections/floating.rs`, and regenerate `crates/micold-client/tests/layout_snapshot.rs`'s snapshot if the entry changes it.
- [ ] T034 [US3] Update the row-list paragraph of `docs/user-guide/worktrees-and-sessions.md` (FR-017): a list with two or more CLIs names the ones not offered for that row's directory with the reason and action; a row with one CLI has no chevron and is unchanged.
- [ ] T044 [US3] [A15] [A16] [A17] [A18] [A19] [A20] Confirm the outer-loop tests A15–A20 from T043 are green with the full suite (`mise run gate`), and record the result in `specs/037-explain-hidden-cli/tdd/cycle-log.md`. A16, A17 and A18 assert an absence and may pass once they compile. For each one that does, record a deliberate mutant that turns it red (for example, drop the "two or more" condition of `start_menu_note`), then revert it.
- [ ] T035 [US3] Run quickstart §B steps B11–B13 with the `visual-pass` skill, in the light and dark themes: the note wraps inside the panel and the panel stays inside the window when opened from the lowest row. Save the screenshots under `specs/037-explain-hidden-cli/evidence/`.

**Checkpoint (M3)**: all three stories hold.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T036 Run quickstart Part A (`mise run test-core`, `mise run gate`) and the whole of Part B (B1–B14, light and dark themes) with the `visual-pass` skill on the merged result (the rendering half of SC-001 is checked on Linux; "each platform" is held by the core and service tests on CI's Linux, macOS and Windows jobs), and record each step's outcome in `specs/037-explain-hidden-cli/evidence/README.md`.
- [ ] T037 Cross-check the spec artifacts against what shipped: every sentence in `specs/037-explain-hidden-cli/contracts/reason-wording.md` W2 and W3 equals the string in `crates/micold-core/src/cli_reason.rs`, and the four user guide pages quote no sentence that differs. Correct the contract or the guide where they differ, and record the check in `specs/037-explain-hidden-cli/evidence/README.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: after Setup. Blocks every story.
- **US1 (Phase 3)**: after Foundational.
- **US2 (Phase 4)**: after Foundational. It shares no file with US1 except `tests/directory_availability.rs`, but ships after it (priority order).
- **US3 (Phase 5)**: after US2. Its outer test of scenario 1b (A17) reads the message US2 ships, and its user-guide paragraph follows US2's section in the same page.
- **Polish (Phase 6)**: after all stories.

### Within Each Phase

- T002 → T007. T003 → T008 (one test file, so T002 and T003 are sequential).
- T004 → T009. T005 → T010 (after T009). T006 → T011 (after T009).
- T038 first in US1. T012, T013, T038 → T014. T015 after T001. T039 after T014. T017 after T014–T016 and T039.
- T040 first in US2. T018 → T022. T019, T041 → T023 (after T022; T041 after T019, one test file). T020 → T024. T021, T040 → T025 (after T022). T042 after T023–T025. T027 after T023–T026 and T042.
- T043 first in US3. T028, T043 → T030. T029 → T031. T032 after T030 and T031. T033 after T031. T044 after T030. T035 after T032–T034 and T044.
- T037 after T036 (both record in `evidence/README.md`).

### Parallel Opportunities

- T004, T005 and T006 (three crates' test files).
- T019, T020 and T021 (different files).
- T028 and T029 (different files).

## Parallel Example: User Story 2

```text
Task: "T019 start-failure tests in crates/micold-daemon/tests/session_start.rs"
Task: "T020 create_session refusal test in crates/micold-daemon/tests/mcp_create_session.rs"
Task: "T021 missing-default tests in crates/micold-client/tests/unavailable_default_says_so.rs"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 + Phase 2 (core classification and wording, the wire field, the service's answer, the client's copy).
2. Phase 3 (US1): the Settings note is truthful in all six states, and documented.
3. Stop and validate with quickstart §B B1–B8 and B14.

### Incremental Delivery

1. M1: Setup + Foundational + US1 → the Settings note ships.
2. M2: US2 → the start failure, the missing-default message and the reply to an AI session ship.
3. M3: US3 → the note in a row's CLI list ships.
4. M4: Polish → the full quickstart pass and the wording cross-check are recorded.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The Settings note gives the reason and the action 🎯 MVP

- **Tasks**: T001–T017, T038–T039
- **Deliverable**: with a CLI a session would not find, the note under **Default AI CLI** (and under *Image reference* in container placement) names it and gives the reason for the state of the home directory's environment (off, no script path, script not found, failed, timed out, applied) with the action that would change it. It no longer says "isn't installed" when no script was applied. Turning environment-include on and saving lists the CLI on the next open, with no restart. The user guide describes it. A start failure and the missing-default message keep today's text until M2.
- **Satisfies**: US1 acceptance scenarios 1–7 and 4a; FR-001 to FR-004a (Settings note), FR-005 (notes), FR-006, FR-007, FR-011 (Settings), FR-012 (one answer carries offer and reason), FR-013, FR-014, FR-015, FR-016, FR-017 (settings.md, sandboxed-daemon.md); SC-001, SC-002, SC-005 (Settings), SC-006
- **Verify**: `mise run test-core` (`crates/micold-core/tests/cli_reason.rs`, `schema_hash.rs`, `protocol_roundtrip.rs`); `mise run gate` (`crates/micold-daemon/tests/ai_cli_availability.rs` S1–S7 and the run count, `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs`, `tests/directory_availability.rs`); quickstart §B B1–B8 and B14 via `visual-pass`
- **Depends on**: —
- **Tier**: full

### M2 — A failed or refused start gives the same reason

- **Tasks**: T018–T027, T040–T042
- **Deliverable**: pressing start on a row whose default CLI is missing posts a message with the reason and action for that row's answer and opens the list. A start or restart failure in the pane and the banner, and the reply an AI session gets from `create_session`, give the reason and action for the session's directory. None says "isn't installed" where no script was applied, and a failed-attempt state never says to install. The user guide describes it.
- **Satisfies**: US2 acceptance scenarios 1–6; the message of US3 acceptance scenario 1b (its outer test lands with M3); FR-002 and FR-012 (all event surfaces), FR-008, FR-009, FR-009a, FR-017 (worktrees-and-sessions.md, agent-tools.md); SC-003, SC-004 (event surfaces)
- **Verify**: `mise run test-core` (`tests/cli_reason.rs` W3); `mise run gate` (`crates/micold-daemon/tests/session_start.rs`, `tests/mcp_create_session.rs`, `crates/micold-client/tests/unavailable_default_says_so.rs`); quickstart §B B9 and B10 via `visual-pass`
- **Depends on**: M1
- **Tier**: full

### M3 — A row's CLI list names what is not offered

- **Tasks**: T028–T035, T043–T044
- **Deliverable**: a row's CLI list with two or more available CLIs and another missing shows, under a divider, a note naming the missing CLIs with the reason and action for that row's directory. The note cannot be pressed. A row with fewer than two available CLIs, and a row with every CLI available, are unchanged. The component showcase shows a menu with a note, and the user guide describes it.
- **Satisfies**: US3 acceptance scenarios 1, 1a, 1b, 2, 3, 4; FR-010, FR-011 (row list), FR-017 (row-list paragraph); SC-004 (row list), SC-005 (row list), SC-007
- **Verify**: `mise run gate` (`crates/micold-client/tests/directory_availability.rs` `start_menu_note`, `src/ui/material/menu_anatomy.rs`); quickstart §B B11–B13 via `visual-pass`
- **Depends on**: M1, M2
- **Tier**: full

### M4 — Polish: the full pass and the wording cross-check are recorded

- **Tasks**: T036–T037
- **Deliverable**: `specs/037-explain-hidden-cli/evidence/README.md` records quickstart Part A and every step of Part B on the merged result, and that the contract's sentences equal the shipped strings.
- **Satisfies**: SC-001 (six of six, both placements, recorded), SC-003 and SC-004 (cross-check)
- **Verify**: `specs/037-explain-hidden-cli/evidence/README.md` records B1–B14 and the cross-check; `mise run test-scripts`
- **Depends on**: M1, M2, M3
- **Tier**: docs

## Notes

- [P] tasks touch different files and have no dependency on an incomplete task.
- Commit after each red → green step (the `tdd.run` hook records it in `tdd/cycle-log.md`).
- M1 is 19 tasks and is not split: all six states come from one `classify` and one `explain`, so no acceptance scenario of Story 1 can ship without the Foundational phase, and none is a deliverable without the note.
- The chevron rule of 026 FR-006 is unchanged (ledger D5). No task amends a closed spec.
- Nothing is persisted. No task touches `settings.json`'s schema or the session store.
