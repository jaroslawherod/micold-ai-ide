# Data Model: Terminal History That Survives a Session Service Restart

Types for [plan.md](./plan.md). Decisions and their reasons are in [research.md](./research.md).
All types are new unless a section says otherwise. None of them is on the wire except §8.

## 1. `HistorySnapshot` (`micold_core::terminal_history`)

What one terminal held at one moment (R2). Built by `capture` in the daemon, consumed by `seed` and
by the file format.

| Type | Fields | Rules |
|---|---|---|
| `HistorySnapshot` | `lines: Vec<LogicalLine>` | Oldest first. History rows, then screen rows down to the last row that shows anything. Empty means "nothing to show" (FR-010) |
| `LogicalLine` | `text: String`, `runs: Vec<StyleRun>` | Grid rows joined by the wrap flag are one line. `text` holds no C0, C1 or `ESC` character. Wide-character spacer cells are skipped; zero-width characters follow their base character |
| `StyleRun` | `chars: u32`, `style: HistoryStyle` | Runs cover `text` in order; the sum of `chars` equals the number of characters. A line in the default style has one run, or none when empty |
| `HistoryStyle` | `fg: HistoryColor`, `bg: HistoryColor`, `flags: StyleFlags` | |
| `HistoryColor` | `Default`, `Basic(u8)` 0 to 15, `Dim(u8)` 0 to 7, `Indexed(u8)`, `Rgb(u8, u8, u8)` | Own numbering, independent of `alacritty_terminal`'s enum. A named colour with no counterpart (cursor, etc.) maps to `Default` |
| `StyleFlags` | bit set: bold, dim, italic, underline, inverse, strikethrough, hidden | Every underline kind maps to underline. Hidden is kept so concealed text is not restored readable |

Not captured: hyperlinks, images, the cursor, terminal modes, the alternate screen's primary grid
(spec Assumptions, R2).

## 2. `SavedHistory` (the file payload)

The `postcard` form inside the file ([contracts/saved-history-file.md](./contracts/saved-history-file.md)).

| Type | Fields |
|---|---|
| `SavedHistory` | `styles: Vec<HistoryStyle>`, `lines: Vec<SavedLine>` |
| `SavedLine` | `text: String`, `runs: Vec<SavedRun>` |
| `SavedRun` | `chars: u32`, `style: u32` (index into `styles`) |

`HistorySnapshot` ↔ `SavedHistory` conversion deduplicates styles. The types are private to
`format.rs`; a change that alters their encoding fails the pinned fixture test until
`FORMAT_VERSION` is bumped.

## 3. `LoadOutcome` and `DamageReason` (`terminal_history::store`, `::format`)

```text
LoadOutcome = None | History(HistorySnapshot) | Damaged(DamageReason)
```

`DamageReason` is one of: `Unreadable(io::ErrorKind)`, `TooLarge`, `NotAHistory` (magic),
`OtherVersion(u32)`, `Truncated`, `Checksum`, `Malformed` (postcard), `BadStyleIndex`,
`BadRunLength`, `ControlCharacter`. Its `Display` is the reason written to the log (FR-017); for
`OtherVersion` it reads "written by another version".

Rules: a missing file is `None`, never `Damaged`. A read never panics and never returns part of a
file (FR-016). A history with no lines loads as `History` with an empty snapshot, which seeds
nothing.

## 4. `SaveSchedule` (`terminal_history::schedule`)

Pure; one per covered running terminal, held by the saver (R6).

| Field | Meaning |
|---|---|
| `last_attempt: Option<Instant>` | When the last save was tried, successful or not |
| `saved_count: Option<u64>` | `VtSignals::output_count` at the last successful save; `None` = must save |

| Operation | Rule |
|---|---|
| `new(output_count)` | A terminal that just started: nothing to save until the count moves |
| `due(now, output_count)` | `saved_count != Some(output_count)` and (`last_attempt` is `None` or `now - last_attempt >= 30 s`) |
| `saved(now, output_count)` | Sets both fields |
| `failed(now)` | Sets `last_attempt` only, so the save is due again 30 s later (FR-007) |
| `mark_due()` | `saved_count = None`: the setting was turned on (FR-027), or a damaged file was skipped (FR-018) |

`SAVE_SPACING = 30 s`, `SAVER_TICK = 5 s`. Worst case from output to disk: 30 + 5 s plus the write,
inside FR-003's 60 s.

## 5. `HistoryStore` (`terminal_history::store`)

Owns the directory. One `Mutex` around its state and its file operations, so a save, a removal and
a change of the setting never interleave (R9). It never takes the service's state lock.

| State | Meaning |
|---|---|
| `dir: PathBuf` | `history_dir()` = `ProjectDirs::from("", "", "micold-ai-ide").data_local_dir()` joined with `terminal-history` (R7) |
| `enabled: bool` | The setting |
| `create_dir: bool` | False in a container (`MICOLD_IMAGE_REFERENCE` set): save only when `dir` exists (R15) |
| `forgotten: HashSet<SessionId>` | Removed sessions; a later `save` for one writes nothing |
| `undeleted: HashSet<SessionId>` | Files whose deletion failed while turning the setting off. An id stays here, and is retried, until its file is deleted or replaced by a new save, also after the setting is turned on again |
| `last_written: HashMap<SessionId, [u8; 32]>` | Checksum of the file last written for each session in this service run |

| Operation | Result | Rule |
|---|---|---|
| `save(id, &snapshot)` | `Saved`, `Unchanged`, `Skipped(why)`, `Err(reason)` | Skipped when not enabled, when `id` is forgotten, or when `dir` is absent and `create_dir` is false. `Unchanged`, with no write, when the encoded bytes have the checksum in `last_written` (FR-004). Otherwise writes through `owner_only::write` and removes `id` from `undeleted`. Encoding happens before the mutex is taken; only the comparison and the write hold it |
| `load(id)` | `LoadOutcome` | `None` when not enabled, and for an `id` in `undeleted` (a history that was to be deleted never comes back) |
| `forget(ids)` | failures | Adds to `forgotten`, deletes each file. Runs whether enabled or not (FR-023) |
| `set_enabled(on)` | failures | Off: deletes every `*.history` and temporary file in `dir`, records failures in `undeleted`, clears `last_written`. On: `undeleted` is kept and still retried |
| `retry_deletions()` | failures | Tries `undeleted` again; called on the saver tick (FR-033) |
| `sweep(keep)` | count | Service start, saving on: deletes every file whose name is not `<id>.history` for an `id` in `keep` (FR-024) |
| `purge()` | failures | Service start, saving off: as `set_enabled(false)` (FR-033) |

## 6. Daemon state (`micold-daemon`: `state.rs`, `history.rs`)

| Item | Type | Lifetime |
|---|---|---|
| `Inner.carried` | `HashMap<SessionId, HistorySnapshot>` | Put when a covered process ends (stop, exit, before a respawn), by the order below; taken by the next start; dropped in `remove_live_by_ids` with the session. Filled whether saving is on or off (FR-015) |
| `Seed` | `None`, `History { snapshot, at: DateTime<Local> }`, `Notice` | Built at a start, passed to `spawn_answering`, consumed before the reader thread exists |
| Saver state | `HashMap<SessionId, SaveSchedule>`, `logged: HashSet<(SessionId, String)>` | Owned by the saver task; a schedule is dropped when its terminal is no longer live |

**Covered terminal**: `SessionProcess::Primary` of a session whose mode is `TerminalMode::AiCli`.
Nothing else is captured, carried, seeded or saved (FR-014).

**Order at a process end** (R4): clone the `SharedTerm`; take the `PtySession` out of the state and
drop it off the state lock (kill, close the master, join the reader); capture from the clone; put
the snapshot in `carried`; save it; then reply to the stop or spawn the next process. No timeout is
involved, and the order is the same on Unix and on Windows.

**Choosing the seed at a start**:

| `carried` has the id | Saving | `load` | Seed |
|---|---|---|---|
| yes, with lines | any | not called | `History` |
| yes, no lines | any | not called | `None` |
| no | off | not called | `None` |
| no | on | `None` or empty | `None` |
| no | on | `History` with lines | `History` |
| no | on | `Damaged(reason)` | `Notice`; one `warn!` with the session and the reason; the new schedule is `mark_due()` |

**`seed(term, seed, limit)`**: keeps the most recent `limit + screen rows` lines; for each, sets the
attributes of each run and feeds the characters through `vte::ansi::Handler` (`terminal_attribute`,
`input`), then `carriage_return` and `linefeed`; then the separator or the notice the same way;
then resets the attributes; then moves the screen into the history and homes the cursor
(`clear_screen(ClearMode::All)`, R17), so the new process starts on a blank screen with the seeded
lines directly above it. Nothing is written to the PTY (FR-009).

## 7. Separator and notice (`terminal_history::text`)

| Function | Full text | Narrow terminal |
|---|---|---|
| `separator_line(at, columns)` | `── session restarted at 2026-10-02 14:31 +02:00 ──` | The rules are dropped first, then the text is cut to `columns`; always one row |
| `notice_line(columns)` | `── earlier output could not be restored ──` | The same |

Both are seeded in the dim style. `at` is the local time of the start with its UTC offset; where the
zone is unknown (a container without `TZ`) it is UTC with `+00:00` (R11). The functions take the
already formatted parts, so `micold-core` needs no time crate.

## 8. Setting: `Settings.save_terminal_history` (existing types, one new field)

`bool`, default `true`, global, service-owned. On disk in `settings.json`, on the wire in
`DaemonSettings` and as `Option<bool>` in `ClientMsg::SettingsSet`
([contracts/setting.md](./contracts/setting.md)).

## 9. Sandbox: history mount (`micold_core::sandbox`)

| Host | Container path `/var/lib/micold-ai-ide/terminal-history` is |
|---|---|
| Linux, macOS | A directory inside the state mount (the host's data directory); no new mount |
| Windows | A new mount whose source is the host's `data_local_dir()\terminal-history`, added when `data_local_dir()` differs from `data_dir()` |

The launcher runs `owner_only::ensure_dir` on the host directory at every bring-up, attach
included. The container also gets `TZ=<host IANA zone>` when it is created (R11, R15).

## State of one terminal's history

```text
            process runs ── output ──▶ schedule due ──▶ save (file replaced)
                 │                                         ▲
   stop / exit   │                                         │ saving on
                 ▼                                         │
        capture ──▶ carried[id] ───────────────────────────┘
                 │
     start       ▼
        Seed::History ──▶ new Term (history + separator) ──▶ process runs

   service stops (idle or requested): capture every covered terminal ──▶ save, bounded 3 s
   service starts: saving off ──▶ purge;  saving on ──▶ sweep(keep = non-archived sessions)
   start with nothing carried, saving on: load ──▶ History | None | Damaged ──▶ Seed
   session removed: forget(id), carried.remove(id)
   setting off: set_enabled(false) deletes every file; carried and every Term unchanged
   setting on: every running schedule mark_due()
```
