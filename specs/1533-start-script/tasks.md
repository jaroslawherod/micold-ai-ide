# Tasks: An optional start script prepares each daemon (#1533)

**Input**: `specs/1533-start-script/` (spec.md, plan.md, research.md, data-model.md, contracts/startup-run.md, quickstart.md). Parent design `specs/daemon-config-file/` (parent tasks T068 to T077 stay there; this file supersedes them for #1533).
**Tests**: mandatory (Constitution I): each implementation task has its failing-test task earlier in the same milestone.
**Format**: `- [ ] T### [P?] [US?] description with file path`. `[P]` = different files, no dependency on an unfinished task.

## Phase 1: Setup

- [x] T001 Add the empty module `crates/micold-daemon/src/startup_script.rs` (registered in `crates/micold-daemon/src/lib.rs`) and the test file `crates/micold-daemon/tests/startup_script.rs` with a helper that starts a daemon on a temp data dir with a given `daemon.toml` and script (reuse the helpers of `tests/config_wiring.rs` / `tests/config_load.rs`; grep before writing new ones)

## Phase 2: Foundational (core, blocks all stories)

- [x] T002 [P] Failing tests in `crates/micold-core/src/env_include.rs` for `run_and_diff(script, shell, cwd, timeout, extra_env)`: explicit exports captured; a value with a newline captured intact; `extra_env` names visible to the script as unexported variables, absent from the diff unless the script runs `export NAME`, values literal (`$`, quotes, comments, blank lines never evaluated); timeout terminates the process group and a backgrounded child holding the pipe does not extend the run; output streamed line by line; `resolve` keeps its existing tests; a PowerShell case (`#[cfg(windows)]`) captures `$env:NAME` by before/after dump and the timeout ends the job
- [x] T003 [P] Failing protocol tests in `crates/micold-core/src/protocol/` (tests beside `messages.rs`): `StartupReport` (`outcome` Ok|Skipped|Warned|Failed, `reason`, `tail` at most 200 lines of at most 4096 bytes, lossy UTF-8, no NUL, `finished_at_secs`) round-trips; `ConfigStatus.startup: Option<StartupReport>` round-trips and is `None` by default; `tests/schema_hash.rs` recomputes after the bump
- [x] T004 Implement `run_and_diff` in `crates/micold-core/src/env_include.rs` (streaming variant using `StreamingChild::terminator()` and a watchdog thread; `resolve` stays on `run_bounded` per research R4 and ledger D3; reuse `run_bounded`/`Terminator` from `crates/micold-core/src/process.rs`)
- [x] T005 Add `StartupReport` and `ConfigStatus.startup` in `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` by one in `crates/micold-core/src/protocol/version.rs` (rebase and take the next number if a delivery issue merged first)
- [x] T006 Carry `startup` (initially `None`) through `config_status` in `crates/micold-daemon/src/state.rs`

## Phase 3: User Story 1 - A start script runs before the daemon serves (P1) 🎯 MVP

**Goal**: a configured script runs before the listener binds and its explicit exports reach the daemon and sessions.
**Independent test**: script exporting `FOO=bar` and extending `PATH`; a new session sees both; no connection is accepted during the run.

- [x] T007 [US1] Failing integration tests in `crates/micold-daemon/tests/startup_script.rs`: exports reach the daemon and a spawned session (US1.1, US2.4 PATH); nothing accepted before the script ends (SC-004, US1.1); `export_env = false` exports nothing (US1.3); no `[startup]` table runs nothing and changes nothing (US1.4); script runs with the data dir as cwd and the output appears in the log under target `startup`; `startup-result.json` written `Ok` with a fresh `finished_at_secs`
- [x] T008 [US1] Implement `StartupPlan` (from `Startup` + kind defaults: shell `/bin/sh` or `powershell.exe`, timeout default 300 s within 1..=3600, `export_env`) and the happy-path runner in `crates/micold-daemon/src/startup_script.rs`: create `<data_dir>` `0700`, call `run_and_diff` with output to the log file and stderr directly (no subscriber yet) and a 200-line ring, drop `MICOLD_*`, `TERM`, `COLORTERM`, `TZ`/`HOME` when configured and the terminal-identity filter, apply exports with `set_var` once, write `startup-result.json` (temp file + rename)
- [x] T009 [US1] Run the plan in `crates/micold-daemon/src/main.rs` after `align_container_home` and before the runtime is built (contract S1), only when `[startup]` is present

**Checkpoint (M1)**: T001 to T009.

## Phase 4: User Story 2 - Fixed variable order; a secret only if exported (P1)

**Goal**: `env_file` values reach sessions only when exported; the precedence is fixed.
**Independent test**: `TOKEN=x` in `env_file`: absent unless the script runs `export TOKEN`; env-include beats the script.

- [x] T010 [US2] Failing integration tests in `crates/micold-daemon/tests/startup_script.rs`: `TOKEN` not in the daemon or a session when only read (US2.1, SC-003); present when `export TOKEN` (US2.2); full precedence daemon env incl. exports, environment-include, `ai.cli[].env`, `session.env`/`secret_env`, session vars, with env-include beating the script (US2.3); a script exporting a per-session variable such as `MICOLD_*` loses (US2.5)
- [x] T011 [US2] Read `env_file` in `crates/micold-daemon/src/startup_script.rs`, pass its values as `extra_env`, write `startup-env.json` (names only, `0600`, after success) and remove the transient `startup-env.dump` (`0600`)
- [x] T012 [US2] Make T010's precedence tests pass: fix the order where the daemon builds a session environment (`crates/micold-daemon/src/terminal.rs` / `structured.rs` spawn paths, wherever T010 shows the order wrong) without new merge code beyond the exports filter

**Checkpoint**: T010 to T012 belong to M2.

## Phase 5: User Story 3 - `run` and `on_failure` behave as specified (P2)

**Goal**: once vs every start, refuse vs warn, timeout, reload never re-runs.
**Independent test**: `once_per_container` runs once across two restarts and again after an edit; `refuse` leaves the daemon down, `warn` serves.

- [x] T013 [US3] Failing tests in `crates/micold-daemon/tests/startup_script.rs` and unit tests in `startup_script.rs`: `startup.done` hash (SHA-256 of length-prefixed shell path, script bytes, `env_file` bytes or absent tag) skips only while unchanged (US3.1, `Skipped`); a failed run is not recorded (US3.2); `refuse` on non-zero exit or timeout: daemon never listens, exits 1 with the S3 block, `startup-result.json` is `Failed` with `Exit(1)` and tail `oops` (US3.3, US4.3 log); `warn` serves, result `Warned`, exports discarded; timeout kills script and children (US3.4); missing script or `env_file` is `FileMissing` handled by `on_failure`; a stale `startup-result.json` is deleted first; a second start waits on `startup.lock`; a reload or changed script never re-runs and adds `startup` to `changed_restart_keys` (US5.2 daemon side, SC-005); tail bounds; an empty script succeeds; `[startup]` with `env_file` but no `script_file` is refused; NUL and binary bytes in output are sanitised; `startup-result.json` and `startup.done` are `0600`
- [x] T014 [US3] Implement `startup.done` hash record (write only after success, temp file + rename), `startup.lock` (blocking; `LockFileEx` on Windows), `Skipped` in `crates/micold-daemon/src/startup_script.rs`
- [x] T015 [US3] Implement the failure policy in `crates/micold-daemon/src/startup_script.rs` and `main.rs`: `Failed`/`Warned` results, reasons `Exit`/`Signal`/`Timeout`/`Spawn`/`FileMissing`, the S3 stderr block, `Err` from `start()` on `refuse`
- [x] T016 [US3] `crates/micold-daemon/src/config_watch.rs`: put a changed `[startup]` table or script/`env_file` hash into `restart_keys` (key `startup`/`startup.script_file`) and fill `ConfigStatus.startup` from the result; `config check` stays unchanged
- [x] T017 [US3] User guide: describe the start script, its relation to environment-include, installs to the user's home not root, and refusal/warn in `docs/user-guide/daemon-configuration.md` (host kind); document the files in `docs/daemon.md`; state that the script output (and so `startup-result.json`) may contain secrets the script prints, and that on Windows export capture works for PowerShell only

**Checkpoint (M2)**: T010 to T017.

## Phase 6: User Story 5 (P2) - Settings → Daemons edits the script and says a restart is needed

**Goal**: the user edits `[startup]` in Settings; invalid values are refused; a restart notice shows.
**Independent test**: inline script saved as a file, `[startup]` rendered with its `script_file`; timeout 0 refused.

- [x] T018 [P] [US5] Failing tests in `crates/micold-core/src/daemons.rs`, `crates/micold-client/src/daemon_config_render.rs` and `crates/micold-client/src/ui/settings/daemons.rs`: the group appears only for kinds whose delivery is enabled (none in M3); restart notice shown when `changed_restart_keys` contains `startup`; geometry-gate case for the group; inline script persisted as `<app data>/daemons/<id>/startup.sh` (text) while `daemons.json` holds only the `Inline` marker; `StartupSettings` serde defaults match the schema (timeout none, `refuse`, `every_start`, `export_env` true); `[startup]` renders with the delivered `script_file`; timeout outside 1 to 3600 and unknown policies refused with the reason before delivery (US5.3)
- [x] T019 [US5] Add `StartupSettings` and `StartupScript` (`File(PathBuf)` | `Inline`) on the daemon entry of all four runtime kinds in `crates/micold-core/src/daemons.rs`
- [x] T020 [US5] Render `[startup]` and save the inline file in `crates/micold-client/src/daemon_config_render.rs`; validate with `daemon_config::validate`
- [x] T021 [US5] "Start script" group in `crates/micold-client/src/ui/settings/daemons.rs`: file picker or inline `TextArea` (add a builder to `micold-material` only if the building block is missing, Principle VIII), shell, timeout, `on_failure`, `run`, `env_file`, `export_env`, validation message; render it only for kinds where `start_script_delivered(kind)` is true, false for every kind in M3 (rule 6: M4, M5, M6 each enable their kind), so no savable group sits on main without delivery
- [x] T022 [US5] "Restart to apply" notice from `ConfigStatus.changed_restart_keys` containing `startup` in `crates/micold-client/src/ui/settings/daemons.rs` (US5.2 client side)
- [x] T023 [US5] (ships in M4, when the group first becomes reachable) User guide: the Start script group in `docs/user-guide/daemon-configuration.md`

**Checkpoint (M3)**: T018 to T022 (T023, the guide, ships in M4). The group is built and tested but hidden for every kind until M4 to M6 enable it.

## Phase 7: User Story 4 and delivery on host and SSH (P1; needs #1526)

**Goal**: script and `env_file` are delivered with the config; a failure shows in the app with its output.
**Independent test**: `echo oops; exit 1` with `refuse` on a host and an SSH daemon: the app's error state shows exit 1 and "oops"; with `warn` a warning with the same text.

- [x] T024 [P] [US4] Failing tests: `crates/micold-core/src/spawn.rs` (host spawn reads `startup-result.json` on a failed start and rejects one older than the spawn); `crates/micold-daemon/src/remote.rs` (bridge relays the failure block and the file); `crates/micold-client/src/shell/daemon_sync.rs` (script and `env_file` delivered beside the config for host `<data_dir>/daemons/<id>/startup.sh` and SSH `~/.micold/remote/config/startup.sh`; no script text or secret in the config file, US5.4); `crates/micold-client/src/ui/daemon_details_dialog.rs` (failure and warning rows with exit status, tail, "Show output")
- [x] T025 [US4] Post-spawn wait loop in `crates/micold-core/src/spawn.rs` that reads the result file after a failed start (US4.1 host)
- [x] T026 [US4] Relay the failure block and file in `crates/micold-daemon/src/remote.rs` (US4.1 SSH)
- [x] T027 [US4] Deliver script and `env_file` for host and SSH in `crates/micold-client/src/shell/daemon_sync.rs` (FR-011, US5.4) and enable the Start script group for host and SSH in `crates/micold-client/src/ui/settings/daemons.rs`
- [x] T028 [US4] Failure and warning display in `crates/micold-client/src/ui/daemon_details_dialog.rs` (US4.1, US4.2)
- [x] T029 [US4] User guide: delivery and failure display for host and SSH in `docs/user-guide/daemon-configuration.md`

**Checkpoint (M4)**: T024 to T029, after #1526 merges.

## Phase 8: Container delivery (needs #1527)

- [x] T030 [P] [US4] Failing tests in `crates/micold-client/src/shell/daemon_sync.rs` (script under `/run/micold/config`, `env_file` under `/run/micold/secrets`) and a `mise run test-sandbox` case (script runs as the daemon's own user, never root, at boot and over SSH; `HOME` equals `MICOLD_HOME`; failure visible, US1.2, US4.1)
- [x] T031 [US4] Deliver script and `env_file` for the container kind in `crates/micold-client/src/shell/daemon_sync.rs` and enable the group for it
- [x] T032 [US4] Container entrypoint start path and error display for a refused container daemon (`packaging/basic/entrypoint.sh` and `crates/micold-daemon/src/remote.rs`)
- [x] T033 [US4] User guide: container delivery and installs under `~/.local` in `docs/user-guide/sandboxed-daemon.md`

**Checkpoint (M5)**: T030 to T033, after #1527 merges.

## Phase 9: Pod delivery (needs #1528)

- [x] T034 [P] [US4] Failing tests in `crates/micold-client/src/shell/daemon_sync.rs` (script and `env_file` through the pod's config and secret volumes) and a failure-display test for a pod daemon
- [x] T035 [US4] Deliver script and `env_file` for the pod kind in `crates/micold-client/src/shell/daemon_sync.rs`, enable the group for it; error display for a refused pod daemon
- [x] T036 [US4] User guide: pod delivery in `docs/user-guide/kubernetes-daemons.md` (the pod page) and the table in `daemon-configuration.md`

**Checkpoint (M6)**: T034 to T036, after #1528 merges.

## Phase 10: Polish

- [ ] T037 Run quickstart Part A (all kinds delivered) and Part B (`visual-pass`), record results in `specs/1533-start-script/quickstart.md`; tick tasks. No code: the close unit does it.

## Dependencies and order

Phase 2 blocks everything. US1 (M1) then US2 and US3 (M2). M3 needs M2 (restart key). M4 needs M2, M3, #1526; M5 needs #1527; M6 needs #1528. If a prerequisite is still open when its turn comes, later milestones that do not depend on it go first.
Parallel: T002 with T003. T007, T010, T013 share `tests/startup_script.rs` and are not parallel.

## Implementation Strategy

MVP is M1 (script runs before serving, exports applied, observable by the daemon integration test). Then M2 (secrets, precedence, policies, reload), M3 (Settings), then one delivery milestone per kind as its prerequisite merges.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Core runner and a script that runs before serving 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: a daemon with a `[startup]` table runs the script before it listens and applies its explicit exports to the daemon and its sessions; `StartupReport` and `ConfigStatus.startup` exist in the protocol.
- **Satisfies**: US1 acceptance scenarios 1, 3, 4; US2.4; US4.3; FR-002 (host path), FR-003 (exports), FR-006 (timeout in `run_and_diff`), SC-004
- **Verify**: `mise run test-core`; `cargo test -p micold-daemon --test startup_script` (via `mise run gate`)
- **Depends on**: — (needs only #1524, closed)
- **Tier**: full

### M2 — Secrets, precedence and run/failure policies

- **Tasks**: T010–T017
- **Deliverable**: `env_file` values stay out of sessions unless exported; the variable order is fixed; `once_per_container`, `refuse`, `warn` and timeout behave as specified; a reload never re-runs and reports `restart_needed`; the user guide describes the host start script.
- **Satisfies**: US2 acceptance scenarios 1–5; US3 acceptance scenarios 1–4; US4.3; US5.2 (daemon side); FR-003 to FR-009, FR-012 (host); SC-003, SC-005
- **Verify**: `cargo test -p micold-daemon --test startup_script`; quickstart Part A steps 1–6
- **Depends on**: M1
- **Tier**: full

### M3 — Settings → Daemons Start script group

- **Tasks**: T018–T022
- **Deliverable**: the Start script group, `[startup]` rendering, validation and restart notice exist and are covered by tests and geometry gates; the group is hidden per kind until M4 to M6 deliver the script (no half-wired UI on main).
- **Satisfies**: US5 acceptance scenarios 1–3, US5.2 (client side); FR-010
- **Verify**: client tests and geometry gates in `mise run gate` (group shown in the showcase with delivery forced on)
- **Depends on**: M2 (T022 reads the restart key T016 reports); #1524 closed
- **Tier**: full

### M4 — Host and SSH delivery and failure display

- **Tasks**: T023–T029
- **Deliverable**: the script and `env_file` reach host and SSH daemons with the config; a refused or warned start shows its exit status and output tail in the app.
- **Satisfies**: US4 acceptance scenarios 1–3 (host, SSH); US5.4 (host, SSH); FR-008, FR-011 (host, SSH); SC-001 (host, SSH)
- **Verify**: `cargo test -p micold-client`, `-p micold-core`, `-p micold-daemon` (via `mise run gate`); quickstart Part A steps 5 and 7, Part B step 2
- **Depends on**: M2, M3, #1526
- **Tier**: full

### M5 — Container delivery

- **Tasks**: T030–T033
- **Deliverable**: a container daemon runs the delivered script as its own user at boot and over SSH, and a failure is visible in the app.
- **Satisfies**: US1.2; US4.1 and US5.4 (container); FR-011 (container); SC-001 (container)
- **Verify**: `mise run image` then `mise run test-sandbox`
- **Depends on**: M2, M3, #1527
- **Tier**: full

### M6 — Pod delivery

- **Tasks**: T034–T036
- **Deliverable**: a pod daemon receives the script and `env_file` through its volumes and a failure is visible in the app.
- **Satisfies**: US4.1 and US5.4 (pod); FR-011 (pod); SC-001 (pod)
- **Verify**: client tests in `mise run gate`; quickstart Part A step 7 (cluster check)
- **Depends on**: M2, M3, #1528
- **Tier**: full

Not a milestone: T037 (quickstart results, task ticks) changes no code and is left to the close unit. The user-guide tasks sit in M2 to M6 so the guide gate passes in each `feat` PR; M1 changes no user-visible behaviour beyond tests and protocol, so it carries the `docs-not-needed` label.
