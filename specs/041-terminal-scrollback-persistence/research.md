# Research: Terminal History That Survives a Session Service Restart

Phase 0 of the plan for [spec.md](spec.md). Each section records one decision, why, and what was
rejected. Paths are relative to the repository root; line numbers are as of `origin/main` at
`bc569992`.

**Status**: R1 to R17 are decided. R16 was decided by the user (ledger D11).

## R1. Where the history is today

**Finding.** The session service keeps one `alacritty_terminal::Term` per process
(`crates/micold-daemon/src/supervisor.rs:363-367`, `scrolling_history` = the *Scrollback lines*
setting). A reader thread is the only writer (`supervisor.rs:375-394`). `Framer`
(`crates/micold-daemon/src/framer.rs`) reads the grid for frames and for
`ClientMsg::ScrollbackRequest`. Nothing is on disk.

The `Term` is dropped with its process:

- `DaemonState::stop_session` removes the `LiveSession` and drops it (`state.rs:1487-1506`).
- `supervise_exited_sessions_at` drops it on a clean exit or a give-up, and `respawn_primary`
  replaces it on a restart (`state.rs:2640-2749`, `state.rs:3012`).
- `unwind` (orderly stop: idle window or a request) marks sessions interrupted, then
  `take_live_sessions` drops them all (`server.rs:419-451`, `state.rs:2228-2256`).

So today a stop and start inside one service run also starts with an empty terminal. FR-015 needs
the same mechanism as FR-008, only without the disk.

## R2. What is captured

**Decision.** A *history snapshot* is taken from the `Term`'s active grid: the history rows plus
the screen rows down to the last row that shows anything (the rule of `Framer::plain_tail`,
`framer.rs:253-271`). Rows joined by the `WRAPLINE` flag become one *logical line*. A logical line
is its text plus style runs: foreground, background (named, indexed, RGB), and the flags bold, dim,
italic, underline, inverse, strikethrough. Wide-character spacer cells are skipped and zero-width
characters follow their base character. Hyperlinks, the cursor and terminal modes are not captured
(spec Assumptions).

**Rationale.** It is what the user could scroll back to, which is what the framer already serves.
Logical lines make the restore reflow at the new width for free (edge case *Terminal size changed*).

**Rejected.**

- *Saving the raw PTY byte stream.* Unbounded, replays cursor movement and queries, and a query in
  the stream would make the terminal answer into the new process (FR-009 forbids input).
- *Saving `WireLine`/`WireStyle` as they are.* It ties the file format to the wire schema: a wire
  change would silently change the file. The history module has its own types and its own format
  version.
- *Reading the inactive (primary) grid while a full-screen program is on the alternate screen.*
  `Term` does not expose it. The spec already says such a program is restored only as its last
  screen.

## R3. How a snapshot is shown again

**Decision.** At a start, before the reader thread exists, the new `Term` is *seeded*: for each
logical line the daemon calls the `Term`'s `vte::ansi::Handler` methods (`terminal_attribute`,
`input`, `carriage_return`, `linefeed`), then writes the separator line the same way, then resets
the attributes. `PtySession::spawn_answering` (`supervisor.rs:296`) gains a seed argument;
`spawn_ai_cli` passes it, `spawn_shell` passes none.

**Rationale.** The seeded lines are ordinary history of that `Term`. Everything downstream works
unchanged: the scrollback limit trims them (FR-012), `ScrollbackRequest` serves them, a later
snapshot includes them and the separators (FR-011), every window sees the same grid (edge case
*Several windows*). The client needs no change for restoring: a new process starts a fresh
`Framer`, whose first frame is `full`, and `GridCache::apply` clears the cache on a `full` frame
(`crates/micold-client/src/grid.rs:109-119`), which is the reattach path
(`crates/micold-daemon/tests/reattach_snapshot.rs`).

No byte stream is involved, so nothing read from a file can become an escape sequence. The decoder
also rejects a text that holds a control character (R5).

**Rejected.**

- *Rendering the snapshot to ANSI bytes and feeding the VT parser.* It works, but a damaged or
  edited file could then carry a query (`DSR`, `OSC 10`) that `DaemonListener` answers into the PTY.
- *Keeping the old `Term` and attaching the new process to it.* The size, modes and cursor of a
  dead process would leak into the new one, and it does nothing for a service restart.
- *A separate client-side "restored" pane or wire message.* FR-008 wants the history reachable
  exactly as live history is; a second surface would need its own scrolling, selection and search.

## R4. One mechanism for a start in the same run and after a service restart

**Decision.** `Inner` (`state.rs:114`) gains `carried: HashMap<SessionId, HistorySnapshot>`, beside
`ended` and `sizes`, which already outlive the `LiveSession`. A snapshot is put there whenever the
AI CLI process of a session ends: in `stop_session`, and in the supervision tick before
`remove_session` or `respawn_primary`. A start (`start_session`, `respawn_primary`) takes the
snapshot from `carried`; when there is none and saving is on, it loads the saved history from disk.
`remove_live_by_ids` (`state.rs:2070`) drops the entry with the session.

`carried` is filled whether saving is on or off (FR-015, User Story 2 scenario 8). The setting
decides only whether the snapshot is also written to disk.

**Covered terminal.** `SessionProcess::Primary` of a session whose `mode` is `TerminalMode::AiCli`
(`state.rs:2415-2447`). A `Shell(_)` instance, and the primary of a `Regular` session, are never
captured, carried or saved (FR-014).

**Order at a process end.** Keep a clone of the session's `SharedTerm` (`supervisor.rs:60`, an
`Arc`), take the session's `Arc<PtySession>` out of the state, and off the state lock call a new
`PtySession::teardown(&self, bound)`; then capture from the clone, put the snapshot in `carried`,
and only then reply to the stop or spawn the next process. The same order serves `stop_session` and
the supervision tick (clean exit, give-up and before `respawn_primary`).

`teardown` does, through a shared reference, what `Drop` does today (`supervisor.rs:548-564`): kill
the process tree, take the master out of its `Mutex<Option<…>>` and close it, then wait for the
reader thread to reach end-of-file and join it (`reader` becomes a `Mutex<Option<JoinHandle>>`).
`Drop` calls it, and it does nothing the second time. So every byte the process wrote has reached
the `Term` before the capture (User Story 1 scenario 10), on Unix and on Windows.

Dropping the `Arc` is not enough. The state holds an `Arc<PtySession>` (`state.rs:237`), and the
stream task of every window that shows the session holds a clone (`server.rs`, `stream_view`). With
a window on the session, which is the usual case at a stop or a self-exit, the state's drop runs no
`Drop`: the master stays open and the reader is not joined, so a capture after it would race the
reader on Unix and come before ConPTY's end-of-file on Windows. A resize that arrives after the
teardown finds no master and is ignored.

The wait for end-of-file is bounded at 2 s (`TEARDOWN_WAIT`): the reader sets `output_ended`, and
`teardown` joins only once it is set. A process outside the killed tree that keeps the terminal
open (a detached grandchild) would otherwise hold the stop for ever. When the bound passes, the
capture takes what the `Term` has parsed, one warning is logged, and the reader thread is left to
end by itself. The bound is a guard, not the mechanism: closing the master first is what brings the
end-of-file.

Waiting for `output_ended` without closing the master does not work on Windows: with ConPTY the
reader sees end-of-file only when the pseudoconsole is closed, which is the close of the master,
not the kill (`supervisor.rs:553-558`). The capture would then always rest on the timeout.

**Rejected.** *Always reading back from disk at a start.* It cannot serve FR-015 while saving is
off, and it makes a stop and start depend on a write that may have failed.

## R5. The saved-history file

**Decision.** One file per session:
`<local data dir>/terminal-history/<session uuid>.history`.

| Part | Content |
|---|---|
| Magic | 8 bytes, `MICOLDTH` |
| Format version | `u32` little-endian, starts at 1 |
| Payload length | `u64` little-endian |
| Payload | `postcard` of `SavedHistory { lines, styles }` (workspace dependency, `Cargo.toml:33`) |
| Checksum | SHA-256 of all bytes before it (`micold_core::protocol::hashing::sha256`) |

Reading checks, in order: file size against a cap derived from `MAX_SCROLLBACK_LINES`
(`crates/micold-core/src/settings.rs:34`), magic, version, length, checksum, then that every style
index and run length is in range and that no text holds a C0, C1 or `ESC` character. Any failure
gives `Damaged(reason)`; a missing file gives `None`. A read never panics and never returns part of
a file.

The whole file is rewritten at each save, through a temporary file renamed over the old one
(`write_owner_only`, R8). The helper syncs the temporary file to disk before the rename
(`File::sync_all`; on Unix it then syncs the directory), so after a power loss the name holds
either the previous complete file or the new complete one (FR-006, User Story 1 scenario 7). The
helper does not sync today (`platform/unix.rs:149-151`); the sync is added when it moves to
`micold-core`. It runs on the blocking pool, off every lock a terminal needs. A file that is torn
all the same (a disk that lies about the sync) fails the checksum and is treated as damaged.

**No write for unchanged content.** The store remembers the checksum of the last file it wrote for
each session in this service run and skips a save whose bytes have the same checksum. A save at a
process end or at a service stop therefore writes nothing for a terminal that printed nothing since
its last save (FR-004).

**Rationale.** 10,000 lines of about 100 characters are about 1 MB; rewriting that at most once per
30 seconds per busy session needs no compression and no append log. The format is the same bytes on
every platform and in the container (FR-022).

**Rejected.**

- *An append-only log of new lines.* The screen rows change in place, so an append log needs a
  rewrite rule anyway, and a torn tail needs a recovery rule.
- *JSON.* Three to five times larger, and slower to restore against FR-013.
- *Compression (`zstd`, `flate2`).* A new direct dependency for a file of about 1 MB.
- *SQLite or sled.* One blob per session needs no store.

A test pins the encoded bytes of one fixture, so a change of the types that changes the format
fails until the version is bumped. A file of another version is `Damaged("written by another
version")` (edge case *Saved history from a newer or older app version*).

## R6. When a save happens

**Decision.** A `SaveSchedule` per covered terminal, pure and in `micold-core`: it holds the time of
the last save and the `VtSignals::output_count` (`crates/micold-daemon/src/terminal.rs:56-76`) seen
at that save. `due(now, output_count)` is true when the count moved and at least 30 seconds passed
since the last save. A saver task ticks every 5 seconds, asks each schedule, and saves the due ones
one at a time on the blocking pool.

| Requirement | How |
|---|---|
| FR-003 two saves at least 30 s apart | the schedule's spacing |
| FR-003 output on disk within 60 s | worst case 30 s spacing + 5 s tick + the write |
| FR-004 idle terminal causes no write | the count did not move |
| SC-003 at most 21 writes in 10 minutes | 600 s / 30 s = 20, plus the save at exit |
| FR-002 save at process exit and orderly stop | the capture points of R4, and a new step in `unwind` before `take_live_sessions` |
| FR-027 turned on: saved within 60 s | the schedule of a running terminal is marked due when the setting turns on |
| FR-007 retry and log once | a failed save leaves the schedule due after 30 s; a set of `(session, reason)` already logged in this run |

**Cost on the terminal.** The capture copies rows under the `Term` lock, which the reader thread
also takes. `Framer::frame` already hashes every history row under that lock on each dirty tick
(`framer.rs:109-112`), so one capture per 30 seconds costs about one more frame tick. Encoding,
hashing and writing run off the lock. SC-005 is measured by a test (R12).

**Rejected.** *A timer per session*, *saving on every frame tick*, *a configurable interval* (spec:
out of scope).

## R7. Where the files are

**Decision.** `directories::ProjectDirs::from("", "", "micold-ai-ide").data_local_dir()` joined with
`terminal-history`, the base the service log already uses
(`crates/micold-daemon/src/logging.rs:241-243`).

| Platform | `data_local_dir()` | Note |
|---|---|---|
| Linux | `~/.local/share/micold-ai-ide` | same as `data_dir()` |
| macOS | `~/Library/Application Support/micold-ai-ide` | same as `data_dir()` |
| Windows | `%LOCALAPPDATA%\micold-ai-ide\data` | `data_dir()` is under `%APPDATA%`, the roaming profile (`logging.rs:422-423`) |

`projects.json` and `settings.json` use `data_dir()` (`crates/micold-core/src/store.rs:612`,
`settings.rs:504`). Saved histories must not (FR-019).

**In the container.** The image sets `XDG_DATA_HOME=/var/lib`, so both functions give
`/var/lib/micold-ai-ide` (`STATE_CONTAINER_DIR`, `crates/micold-core/src/sandbox/mod.rs:362-367`),
which is the `StateMount` of the host's `data_dir()` (`sandbox/mod.rs:657-660`). On Linux and macOS
hosts that is the same directory a host service uses, so FR-021 and FR-022 hold with no new mount.

On a Windows host the state mount is the roaming directory. **Decision:** the launch spec gains a
*history mount*, host `data_local_dir()/terminal-history` to container
`/var/lib/micold-ai-ide/terminal-history`, added only when the host's `data_local_dir()` differs
from its `data_dir()`. The launcher creates the host directory owner-only before the container
starts (R8).

**Rejected.** *Using `data_dir()` everywhere* (roams on Windows, against FR-019 and D6). *A separate
container path* (the daemon would then need to be told where; today it finds its directory "without
being told").

## R8. Only the user can read them

**Decision.** Reuse `platform::write_owner_only(dir, file, bytes)`
(`crates/micold-daemon/src/platform/mod.rs:65`): directory `0700` and file `0600` on Unix
(`platform/unix.rs:129-153`), a protected DACL with one ACE for the current user on Windows
(`platform/windows.rs:155-223`), written to a temporary file that is owner-only before it is renamed
into place. It already serves the tool-server binding files (`mcp/server.rs:87`).

The launcher (R7) needs the directory part on the host, and the launcher is not in
`micold-daemon`. **Decision:** move the helper to a new `micold_core::owner_only` module
(`write`, `ensure_dir`); `platform::write_owner_only` delegates to it, so its callers and tests do
not change. `micold-core` already has the Windows SID code it needs (`endpoint::user_sid`).

**Rejected.** *A second copy of the DACL code in the launcher* (two copies of a security check).
*Relying on the profile directory's inherited ACL on Windows* (FR-020 says from the moment they are
created).

## R9. Removing, sweeping and turning saving off

**Decision.** A `HistoryStore` owns the directory. Its operations take one mutex, so a save and a
delete of the same session never interleave:

| Operation | Used by |
|---|---|
| `save(id, snapshot)` | the saver, the capture points of R4. Refuses when saving is off or `id` was forgotten. |
| `load(id)` → `None`, `History`, `Damaged(reason)` | a start with nothing in `carried` |
| `forget(id)` | every path that archives a session; records `id` so a save already in flight writes nothing (User Story 4 scenario 5) |
| `set_enabled(false)` | the setting; deletes every file, and keeps the ids whose deletion failed for a retry every 30 s on the saver's tick (FR-033) |
| `sweep(keep)` | service start with saving on: deletes each file whose id is not a non-archived session of the catalog (FR-024) |
| `purge()` | service start with saving off, before the accept loop (FR-033, edge case *Setting turned off while the service is not running*) |

Every archive path already meets in `DaemonState::revoke_tool_credentials(&ids)`
(`state.rs:2050`; its callers: `archive_and_remove_worktree_sessions`, `forget_project`,
`delete_session`, `prune_empty_sessions`). `forget` is called there, outside the state lock, before
the handler replies, so the files are gone before the action is reported as done (FR-023). The
store never takes the state lock, so the order state lock then store mutex cannot invert.

Turning the setting off does not touch `carried` or any `Term` (FR-033: the history a running
terminal shows is unchanged).

**Rejected.** *Asking the catalog under the store mutex whether a session still exists* (two locks
in two orders). *Deleting lazily at the next tick* (FR-023 says before the action is reported).

## R10. The setting

**Decision.** `save_terminal_history: bool`, default `true`, service-owned like
`pi_activity_component`, whose path it copies end to end:

| Place | Pattern to copy |
|---|---|
| `Settings` and its on-disk form | `crates/micold-core/src/settings.rs:145-146, 180, 194, 401-402, 448, 479` |
| `DaemonSettings`, `ClientMsg::SettingsSet` | `crates/micold-core/src/protocol/messages.rs:529, 1098` |
| `Catalog::set_…`, `settings_wire` | `crates/micold-daemon/src/catalog.rs:158, 298, 355-362` |
| `DaemonState::set_…` and the handler | `state.rs:1330`, `server.rs:968-1010` |
| Client draft, message, persist | `crates/micold-client/src/features/settings.rs`, `shell/persist.rs`, `shell/daemon_sync.rs` |
| The control | `Checkbox` with `field_note`, as `crates/micold-client/src/ui/settings/environment.rs:113-127`; added to `ui/settings/terminal.rs` and its `SETTINGS` list |

`PROTOCOL_VERSION` (`crates/micold-core/src/protocol/version.rs:78`, now 20) is bumped by one for
the new field; features 039 and 040 are in flight, so the milestone takes the next free number when
it is implemented. `#[serde(default)]` on the stored field keeps an old `settings.json` readable
(FR-029).

The sentence beside the control: "Terminal output is written to this computer's disk while this is
on. Turning it off deletes the saved history."

## R11. The separator and the notice

**Decision.** Both are lines seeded into the `Term` (R3), in the dim style, so they are part of the
history and never reach the process.

- Separator: `── session restarted at 2026-10-02 14:31 +02:00 ──`. The text between the rules is
  fixed by FR-009. The rules are dropped, then the text is cut, when the terminal is narrower, so
  the separator is always one row.
- Notice, shown instead of history and separator when `load` gave `Damaged`:
  `── earlier output could not be restored ──`. The warning in the log names the session and the
  reason; warnings already reach *Session service diagnostics* through `RecentErrorsLayer`
  (`logging.rs:83`).
- No snapshot, or one with no lines: nothing is seeded (FR-010).

**Time.** `chrono::Local` in the daemon. `chrono` is already built (`Cargo.lock`, through
`file-rotate`); it becomes a direct dependency of `micold-daemon`. The app formats no date anywhere
today, so the format is set here: `YYYY-MM-DD HH:MM` and the UTC offset. The offset keeps an old
separator true after a time-zone change (edge case *Clock changes*).

**In the container** the zone is UTC: the image has no `tzdata` and no `TZ`
(`packaging/sandbox/Containerfile`, `crates/micold-core/src/sandbox/argv.rs:128-146`). **Decision:**
the launcher passes `TZ=<host IANA zone>` with the other `-e` values (`iana-time-zone` is already in
`Cargo.lock`), and the image installs `tzdata`. Where the zone cannot be found the separator shows
UTC with `+00:00`, which is still a true time.

**Rejected.** *The client sends its offset* (an automatic restart has no client; the offset goes
stale at a DST change). *A time without an offset* (wrong by hours in a container without `TZ`).

## R12. Which layer tests what

| Layer | What |
|---|---|
| `micold-core` unit | file format round trip, each way a file is damaged, the pinned fixture, `SaveSchedule` (30 s spacing, idle, 21 writes in 10 minutes, retry), separator and notice text and widths, `HistoryStore` on a temp directory (save, load, forget against a save in flight, set_enabled(false), sweep, purge, modes `0700`/`0600`) |
| `micold-daemon` unit | capture from a `Term` (colours, styles, wrapped lines, wide and zero-width characters), seed into a `Term`, capture after seed equals the input |
| `micold-daemon` integration (`tests/`, `Catalog` on a temp directory, a fake CLI) | every acceptance scenario of User Stories 1 to 4 that needs a process: stop and start, exit and restart, a second `DaemonState` on the same directory as a service restart, two sessions, damaged file, removal paths, the setting both ways. Patterns: `tests/daemon_lifecycle.rs`, `tests/session_survival.rs`, `tests/reattach_snapshot.rs` |
| `micold-daemon` timing tests | FR-013 / SC-004 (10,000 lines seeded in under 1 s), SC-005 (ten terminals printing, echo delay with saving on against off) |
| `micold-client` | settings draft, message and persist tests (`tests/features_settings.rs`, `tests/settings_sections.rs`), protocol round trip and schema hash (`crates/micold-core/tests/protocol_roundtrip.rs`, `schema_hash.rs`) |
| Sandbox real-runtime suite (`mise run test-sandbox`) | history saved by a host service is restored by a container service and the reverse; file modes seen from the host; `TZ` reaches the container |
| quickstart Part B (visual pass) | the separator and the notice in the real terminal, the Settings control, and a resume of each real AI CLI (R13) |

Windows and macOS: the file modes and DACL are tested by `cfg` tests that CI runs on all three
platforms; `cargo check --target aarch64-apple-darwin` before pushing a `cfg` arm.

## R13. What a program's own erase does

**Finding.** Restored lines are ordinary history (FR-011). A program that erases the scrollback
(`ESC [ 3 J`) erases them, as it erases live history today. `ESC [ 2 J` on the primary screen moves
the screen into history and loses nothing: in `alacritty_terminal` 0.26.0, `clear_screen` with
`ClearMode::All` calls `grid.clear_viewport()` when the terminal is not on the alternate screen,
and only `ClearMode::Saved` calls `grid.clear_history()` (`src/term/mod.rs:1788-1814` of the
crate).

**Measured**, 2026-10-02, each CLI in a 40 × 120 pseudo-terminal with a clean environment, 16
seconds from start:

| CLI | Command | `ESC[3J` | `ESC[2J` | Enters the alternate screen (`ESC[?1049h`) |
|---|---|---|---|---|
| Claude Code 2.1.288 | `claude --resume <id>` (a conversation of 120 lines) | 0 | 1 | yes, and does not leave it |
| Claude Code 2.1.288 | the same with `CLAUDE_CODE_NO_FLICKER=0` | 0 | 0 | no |
| Copilot CLI | `copilot` | 0 | 2 | yes, and does not leave it |
| Pi Coding Agent | `pi` | 0 | 0 | no |

No CLI erased the scrollback at start-up, so the risk this section recorded did not show. The
Claude Code binary does hold the sequence `ESC[2J ESC[3J ESC[H`, so it can send it later in a
session; that erases live history today as well.

**Decision.** Honour the erase. An integration test with a fake CLI that prints `ESC [ 2 J ESC [ H`
at start asserts that the restored history and the separator are still there. No guard against
`ESC [ 3 J` is planned.

The table's last column is the larger finding: see R16.

## R14. What makes a stop orderly

**Finding.** `unwind` (`crates/micold-daemon/src/server.rs:419`) is called in two places, both
with `StopReason::Idle` (`server.rs:99`, `server.rs:353`). `StopReason::Requested`
(`idle.rs:219-224`) is never raised. The service handles no signal (`grep` for `SIGTERM`,
`signal::`, `ctrl_c`, `signal_hook`, `sigaction` in `crates/micold-daemon` finds none), and
workspace `tokio` is built without its `signal` feature (`Cargo.toml:70-77`). So of the stops the
spec calls orderly, only the idle stop saves anything today:

| Stop | How it arrives | Reaches `unwind` today |
|---|---|---|
| Idle for 30 minutes | the idle timer | yes |
| **Restart service** (Linux, macOS) | `SIGTERM` to the recorded pid (`crates/micold-core/src/spawn.rs:146-159`) | no: the default action ends the process |
| **Restart service** (Windows) | `TerminateProcess` (`spawn.rs:168-250`) | no, and it cannot be caught |
| Update (Windows installer) | `Stop-Process -Force`, then `taskkill` (`packaging/windows/micold-ai-ide.iss:102-111`) | no |
| Logout, reboot (Linux, macOS) | `SIGTERM` from the session manager, then `SIGKILL` | no |
| Logout, reboot (Windows) | the process is ended; it has no console (`DETACHED_PROCESS`, `spawn.rs:349-351`) and no window, so it is told nothing | no |
| Sandbox stop (`<runtime> stop`, `sandbox/cli.rs:262`) | `SIGTERM` to pid 1, which is the service (`packaging/sandbox/Containerfile:123`); pid 1 ignores a signal it has no handler for, so the runtime kills it after its grace period | no |

**Decision.** A `platform::stop_requested()` future in `micold-daemon` that the two accept loops
select on beside the idle timer, and that leads to `unwind(StopReason::Requested)`:

- **Unix**: `SIGTERM`, `SIGINT` and `SIGHUP` through `tokio::signal::unix` (the `signal` feature of
  the `tokio` already in the workspace; no new crate). This covers Restart service, logout, reboot
  and the sandbox stop.
- **Windows**: a named event `Local\Micold.Daemon.Stop.<SID>`, created with the pipe's owner-only
  DACL. `terminate_daemon` sets it, waits up to 5 seconds for the process to exit, and only then
  falls back to `TerminateProcess`; the installer script does the same before `Stop-Process`.
  For logout and reboot, a hidden top-level window on its own thread answers `WM_ENDSESSION` by
  raising the same request and waiting for the save.

**The save in `unwind`.** A new step before `take_live_sessions`: capture every covered terminal
from its live `Term` (no wait for the process, which is about to be killed anyway), then save them
in parallel on the blocking pool, the whole step bounded at 3 seconds. A terminal whose save does
not finish in time keeps its previous file (at most 60 seconds old). The endpoint is released
after `unwind`, as today, so a new service cannot start before the old one has saved.

**Not testable here.** A real Windows logout cannot be run in CI; a `cfg(windows)` test sets the
event and sends `WM_ENDSESSION` to the window instead. Where the request does not arrive, the
outcome is that of a kill: at most the last 60 seconds are missing (SC-002).

**Rejected.** *Leaving these stops as kills and relying on the 30-second saves*: the spec's Terms
call them orderly and SC-001 asks for every line. *A wire message that asks the service to stop*:
Restart service exists for a client that cannot complete the handshake
(`crates/micold-client/src/shell/service_control.rs:30-36`).

## R15. The sandbox on a Windows host, and a container made before this feature

**Finding.** The sandbox is supported on a Windows host (`sandbox/pathmap.rs:23`,
`sandbox/mod.rs:15`), so the history mount of R7 and the move of R8 stay. The launcher gets the
state directory from `ProjectDirs::…data_dir()` in `crates/micold-client/src/shell/startup.rs:252-253`
and passes it to `MountSet::build` (`shell/sandbox.rs:297`); the history mount is computed beside
it from `data_local_dir()`.

A container that already exists is attached to or started as it is, with the mounts it was created
with (`sandbox/lifecycle.rs:305-314`). On a Windows host such a container has no history mount, so
`/var/lib/micold-ai-ide/terminal-history` inside it would be a directory of the state mount, which
is the roaming profile (against FR-019).

**Decision.** In a container (`MICOLD_IMAGE_REFERENCE` is set, `state.rs:353-356`) the service
never creates the history directory: it saves only when the directory exists. The launcher creates
it on the host, owner-only, at every bring-up, attach included: on Linux and macOS inside the state
directory, on Windows as the history mount's source. A container made before this feature on a
Windows host therefore has no such directory and saves nothing; the service logs one warning that
says to recreate the sandbox, and the user guide says the same. `TZ` (R11) is likewise set only
when a container is created; an older container shows the separator in UTC with `+00:00`.

**Rejected.** *Replacing an existing container that lacks the mount*: it stops the sessions of a
sandbox the user asked to keep running. *An environment variable naming the directory*: it is also
fixed at creation and adds a second way to find the directory.

## R16. AI CLIs that draw on the alternate screen

**Finding.** Two of the three supported CLIs run full-screen by default on the measured machine
(R13's table): Claude Code 2.1.288 and Copilot CLI switch to the alternate screen at start, turn on
mouse reporting and stay there. Pi does not, and Claude Code does not with
`CLAUDE_CODE_NO_FLICKER=0`. Nothing in `crates/micold-core/src/provider.rs` sets that variable.

For a terminal on the alternate screen:

- it has no scrollback. What the agent did is shown and scrolled by the CLI itself; the app sends
  the wheel to the program (`docs/user-guide/worktrees-and-sessions.md:1215`). There is nothing to
  scroll back to before a restart either;
- R2 captures only the last screen, as the spec's edge case *Full-screen programs* says;
- R3 seeds that screen and the separator on the primary screen. The CLI covers it with its own
  view as soon as it starts, so the user sees neither until the CLI exits. FR-008, SC-001 and
  SC-010 cannot be observed for such a CLI;
- after a service restart the CLI's own `--resume` redraws the conversation, which is what the
  issue asks for ("see what an agent did before the restart").

So the feature as specified changes what the user sees only for a CLI that prints on the primary
screen. The spec treats full-screen programs as an edge case; for two of three CLIs it is the
normal case.

**Decision** (the user, 2026-10-02). Ship as specified for output on the normal screen, and say so
in the spec and the guide. No new setting, no change to how a CLI is started. The design of R2 and
R3 is unchanged: on the alternate screen the capture is the last screen, and the seed goes to the
primary screen before the process starts.

**Consequences for the plan.** The fake CLI of the integration tests prints on the normal screen.
One test starts a fake CLI that enters the alternate screen and asserts that the seeded lines and
the separator are in the primary grid's history and are not lost when it leaves. Quickstart Part B
checks the restored history with Pi and with `CLAUDE_CODE_NO_FLICKER=0 claude`, and records what a
default Claude Code and Copilot session shows.

**Rejected.** *Starting the CLIs in a scrolling mode behind a setting* (new scope; Copilot has no
known one). *Stopping the feature.*

## R17. Seeding and the Windows pseudoconsole

**Finding.** On Unix the new process writes at the terminal's cursor, so after a seed its first
output lands below the separator. On Windows the process draws through ConPTY, which keeps its own
screen buffer: it starts blank with the cursor at home and addresses rows absolutely. Its first
paint could overwrite seeded rows that are still on the screen. This was not run on a Windows
machine; R13's measurements are from a Unix pseudo-terminal.

**Decision.** `seed` ends by moving the seeded rows out of the screen into the history and homing
the cursor, on every platform: the `Term`'s `clear_screen(ClearMode::All)`, which on the primary
screen moves the screen into history and loses nothing (R13). The new process then starts on a blank
screen, as it does today, with the earlier output and the separator directly above it in the
history. The tests assert the order (earlier output, separator, new output) in the captured
history, not a screen row, and one integration test with a real pseudoconsole runs under
`cfg(windows)` in CI.

**Rejected.** *Leaving the seeded rows on the screen on Unix only*: two behaviours to test, and a
CLI that homes the cursor and repaints (as Claude Code's scrolling mode does on resume) would
overwrite them there too. *Telling ConPTY the cursor position*: there is no such call; ConPTY
learns it only by asking the terminal, which the daemon answers from the `Term` after the fact.
