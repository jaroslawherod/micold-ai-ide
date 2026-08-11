# Research: Reopen on the session I was last using

Feature 025. Everything below was checked against the code, not recalled. Line references are to the
tree at the time of writing.

## R1 — Does the memory already exist?

**Decision**: Yes, in full, and it is discarded at exit. This feature persists it; it does not
invent it.

`State::foreground_by_project: BTreeMap<PathBuf, SessionId>` is written by `record_foreground()`
before every activation and read by `explain_foreground()` after it
(`crates/micold-client/src/features/session.rs`). Feature 008's
[BUG-001](../008-background-project-switching/bugs/BUG-001.md) has just made the restore honour a
session whose process has stopped, which is the case that matters here: run state is not persisted,
so **every** session is idle at launch, and a running-only rule would have made this feature
restore nothing.

**Rationale**: The whole of FR-002 and FR-003 is already implemented for the within-a-run case. What
is missing is a home on disk and a caller at launch.

**Alternatives considered**: Writing a fresh "last session" concept beside the existing one —
rejected, it is the same fact, and two of them can disagree.

## R2 — Where does the memory live, so the right process can write it?

**Decision**: Move `foreground_by_project` into `micold_core::workspace::Workspace`, and persist it
in the per-project state file (`StoredProjectState`) beside that project's sessions.

**Rationale**: Two constraints decide this between them.

- **The client must not write the store.** `main.rs` states it plainly: *"The client does NOT write
  `projects.json`. The daemon's Catalog is its single writer […] `store.rs` has no locking, so a
  client-side save silently clobbers whatever the daemon wrote since this process loaded."* That
  bug has bitten before (the terminal-mode clobber recorded in the same comment).
- **The client must be able to read it before the daemon answers.** `boot()` already loads the
  workspace from `JsonFileStore` (`main.rs:510`), sessions included, and prunes empty sessions from
  it. A memory in that same file is available on the first frame.

Putting the map on `Workspace` satisfies both: the daemon writes the file, the client reads it, and
the map sits with the sessions its ids refer to.

**Alternatives considered**: Leaving the map on the client's `State` and persisting it from the
client — rejected on the clobber hazard above. A separate memory file owned by the client —
rejected: it would be a second source of truth for per-project state, and it would need its own
pruning when a project is forgotten, which `remove_project_state` already does for the existing
file.

## R3 — Should the memory come over the wire instead?

**Decision**: No. No protocol change.

**Rationale**: `ProjectSnapshot` would be the obvious place, and it costs more than it gives:

- It is a schema change. `SCHEMA_HASH` is baked from the canonical text of `protocol/messages.rs`
  by `build.rs`, and `tests/schema_hash.rs` exists specifically to prove the hash is *sensitive* to
  edits of a message struct. Editing one means the handshake tuple changes and both sides move
  together.
- It would arrive **too late**. The catalog reaches the client after connect and attach, so the
  first frames would show the project overview and then jump to a session — visibly worse than what
  we have, and it fails SC-001's "with zero clicks" in spirit if not in letter.

Reading the value from the client's own load avoids both. The daemon still owns writing it.

**Alternatives considered**: Sending it in the `Attached` reply — same lateness, same schema cost.

## R4 — Does the daemon already know which session is in front of the user?

**Decision**: Yes, on every path that matters, and it already stores it — per client, in memory.

`ClientMsg::SetViewedSession { project, session }` is sent by the client from four places
(`main.rs:801` welcome/attach, `:1152` forced re-attach, `:2180` `view_and_start`, `:2313`
`switch_daemon_attachment`), and `view_and_start` is what runs when a session is selected
(`main.rs:1562`, `:1650`). The daemon handles it at `server.rs:485` → `State::set_viewed`, which
writes `client.viewed` — a per-client map that dies with the connection.

So the feature's daemon-side work is small and precise: the same handler also records the value in
the catalog's workspace and persists it.

**Rationale**: The message exists, is sent at the right moments, and carries exactly the pair the
memory needs. Inventing a second message would duplicate it.

**Alternatives considered**: Persisting on `Detach` or at daemon shutdown — rejected: a force-kill
would lose the memory entirely, and the spec's Assumptions choose "written as the current session
changes" precisely so a kill loses at most the latest change.

## R5 — Where does the launch apply it, and why not reuse `restore_after_activation`?

**Decision**: `boot()` resolves the memory itself and calls `set_current_session`. It does **not**
call `restore_after_activation`.

**Rationale**: `restore_after_activation` is the switch path, and feature 023 added a line to it:

```rust
self.focus_terminal();
```

with the reasoning that *"a project switch is deliberate, and the terminal you are looking at is the
one you meant"*. That is right for a switch and wrong for a launch — FR-013 says starting the
application must not put keystrokes into a terminal, because the user has not yet looked at the
screen. Reusing the function would import the focus with the restore.

The rest of what it does is also switch-specific (`default_expanded = false`,
`show_agent_worktrees = false`, the background-restart notice) and is already the default at boot.

So the launch path is two lines: resolve with `explain_foreground`, apply with
`set_current_session`. Feature 024's arming rule then reveals the row for free — it fires on *any*
app-initiated transition to `Some`, which is exactly why it was written that way rather than as a
list of call sites (024 research R12).

**Alternatives considered**: Adding a `focus: bool` parameter to `restore_after_activation` —
rejected, a boolean parameter that changes what a function *means* at two call sites is the shape
that made this ambiguous in the first place.

## R6 — What happens to a memory that cannot be honoured?

**Decision**: Nothing special is needed. The existing resolution already answers it.

`explain_foreground` returns `NoSessionsForKey` when nothing is filed under the project,
`NoneActive { sessions }` when none can be chosen, and skips an `archived` (closed) session. At boot
`prune_empty_sessions` has already dropped sessions with no conversation on disk, so a memory
pointing at one resolves to nothing by the same path.

FR-005, FR-006 and FR-007 therefore fall out of code that exists and is tested, rather than needing
launch-specific handling.

**Rationale**: This is the payoff of feature 024's `ForegroundChoice` work — the resolution already
names *why* it landed where it did, and the client log already records it, so a launch that restores
nothing is diagnosable rather than mysterious.

**Alternatives considered**: Validating the memory at load time and clearing it — rejected as a
second implementation of the same question, and one that would have to re-run whenever sessions
change anyway.

## R7 — Is the added field backward and forward compatible?

**Decision**: Yes, with `#[serde(default)]`, and no schema version moves.

**Rationale**: `StoredProjectState` already relies on this exact argument. `store.rs` records it for
the BUG-001 split: *"`#[serde(default)]` lets an old (pre-split) `projects.json` […] deserialize
normally. No `schema_version` bump (an old reader already tolerates unknown/missing fields; a new
reader tolerates these being present or absent)."*

An old file has no `last_session` → `None` → the application behaves exactly as it does today. A new
file read by an older build has an unknown field, which `serde` ignores by default.

FR-010's "unreadable or outdated is treated as no memory" is already the file-level behaviour:
`load_project_state` recovers rather than failing (Principle IV).

**Alternatives considered**: A schema version bump — unnecessary, and it would force a migration
path for a field whose absence is already meaningful.

## R8 — Where is the risk?

Ordered by expected trouble:

1. **Moving `foreground_by_project` into `Workspace`.** It is read and written by the client today
   and becomes shared state that the daemon also writes. The risk is not the move but the
   *ownership*: the client must keep reading and stop short of persisting, exactly as it does for
   sessions. Contained by the client having no store-write path to begin with.
2. **The launch not focusing.** Easy to get wrong by reusing the switch path (R5) and invisible in
   a test that only asserts which session is current. Needs its own assertion.
3. **Two windows.** Both write via their own daemon connection to one daemon, which serialises
   them; last write wins, which the spec's Edge Cases already accept. No new hazard, but worth a
   line in the contract so it is a decision rather than an accident.
4. Everything else — a defaulted field and a lookup that already exists.
