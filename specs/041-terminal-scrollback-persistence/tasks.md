---
description: "Task list for feature 041 — terminal history that survives a session service restart"
---

# Tasks: Terminal History That Survives a Session Service Restart

**Input**: Design documents from `/specs/041-terminal-scrollback-persistence/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY and must be observed failing before their implementation. Every phase lists its failing
tests first. The GUI exception is claimed only for the composition of the one Settings control,
verified by the layout snapshot and the recorded quickstart Part B pass. Test tasks carry the behavior
ids (`[A#]`, `[U#]`) of [tdd/test-list.md](./tdd/test-list.md); `/speckit.tdd.run` ticks a task from them.

**Documentation**: Per Constitution Principle VII, each slice that changes what the user sees carries
its own user-guide task in the milestone that ships it (CI's user-guide gate).

**Cross-platform**: Per Constitution Principle VI, every `cfg` arm has both sides and a test CI runs
on Linux, macOS and Windows: the owner-only helper (T016, T020) and the stop request (T031 to T035,
T061 to T064). Run `cargo check --target aarch64-apple-darwin` before pushing a `cfg` arm.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task belongs to (US1–US4)
- Every description carries an exact file path

## Path Conventions

Three-crate workspace: `crates/micold-core/`, `crates/micold-daemon/`, `crates/micold-client/`.
Abbreviations: **DM** = [data-model.md](./data-model.md), **HF** =
[contracts/saved-history-file.md](./contracts/saved-history-file.md), **ST** =
[contracts/setting.md](./contracts/setting.md), **SR** =
[contracts/stop-request.md](./contracts/stop-request.md), **R#** = a section of
[research.md](./research.md), which names the existing code each task touches by file and line.

A **fake CLI** in a daemon integration test is a small script or test binary run as the session's
AI CLI: it prints what the test needs on the normal screen and records what it reads on stdin.
A **service restart** in a daemon integration test is a second `DaemonState` (with its own
`HistoryStore`) built on the same catalog and history directories after the first was dropped,
as `crates/micold-daemon/tests/daemon_lifecycle.rs` does.

---

## Phase 1: Setup (Shared Infrastructure)

- [x] T001 Create the empty modules and wire them in: `crates/micold-core/src/terminal_history/mod.rs` (declared in `crates/micold-core/src/lib.rs`) and `crates/micold-daemon/src/history.rs` (declared in `crates/micold-daemon/src/lib.rs`); add `chrono` (the version already in `Cargo.lock`, `default-features = false`, features `clock`) to `[workspace.dependencies]` in `Cargo.toml` and to `crates/micold-daemon/Cargo.toml`. `cargo deny`/`Cargo.lock` gain no new crate.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the snapshot types every later phase uses (DM §1).

- [ ] T002 [U1] [U2] [U3] [U4] [U5] [U6] [U7] Write `crates/micold-core/tests/terminal_history_snapshot.rs`: `HistorySnapshot::validate` accepts a line whose runs' `chars` sum to its number of characters and rejects one whose sum differs; rejects a `text` holding a C0 character (`\u{7}`), a C1 character (`\u{9b}`) and `ESC`; `HistoryColor::Basic` accepts 0 to 15 and `Dim` 0 to 7 only; an empty snapshot `is_empty()`; `StyleFlags` round-trips each of bold, dim, italic, underline, inverse, strikethrough, hidden.
- [ ] T003 Implement DM §1 in `crates/micold-core/src/terminal_history/mod.rs`: `HistorySnapshot { lines }`, `LogicalLine { text, runs }`, `StyleRun { chars: u32, style }`, `HistoryStyle { fg, bg, flags }`, `HistoryColor` (`Default`, `Basic(u8)`, `Dim(u8)`, `Indexed(u8)`, `Rgb(u8, u8, u8)`), `StyleFlags`, `validate`, `is_empty`. No dependency on `alacritty_terminal`.

**Checkpoint**: T002 passes under `mise run test-core`.

---

## Phase 3: User Story 1, slice A — a stop and start keeps the history (Priority: P1) 🎯 MVP

**Goal**: when a session's AI CLI process is started again while the session service keeps running,
its terminal shows the earlier output, one separator line, then the new output (FR-015). Nothing is
written to disk.

**Independent Test**: run a session on a fake CLI that prints 200 lines, some coloured and bold;
stop it and start it; the terminal's history holds the 200 lines with their styles, then the
separator, then the new output.

### Tests for User Story 1, slice A (MANDATORY — Constitution Principle I) ⚠️

- [ ] T004 [P] [US1] [U8] [U9] [U10] [U11] Write `crates/micold-core/tests/terminal_history_text.rs` for `separator_line` (DM §7): at 80 columns the text is `── session restarted at 2026-10-02 14:31 +02:00 ──`; at a width narrower than the full text the rules are dropped; narrower still the text is cut to the width; the result is never wider than `columns` and never holds a line break (FR-009).
- [ ] T005 [US1] [U14] [U15] [U16] [U17] [U18] [U19] [U20] [U21] [U22] Write the capture unit tests in `crates/micold-daemon/src/history.rs` against a real `Term` with no process (R2, DM §1): text and order of history and screen rows; each of the 16 basic colours, an indexed colour and an RGB colour as foreground and as background; each flag of `StyleFlags`; two rows joined by the wrap flag are one `LogicalLine`; a wide character is one character and its spacer is skipped; a zero-width character follows its base; trailing empty screen rows are not captured (the rule of `Framer::plain_tail`); a `Term` that printed nothing gives an empty snapshot (FR-001).
- [ ] T006 [US1] [U23] [U24] [U25] [U26] [U27] [U28] Write the seed unit tests in `crates/micold-daemon/src/history.rs` (R3, DM §6): capture after `seed(Seed::History)` equals the input lines followed by the separator in the dim style; afterwards the screen is blank, the cursor is at home, the attributes are reset and the seeded lines are all in the history (R17); a snapshot longer than the `Term`'s history limit leaves the most recent lines (FR-012); seeding at a narrower width wraps and a later capture gives the same logical lines (edge case *Terminal size changed*); `Seed::None` leaves the `Term` untouched (FR-010); a second seed after more output keeps the first separator (FR-011).
- [ ] T007 [US1] [A9] [A10] [U30] [U31] [U32] [U33] [U34] [U35] [U36] [U37] [U38] [U133] [U134] [U135] Write `crates/micold-daemon/tests/history_restart_in_run.rs` with a fake CLI (patterns: `tests/reattach_snapshot.rs`, `tests/daemon_lifecycle.rs`): stop then start shows 200 styled lines, one separator, the new output (story 1 scenario 9, SC-011); a process that exits by itself and is restarted shows its last lines above one separator (scenario 10); a session with no output shows no separator (FR-010); two stops and starts show two separators in order (FR-011); two sessions each show only their own lines (FR-025); the fake CLI's recorded stdin is empty after a start (FR-009); a Regular Terminal instance stopped and started is empty (FR-014); a fake CLI that prints `ESC[2J ESC[H` at start leaves the seeded lines and separator in the history (R13); a fake CLI that enters and leaves the alternate screen leaves them in the primary grid's history (R16); a second attached client receives the same lines in its first `full` frame (edge case *Several windows*); with a client attached and streaming at the process end, a stop and a self-exit each keep the last line the process printed (R4: the state's `Arc` is not the last one); a fake CLI that leaves a detached grandchild holding the terminal open is stopped with a reply within 3 s and its parsed output is carried (`cfg(unix)`, R4's bound). Every case asserts the order earlier output, separator, new output in the captured history, not a screen row, and the cases of scenarios 9 and 10, with and without a client attached, run on a real pseudoconsole under `cfg(windows)` too (R17, FR-030).

### Implementation for User Story 1, slice A

- [ ] T008 [P] [US1] Implement `separator_line(date_time_offset: &str, columns: usize) -> String` in `crates/micold-core/src/terminal_history/text.rs` (declared in `terminal_history/mod.rs`) to pass T004.
- [ ] T009 [US1] Implement `capture(&Term) -> HistorySnapshot` in `crates/micold-daemon/src/history.rs` to pass T005: maps `alacritty_terminal` colours and flags to the types of DM §1 through an explicit table; an unknown named colour maps to `Default`.
- [ ] T010 [US1] Implement `Seed` (`None`, `History { snapshot, at }`) and `seed(&mut Term, Seed)` in `crates/micold-daemon/src/history.rs` to pass T006: through `vte::ansi::Handler` (`terminal_attribute`, `input`, `carriage_return`, `linefeed`) only; keeps the most recent `limit + screen rows` lines; ends with `clear_screen(ClearMode::All)` on the primary screen (R17); formats `at` as `YYYY-MM-DD HH:MM ±HH:MM` with `chrono`.
- [ ] T011 [US1] Give `PtySession::spawn_answering` in `crates/micold-daemon/src/supervisor.rs` a `Seed` argument applied to the new `Term` before the reader thread is spawned (R3); `spawn_ai_cli` passes its caller's seed, `spawn_shell` passes `Seed::None`.
- [ ] T012 [US1] In `crates/micold-daemon/src/state.rs` add `Inner.carried: HashMap<SessionId, HistorySnapshot>` (DM §6, R4): at every process end (`stop_session`, and the supervision tick's clean exit, give-up and `respawn_primary`) follow DM §6's order: clone the `SharedTerm`, take the `Arc<PtySession>` out, call `PtySession::teardown(&self, TEARDOWN_WAIT)` off the state lock, capture from the clone, insert into `carried`, then reply or spawn. Add `teardown` to `crates/micold-daemon/src/supervisor.rs` (R4): kill, take and close the master, wait up to 2 s for `output_ended` and then join the reader (`reader` becomes a `Mutex<Option<JoinHandle>>`), a second call does nothing, `Drop` calls it, and a resize after it is ignored; when the wait runs out, log one warning and capture anyway; only for `SessionProcess::Primary` of a `TerminalMode::AiCli` session; `start_session` and `respawn_primary` take the entry and pass `Seed::History` with `chrono::Local::now()` when it has lines; `remove_live_by_ids` drops the entry. T007 passes.
- [ ] T013 [US1] Update `docs/user-guide/worktrees-and-sessions.md`: a section on terminal history kept across a stop and start of a session, what the "session restarted at" line means, that Regular Terminal instances are not covered, and that for an AI CLI that draws full-screen (Claude Code and Copilot CLI by default) only its last screen is kept and the CLI's own resume shows the conversation (FR-032, D11).

**Checkpoint**: `cargo test -p micold-daemon --test history_restart_in_run` passes.

---

## Phase 4: User Story 1, slice B — history saved at a process end is restored after a service restart (Priority: P1)

**Goal**: the history a terminal held when its process ended is written to an owner-only file and
shown again when the session is started under a new session service.

**Independent Test**: run a session on a fake CLI, stop it, build a second `DaemonState` on the same
directories, start the session: the history and one separator are shown.

### Tests for User Story 1, slice B (MANDATORY — Constitution Principle I) ⚠️

- [ ] T014 [P] [US1] [U39] [U40] [U41] [U42] [U43] Write `crates/micold-core/tests/terminal_history_format.rs` (HF §2, §4, §5): encode then decode gives the same snapshot (empty, one line, 10,000 lines, every colour kind and flag); the header bytes are those of HF §2; one case per row of HF §4's table gives that `DamageReason` (too large, wrong magic, shorter than 52 bytes, version 2, a truncated tail, one flipped payload bit, trailing payload bytes, a style index out of range, a run sum that differs, a text with `ESC`); 1,000 random byte strings and every prefix of a valid file decode to `Damaged` without a panic; the bytes of `crates/micold-core/tests/fixtures/terminal_history/v1.history` equal the encoding of a fixed snapshot built in the test.
- [ ] T015 [P] [US1] [U44] [U45] [U46] [U47] [U48] [U49] [U50] [U51] [U52] [U53] [U54] Write `crates/micold-core/tests/terminal_history_store.rs` on a temp directory (DM §5, HF §1, §3): `save` then `load` gives `History`; the file is `<dir>/<session uuid>.history`; `load` of an absent file gives `None`; `load` of a damaged file gives `Damaged`; `load` of a file with mode `000` gives `Damaged(Unreadable)` (`cfg(unix)`); a save over an existing file leaves no temporary file; a save interrupted before the rename (a temporary file left behind) leaves the previous file loadable (FR-006); two ids have two files and never each other's content (FR-025); a second `save` of an equal snapshot returns `Unchanged` and does not touch the file (its modification time and inode stay) (FR-004); with `create_dir = false` and no directory `save` returns `Skipped` and creates nothing, and with the directory present it saves (R15); directory mode `0700` and file mode `0600` (`cfg(unix)`); `history_dir()` ends in `terminal-history` under `data_local_dir()` and on Windows is not under `data_dir()` (`cfg(windows)`, FR-019).
- [ ] T016 [P] [US1] [U55] [U56] [U57] [U58] [U59] Write `crates/micold-core/tests/owner_only.rs` (R8): `write` creates the directory `0700` and the file `0600` and replaces an existing file through a rename (`cfg(unix)`); `ensure_dir` creates a missing directory `0700` and tightens an existing looser one (`cfg(unix)`); on Windows both carry a protected DACL with exactly one entry, for the current user (`cfg(windows)`, the assertions of `crates/micold-daemon/tests/mcp_binding_file_mode.rs`); the temporary file is owner-only before content is written (FR-020); `write` of a file whose directory is read-only returns the error and leaves no temporary file.
- [ ] T017 [US1] [A1] [A2] [A3] [A4] [A5] [A6] [A10] [A19] [U60] [U61] [U62] [U63] Write `crates/micold-daemon/tests/history_service_restart.rs`: after a stop and a service restart, a start shows the 200 styled lines, one separator with the time of that start, then new output (story 1 scenario 2); a session that printed nothing shows no separator after a restart (scenario 3); a second restart shows output, separator, output, separator in order (scenario 4); a history longer than the scrollback limit restores the most recent lines up to the limit (scenario 5), and with a smaller limit in force at the restore, up to the new limit (edge case *Scrollback limit changed*); two sessions each restore their own history (scenario 6, FR-025); a process that exits by itself is saved at that exit and restored (FR-002); a Regular Terminal instance has no file (FR-014); a file of random bytes: the session starts, shows no history and no damaged content, and one warning is logged naming the session (FR-016); a file is not read while a carried snapshot exists.
- [ ] T018 [P] [US1] [U64] Write `crates/micold-daemon/tests/history_timing.rs`: a saved history of 10,000 lines of 100 characters is loaded and seeded, and the session reaches its running state, no more than 1 s later than the same start with no file (FR-013, SC-004). Pattern: `crates/micold-daemon/tests/mcp_read_latency.rs`.

### Implementation for User Story 1, slice B

- [ ] T019 [P] [US1] Implement `crates/micold-core/src/terminal_history/format.rs` to pass T014: `FORMAT_VERSION = 1`, `MAX_FILE_BYTES = MAX_SCROLLBACK_LINES × 512`, private `SavedHistory`/`SavedLine`/`SavedRun` (DM §2), `encode(&HistorySnapshot) -> Vec<u8>`, `decode(&[u8]) -> Result<HistorySnapshot, DamageReason>` with the ten checks of HF §4 in order, `DamageReason` and its `Display` (DM §3); write the fixture `crates/micold-core/tests/fixtures/terminal_history/v1.history`.
- [ ] T020 [US1] Move the owner-only helper into `crates/micold-core/src/owner_only.rs` (`write(dir, file, bytes)`, `ensure_dir(dir)`, both `cfg` arms, declared in `lib.rs`) to pass T016, add `sync_all` on the temporary file before the rename and a sync of the directory after it on Unix (HF §3, R5), and make `platform::write_owner_only` in `crates/micold-daemon/src/platform/mod.rs`, `unix.rs` and `windows.rs` delegate to it; `crates/micold-daemon/tests/mcp_binding_file_mode.rs` passes unchanged.
- [ ] T021 [US1] Implement `HistoryStore` in `crates/micold-core/src/terminal_history/store.rs` to pass T015: `history_dir()`, `new(dir, create_dir)`, `save(id, &snapshot)` through `owner_only::write`, `load(id) -> LoadOutcome`, `last_written` and the `Unchanged` result, one `Mutex` around state and file operations with encoding done before it is taken (DM §5).
- [ ] T022 [US1] In `crates/micold-daemon/src/main.rs` and `crates/micold-daemon/src/state.rs` build one `HistoryStore` at service start (`create_dir = false` when `MICOLD_IMAGE_REFERENCE` is set) and hold it in `DaemonState`; at each capture point of T012 also `save` the snapshot, on the blocking pool and outside the state lock; a failed save is logged as a warning with the session and the reason (FR-007).
- [ ] T023 [US1] In `crates/micold-daemon/src/state.rs` choose the seed at a start by DM §6's table: nothing carried → `load`; `History` with lines → `Seed::History`; `Damaged(reason)` → `Seed::None` and one `warn!` with the session and the reason (the notice line arrives with T054). T017 and T018 pass.
- [ ] T024 [US1] Update `docs/user-guide/worktrees-and-sessions.md`: history is also kept across a restart of the session service when the session was stopped or its process had exited before it; where the files are on each platform and that only the user can read them (FR-032, HF §1).

**Checkpoint**: `cargo test -p micold-daemon --test history_service_restart` passes.

---

## Phase 5: User Story 1, slice C — a running terminal is saved periodically (Priority: P1)

**Goal**: a terminal that is printing is saved at most once per 30 seconds, an idle one never, so a
service killed without warning loses at most the last 60 seconds.

**Independent Test**: a fake CLI prints continuously; the saver is ticked through 10 minutes of
injected time; the file was written at most 21 times; dropping the service without an unwind and
restarting restores the history up to the last save.

### Tests for User Story 1, slice C (MANDATORY — Constitution Principle I) ⚠️

- [ ] T025 [P] [US1] [U65] [U66] [U67] [U68] [U69] [U70] [U71] Write `crates/micold-core/tests/terminal_history_schedule.rs` (DM §4): `new(count)` is not due while the count is unchanged (FR-004); due when the count moved and no save was tried; not due again until 30 s after `saved`; due at exactly 30 s with a moved count; ticking every 5 s for 600 s with a count that always moves gives 20 saves (SC-003); `failed(now)` makes it due again 30 s later with the same count (FR-007); `mark_due()` makes it due with an unchanged count, still spaced 30 s from the last attempt.
- [ ] T026 [US1] [A7] [A24] [U72] [U73] [U74] [U75] [U76] [U77] [U78] Write the saver cases in `crates/micold-daemon/tests/history_periodic_save.rs` driving `save_due_at(now)` with injected time: a printing terminal's file changes once per 30 s and holds the output printed before the tick (FR-003); output is on disk no later than 60 s after it was printed; an idle terminal's file is not rewritten (its modification time and a write counter stay) (FR-004); two printing sessions are each saved on their own schedule and neither file holds the other's lines (edge case *Several busy sessions*); a Regular Terminal instance is never saved (FR-014); dropping the `DaemonState` without `unwind` and restarting restores the history up to the last save (story 1 scenario 7, SC-002); a save that fails (the directory made read-only, `cfg(unix)`) leaves the session running, logs a warning and is tried again 30 s later (story 3 scenario 6 as far as the retry); input written and a resize sent during a save reach the fake CLI (FR-005).

### Implementation for User Story 1, slice C

- [ ] T027 [P] [US1] Implement `SaveSchedule` with `SAVE_SPACING` (30 s) and `SAVER_TICK` (5 s) in `crates/micold-core/src/terminal_history/schedule.rs` to pass T025.
- [ ] T028 [US1] Implement the saver in `crates/micold-daemon/src/history.rs`: `save_due_at(now)` lists the covered live terminals (id, `VtSignals::output_count`, `Term` handle), keeps a `HashMap<SessionId, SaveSchedule>`, drops schedules of terminals no longer live, and for each due one captures under the `Term` lock, then encodes and saves off the lock, one terminal at a time on the blocking pool; never holds the state lock while writing (R6).
- [ ] T029 [US1] Spawn the saver task (a `SAVER_TICK` interval calling `save_due_at(Instant::now())`) from `crates/micold-daemon/src/main.rs` beside the supervision tick; give `DaemonState` in `crates/micold-daemon/src/state.rs` the accessor the saver needs for the covered live terminals. T026 passes.
- [ ] T030 [US1] Update `docs/user-guide/worktrees-and-sessions.md`: a running session's history is saved at most every 30 seconds, so after a crash or a power loss up to the last minute of output can be missing (FR-032).

**Checkpoint**: `cargo test -p micold-daemon --test history_periodic_save` passes.

---

## Phase 6: User Story 1, slice D — an orderly stop loses nothing (Priority: P1)

**Goal**: when the session service stops itself or is asked to stop, every covered terminal is saved
first (FR-002, SR).

**Independent Test**: a fake CLI prints 200 lines with no periodic save due; the service process is
sent `SIGTERM`; a new service restores all 200 lines.

### Tests for User Story 1, slice D (MANDATORY — Constitution Principle I) ⚠️

- [ ] T031 [US1] [A1] [A8] [U79] [U80] [U81] [U82] [U83] [U84] [U132] Write `crates/micold-daemon/tests/history_stop_request.rs`: `unwind` with the idle reason saves every running covered terminal, and a restart restores all 200 lines with nothing missing (story 1 scenarios 1 and 8, SC-001); a real service process sent `SIGTERM`, `SIGINT` or `SIGHUP` exits within 5 s and its file holds the last line printed (`cfg(unix)`, SR §6; pattern `tests/daemon_stop.rs`); a save that blocks does not hold `unwind` longer than 3 s and the previous file stays (SR §4); with a store that refuses to save (`create_dir = false` and no directory: the `Skipped` result of T015) nothing is written; a terminal with no output since its last save is not rewritten by the unwind (FR-004); the endpoint is released only after the saves (a second service started during the unwind loads the complete file); ten running sessions each holding 10,000 lines of 100 characters are all saved by one `unwind` within its 3 s bound (FR-002, SC-001).
- [ ] T032 [P] [US1] [U85] Write the unit test of `stop_requested()` in `crates/micold-daemon/src/platform/unix.rs`: the future is pending until the process receives `SIGTERM`, then completes; a second signal while it is completed changes nothing (SR §1).

### Implementation for User Story 1, slice D

- [ ] T033 [US1] Add the `signal` feature to the workspace `tokio` in `Cargo.toml`; implement `stop_requested()` in `crates/micold-daemon/src/platform/unix.rs` (`SIGTERM`, `SIGINT`, `SIGHUP` through `tokio::signal::unix`) and declare it in `crates/micold-daemon/src/platform/mod.rs`; in `crates/micold-daemon/src/platform/windows.rs` it is a future that never completes until T063.
- [ ] T034 [US1] In `crates/micold-daemon/src/server.rs` select on `platform::stop_requested()` beside the idle timer in both accept loops and run `unwind(StopReason::Requested)` when it completes (SR §1).
- [ ] T035 [US1] Add the save step to `unwind` in `crates/micold-daemon/src/server.rs`, implemented as `save_all_live` in `crates/micold-daemon/src/history.rs`: before `take_live_sessions`, capture every covered terminal from its live `Term`, encode them in parallel on the blocking pool and save each, the whole step bounded at 3 s (SR §4). T031 passes.
- [ ] T036 [US1] Update `docs/daemon.md` (the service now unwinds on `SIGTERM`, `SIGINT` and `SIGHUP`, and saves terminal history before it stops) and `docs/user-guide/worktrees-and-sessions.md` (after **Restart service**, a logout or a reboot on Linux and macOS, and after the service stopped itself when idle, no output is missing) (FR-032).

**Checkpoint**: `cargo test -p micold-daemon --test history_stop_request` passes.

---

## Phase 7: User Story 2 - Keep terminal output off the disk (Priority: P2)

**Goal**: one setting in Settings → Terminal turns saving off, deletes what was saved, and applies to
running sessions at once.

**Independent Test**: spec.md, User Story 2, *Independent Test*.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T037 [P] [US2] [U86] [U87] [U88] Extend the settings tests in `crates/micold-core/src/settings.rs` (beside those of `pi_activity_component`): `save_terminal_history` defaults to `true`; a `settings.json` without the field reads as `true`; `false` round-trips (FR-026, FR-029, ST §1).
- [ ] T038 [P] [US2] [U89] [U90] [U91] Extend `crates/micold-core/tests/protocol_roundtrip.rs` and `crates/micold-core/tests/schema_hash.rs`: `DaemonSettings.save_terminal_history` and `ClientMsg::SettingsSet { save_terminal_history: Some(false) }` round-trip, `None` leaves the value unchanged, and the pinned schema hash and `PROTOCOL_VERSION` are the new ones (ST §2).
- [ ] T039 [P] [US2] [U92] [U93] [U94] [U95] [U96] [U97] Extend `crates/micold-core/tests/terminal_history_store.rs` (DM §5): with `enabled = false`, `save` returns `Skipped` and `load` returns `None`; `set_enabled(false)` deletes every `.history` file and every temporary file and reports none left; a file that cannot be deleted (its directory made read-only, `cfg(unix)`) is reported, kept in the retry set, and removed by `retry_deletions()` once deletable; `purge()` does the same at start; `set_enabled(true)` restores nothing, keeps a failed deletion in the retry set, and `load` of such an id returns `None` until a new `save` replaced the file (story 2 scenario 7); a `save` racing `set_enabled(false)` from another thread leaves no file (FR-033).
- [ ] T040 [US2] [A12] [A13] [A14] [A15] [A16] [A17] [A18] [U98] [U99] [U100] Write `crates/micold-daemon/tests/history_setting.rs`: with the setting off, a session's output and a service restart leave no file and the terminal starts empty with no separator (story 2 scenario 2, FR-028); turning it off while a session prints stops every later write without a restart (scenario 3); turning it on saves the running session's history, including output printed while off, within 60 s of injected time (scenario 4); turning it off deletes the files of running and stopped sessions before `SettingsSet` is answered (scenario 5, SC-008) and the running terminal's history is unchanged (scenario 6); off then on then a service restart shows only what was saved after it was turned on (scenario 7); with the setting off a stop and start still shows the history and separator and writes nothing (scenario 8, FR-015); a service that starts with the setting off deletes every file before a session can start (edge case *Setting turned off while the service is not running*); a failed deletion is logged once as a warning with the session and the reason and retried every 30 s (FR-033); `SettingsChanged` is broadcast with the new value.
- [ ] T041 [P] [US2] [A11] [U101] [U102] [U103] [U104] [U105] Extend `crates/micold-client/tests/features_settings.rs` and `crates/micold-client/tests/settings_sections.rs`: the draft holds `save_terminal_history` from the service's settings; toggling marks the Terminal section dirty; Save sends `SettingsSet` with `Some(value)` and no other field changed; a `SettingsChanged` from the service updates the draft; the Terminal section's `SETTINGS` list holds the entry with the label and note of ST §4 (story 2 scenario 1).

### Implementation for User Story 2

- [ ] T042 [US2] Add `save_terminal_history: bool` (`#[serde(default)]` = `true`) to `Settings` and its on-disk form in `crates/micold-core/src/settings.rs`, copying `pi_activity_component` (R10). T037 passes.
- [ ] T043 [US2] Add the field to `DaemonSettings` and `ClientMsg::SettingsSet` in `crates/micold-core/src/protocol/messages.rs` and bump `PROTOCOL_VERSION` to the next free number in `crates/micold-core/src/protocol/version.rs`; update the pinned hash. T038 passes.
- [ ] T044 [US2] Add `enabled`, `set_enabled`, `retry_deletions` and `purge` to `HistoryStore` in `crates/micold-core/src/terminal_history/store.rs` to pass T039.
- [ ] T045 [US2] In `crates/micold-daemon/src/catalog.rs`, `crates/micold-daemon/src/state.rs` and `crates/micold-daemon/src/server.rs` add `set_save_terminal_history` end to end as `pi_activity_component` (R10): store, apply to the `HistoryStore` before the reply, broadcast; when turned on, mark every saver schedule due (`mark_all_due` in `crates/micold-daemon/src/history.rs`); the saver tick calls `retry_deletions()` every 30 s and logs each failure once; `crates/micold-daemon/src/main.rs` builds the store with the stored value and calls `purge()` before the accept loop when it is off. T040 passes.
- [ ] T046 [US2] In `crates/micold-client/src/features/settings.rs`, `crates/micold-client/src/shell/persist.rs` and `crates/micold-client/src/shell/daemon_sync.rs` add the draft field, its message and the `SettingsSet` field, copying `pi_activity_component`.
- [ ] T047 [US2] Add the control to `crates/micold-client/src/ui/settings/terminal.rs` below *Scrollback lines*: the shared `Checkbox` with `field_note`, label and note of ST §4, and its `SETTINGS` entry; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`. T041 passes (FR-031).
- [ ] T048 [US2] Update `docs/user-guide/settings.md` (the control, its default, that turning it off deletes the saved history without asking, that a very large *Scrollback lines* value makes each save larger) and `docs/user-guide/worktrees-and-sessions.md` (with saving off a stop and start still shows the earlier output; a service restart does not) (FR-032).
- [ ] T074 [U131] Extend `crates/micold-daemon/tests/history_timing.rs` with SC-005: ten fake CLIs printing continuously, 200 keystroke-to-echo samples in one of them with saving on and with saving off, saves forced through `save_due_at`; the 95th percentile with saving on is at most 20 ms above that with saving off. Fix what it finds in `crates/micold-daemon/src/history.rs`. It is here, after T045, because its baseline is saving turned off.

**Checkpoint**: `cargo test -p micold-daemon --test history_setting` passes and the control is in
Settings → Terminal.

---

## Phase 8: User Story 3 - A damaged saved history never stops a session (Priority: P2)

**Goal**: a saved history that cannot be read is skipped, said so in the terminal with one line, and
reported once in the log.

**Independent Test**: spec.md, User Story 3, *Independent Test*.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T049 [P] [US3] [U12] [U13] Extend `crates/micold-core/tests/terminal_history_text.rs` for `notice_line(columns)` (DM §7): `── earlier output could not be restored ──` at 80 columns; rules dropped, then cut, when narrower; one row.
- [ ] T050 [US3] [U29] Add the `Seed::Notice` unit test in `crates/micold-daemon/src/history.rs`: the `Term` holds exactly one line, the notice in the dim style, and no separator; the cursor is on the row below.
- [ ] T051 [US3] [A19] [A20] [A21] [A22] [A23] [A24] [U106] [U107] Write `crates/micold-daemon/tests/history_damaged.rs`: with a file of random bytes the session starts and runs as one with no file (story 3 scenario 1, SC-006); its terminal shows the notice line and none of the file's bytes, the log holds exactly one warning naming the session and the reason, and the `RecentErrors` of the diagnostics hold it (scenario 2, FR-017; pattern `tests/diagnostics.rs`); a file with mode `000` gives the same (`cfg(unix)`, scenario 3); a file with another format version gives the same with the reason "written by another version"; a second session with an intact file shows its full history (scenario 4); after the skip the terminal is saved at the next tick even with no new output, the file is replaced, and a later restart restores the notice and the new output (scenario 5, FR-018); a save that keeps failing for the same reason is logged once per service run, and a different reason is logged again (scenario 6, FR-007).

### Implementation for User Story 3

- [ ] T052 [P] [US3] Implement `notice_line` in `crates/micold-core/src/terminal_history/text.rs` to pass T049.
- [ ] T053 [US3] Add `Seed::Notice` to `seed` in `crates/micold-daemon/src/history.rs` to pass T050.
- [ ] T054 [US3] In `crates/micold-daemon/src/state.rs` map `LoadOutcome::Damaged` to `Seed::Notice` (replacing T023's `Seed::None`) and mark the new terminal's schedule due; in the saver of `crates/micold-daemon/src/history.rs` add `logged: HashSet<(SessionId, String)>` so a repeated save failure with the same reason is logged once per service run. T051 passes.
- [ ] T055 [US3] Update `docs/user-guide/worktrees-and-sessions.md`: what the "earlier output could not be restored" line means, that the session is unaffected, and where the reason is shown (*Session service diagnostics*) (FR-032).

**Checkpoint**: `cargo test -p micold-daemon --test history_damaged` passes.

---

## Phase 9: User Story 4 - Saved history goes away with its session (Priority: P3)

**Goal**: removing a session removes its saved history before the action is reported, and a service
start removes every file that belongs to no session that can still be shown.

**Independent Test**: spec.md, User Story 4, *Independent Test*.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T056 [P] [US4] [U108] [U109] [U110] [U111] Extend `crates/micold-core/tests/terminal_history_store.rs` (DM §5): `forget(ids)` deletes those files and no other; a `save` for a forgotten id returns `Skipped` and leaves no file, also when it started before the `forget` on another thread (story 4 scenario 5); `forget` deletes while `enabled` is false (edge case *Session removed while the setting is off*); `sweep(keep)` deletes every `.history` file whose id is not in `keep`, every file with another name and every temporary file, and keeps the rest (FR-024).
- [ ] T057 [US4] [A25] [A26] [A27] [A28] [A29] [A30] [U112] [U113] [U114] Write `crates/micold-daemon/tests/history_removal.rs`: **Remove** (`delete_session`) deletes the file before the reply (story 4 scenario 1, SC-007); **Close** (archive) does the same (scenario 2); deleting a worktree and forgetting a project delete the files of all their sessions (scenario 3); pruning a never-used session deletes its file and its carried snapshot; a file of an unknown id and one of an archived session are gone after a service start (scenario 4); a removal during a save leaves no file (scenario 5); stopping a session keeps its file, and it is restored in the same run and after a restart (scenario 6); the carried snapshot of a removed session is dropped; removal with the setting off deletes a file left by a failed deletion.

### Implementation for User Story 4

- [ ] T058 [US4] Add `forgotten`, `forget(ids)` and `sweep(keep)` to `HistoryStore` in `crates/micold-core/src/terminal_history/store.rs` to pass T056.
- [ ] T059 [US4] Call `forget(&ids)` from `DaemonState::revoke_tool_credentials` in `crates/micold-daemon/src/state.rs`, outside the state lock and before the handler replies (R9); in `crates/micold-daemon/src/main.rs` call `sweep(keep)` with the catalog's non-archived session ids before the accept loop when saving is on. T057 passes.
- [ ] T060 [US4] Update `docs/user-guide/worktrees-and-sessions.md`: Close, Remove, deleting a worktree and forgetting a project delete the session's saved history; stopping a session keeps it (FR-032).

**Checkpoint**: `cargo test -p micold-daemon --test history_removal` passes.

---

## Phase 10: User Story 1 on Windows — the stop request (Priority: P1 parity, FR-030)

**Goal**: **Restart service**, an update and a logout on Windows save the terminal histories first,
as they do on Linux and macOS since Phase 6 (SR §1 to §3).

**Independent Test**: on Windows, a service process with a printing fake CLI has its stop event set;
it exits within 5 s and a new service restores every line.

### Tests for the Windows stop request (MANDATORY — Constitution Principle I) ⚠️

- [ ] T061 [US1] [A1] [U115] [U116] [U117] [U118] Extend `crates/micold-daemon/tests/history_stop_request.rs` with `cfg(windows)` cases (SR §6): setting the event `Local\Micold.Daemon.Stop.<SID>` makes a real service process exit within 5 s with its file holding the last line; `WM_ENDSESSION` sent to the service's hidden window raises the same request; the event's DACL has one entry, for the current user; `micold_core::spawn::stop_running_daemon` against the real service (`CARGO_BIN_EXE_micold-daemon`) makes it exit with code 0, the orderly exit after the unwind, not the 1 of `TerminateProcess`, and its file holds the last line (SR §2). The cases take a `SERIAL` lock as `tests/daemon_stop.rs` does: the event's name is one per user.
- [ ] T062 [P] [US1] [U119] [U120] [U136] Add one `cfg(windows)` test beside `terminate_daemon` in `crates/micold-core/src/spawn.rs` for its two fallbacks (SR §2, §3), run one after the other in the same test because the event's name is one per user: the target is a system executable copied to a temporary directory as `micold-daemon.exe`, so the image check passes; with the event created by the test and never answered (the test skips with a message when the event already exists, which means a service of this user is running), `terminate_daemon` falls back to `TerminateProcess` after 5 s; with no event to open it falls back at once. The cooperative case needs the real service and is in T061. Add to `crates/micold-core/tests/windows_installer_in_use.rs` a test that uses `routine_body`: inside `StopDaemon` of `packaging/windows/micold-ai-ide.iss` the step that sets the stop event and waits comes before the `Stop-Process` and `taskkill` steps (runs on every platform).

### Implementation for the Windows stop request

- [ ] T063 [US1] Implement `stop_requested()` in `crates/micold-daemon/src/platform/windows.rs`: create the manual-reset named event with the pipe's owner-only DACL, wait for it on a blocking thread, and run a hidden top-level window on its own thread whose `WM_QUERYENDSESSION`/`WM_ENDSESSION` handler raises the request and waits for the unwind to finish (SR §1, §3; `windows-sys` features added to `Cargo.toml` as needed).
- [ ] T064 [US1] Change `terminate_daemon` in `crates/micold-core/src/spawn.rs` on Windows to open and set the event, wait up to 5 s for the process to exit, then fall back to `TerminateProcess` (SR §2). T061 and T062 pass.
- [ ] T065 [US1] In `packaging/windows/micold-ai-ide.iss` set the stop event and wait up to 5 s before the existing `Stop-Process` and `taskkill` steps (SR §2); T062's test in `windows_installer_in_use.rs` passes.
- [ ] T066 [US1] Update `docs/daemon.md` (the Windows stop event and the end-of-session window), `docs/development/windows-packaging.md` (the installer asks first) and `docs/user-guide/worktrees-and-sessions.md` (the sentence of T036 now names Windows too) (FR-032).

**Checkpoint**: CI's Windows job passes `history_stop_request`.

---

## Phase 11: User Stories 1 and 2 in the sandbox (Priority: P1 parity, FR-021, FR-022)

**Goal**: a session service in a container saves to and restores from the host's history directory,
on every host, and shows the host's local time in the separator.

**Independent Test**: `mise run image && mise run test-sandbox` runs `sandbox_real_history`.

### Tests for the sandbox (MANDATORY — Constitution Principle I) ⚠️

- [ ] T067 [P] [US1] [U121] [U122] Add unit tests in `crates/micold-core/src/sandbox/mod.rs` and `crates/micold-core/src/sandbox/argv.rs` (DM §9): `MountSet::build` adds the history mount (host `data_local_dir()/terminal-history` → `/var/lib/micold-ai-ide/terminal-history`) only when the history directory is not inside the state directory, and none otherwise; the container arguments carry `-e TZ=<zone>` when a zone is given and no `TZ` when none is.
- [ ] T068 [P] [US1] [U123] [U124] Add tests for the launcher in `crates/micold-client/src/shell/sandbox.rs`: at bring-up, attach included, the host history directory is created through `owner_only::ensure_dir` before the runtime is called; the zone passed is the host's IANA zone.
- [ ] T069 [US1] [U125] [U126] [U127] [U128] [U129] [U130] Write `crates/micold-daemon/tests/sandbox_real_history.rs` behind the `sandbox-real-runtime` feature (pattern `tests/sandbox_real_session_start.rs`; every test function is named `sandbox_real_history_*`, because `mise run test-sandbox` and CI filter on test names): a history saved by a host service is restored by a container service on the same data directory, and the reverse (FR-022); a file written by the container is `0600` in a `0700` directory as seen from the host (FR-021, SC-009); after the container is recreated the history is restored (FR-021); `<runtime> stop` on a sandbox with a printing session leaves a file holding the last line (SR §6); the separator carries the host's UTC offset; a container without the directory saves nothing and logs one warning that says to recreate the sandbox (R15).

### Implementation for the sandbox

- [ ] T070 [US1] Add the history mount to `MountSet` in `crates/micold-core/src/sandbox/mod.rs` and `TZ` to the container arguments in `crates/micold-core/src/sandbox/argv.rs` to pass T067.
- [ ] T071 [US1] In `crates/micold-client/src/shell/startup.rs` and `crates/micold-client/src/shell/sandbox.rs` compute the host history directory from `data_local_dir()`, call `owner_only::ensure_dir` on it at every bring-up, pass it to `MountSet::build`, and pass the host's zone from `iana-time-zone` (added to `crates/micold-client/Cargo.toml`) to pass T068.
- [ ] T072 [US1] Install `tzdata` in `packaging/sandbox/Containerfile`; in `crates/micold-daemon/src/history.rs` log, once per service run, the warning for a container whose history directory is absent ("terminal history is not saved: recreate the sandbox"). T069 passes under `mise run test-sandbox`.
- [ ] T073 [US1] Update `docs/user-guide/sandboxed-daemon.md`: saved history is shared between a service on the computer and one in the sandbox; a sandbox created before this version on Windows saves no history, and any older sandbox shows the separator in UTC, until it is recreated (FR-032, R15).

**Checkpoint**: `mise run test-sandbox` passes `sandbox_real_history`.

---

## Phase 12: Polish & Cross-Cutting Concerns

- [ ] T075 [P] Update `docs/development/architecture.md`: the `terminal_history` core module, the daemon's `history` module, where capture, carry, save, load and seed happen, and the stop request.
- [ ] T076 Run [quickstart.md](./quickstart.md) Part B with the `visual-pass` skill, save the screenshots under `specs/041-terminal-scrollback-persistence/evidence/`, and record B12 to B15 and the three manual Windows checks as run, not run, or covered by Part A, with the reason.
- [ ] T077 Run `mise run gate` and `cargo check --target aarch64-apple-darwin`; confirm every file under [quickstart.md](./quickstart.md) Part A exists and passes.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 → Phase 2 → Phase 3**: in order. Phase 3 is the MVP.
- **Phase 4** needs Phase 3 (capture, seed, `carried`).
- **Phase 5** needs Phase 4 (the store). **Phase 6** needs Phase 5 (it relies on the saver's file as
  the fallback of SR §4) and Phase 4.
- **Phase 7 (US2)** needs Phase 5 (the saver's schedules). **Phase 8 (US3)** needs Phase 5.
  **Phase 9 (US4)** needs Phase 7 (removal while the setting is off).
- **Phase 10** needs Phase 6. **Phase 11** needs Phases 6 and 7 (the sandbox stop, and `create_dir`).
- **Phase 12** needs all.

### Within Each Phase

- Tests are written first and seen failing, then the implementation tasks in the order listed.
- The user-guide task is the last task of its phase and ships in the same PR.

### Parallel Opportunities

- Phase 4: T014, T015, T016 and T018 touch four different test files; T019 is independent of T020.
- Phase 7: T037, T038, T039 and T041 touch different files.
- Phases 8 and 9 touch different daemon test files and can be developed side by side after Phase 7,
  but both edit `state.rs` and `store.rs`, so they merge in order.

---

## Parallel Example: User Story 1, slice B

```text
T014 terminal_history_format.rs   ┐
T015 terminal_history_store.rs    ├─ written together, each fails for its own missing item
T016 owner_only.rs                ┘
then T019 (format.rs) beside T020 (owner_only.rs), then T021 (store.rs), T022, T023
```

---

## Implementation Strategy

MVP first: Phases 1 to 3 give a visible result with nothing on disk. Phases 4 to 6 add the disk in
three steps, each closing one group of story 1's scenarios. Then one phase per remaining story, then
platform parity (Windows stop request, sandbox), then Polish. Saving is on by default from Phase 4,
before the setting (Phase 7) and removal (Phase 9) exist on `main`: the files are owner-only from
the start, Phase 9's sweep cleans up every leftover, and no release is cut between M2 and M7
([plan.md, Risks](./plan.md#risks)).

---

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — A stop and start keeps the terminal history 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: a session stopped and started again (or restarted after its process exited) shows its earlier output, a "session restarted at …" line, then the new output; nothing is written to disk.
- **Satisfies**: US1 acceptance scenarios 9–10; FR-009, FR-010, FR-011, FR-012, FR-014, FR-015, FR-025 (in memory); SC-011
- **Verify**: `cargo test -p micold-daemon --test history_restart_in_run`
- **Depends on**: —
- **Tier**: full

### M2 — History saved at a process end is restored after a service restart

- **Tasks**: T014–T024
- **Deliverable**: a session stopped before the session service restarts shows its history and a separator when started under the new service; the file is in `terminal-history/`, readable only by the user.
- **Satisfies**: US1 acceptance scenarios 2–6; FR-001, FR-006, FR-008, FR-013, FR-016, FR-019, FR-020, FR-025; FR-002 (process exit); SC-004
- **Verify**: `cargo test -p micold-daemon --test history_service_restart --test history_timing`
- **Depends on**: M1
- **Tier**: full

### M3 — A running terminal is saved every 30 seconds

- **Tasks**: T025–T030
- **Deliverable**: a printing session's file is rewritten at most once per 30 s and an idle one never; after the service is killed, a restart restores the history up to the last save.
- **Satisfies**: US1 acceptance scenario 7; US3 acceptance scenario 6 (retry); FR-003, FR-004, FR-005 (nothing dropped; its delay bound is SC-005, measured in M5), FR-007 (warning and retry); SC-002, SC-003
- **Verify**: `mise run test-core` (`terminal_history_schedule`) and `cargo test -p micold-daemon --test history_periodic_save`
- **Depends on**: M2
- **Tier**: full

### M4 — An orderly stop of the session service loses no output

- **Tasks**: T031–T036
- **Deliverable**: after the service stops itself when idle, or is sent `SIGTERM` (**Restart service**, logout, reboot on Linux and macOS), a restart restores every line the terminals held.
- **Satisfies**: US1 acceptance scenarios 1, 8; FR-002; SC-001
- **Verify**: `cargo test -p micold-daemon --test history_stop_request`
- **Depends on**: M3
- **Tier**: full

### M5 — The setting: keep terminal output off the disk

- **Tasks**: T037–T048, T074
- **Deliverable**: Settings → Terminal has **Save terminal history**; unticking it and saving deletes every saved history at once and stops further writes, with no restart.
- **Satisfies**: US2 acceptance scenarios 1–8; FR-005 (the delay bound), FR-026, FR-027, FR-028, FR-029, FR-031, FR-033; SC-005, SC-008
- **Verify**: `cargo test -p micold-daemon --test history_setting --test history_timing` and quickstart Part B steps B6–B8
- **Depends on**: M3
- **Tier**: full

### M6 — A damaged saved history never stops a session

- **Tasks**: T049–T055
- **Deliverable**: a session whose saved file is damaged, unreadable or from another version starts normally and shows "earlier output could not be restored"; the reason is in *Session service diagnostics*.
- **Satisfies**: US3 acceptance scenarios 1–6; FR-007 (logged once), FR-016, FR-017, FR-018; SC-006
- **Verify**: `cargo test -p micold-daemon --test history_damaged`
- **Depends on**: M3
- **Tier**: full

### M7 — Saved history goes away with its session

- **Tasks**: T056–T060
- **Deliverable**: Close, Remove, deleting a worktree and forgetting a project delete the session's saved history before the action completes; a service start removes leftover files.
- **Satisfies**: US4 acceptance scenarios 1–6; FR-023, FR-024; SC-007
- **Verify**: `cargo test -p micold-daemon --test history_removal`
- **Depends on**: M5
- **Tier**: full

### M8 — The stop request on Windows

- **Tasks**: T061–T066
- **Deliverable**: on Windows, **Restart service**, an update and a logout ask the service to stop, and it saves every terminal's history before it exits.
- **Satisfies**: US1 acceptance scenario 1 on Windows; FR-002, FR-030
- **Verify**: CI's Windows job: `cargo test -p micold-daemon --test history_stop_request` and `cargo test -p micold-core spawn`; on any platform `cargo test -p micold-core --test windows_installer_in_use`
- **Depends on**: M4
- **Tier**: full

### M9 — Saved history in the sandbox

- **Tasks**: T067–T073
- **Deliverable**: a session service in the sandbox restores a history saved on the host and the reverse, keeps it when the container is recreated, and shows the host's local time in the separator.
- **Satisfies**: FR-019 (Windows hosts), FR-021, FR-022; SC-009 (container); edge case *Where the service runs*
- **Verify**: `mise run image && mise run test-sandbox`; its output lists the `sandbox_real_history_*` tests as run, not filtered out
- **Depends on**: M4, M5
- **Tier**: full

### M10 — Architecture page and the recorded visual pass

- **Tasks**: T075–T077
- **Deliverable**: `docs/development/architecture.md` describes the history modules and the stop request, and quickstart Part B is recorded with screenshots.
- **Satisfies**: SC-010; quickstart Part B
- **Verify**: `mise run gate` and the files under `specs/041-terminal-scrollback-persistence/evidence/`
- **Depends on**: M1–M9
- **Tier**: full
