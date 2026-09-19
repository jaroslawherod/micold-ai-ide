---
description: "Task list for feature 029 — run a session on the Pi coding agent"
---

# Tasks: Run a session on the Pi coding agent

**Input**: Design documents from `/specs/029-pi-cli-provider/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/pi-cli.md](./contracts/pi-cli.md),
[quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY. Every user story writes its failing tests before its implementation. The one exception
the constitution allows — GUI/process-spawn wiring — is `crates/micold-daemon/assets/pi-activity.ts`
and the spawn preparation that loads it, validated by the recorded procedure in
[quickstart.md](./quickstart.md) §C.

**Documentation**: Per Constitution Principle VII, each user-facing story carries its own user-guide
task in the same change (FR-022).

**Cross-platform**: Per Constitution Principle VI, no `cfg` arm is added anywhere in this feature
(research R3). Pi's base directory is home-relative on all three platforms and `resolves_on_path`
already answers the `.exe`/`.cmd` question through `PATHEXT`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story the task belongs to (US1–US4)
- Every description carries an exact file path

## Path Conventions

Three-crate Rust workspace, unchanged by this feature:
`crates/micold-core/`, `crates/micold-daemon/`, `crates/micold-client/`, plus
`packaging/sandbox/Containerfile` and `docs/`. Build and test through `mise run <task>`, never bare
`cargo` (CLAUDE.md).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the fixture material every story's tests read. No production code.

- [X] T001 Create a Pi session-store fixture tree under `crates/micold-core/tests/fixtures/pi/`, mirroring `crates/micold-core/tests/fixtures/copilot/`: one `sessions/--<encoded cwd>--/<timestamp>_<uuid>.jsonl` conversation with the v3 header line, a `session_info` name entry, a first user message, and a second conversation that has a name only far past the label bound — shapes per [contracts/pi-cli.md](./contracts/pi-cli.md)
- [X] T002 [P] Add Pi fixture helpers to `crates/micold-core/tests/support/mod.rs`: the cwd encoder (leading separator stripped, `/`, `\`, `:` → `-`, wrapped in `--…--`), a conversation-path builder, and a writer that appends JSONL lines to a fixture conversation

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the two seam changes and the third enum member every user story is built on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests (write first, confirm they fail)

- [X] T003 [P] Extend `crates/micold-core/tests/ai_cli_registry.rs`: `AiCli::ALL` is `[ClaudeCode, Copilot, Pi]` in that order, `AiCli::default()` is still `ClaudeCode`, and every variant round-trips its serde representation (FR-002, FR-021, SC-003)
- [X] T004 [P] Extend `crates/micold-core/tests/ai_cli_provider_seam.rs`: every `AiCli::ALL` member resolves a provider whose `id()` matches it, and `ActivitySource::Extension { log }` is constructible and matchable through the seam like the existing variants (FR-019, FR-020)
- [X] T005 [P] Extend `crates/micold-core/tests/ai_cli_provider.rs`: every provider answers `launch_env()`, the existing two returning nothing, so a per-launch environment is a seam capability rather than a conditional on which CLI a session runs (FR-020)
- [X] T006 [P] Extend `crates/micold-core/tests/settings_roundtrip.rs`: a settings file written before Pi existed loads unchanged, and `default_ai_cli` round-trips Pi (FR-002, SC-003)

### Implementation

- [X] T007 Add `Pi` to `AiCli` in `crates/micold-core/src/session.rs`, widen `ALL` to `[AiCli; 3]` keeping it sorted, and leave `#[default] ClaudeCode` untouched (FR-002, FR-021)
- [X] T008 Add `ActivitySource::Extension { log: PathBuf }` to `crates/micold-core/src/provider.rs`, documenting beside the payload-free `Hooks` variant why this one carries its path (the provider derives it; the daemon does not choose it) — per [data-model.md](./data-model.md) §3
- [X] T009 Add `launch_env(&self) -> Vec<(String, String)>` to the `AiCliProvider` trait in `crates/micold-core/src/provider.rs` with no default, and implement it as empty for `ClaudeProvider`, `CopilotProvider` and `FakeAiCliProvider` — the seam change research R9's offline posture needs (FR-020, Principle IV)
- [X] T010 Add the `PiProvider` unit struct to `crates/micold-core/src/provider.rs` with `id()`, `display_name()` = `"Pi Coding Agent"` and `command()` = `"pi"`, and add the `AiCli::Pi => &PiProvider` arm to `AiCli::provider()` (FR-001a, FR-019)
- [X] T011 Resolve the exhaustive-match breakage the two new variants produce in `crates/micold-daemon/src/state.rs` and `crates/micold-client/src/`, with no arm outside `crates/micold-core/src/provider.rs` naming Pi (FR-019, SC-008)

**Checkpoint**: the workspace compiles with three providers; `mise run test-core` is green except for the story phases below. `crates/micold-daemon/tests/sandbox_real_ai_cli.rs` is now red for `pi` without being edited — that is User Story 2's Test-First red, arriving for free.

---

## Phase 3: User Story 1 - Start and resume a session on Pi (Priority: P1) 🎯 MVP

**Goal**: Pi is selectable as a default or a per-session override, a session runs `pi` in its own
working directory, and reopening the application resumes that same conversation.

**Independent Test**: With `pi` installed, set the default to Pi, start a session, confirm `pi` is
the process in that session's terminal with the session's cwd. Restart the application and confirm
the same conversation comes back — then resume it from a bare `pi --session-id <that id>` in a
terminal and see the same history ([quickstart.md](./quickstart.md) §B).

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these first; confirm they FAIL before implementing.

- [X] T012 [P] [US1] Create `crates/micold-core/tests/pi_provider.rs` covering the launch surface: `command()` is `pi`, `display_name()` is `"Pi Coding Agent"`, and `launch_args()` returns the **same** single `--session-id <uuid>` vector for both `LaunchMode::Fresh` and `LaunchMode::Resume` (research R1, FR-005a)
- [X] T013 [P] [US1] Add `config_dir()` coverage to `crates/micold-core/tests/pi_provider.rs`: `PI_CODING_AGENT_DIR` wins, otherwise home-relative `~/.pi/agent`, and `None` when no home resolves — with no platform branch in the assertions (research R8, Principle VI)
- [X] T014 [P] [US1] Add `has_recorded_conversation()` coverage to `crates/micold-core/tests/pi_provider.rs` against the T001 fixture, including an absent store directory and an unreadable one, both answering `false` rather than failing (Edge Cases)
- [X] T015 [P] [US1] Add `launch_env()` coverage to `crates/micold-core/tests/pi_provider.rs`: `PI_OFFLINE=1`, `PI_SKIP_VERSION_CHECK=1`, `PI_TELEMETRY=0` (research R9, Principle IV)
- [X] T016 [P] [US1] Extend `crates/micold-daemon/tests/settings_default_ai_cli.rs`: a session started with the default set to Pi records Pi, and still records Pi after a daemon restart (FR-008, SC-002)
- [X] T017 [P] [US1] Extend `crates/micold-daemon/tests/resume_failure_reported.rs`: resuming a Pi session whose conversation Pi no longer holds reports the failure with a reason and starts nothing under that session's identity (FR-005, Scenario 1.4)
- [X] T018 [P] [US1] Extend `crates/micold-daemon/tests/exclusivity.rs`: opening a Pi conversation one of the application's own sessions already holds is refused, naming that session, and starts no second process (FR-006a)

### Implementation for User Story 1

- [ ] T019 [US1] Implement `PiProvider::is_available()` in `crates/micold-core/src/provider.rs` through the existing `resolves_on_path("pi")` — no spawn, no version read (FR-003, FR-003a, SC-006a) (reopened — BUG-001: it walked the daemon's own `PATH`, not the spawn environment's; closes with T064)
- [X] T020 [US1] Implement `PiProvider::launch_args()` in `crates/micold-core/src/provider.rs`: one `--session-id <uuid>` vector for both launch modes, with a comment recording why `--session`, `--no-session`, `--name` and `--session-dir` are excluded ([contracts/pi-cli.md](./contracts/pi-cli.md) §Not used)
- [X] T021 [US1] Implement `PiProvider::config_dir()` in `crates/micold-core/src/provider.rs`: `PI_CODING_AGENT_DIR` else home-relative `~/.pi/agent`, with no `cfg` arm (Principle VI)
- [X] T022 [US1] Add the per-cwd store helpers to `PiProvider` in `crates/micold-core/src/provider.rs` — cwd encoding, `session_dir()`, `conversation_path()` — and implement `has_recorded_conversation()` on them (FR-005a)
- [X] T023 [US1] Implement `PiProvider::launch_env()` in `crates/micold-core/src/provider.rs` returning the three offline variables, and merge a provider's `launch_env()` into the spawn environment beside `merge_with_term` in `crates/micold-daemon/src/state.rs` — for every provider, not for Pi specifically (FR-020, Principle IV)
- [X] T024 [US1] Document Pi in `docs/user-guide/worktrees-and-sessions.md`: how to select it, that its conversation lives in Pi's own store and is therefore resumable by a bare `pi` outside the application, and that the in-use warning is advisory rather than a guarantee — and that on Pi it never fires, because Pi leaves no liveness indicator (FR-022, FR-005a, FR-006b, FR-006c, research R2)
- [X] T025 [P] [US1] List Pi as a supported AI CLI in `README.md` and `docs/README.md`, naming it "Pi Coding Agent" (FR-022, FR-001a, SC-010)

**Checkpoint**: a Pi session starts, resumes across restart, and is resumable from a bare `pi`. This is the MVP.

---

## Phase 4: User Story 2 - Pi works in the sandboxed runtime without installing anything (Priority: P2)

**Goal**: the published image ships `pi` at a pinned version so a sandboxed user starts a Pi session
having installed nothing.

**Independent Test**: `mise run image` then `mise run test-sandbox` with no `pi` on the host — a Pi
session starts inside the image as the sandbox uid ([quickstart.md](./quickstart.md) §D).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T026 [US2] Confirm `crates/micold-daemon/tests/sandbox_real_ai_cli.rs` fails for `pi` **without editing it** — it iterates `AiCli::ALL`, so T007 made it red — via `mise run image` then `mise run test-sandbox` (research R6, FR-021)

### Implementation for User Story 2

- [X] T027 [US2] Add `ARG PI_CLI_VERSION=0.85.1` beside the existing `CLAUDE_CODE_VERSION`/`COPILOT_CLI_VERSION` args in `packaging/sandbox/Containerfile` and add `"@earendil-works/pi-coding-agent@${PI_CLI_VERSION}"` to the **existing single** `RUN npm install -g --omit=dev` layer — no second layer, no extra runtime, since `jiti` ships inside the package (FR-017, FR-017a, research R4)
- [X] T028 [P] [US2] Set `PI_OFFLINE=1`, `PI_SKIP_VERSION_CHECK=1` and `PI_TELEMETRY=0` in `packaging/sandbox/Containerfile` alongside the existing CLI environment, so the image's default posture matches the launch environment of T023 (Principle IV, research R9)
- [X] T029 [US2] Verify the image-contents check and the FR-018 substituted-image report in `crates/micold-daemon/tests/sandbox_real_ai_cli.rs` and `crates/micold-core/tests/sandbox_parity.rs` cover Pi without either file naming it, and that neither gained an entry by hand (FR-018, FR-021, SC-007, SC-009)
- [X] T030 [US2] Rebuild with `mise run image` and take `mise run test-sandbox` to green, confirming a Pi session starts in the image and that activity reporting works there identically to the host because the component travels with the session service (SC-007, SC-007a, FR-017a)

**Checkpoint**: Pi works on host and in the sandbox on the same terms.

---

## Phase 5: User Story 3 - Tell Pi sessions apart from the others at a glance (Priority: P3)

**Goal**: a sidebar row names its CLI as `pi`, carries the conversation's own title, and shows the
same busy/idle badge vocabulary the other two CLIs use — with the user able to decline the component
that supplies the signal.

**Independent Test**: sessions on all three CLIs in one project; each row names its CLI as text and
carries its own title; a Pi session that starts working shows working within a second and still
reads working while Pi thinks and writes nothing ([quickstart.md](./quickstart.md) §C).

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T031 [P] [US3] Extend `crates/micold-client/tests/provider_choice_surfaces.rs`: Pi appears in the Settings default and the per-session override reading "Pi Coding Agent", never `pi` (FR-001, FR-001a, SC-004a)
- [X] T032 [P] [US3] Extend `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs` and `crates/micold-client/tests/unavailable_default_says_so.rs`: a missing `pi` is reported as "Pi Coding Agent" and names the installed version where one was read (FR-001a, FR-003a)
- [X] T033 [P] [US3] Extend `crates/micold-client/tests/sidebar_tree.rs` and `crates/micold-client/tests/terminal_tabs.rs`: a Pi row and a Pi pinned AI tab read `pi` as text, from `command()` and not `display_name()` (FR-009, FR-010, SC-004, SC-004a)
- [X] T034 [P] [US3] Add `read_title()` coverage to `crates/micold-core/tests/pi_provider.rs` for the bounded-prefix chain — latest in-prefix `session_info` name, else the first user message, else `None` — including the T001 conversation whose name lies past the bound, which must fall back without failing and without reading further (FR-011, SC-006b, research R5)
- [X] T035 [P] [US3] Add `activity_source()` coverage to `crates/micold-core/tests/pi_provider.rs`: `ActivitySource::Extension { log }` at `<base>/micold-activity/<session-id>.jsonl`, outside `sessions/`, and reported identically whether or not the FR-012e switch is on — the provider is pure (data-model §3)
- [X] T036 [P] [US3] Create `crates/micold-daemon/tests/pi_activity.rs` mirroring `crates/micold-daemon/tests/copilot_activity.rs`: the event→signal table from [contracts/pi-cli.md](./contracts/pi-cli.md), unknown event types ignored rather than rejected, and no log at all reading `Unknown` (FR-012, FR-012d)
- [X] T037 [P] [US3] Extend `crates/micold-daemon/tests/activity_pipeline.rs`: a `turn_start` line appended to a Pi activity log moves the badge to working within the existing bound, and `agent_settled` moves it to awaiting input, through the unchanged `Activity` state machine (SC-005)
- [X] T038 [P] [US3] Extend `crates/micold-core/tests/settings_ai_cli.rs`: the activity-component switch defaults on, persists across a save/load cycle, and is absent-means-on for a settings file written before this feature (FR-012e)
- [X] T039 [P] [US3] Extend `crates/micold-client/tests/features_settings.rs`: the switch is exactly one application-wide row, with no per-project or per-session copy, and turning it off is not presented as a fault (FR-012e, FR-012f, SC-005a)

### Implementation for User Story 3

- [X] T040 [US3] Implement `PiProvider::read_title()` in `crates/micold-core/src/provider.rs` as a bounded prefix read with the three-step fallback chain, never reading past the bound (FR-011, SC-006b)
- [X] T041 [US3] Implement `PiProvider::activity_source()` in `crates/micold-core/src/provider.rs` returning `ActivitySource::Extension { log: <base>/micold-activity/<session-id>.jsonl }` (FR-012, data-model §3)
- [X] T042 [US3] Create `crates/micold-daemon/assets/pi-activity.ts` — subscribe to the seven events in the contract's mapping table and append one `{"type":"…","at":"…"}` line each to `$MICOLD_PI_ACTIVITY_LOG`; no conversation content, no other destination, no tool, command, shortcut or flag, and the whole extent reviewable in this one file (FR-012b)
- [X] T043 [US3] Add `pi_event()` beside `copilot_event()` in `crates/micold-daemon/src/activity.rs` implementing the contract's mapping table, ignoring unknown types (FR-012, FR-019)
- [X] T044 [US3] Prepare the launch for `ActivitySource::Extension { log }` in `crates/micold-daemon/src/state.rs` — materialise `assets/pi-activity.ts` beside the session service, append `-e <that path>`, set `MICOLD_PI_ACTIVITY_LOG=<log>`, create the log's parent, and tail `log` with the existing `EventLogTail` unchanged; a failure to materialise warns and leaves the badge `Unknown` rather than failing the start (FR-012a, FR-012d, FR-014)
- [X] T045 [US3] Honour the FR-012e switch in `crates/micold-daemon/src/state.rs`: when it is off, omit the `-e` argument and the environment variable and open no tail, leaving the session otherwise identical — the provider still reports `Extension`, so there is no second code path (FR-012e, FR-012f)
- [X] T046 [US3] Add the switch field to `Settings` in `crates/micold-core/src/settings.rs` beside `default_ai_cli`, defaulting on and serde-compatible with files written before it existed (FR-012e)
- [X] T047 [US3] Add the switch's row and message to `crates/micold-client/src/features/settings.rs`, in the same Environment section as the default-CLI setting and built from the existing shared primitives (FR-012e, Principle VIII)
- [X] T048 [US3] Document the component in `docs/user-guide/settings.md`: that starting a Pi session loads a component of this application into Pi, exactly what it reports, that it never reads or transmits conversation content, that Pi grants extensions full system permissions, and the switch that declines it — with the unknown badge named as the expected consequence rather than a fault (FR-012c, FR-012e, FR-012f, FR-022)

**Checkpoint**: three CLIs are legible side by side, and the badge moves for Pi.

---

## Phase 6: User Story 4 - Find Pi conversations the application did not start (Priority: P4)

**Goal**: conversations `pi` recorded by hand for a location are listed as Pi sessions on project
open, and a session the user closes stays closed.

**Independent Test**: run `pi` by hand in a project directory, open the project, see the
conversation listed as a Pi session; close it, reopen the project, confirm it is not rediscovered.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [X] T049 [P] [US4] Add `recorded_session_ids()` coverage to `crates/micold-core/tests/pi_provider.rs`: every `.jsonl` conversation for a cwd is listed with no cap by count and no cut-off by age, a forked conversation is listed like any other with no parentage read, and non-`.jsonl` entries including `.archived` markers are ignored (FR-015)
- [X] T050 [P] [US4] Add suppression coverage to `crates/micold-core/tests/pi_provider.rs`: `mark_archived()` writes an empty `<session-id>.archived` beside the conversation, `is_archived()` reads it, and neither deletes or truncates the user's conversation (FR-016)
- [X] T051 [P] [US4] Extend `crates/micold-daemon/tests/session_discovery.rs`: conversations recorded for a location the application has no record of are listed as Pi sessions on every project open and reopen, at a cost proportional to the number of locations (FR-015, SC-006b)
- [X] T052 [P] [US4] Extend `crates/micold-daemon/tests/session_archive_durable_marker.rs`: a closed Pi session is not rediscovered after the application's own store is lost (FR-016)
- [X] T053 [P] [US4] Extend `crates/micold-core/tests/session_reconciliation.rs`: a discovered-but-unsupervised Pi session reads `Unknown` rather than idle or working, and causes no observation of its storage and no scheduled work (FR-013, FR-014, SC-006)

### Implementation for User Story 4

- [X] T054 [US4] Implement `PiProvider::recorded_session_ids()` in `crates/micold-core/src/provider.rs` — list the cwd's store directory, parse `<timestamp>_<uuid>.jsonl`, never open a conversation and never read `parentId` (FR-015)
- [X] T055 [US4] Implement `PiProvider::mark_archived()` and `is_archived()` in `crates/micold-core/src/provider.rs` against `<base>/sessions/--<encoded cwd>--/<session-id>.archived`, best-effort and never destructive (FR-016)

**Checkpoint**: all four stories independently functional.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T056 Run the recorded validation procedure in `specs/029-pi-cli-provider/quickstart.md` §A–§D end to end and record the §C result, which is the Principle I evidence for the spawn-wiring exception (`pi-activity.ts` and the spawn preparation)
- [X] T057 [P] Confirm `crates/micold-client/tests/no_concrete_implementations.rs` passes **unchanged** with three providers present — no session, storage, sidebar or terminal code names Pi (FR-019, SC-008)
- [X] T058 [P] Cross-cutting documentation review: index and navigation updates across `docs/`, and the link check in the same change (FR-022, SC-010)
- [X] T059 [P] Confirm no `cfg` arm was added for Pi and cross-check the other two platforms with `cargo check --target aarch64-apple-darwin` and the Windows target before pushing (Principle VI)
- [X] T060 Run the full local gate — `cargo fmt --check`, clippy, and `mise run test` — remembering the gate omits `fmt` by default and CI stops there first
- [X] T061 Verify the Success Criteria that are checked structurally rather than measured: SC-006 (zero polling timers, zero per-idle-session work), SC-006b (project-open cost independent of conversation length), SC-009 (a fourth CLI would be found by availability, image contents and the one-place guard without editing any of them)

---

## Phase 8: Bugfix — BUG-001 (availability follows the session environment)

**Goal**: A CLI is offered exactly when a session started on it would find it (FR-003b, SC-001a).
Every task applies to all providers; none names Pi (FR-019, FR-021). See `plan.md` §Bugfix
increment: BUG-001.

### Tests (write first, confirm they fail)

- [X] T062 [P] [BUG-001] [U1] [U2] *(test)* In `crates/micold-core/tests/`, pin that `available_in(path)` finds a command present only in a directory of the given `PATH` value, and not one that is only on the test process's own `PATH`. Build the directories in a tempdir; do not change the process environment (FR-003b)
- [X] T063 [P] [BUG-001] [A1] [A3] [U4] [U5] [U6] [U7] *(test)* Extend `crates/micold-daemon/tests/ai_cli_availability.rs`: with env-include enabled and a script that prepends a tempdir holding a fake `pi` to `PATH`, an `AiCliAvailabilityRequest` with that `cwd` lists Pi. The same request with env-include disabled does not. A second request is served without re-running the script (shared cache) (FR-003b, SC-006a)
- [X] T071 [P] [BUG-001] [A2] [U8] *(test)* In `crates/micold-daemon/tests/session_start.rs`: with `pi` only on the env-include `PATH` for a worktree, starting a Pi session there is not refused with the missing-CLI reason. `start_session`'s gate at `crates/micold-daemon/src/state.rs` (`!provider.is_available()`) walks the process `PATH` today, so a CLI offered under FR-003b would still be refused at launch (FR-003b, SC-001a). Added by `/speckit.tdd.plan`: the patch missed this second check
- [X] T072 [P] [BUG-001] [U9] [U10] *(test)* In `crates/micold-daemon/tests/ai_cli_availability.rs`: a request with no `cwd` runs the env-include script in the home directory; and a request whose script sleeps does not hold up a second request on the same connection (FR-003b)
- [ ] T073 [P] [BUG-001] [U11] [U12] *(test)* In `crates/micold-client/src/main_tests.rs`: opening the start menu for a location sends `AiCliAvailabilityRequest` with that location's directory, and opening Settings sends it with `cwd: None` (FR-003b)

### Implementation

- [ ] T064 [BUG-001] [U1] [U2] `crates/micold-core/src/provider.rs`: make `resolves_on_path` take the `PATH` value to walk, add `available_in(path: &OsStr)`, and thread the value through the provider trait's `is_available` for every provider. Keep `PATHEXT` handling and add no `cfg` arm (FR-003b, FR-020, Principle VI). Closes the reopened T019
- [ ] T065 [BUG-001] `crates/micold-core/src/protocol/messages.rs`: add `cwd: Option<PathBuf>` to `ClientMsg::AiCliAvailabilityRequest`, bump `PROTOCOL_VERSION` 13 → 14 in `crates/micold-core/src/protocol/version.rs`, and regenerate the schema hash
- [X] T066 [BUG-001] [A1] [A3] [U4] [U5] [U6] [U7] [U9] [U10] `crates/micold-daemon/src/server.rs` and `crates/micold-daemon/src/state.rs`: answer the request from the `PATH` in `env_include_vars_for(cwd.unwrap_or(home))`, falling back to the process `PATH`. Run it on `spawn_blocking` and never under the state lock, so the connection loop keeps serving other requests (FR-003b). Makes T063 and T072 pass
- [X] T074 [BUG-001] [A2] [U8] `crates/micold-daemon/src/state.rs` (`start_session`): check availability against the same spawn-environment `PATH` as T066, not the process's own. Makes T071 pass (FR-003b)
- [ ] T067 [BUG-001] [U11] [U12] `crates/micold-client/src/shell/daemon_sync.rs` (`ask_cli_availability`) and its callers: pass the project or worktree directory when the per-session override or missing-CLI list is opened, and `None` from Settings. Update `crates/micold-client/tests/cli_availability_comes_from_the_service.rs` and `crates/micold-client/src/main_tests.rs` for the new field
- [ ] T068 [P] [BUG-001] `docs/user-guide/settings.md`: state that the CLIs offered follow the session environment. Explain what to do when a version-manager install is not offered: keep env-include on, or put the CLI on the login `PATH`. Revise line 68's "a `PATH` that hasn't loaded" to match (FR-022)
- [ ] T069 [P] [BUG-001] ~~Find how `"env_include_script_path": "/tmp/does-not-exist.sh"` with `env_include_enabled: false` reached a real `~/.local/share/micold-ai-ide/settings.json` (grep tests, quickstarts and visual-pass scripts for the fixture path). Make the writer use a private data directory. Report the finding in `bugs/BUG-001.md`~~ **Moved out of BUG-001** (2026-09-19, bug-rubric review F3): a separate defect with prior evidence in `specs/010-daemon-session-persistence/bugs/BUG-025.md`; likely writer `crates/micold-client/src/main_tests.rs:1587`. Recorded under the ledger's *Follow-ups not done*. Not part of M1.

### Verification

- [ ] T075 [BUG-001] [A1] [A2] [A3] Outer loop closes: the acceptance behaviors A1–A3 are green through the daemon protocol before BUG-001 is considered fixed (US1 scenario 5b, SC-001a)
- [ ] T070 [BUG-001] Run `mise run gate` and `cargo check --target aarch64-apple-darwin`. Then run quickstart §B on Xvfb (`visual-pass` skill) with the daemon started on a `PATH` lacking `pi` and an env-include script that adds it. Confirm Pi is offered in Settings and the override, and that a session starts `pi` (SC-001a). Record the result in `evidence/`. The handoff tells the reporter to turn environment-include back on, since their settings have it off (BUG-001 §Steps to Reproduce)

**Checkpoint**: a `pi` installed under a Node version manager is offered and starts, with no change
to the user's installation.

**Bugfix**: 2026-09-18 — BUG-001 Updated from bugfix patch. **Requirements added**: FR-003b, SC-001a
and US1 scenario 5b; SC-006a amended — see `spec.md`. **One task reopened**: T019, which met its
text but walked the wrong `PATH`. **Tasks added**: T062–T070. See `bugs/BUG-001.md`.

**TDD plan**: 2026-09-19 — `/speckit.tdd.plan` added behavior markers to T062–T064, T066 and T067, and added T071–T075. See `tdd/test-list.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately
- **Foundational (Phase 2)**: depends on Setup — **blocks every user story**
- **US1 (Phase 3)**: depends on Foundational
- **US2 (Phase 4)**: depends on Foundational; T026's red arrives from T007 alone
- **US3 (Phase 5)**: depends on Foundational
- **US4 (Phase 6)**: depends on Foundational
- **Polish (Phase 7)**: depends on every story that is being shipped
- **Bugfix BUG-001 (Phase 8)**: depends on US1. T062/T063/T071/T072/T073 first (red), then T064 → T065 → T066 → T074 → T067; T068 in parallel; T069 moved out of BUG-001; T075 then T070 last

### User Story Dependencies

- **US1 (P1)**: independent. Delivers the feature on its own.
- **US2 (P2)**: independent of US1 in code — it touches only `packaging/sandbox/Containerfile` — but only observable once a Pi session can start, so demo it after US1.
- **US3 (P3)**: independent. Its provider methods (`read_title`, `activity_source`) are ones US1 does not implement, so the two phases touch different parts of `provider.rs`.
- **US4 (P4)**: independent. Its provider methods (`recorded_session_ids`, `mark_archived`, `is_archived`) are again disjoint from US1's and US3's.

The split of `PiProvider`'s trait methods across stories is deliberate: US1 takes the launch surface,
US3 the label and the activity source, US4 discovery and suppression. Each story therefore compiles
and tests on its own, and the file conflicts are limited to `crates/micold-core/src/provider.rs`
(different methods) and `crates/micold-core/tests/pi_provider.rs` (different sections).

### Within Each User Story

- Tests are written and FAIL before implementation (Principle I)
- Provider methods before the daemon wiring that calls them
- Daemon wiring before the client surfaces that render it
- The story's user-guide documentation lands in the same change (Principle VII)
- A story is done when its tests pass, its docs exist, and no `cfg` arm was added

### Parallel Opportunities

- T002 runs alongside T001's fixture authoring once the layout is fixed
- Phase 2's four test tasks (T003–T006) are four different files — all parallel
- US1's seven test tasks (T012–T018) are parallel; T012–T015 share `pi_provider.rs`, so write them as one pass or serialize those four
- US3's nine test tasks (T031–T039) are parallel apart from T034/T035, which share `pi_provider.rs`
- US4's five test tasks (T049–T053) are parallel apart from T049/T050, which share `pi_provider.rs`
- T042 (`pi-activity.ts`) has no Rust dependency and can be written any time after Phase 2
- T028 (image environment) is parallel to T027 (image packages)
- Once Phase 2 completes, US1, US3 and US4 can be worked by different people; US2 is one file and can go at any point

---

## Parallel Example: User Story 3

```bash
# Tests first — nine files, one pass:
Task: "Settings and override name Pi Coding Agent in crates/micold-client/tests/provider_choice_surfaces.rs"
Task: "Missing pi is reported by product name in crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs"
Task: "Row and tab read pi in crates/micold-client/tests/sidebar_tree.rs and terminal_tabs.rs"
Task: "Bounded-prefix label chain in crates/micold-core/tests/pi_provider.rs"
Task: "Pi event mapping in crates/micold-daemon/tests/pi_activity.rs"
Task: "Badge moves within the bound in crates/micold-daemon/tests/activity_pipeline.rs"
Task: "Switch defaults on and persists in crates/micold-core/tests/settings_ai_cli.rs"
Task: "One application-wide switch row in crates/micold-client/tests/features_settings.rs"

# Then the implementation, which is mostly serial through provider.rs and state.rs.
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1: Setup
2. Phase 2: Foundational — **blocks everything**
3. Phase 3: User Story 1
4. **STOP and VALIDATE**: [quickstart.md](./quickstart.md) §A and §B
5. A user can start, resume and hand off a Pi conversation to their own `pi`. That is the feature.

### Incremental Delivery

1. Setup + Foundational → three providers compile, the seam is exercised
2. + US1 → Pi sessions start and resume (MVP)
3. + US2 → Pi works in the sandbox, which must ship with US1 for sandboxed users
4. + US3 → the sidebar is legible and the badge moves
5. + US4 → conversations started outside the application show up

US2 is the one story with a delivery constraint of its own: the moment Pi is an offered CLI, a
sandboxed user who picks it gets a session that cannot start its agent unless the image carries `pi`
(spec, US2 rationale). Ship Phase 4 with Phase 3, even though the code is independent.

### Parallel Team Strategy

1. Everyone lands Setup + Foundational together — it is small and blocks all four stories
2. Then: Developer A takes US1 and US2, Developer B takes US3 (the largest), Developer C takes US4
3. The only shared files are `crates/micold-core/src/provider.rs` and
   `crates/micold-core/tests/pi_provider.rs`, and each story owns disjoint methods and sections of
   them

---

## Notes

- Build and test through `mise run <task>`; a bare `cargo` uses a different target directory
  (CLAUDE.md). Expect to queue behind another worktree's build and leave it that way.
- The feature adds **no new Rust crate and no new module tree** — which is itself the evidence for
  the spec's second purpose, that 026's seam holds for a CLI it was not designed around.
- Two seam changes and no more: `ActivitySource::Extension { log }` (T008) and `launch_env()` (T009).
  Both are available to every provider. Anything that needs a third is a signal to re-read FR-020
  before writing it.
- FR-005b's correspondence store is never built: `pi --session-id <id>` creates-if-missing, so one
  argument vector serves both launch modes (research R1).
- FR-006b's best-effort liveness check has no mechanism on Pi — Pi takes no lock and writes no
  marker — so FR-006c's silence is operative and T024 is where that is said out loud (research R2).
- Commit after each task or logical group; stop at any checkpoint to validate a story on its own.
