# Contract: wire changes

`PROTOCOL_VERSION` 40 to 41, once for the whole feature, with the `version.rs` history entry. All
additions are optional and serde-defaulted, so an older frame decodes and the old behaviour is the
default; `SCHEMA_HASH` changes with them.

## `ClientMsg::WorktreeCreate`

Adds `session: Option<SessionDefinition>` (default `None`).

```text
SessionDefinition { ai_cli: AiCli, prompt: Option<String> }
```

- `None`: the worktree is created with no session (FR-010), as today.
- `Some`: `ops::create_worktree_with_session`: CLI check at the worktree's future cwd, then create,
  then the session started fresh with the prompt typed once the CLI is ready. A missing CLI
  answers `OperationError { kind: Refused }` with `cli_reason`'s text and creates nothing.
- Reply `OperationResult::WorktreeCreated { dir_name, session: Option<SessionId> }`;
  `session` is `None` when no definition was sent. If the worktree was created but the session
  could not be, the reply is `OperationError` with the worktree's `dir_name` named in `detail`;
  the worktree remains and has no session.

## `ClientMsg::SessionCreate`

Unchanged shape. New outcome: for a location that has a session the reply is
`OperationError { kind: Refused, message: <occupancy::refusal_text>, detail: Some(<existing id>) }`
and the catalog is unchanged.

## `ClientMsg::SessionStart` / `SessionRestart`

Unchanged shape. Both are fire-and-forget (no reply), so the refusal has two halves. Daemon:
when another un-archived session of the location is starting, running or restarting (legacy data
only), `DaemonState::start_refusal` makes the message a no-op before `begin_start`: nothing is
spawned, no record changes (a `Failed` mark would rewrite a legacy record, which FR-013 forbids),
and the live one is untouched. Client: the reducer asks the same `occupancy::start_block`, so
selecting or starting the blocked row views it without starting and shows `start_block_text` as
the row's tooltip and a notice; the daemon check is the authority for a stale window.

## `ClientMsg::AttachApply`

Adds `replace: bool` to `AttachItem::Session { id, replace }` (default `false`), per item, so a
confirmation covers exactly the pair it named. A single explicit resume is a batch of one session
item and is never capped; the cap applies to batches of more than one session item ("attach
all"). For an `AttachItem::Session` whose location holds a different session:

- `replace: false` (or absent): that item's outcome is `Refused(LocationHasSession)` with `existing` set; nothing is
  stopped or written.
- `replace: true`: the old session's processes are stopped first, then the new record is added and
  persisted first and the old one archived only after that succeeds (`Catalog::replace_session`,
  research R7); outcome `Attached`. A failed write leaves the old session listed.

`AttachApply` of a worktree item and of a session into a `Free` location are unchanged.
Batches that carry several sessions for one location keep only the most recent
(`cap_per_location`); the rest are reported `Refused(LocationHasSession)` and stay listed as
resumable.

## `RefuseReason`

Adds the unit variant `LocationHasSession` (keeps `Copy`; text: `that worktree already has a
session`). `AttachResult` gains `existing: Option<ExistingSession>` (default `None`), set for this
reason so the confirmation can name the session being replaced.

## Not changed

`CatalogSnapshot`, session records, `AttachDiscover`, store files. No catalog migration.
