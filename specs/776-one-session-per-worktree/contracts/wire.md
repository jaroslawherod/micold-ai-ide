# Contract: wire changes

`PROTOCOL_VERSION` 40 to 41 (M1; superseded by the M2 and M3 amendments below, 42 and 43), with the `version.rs` history entry. All
additions are optional and serde-defaulted, so an older frame decodes and the old behaviour is the
default; `SCHEMA_HASH` changes with them.

## Amendment 2026-10-13: the explicit request (M2)

`PROTOCOL_VERSION` 41 to 42 (the 41 of M1 is merged). `allow_additional_session: bool`
(serde-default `false`, so an older frame decodes to the default refusal) is added to
`ClientMsg::SessionCreate`, `SessionStart`, `SessionRestart` and `AttachItem::Session` (beside
`replace`; both true on one item is refused as malformed). `true` skips the occupancy check of
`SessionCreate` (M2) and the start block of `SessionStart`/`SessionRestart` (M7, which builds the
block) and lets a resume item keep both (M6); M2 only carries the field on those three and the
daemon ignores it there until its milestone. Both `replace` and the flag on one item is refused as
malformed (M6). The refusal texts gain the way forward (research R3, R12).

## Amendment 2026-10-14: the main session (M3)

`PROTOCOL_VERSION` 42 to 43 (the catalog snapshot shape changes). `Session` gains `main: bool`
(serde-default `false`; persisted with the record and carried in the catalog snapshot; an older
frame or catalog decodes to `false` and the reader derives the main, research R14). New
`ClientMsg::SessionSetMain { id: SessionId }`, fire-and-forget like `SessionStart`: the daemon
sets `main` on `id` and clears it on the location's other sessions in one persisted write; the
catalog update is the answer. A no-op for an unknown, archived or already-main id. The create,
replace, adopt and archive paths write the marker as research R14 lists, with no new message.
Refusal texts name "the main session" (contracts/mcp-tools.md). No existing message changes
shape.

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

`CatalogSnapshot`'s shape beyond `Session.main`, `AttachDiscover`, store files. No catalog migration: records written before M3 load with `main` false and derive it.
