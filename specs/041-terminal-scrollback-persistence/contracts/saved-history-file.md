# Contract: the saved-history file

What the session service writes to disk, where, and what it accepts back. Research:
[R5](../research.md#r5-the-saved-history-file), [R7](../research.md#r7-where-the-files-are),
[R8](../research.md#r8-only-the-user-can-read-them). Types: [data-model §1 to §3](../data-model.md).

## 1. Location

`<data_local_dir>/terminal-history/<session uuid>.history`, one file per session.

| Where the service runs | `<data_local_dir>` |
|---|---|
| Linux | `~/.local/share/micold-ai-ide` |
| macOS | `~/Library/Application Support/micold-ai-ide` |
| Windows | `%LOCALAPPDATA%\micold-ai-ide\data` (never `%APPDATA%`, FR-019) |
| Container | `/var/lib/micold-ai-ide` (mounted from the host, FR-021) |

The session uuid is the hyphenated lower-case form the catalog uses. Nothing else is in the
directory except temporary files of a write in progress.

## 2. Bytes

| Offset | Size | Content |
|---|---|---|
| 0 | 8 | Magic `MICOLDTH` |
| 8 | 4 | `FORMAT_VERSION`, `u32` little-endian; `1` |
| 12 | 8 | Payload length `n`, `u64` little-endian |
| 20 | `n` | `postcard` encoding of `SavedHistory` |
| 20 + `n` | 32 | SHA-256 of bytes `0 .. 20 + n` |

The bytes are the same on every platform and in the container (FR-022).

## 3. Writing

- The whole file is written at each save: to a temporary file in the same directory, made
  owner-only before any content is written, synced to disk (`sync_all`), then renamed over the old file; on Unix the
  directory is synced after the rename. A save that is interrupted, by a kill or by a power loss,
  leaves the previous complete file or the new complete one, never a mixture (FR-006).
- A save whose bytes have the same checksum as the file this service run last wrote for that
  session writes nothing (FR-004).
- Directory mode `0700`, file mode `0600` on Linux and macOS; on Windows a protected DACL with one
  entry for the current user on both (FR-020). The temporary file has the same protection.
- A service in a container does not create the directory: when it is absent, nothing is saved and
  one warning is logged per service run (R15).
- Nothing is written while the setting is off, and nothing for a session that was removed.

## 4. Reading

Checks, in this order; the first that fails gives `Damaged(reason)`:

| # | Check | Reason |
|---|---|---|
| 1 | The file opens and reads | `Unreadable` |
| 2 | Size ≤ `MAX_FILE_BYTES` (`MAX_SCROLLBACK_LINES` × 512) | `TooLarge` |
| 3 | Size ≥ 52 and magic matches | `NotAHistory` |
| 4 | Version equals `FORMAT_VERSION` | `OtherVersion` |
| 5 | Size equals 20 + `n` + 32 | `Truncated` |
| 6 | Checksum matches | `Checksum` |
| 7 | The payload decodes with nothing left over | `Malformed` |
| 8 | Every run's style index is inside `styles` | `BadStyleIndex` |
| 9 | Each line's run lengths sum to its number of characters | `BadRunLength` |
| 10 | No text holds a C0, C1 or `ESC` character | `ControlCharacter` |

Check 7 also covers semantic validity: after the ten checks, `decode` runs `HistorySnapshot::validate`
on the decoded snapshot and reports any failure (for example a basic or dim colour index outside its
palette) as `Malformed`.

A missing file is "no saved history", not damage. A read returns the whole history or none of it,
and never panics, for any bytes (a test feeds random and truncated input).

What a damaged file leads to: the session starts as one with no saved history, its terminal shows
the notice line, the log gets one warning naming the session and the reason, and the next save
replaces the file (FR-016 to FR-018).

## 5. Versions

- `FORMAT_VERSION` is independent of `PROTOCOL_VERSION` and of `SETTINGS_VERSION`.
- A file of any other version is damaged (`OtherVersion`); it is never migrated. The next save
  replaces it.
- `crates/micold-core/tests/fixtures/terminal_history/v1.history` pins the bytes of one snapshot. A
  change of the types that changes the encoding fails that test until the version is bumped.

## 6. Removal

| Event | Files removed | When |
|---|---|---|
| A session is removed (Close, Remove, worktree deleted, project forgotten, an unused session discarded) | That session's | Before the action is reported as done (FR-023) |
| The setting is turned off | All | Within 5 s; failures are retried every 30 s (FR-027, FR-033) |
| Service start, saving off | All | Before the first connection is accepted (FR-033) |
| Service start, saving on | Every file that is not `<id>.history` of a session that can still be shown | Before the first connection is accepted (FR-024) |

Stopping a session removes nothing.
