# Implementation Plan: Terminal History That Survives a Session Service Restart

**Branch**: `feat/terminal-scrollback-persistence` | **Date**: 2026-10-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/041-terminal-scrollback-persistence/spec.md`

## Summary

A session's AI CLI terminal keeps its history when its process is started again: after a stop and
start, after the process exited, and after the session service itself restarted. The earlier output
is shown above one line reading "session restarted at …". The session service saves each such
terminal's history to an owner-only file in its local data directory, at most once per 30 seconds
and at every exit and orderly stop. One setting in Settings → Terminal turns saving off and deletes
what was saved. Removing a session removes its file.

The design rests on six decisions:

1. **A history snapshot is logical lines with style runs, taken from the terminal grid**
   ([R2](./research.md#r2-what-is-captured)). It is not the PTY byte stream and not the wire
   schema.
2. **A snapshot is shown again by seeding the new terminal before its process starts**
   ([R3](./research.md#r3-how-a-snapshot-is-shown-again)). Seeded lines are ordinary history, so
   scrolling, the scrollback limit, later saves and every window work with no client change.
3. **One mechanism serves a start in the same service run and a start after a service restart**
   ([R4](./research.md#r4-one-mechanism-for-a-start-in-the-same-run-and-after-a-service-restart)).
   The service carries the snapshot of an ended process in memory; the file is read only when
   nothing is carried.
4. **One file per session, rewritten whole through a rename, with a version and a checksum**
   ([R5](./research.md#r5-the-saved-history-file)), in the *local* data directory
   ([R7](./research.md#r7-where-the-files-are)), owner-only from creation
   ([R8](./research.md#r8-only-the-user-can-read-them)).
5. **A pure schedule decides when to save; one store owns the directory**
   ([R6](./research.md#r6-when-a-save-happens),
   [R9](./research.md#r9-removing-sweeping-and-turning-saving-off)). The store's mutex orders a
   save against a removal and against the setting being turned off.
6. **The service learns that it is asked to stop**
   ([R14](./research.md#r14-what-makes-a-stop-orderly)). Today only the idle stop unwinds; a
   signal on Unix and a named event on Windows now lead to the same unwind, which saves first.

The setting is a service-owned setting like `pi_activity_component`
([R10](./research.md#r10-the-setting)). The sandbox needs a mount on Windows hosts, the host's time
zone, and a rule for containers made before this feature
([R7](./research.md#r7-where-the-files-are), [R11](./research.md#r11-the-separator-and-the-notice),
[R15](./research.md#r15-the-sandbox-on-a-windows-host-and-a-container-made-before-this-feature)).
The feature is scoped to output on the normal screen
([R16](./research.md#r16-ai-clis-that-draw-on-the-alternate-screen), ledger D11).

## Technical Context

**Language/Version**: Rust, edition 2021, workspace MSRV (unchanged)

**Primary Dependencies**: `alacritty_terminal` 0.26 (`Term`, `vte::ansi::Handler`), `postcard` and
`micold_core::protocol::hashing::sha256` for the file, `directories` for the location, `tokio`
(blocking pool; the `signal` feature is added to the workspace dependency), `windows-sys` (already
a dependency of all three crates) for the stop event and the end-of-session window, `chrono`
(already in `Cargo.lock` through `file-rotate`; becomes a direct dependency of `micold-daemon`),
`iana-time-zone` (already in `Cargo.lock`; becomes a direct dependency of `micold-client` for the
launcher). **No crate new to `Cargo.lock`.**

**Storage**: new directory `<data_local_dir>/terminal-history/`, one file `<session uuid>.history`
per session ([contracts/saved-history-file.md](./contracts/saved-history-file.md)). `settings.json`
gains `save_terminal_history` (additive, `#[serde(default)]` = true; `SETTINGS_VERSION` unchanged).

**Testing**: `mise run test-core` for the format, the schedule, the store and the texts;
`mise run gate` for the workspace; daemon integration tests under `crates/micold-daemon/tests/`;
client tests under `crates/micold-client/tests/`; `mise run test-sandbox` for the container;
`quickstart.md` Part B for the real terminal and the real CLIs.

**Target Platform**: Linux, macOS, Windows desktop, and the sandbox container (Linux) on each.
New `cfg` arms: the stop request (`platform/unix.rs`, `platform/windows.rs`) and the moved
owner-only helper.

**Project Type**: desktop application (three-crate Cargo workspace).

**Performance Goals**: restoring 10,000 lines adds at most 1 s to a start (FR-013, SC-004); with
ten terminals printing, the 95th-percentile echo delay grows by at most 20 ms (SC-005); at most 21
writes per terminal in 10 minutes (SC-003).

**Constraints**: nothing read from a file may reach a process or be interpreted as an escape
sequence (FR-009, FR-016); no file outside the local data directory (FR-019); no file readable by
another account at any moment (FR-020); a save never holds the state lock, and holds the terminal
lock only to copy rows.

**Scale/Scope**: one new core module (`terminal_history`) and one moved helper (`owner_only`); one
new daemon module (`history`) and edits to `state.rs`, `supervisor.rs`, `server.rs`, `platform/`;
one settings field with one protocol bump; one Settings control; launcher and image changes for the
sandbox; the user guide's sessions, settings and sandbox chapters, `docs/daemon.md` and the
architecture page.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. The file format, the damage checks, `SaveSchedule`,
  `HistoryStore`, the separator and notice texts are pure code in `micold-core`, tested before any
  caller exists. Capture and seed are tested against a real `Term` with no process. Each acceptance
  scenario that needs a process has a daemon integration test with a fake CLI, written first. The
  GUI exception is claimed only for the one Settings control's composition, covered by the layout
  gates and quickstart Part B. `speckit.tdd.plan` derives `tdd/test-list.md`.
- [x] **II. Multi-Session Support**: PASS. Everything is keyed by session id: the carried snapshot,
  the schedule, the file name. A save of one terminal runs off its lock and never waits on another
  (edge case *Several busy sessions*). Tests start two sessions and check each sees only its own
  history (FR-025), and that a damaged file of one leaves the other whole (FR-018).
- [x] **III. Worktree Integration**: PASS. No worktree is created, changed or removed. Deleting a
  worktree or forgetting a project removes the saved histories of its sessions through the path all
  archiving already shares (R9, FR-023). A worktree's files are not touched.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Histories are files on this computer, in
  the local (not roaming) profile, owner-only, never sent anywhere (FR-019, FR-020). There is no
  network path in the feature. The user can turn saving off, which deletes the files (FR-027).
- [x] **V. Rust + iced Stack**: PASS. Rust only; no crate new to `Cargo.lock`. Types narrow the
  states: `LoadOutcome` is `None | History | Damaged`, so a partly read file is unrepresentable;
  `Seed` is `None | History | Notice`, so a damaged history can never be seeded with a separator.
- [x] **VI. Cross-Platform Parity**: PASS. The file bytes are the same everywhere (FR-022). The
  per-OS parts each have both arms and tests CI runs on all three systems: owner-only writing
  (existing `platform` tests move with the helper), the data directory (R7), the stop request
  (signals on Unix, a named event and `WM_ENDSESSION` on Windows, R14). One difference stays and is
  recorded under Risks: a real Windows logout cannot be exercised in CI.
- [x] **VII. Documentation First-Class**: PASS. The user guide is updated in the milestone that
  ships each behaviour (FR-032): the restored history and the separator, the full-screen CLI limit
  (D11), the setting, the damaged-history line, removal, the sandbox notes (R15). `docs/daemon.md`
  and `docs/development/architecture.md` get the stop request and the history module.
- [x] **VIII. Reusable UI Component Foundation**: PASS. The control is the shared `Checkbox` with
  `field_note`, as in `ui/settings/environment.rs` (FR-031). No new component, no widget styled in
  place. The separator and the notice are terminal content, not UI components.

### Re-check after Phase 1 design

All eight still **PASS**. Recorded rather than waved through:

- **The saved-history file is a new persistence format.** It has its own version, independent of
  the wire protocol, and a pinned fixture; a file of another version is treated as damaged, never
  migrated ([contracts/saved-history-file.md §5](./contracts/saved-history-file.md)).
- **A signal handler is new process-level behaviour.** `SIGTERM`, `SIGINT` and `SIGHUP` now unwind
  instead of ending the process at once. The unwind is bounded (3 s for the saves), so a stop is
  never held longer than that ([contracts/stop-request.md](./contracts/stop-request.md)).
- **The setting changes the wire schema**, so `SCHEMA_HASH` changes and builds of different
  versions refuse each other at the handshake, as intended. One bump, in the milestone of story 2.
- **One style flag beyond FR-001's list is saved: hidden (SGR 8).** Without it, text a program
  printed concealed would be restored readable.

No entry in Complexity Tracking.

## Requirement → design map

| Requirement | Where |
|---|---|
| FR-001 | `capture` of the grid into `HistorySnapshot`; colours and flags of [data-model §1](./data-model.md); the limit is the `Term`'s own history size (R2) |
| FR-002 | Capture points at process end (R4) save through `HistoryStore::save`; the save step of `unwind`, reached by the idle stop and by the stop request ([contracts/stop-request.md](./contracts/stop-request.md), R14) |
| FR-003, FR-004 | `SaveSchedule::due` and the 5 s saver tick ([data-model §4](./data-model.md), R6); `HistoryStore::save` writes nothing when the content equals the last written file, so a save at a process end or a service stop of an unchanged terminal causes no write ([data-model §5](./data-model.md)) |
| FR-005 | Capture copies rows under the `Term` lock; encode, hash and write run on the blocking pool; the saver never takes the state lock while writing (R6) |
| FR-006 | Whole-file write to a temporary file, synced, then renamed; the directory synced on Unix ([contracts/saved-history-file.md §3](./contracts/saved-history-file.md), R5) |
| FR-007 | A failed save leaves the schedule due after 30 s; `logged: HashSet<(SessionId, reason)>` in the saver ([data-model §4, §6](./data-model.md)) |
| FR-008, FR-015 | The capture at a process end runs after the reader thread was joined, on every platform (R4, [data-model §6](./data-model.md)); `Seed::History` built from `carried` or from `HistoryStore::load`; `seed` writes it into the new `Term` before the reader thread exists (R3, R4, [data-model §6](./data-model.md)) |
| FR-009 | `separator_line(time, columns)` in the dim style; seeded through `Handler`, so nothing is written to the PTY ([data-model §7](./data-model.md), R11) |
| FR-010 | `Seed::None` when there is no snapshot or it has no lines |
| FR-011, FR-012 | Seeded lines are the `Term`'s history: the `Term` trims to its limit, and the next capture includes them; `seed` skips lines beyond the limit first (R3) |
| FR-013 | `load` + `seed` measured by a timing test with 10,000 lines (R12) |
| FR-014 | Only `SessionProcess::Primary` of a `TerminalMode::AiCli` session is captured, carried or saved; `spawn_shell` passes no seed (R4) |
| FR-016, FR-017, FR-018 | `decode` never returns part of a file; `LoadOutcome::Damaged(reason)` → `Seed::Notice`, one `warn!` (reaches *Session service diagnostics* through `RecentErrorsLayer`), and the schedule marked due so the next save replaces the file ([contracts/saved-history-file.md §4](./contracts/saved-history-file.md), [data-model §3](./data-model.md)) |
| FR-019 | `history_dir()` = `data_local_dir()/terminal-history` (R7); the history mount on Windows hosts ([data-model §9](./data-model.md)) |
| FR-020 | `micold_core::owner_only::{write, ensure_dir}` (R8) |
| FR-021, FR-022 | The state mount on Linux and macOS hosts, the history mount on Windows hosts; identical file bytes; the launcher creates the directory, the service in a container never does (R7, R15) |
| FR-023 | `HistoryStore::forget(ids)` in `DaemonState::revoke_tool_credentials`, before the handler replies; `carried` entries dropped in `remove_live_by_ids` (R9) |
| FR-024 | `HistoryStore::sweep(keep)` at service start with saving on (R9) |
| FR-025 | The file name is the session id; `carried` is keyed by it; a file whose name is not a known id is swept |
| FR-026, FR-031 | `Checkbox` with `field_note` in `ui/settings/terminal.rs`; text of [contracts/setting.md §4](./contracts/setting.md) |
| FR-027, FR-028, FR-033 | `HistoryStore::set_enabled`; `purge` at start with saving off; retry of failed deletions on the saver tick; every running schedule marked due when turned on ([contracts/setting.md §3](./contracts/setting.md), R9) |
| FR-029 | `Settings.save_terminal_history`, `#[serde(default)]` = true ([contracts/setting.md §1](./contracts/setting.md)) |
| FR-030 | Constitution VI above; `cfg` tests on all three systems |
| FR-032 | User-guide tasks in each story's milestone (Delivery order, below) |

## Test strategy by layer

| Layer | What it holds | Requirements |
|---|---|---|
| `micold-core` unit (`mise run test-core`) | Format round trip; each damage case of [contracts/saved-history-file.md §4](./contracts/saved-history-file.md); the pinned fixture; `SaveSchedule` (spacing, idle, 21 writes in 10 minutes, retry, marked due); separator and notice text at each width; `HistoryStore` on a temp directory (save, load, forget against a save in flight, `set_enabled(false)` with a failing deletion, sweep, purge, refusal in a container without the directory, modes `0700`/`0600`); `owner_only` on each OS; settings round trip and default; protocol round trip and schema pin; `MountSet` with and without the history mount; `TZ` in the container arguments | FR-003, FR-004, FR-006, FR-007, FR-009, FR-010, FR-016, FR-019, FR-020, FR-023 to FR-025, FR-027 to FR-029, FR-033, SC-003, SC-009 |
| `micold-daemon` unit | `capture` from a `Term` (16, 256 and RGB colours, each flag, wrapped lines, wide and zero-width characters, the last-row rule); `seed` into a `Term`; capture after seed equals the input; seed at a narrower width; seed longer than the limit; a seeded text can hold no control character | FR-001, FR-009, FR-011, FR-012, edge cases *size changed*, *limit changed* |
| `micold-daemon` integration (`tests/`, a `Catalog` on a temp directory, a fake CLI) | Every acceptance scenario of stories 1 to 4 that needs a process: stop and start, exit and restart, a second `DaemonState` on the same directories as a service restart, two sessions, a killed service (no unwind), the idle stop, the stop request (`SIGTERM` on Unix, the event on Windows), a damaged and an unreadable file, each removal path, removal during a save, the setting both ways, a shell instance never saved, a fake CLI that prints `ESC[2J`, a fake CLI on the alternate screen (R13, R16). Patterns: `tests/daemon_lifecycle.rs`, `tests/idle_stop.rs`, `tests/reattach_snapshot.rs` | stories 1 to 4, FR-002, FR-005, FR-008, FR-014, FR-015, FR-017, FR-018, FR-023, FR-027, FR-028, FR-033, SC-001, SC-002, SC-006 to SC-008, SC-011 |
| `micold-daemon` timing tests | 10,000 lines loaded and seeded in under 1 s; ten terminals printing, echo delay with saving on against off | FR-005, FR-013, SC-004, SC-005 |
| `micold-client` reducer and gates | Settings draft, message and persist (`tests/features_settings.rs`, `tests/settings_sections.rs`); the layout snapshot of the Terminal section regenerated; `material_builder_api`; the launcher's mount set and directory creation (`shell/sandbox.rs`) | FR-026, FR-029, FR-031, FR-021 |
| Sandbox real-runtime suite (`mise run test-sandbox`) | A history saved by a host service is restored by a container service and the reverse; file modes seen from the host; `TZ` reaches the container; `<runtime> stop` saves before the container ends | FR-021, FR-022, SC-009 |
| quickstart Part B (`visual-pass`) | The separator and the notice in the real terminal in both themes; the Settings control; Pi and `CLAUDE_CODE_NO_FLICKER=0 claude` after a real service restart; what default Claude Code and Copilot show; **Restart service** from the app | FR-008, FR-009, FR-017, FR-026, FR-031, SC-010 |
| Documentation (CI's user-guide gate, review B) | The guide chapters updated in the milestone that ships each behaviour | FR-032 |

FR-030 is held by CI running the first four layers on Linux, macOS and Windows.

## Project Structure

### Documentation (this feature)

```text
specs/041-terminal-scrollback-persistence/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── saved-history-file.md      # location, bytes, write rule, damage checks, version rule
│   ├── setting.md                 # the stored field, the wire, what on and off do, the text
│   └── stop-request.md            # signals, the Windows event and window, the save in unwind
└── tasks.md                       # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/terminal_history/          # NEW module
│   ├── mod.rs                     # HistorySnapshot, LogicalLine, StyleRun, HistoryStyle, HistoryColor
│   ├── format.rs                  # encode, decode, DamageReason, FORMAT_VERSION
│   ├── schedule.rs                # SaveSchedule
│   ├── store.rs                   # HistoryStore, LoadOutcome, history_dir()
│   └── text.rs                    # separator_line, notice_line
├── src/owner_only.rs              # NEW: write, ensure_dir (moved from micold-daemon's platform)
├── src/settings.rs                # Settings.save_terminal_history
├── src/protocol/messages.rs       # DaemonSettings and ClientMsg::SettingsSet gain the field
├── src/protocol/version.rs        # PROTOCOL_VERSION + 1
├── src/spawn.rs                   # terminate_daemon asks first (Windows event)
├── src/sandbox/mod.rs, argv.rs    # history mount, TZ
└── tests/                         # terminal_history_*.rs, fixtures/terminal_history/v1.history, schema_hash.rs, protocol_roundtrip.rs

crates/micold-daemon/
├── src/history.rs                 # NEW: capture, seed, Seed, the saver task, the unwind save
├── src/supervisor.rs              # spawn_answering takes a Seed
├── src/state.rs                   # Inner.carried; capture at stop, exit, respawn; forget in revoke_tool_credentials
├── src/server.rs                  # stop_requested in both accept loops; save step in unwind; SettingsSet field
├── src/catalog.rs                 # save_terminal_history, as pi_activity_component
├── src/platform/{mod,unix,windows}.rs   # stop_requested(); write_owner_only delegates to micold-core
├── src/main.rs                    # the store, purge or sweep, the saver, before the accept loop
└── tests/                         # history_restart_in_run.rs, history_service_restart.rs, history_stop_request.rs,
                                   # history_setting.rs, history_damaged.rs, history_removal.rs, history_timing.rs,
                                   # sandbox_real_history.rs

crates/micold-client/
├── src/features/settings.rs       # draft field + Msg
├── src/shell/persist.rs, shell/daemon_sync.rs   # SettingsSet field, settings mirror
├── src/shell/startup.rs, shell/sandbox.rs       # history directory and mount, TZ
├── src/ui/settings/terminal.rs    # the control
└── tests/                         # features_settings.rs, settings_sections.rs, fixtures/layout_snapshot.txt

packaging/sandbox/Containerfile                  # tzdata
packaging/windows/micold-ai-ide.iss              # ask the service to stop before Stop-Process
docs/user-guide/worktrees-and-sessions.md, settings.md, sandboxed-daemon.md,
docs/daemon.md, docs/development/architecture.md
```

**Structure Decision**: the existing three-crate workspace. Everything that needs no terminal
emulator is in `micold-core` (format, schedule, store, texts, owner-only writing), so it is tested
without a process and is usable by the launcher. Only the code that touches a `Term` or the
service's state is in `micold-daemon`. The client changes only for the setting and the launcher.

## Delivery order

Story 1 is split along its acceptance scenarios, because it holds the whole mechanism
(`references/milestones.md` rule 3). Each step is one milestone of [tasks.md](./tasks.md) (M1 to
M10) and ships something observable.

1. **A stop and start keeps the history** (story 1 scenarios 9 and 10; FR-015): snapshot types,
   capture, seed, separator, `carried`. Nothing on disk. User guide: the restored history, the
   separator, the full-screen CLI limit, shells not covered.
2. **History saved at a process end survives a service restart** (story 1 scenarios 2 to 6):
   `owner_only` with the sync, the file format, the store's `save` and `load`, the save at a process
   end, the load at a start. A file that cannot be read is skipped with a warning; the service in a
   container saves only where the directory exists. User guide: service restart, where the files
   are.
3. **A running terminal is saved every 30 seconds** (story 1 scenario 7): the schedule, the saver.
4. **An orderly stop loses nothing** (story 1 scenarios 1 and 8): the save step of `unwind`, the
   stop request on Unix. `docs/daemon.md`.
5. **Story 2**: the setting end to end, `set_enabled`, `purge`, the deletion retry. User guide.
6. **Story 3**: the notice line, the damage reasons in the log, log-once of a failed save. User
   guide.
7. **Story 4**: `forget` on every removal path, the in-flight rule, `sweep`. User guide.
8. **The stop request on Windows**: the event, `terminate_daemon`, the installer, the
   end-of-session window.
9. **The sandbox**: the launcher creates the directory, the history mount, `TZ` and `tzdata`, the
   warning for an older container, the real-runtime tests. User guide's sandbox chapter.
10. **Polish**: the timing test of SC-005, the architecture page, quickstart Part B.

## Risks

| Risk | Mitigation |
|---|---|
| Between step 2 and step 5 `main` saves terminal output with no way to turn it off, and until step 7 a removed session's file stays | The files are owner-only from step 2. Step 7's sweep removes every leftover file at the first service start. **No release is cut between step 2 (M2) and step 7 (M7)**; each of those PR bodies says so, and the ledger carries it |
| The spec's Terms call restart, update, logout and reboot orderly, but today only the idle stop unwinds (R14) | The stop request (steps 4 and 8). Where a request does not arrive the outcome is that of a kill: at most the last 60 seconds are missing (SC-002) |
| A real Windows logout or reboot cannot be run in CI | A `cfg(windows)` test sets the event and sends `WM_ENDSESSION` to the window; quickstart Part B lists the manual check on a Windows machine as not automatable here |
| A sandbox container made before this feature, on a Windows host, has no history mount (R15) | The service in a container never creates the directory, so it saves nothing rather than writing into the roaming profile; one warning says to recreate the sandbox; the user guide says the same. Its separator shows UTC |
| An AI CLI erases the scrollback (`ESC[3J`) and with it the restored lines (R13) | Not seen at start-up in any of the three CLIs; honoured as it is for live history; a test pins that `ESC[2J` loses nothing |
| Two of three CLIs draw full-screen, so the restored lines are covered (R16) | Decided by the user (D11): shipped for the normal screen, said in the spec and the guide; quickstart Part B records what each CLI shows |
| Another feature takes the next protocol version first (039 and 040 are in flight) | Take the next free number when story 2 is implemented (R10) |
| A scrollback limit near the maximum (1,000,000 lines) makes a file of about 100 MB, rewritten every 30 s while the terminal prints | The write is off the terminal's lock and one terminal at a time; the timing test of SC-005 runs at the default limit; the user guide's setting text names the cost of a very large limit |
| A stopped session's snapshot stays in memory until the session is started or removed | About 1 MB per stopped session at the default limit; dropped with the session (FR-023) |
| Seeding changes what a new process sees on screen at start, and on Windows ConPTY paints from its own blank buffer (R17, not run on a Windows machine) | The seed ends by moving the seeded rows into the history and homing the cursor, so every process starts on a blank screen as today; tests assert the order earlier output, separator, new output; one integration test runs on a real pseudoconsole in CI's Windows job in the first milestone |
| A power loss tears a file despite the sync before the rename (a disk that reports a sync it did not do) | The checksum fails, the file is skipped as damaged and replaced at the next save; nothing else is affected |

## Complexity Tracking

No violations.
