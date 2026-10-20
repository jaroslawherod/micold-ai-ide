# Data Model: Jira Ticket Status

Module `micold_core::jira_status` (new), plus one field on `Issue`.

## StatusCategory
Enum `ToDo | InProgress | Done`. From Jira `statusCategory.key`: `new` -> ToDo, `indeterminate` ->
InProgress, `done` -> Done; anything else -> no category.

## TicketStatus
- `name: String`: trimmed, non-empty (a blank name makes the whole status unusable -> `None`).
- `category: Option<StatusCategory>`: `None` renders the name in a neutral tint (edge case).
- Built by `TicketStatus::from_jira(name, category_key) -> Option<TicketStatus>`.

## Issue (existing, `github.rs`)
+ `status: Option<TicketStatus>`; `None` for GitHub issues and for Jira issues without a usable
status. Set by `Issue::with_status`. No effect on matching or ordering (FR-011).

## StatusEntry
`{ status: TicketStatus, stale: bool }`.

## StatusBoard
- `entries: BTreeMap<String /* Jira key */, StatusEntry>`
- `last_read: Option<Duration>`: monotonic time of the last `Ok` read (injected clock value).
- `failures: u8`: consecutive failed refreshes.
- `in_flight: bool`.
- `epoch: u64`: bumped by `clear()`.

Owner: one board in the client's core state (`app.core`), for the open project only; replaced
(cleared) on a project change, so it is not keyed by project. Every window and session of the app
reads it.

Operations (all pure):
- `due(now, trigger) -> bool`: `Open` always (when not in flight); `Focus` when `last_read` is `None`
  or `now - last_read > 5 min`; false while `in_flight`.
- `begin(keys) -> Option<(u64 /*epoch*/, Vec<String>)>`: sets `in_flight`, returns the epoch and keys to ask (empty -> `None`).
- `apply(epoch, keys_asked, Result<Vec<(String, Option<TicketStatus>)>, ReadFailure>, now)`:
  - `Ok`: for each asked key, replace its entry (or remove when absent/unusable); `failures = 0`;
    `last_read = now`; clear `in_flight`.
  - `Err`: mark all entries `stale`; `failures += 1`; at 3, clear entries; clear `in_flight`.
- `apply` with an `epoch` that is not the current one is dropped (a read begun before a clear).
- `clear()` (bumps `epoch`, clears `in_flight`): switch off, connection removed, project switch.
- `status_of(key) -> Option<&StatusEntry>`: the only read path for the surfaces (FR-010).

State transitions: absent -> fresh (Ok) -> stale (Err) -> fresh (Ok) | absent (3rd Err, not found,
clear).

## ReadFailure
Reuses `IssueLoadError`; the board does not distinguish kinds.
