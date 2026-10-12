# Data Model: One Session per Worktree

No stored field is added. The invariant is a function of existing records.

## Existing (read, unchanged)

- `Session` (`micold-core/src/session.rs`): `id`, `location: SessionLocation`, `label`,
  `provider: AiCli`, `lifecycle: SessionLifecycle`, `archived: bool`.
- `SessionLocation`: `Default` | `Worktree(dir_name)`.
- Catalog `Workspace.sessions: Map<project, Vec<Session>>` (persisted by the project store).

## New types (`micold-core/src/occupancy.rs`)

| Type | Fields | Rule |
|---|---|---|
| `ExistingSession` | `id: SessionId`, `name: String` (the displayed label) | what a refusal names (FR-003) |
| `Occupancy` | `Free` \| `Taken(ExistingSession)` | `Taken` when an un-archived session has the location; first in list order |
| `StartBlock` | `Clear` \| `Blocked { live: ExistingSession }` | `Blocked` when another un-archived session of the location is starting, running or restarting (FR-013) |

`SessionDefinition { ai_cli: AiCli, prompt: Option<String> }` lives in `protocol/messages.rs`:
whole or absent; no model (R4).

Functions: `occupancy(sessions, &location) -> Occupancy`;
`start_block(sessions, id, live_ids: &HashSet<SessionId>) -> StartBlock`; `refusal_text(&ExistingSession, &SessionLocation)
-> String`; `start_block_text(...)`.

## Daemon (`micold-daemon/src/catalog.rs`)

`CreateSessionError`: `Occupied(ExistingSession)` | `Io(io::Error)`.
`Catalog::replace_session(project, old: SessionId, new: Session)`: adds `new` and persists
(rolled back on failure), then archives `old` with its durable marker last (research R7).

## Attach (`micold-core/src/attach.rs`)

`RefuseReason::LocationHasSession` (unit variant, `Copy` kept) added to the existing enum (its
`text()` gains the arm); `AttachResult.existing: Option<ExistingSession>` carries the occupant. `cap_per_location` keeps the candidate with the greatest `last_activity` per
`AttachTarget`, drops every candidate for an occupied one.

## Validation rules

- A location has at most one un-archived session once created through any path (FR-001); legacy
  data may hold more and is only ever read (FR-013).
- `SessionDefinition.prompt`, when `Some`, is non-empty after trim; empty is `None`.
- A definition is accepted only with a worktree create (or, for an existing location, only
  `create_session` on a `Free` one); there is no "update definition" operation.

## State transitions

Location occupancy: `Free` --create/resume/adopt--> `Taken`; `Taken` --archive (delete)-->
`Free`; `Taken` --replace (confirmed resume)--> `Taken` (new id, old archived). Session
lifecycle is unchanged: restart and start act on the same record.
