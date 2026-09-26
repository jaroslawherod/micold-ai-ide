# Phase 0 Research: A session keeps its name when nothing is running it

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-09-12

No `NEEDS CLARIFICATION` markers survived the spec, so this phase is about the *mechanism*: where
the name is observed today, why it is lost, and which of several plausible seams the durable write
belongs at.

---

## R0 — Why the name is lost (the diagnosis this plan rests on)

**Finding**: The name is observed, projected, and never recorded. Three facts, each verified in the
tree at `49d05026`:

1. `DaemonState::drain_signals` (`crates/micold-daemon/src/state.rs:1567`) reads each live session's
   OSC-0 title on the 250 ms supervisor tick and stores it in `LiveSession::last_title`, whose own
   doc comment says *"Not persisted"* (`state.rs:185`).
2. `overlay_live_summaries` (`state.rs:503`) copies that value onto the outgoing
   `SessionSummary.title` — overwriting the catalog's `SessionLabel` rather than updating it. Its
   doc comment states the intended relationship outright: *"the persisted `label` lags the terminal
   title"*. That is the bug, written down as a design note.
3. `Session::set_title` (`micold-core/src/session.rs:480`) — the only mutator of the label — has
   exactly one non-test caller in the whole workspace, `micold-client/src/features/session.rs:843`,
   reached only by `SessionMsg::TitleUpdated`, which is itself dispatched only from tests. The
   client-side title sync it belonged to was superseded when the daemon took over the catalog; its
   replacement projects instead of persisting.

So `StoredSession::title` is written as `Some(..)` only for a session that entered the catalog
already `Named` — which in practice means one the FR-014 discovery pass adopted
(`state.rs:911` reads `provider.read_title` and builds a `Named` label). A session this application
started is persisted `title: None`, restores as `SessionLabel::Pending`, and reads "New session"
until something runs it again and an OSC-0 title arrives.

**Consequence for scope**: the persistence layer needs no change at all. `StoredSession::title`,
`from_session`, and `into_session` (`micold-core/src/store.rs:235`, `:248`) already map
`SessionLabel::Named(t) ↔ Some(t)` and `Pending ↔ None` in both directions. The whole feature is
about making something call the mutator.

---

## R1 — Is the OSC-0 title actually the session's *name*, or a status line?

**Decision**: It is the name, and it is the right thing to persist.

**Rationale**: The daemon does not take the raw title. `DaemonListener` (`micold-daemon/src/terminal.rs:141`)
passes every `Event::Title` through `strip_status_glyph`, which drops a single leading
Private-Use-Area/symbol glyph plus its space — the spinner and activity prefixes agents put in front
of the title — and records the braille-spinner edge *separately*, as activity evidence, before
stripping. What lands in `last_title` is therefore the title text with the churning part removed:
`"✳ Fixing the parser"` is stored as `"Fixing the parser"`, and the existing daemon integration test
`activity_pipeline.rs:219` asserts exactly that round trip against a real PTY.

`drain_signals` additionally debounces — it only marks a change when the stripped title *differs*
from the last one — so a spinner cycling through glyph frames on an otherwise stable title produces
one change, not thirty.

This also settles FR-003's "whenever the name it shows changes": what to persist is precisely the
value the projection already displays, so the record and the screen cannot diverge by construction.

**Alternatives considered**:

- *Persist `AiCliProvider::read_title` (the transcript's `ai-title` record) instead, on a timer.*
  Rejected as the primary path: it is blocking I/O per session per tick for a value the daemon is
  already handed for free in memory, and it would make the recorded name lag the displayed one —
  reintroducing the divergence from the other side. It remains the right source for **recovery**
  (R4), where there is no live process to ask.
- *Persist the raw, unstripped title.* Rejected: it would write a spinner glyph into the catalog and
  make the persisted name flicker between glyph frames, turning a once-per-conversation write into a
  once-per-tick one.

---

## R2 — Where does the durable write go?

**Decision**: `drain_signals` keeps observing and stops there; it returns the observed name changes
to the supervisor tick, which persists them in a `spawn_blocking` hop — and only when there is
something to write.

Concretely, `drain_signals` grows a return value carrying both the existing `bool` and
`Vec<(SessionId, String)>`; `spawn_supervisor` (`micold-daemon/src/server.rs:243`) hands a non-empty
vector to a new blocking call that takes the lock once and writes through the catalog.

**Rationale**: The module's stated invariant is that blocking work never runs on the async runtime,
and `drain_signals`'s own doc comment sells it as *"cheap and lock-only, never blocking I/O"* —
which is why the supervisor calls it directly on the async task while `supervise_exited_sessions`
gets a blocking hop. Writing the project state file inside it would quietly break that, on the
hot 250 ms path, while holding the state lock. Returning the changes instead keeps the observation
step pure enough to unit-test without a PTY (the Principle I benefit recorded in the plan's
post-design re-check) and puts the I/O where the rest of the daemon's I/O already lives.

The extra hop costs nothing at idle: a title change is a rare event, and no change means no hop.

**Alternatives considered**:

- *Write inside `drain_signals`, under the lock.* Rejected: blocking I/O on the async runtime, on
  the tick path, holding the lock every other tick reads. It is the smallest diff and the worst one.
- *Fold the write into the existing `supervise_exited_sessions` blocking hop.* Rejected on ordering:
  that hop runs *before* the drain, so it would always persist the previous tick's observation —
  correct eventually, wrong for one tick, and confusing to read.
- *Persist on session stop / daemon shutdown instead of on change.* Rejected: the reported failure
  mode is a daemon restart, and a force-kill or a crash is exactly when a shutdown-only write is not
  taken. Feature 025 faced the same choice for the foreground-session pointer and resolved it the
  same way — write on change, lose at most the single most recent one.

---

## R3 — Write amplification: is "persist on every change" affordable?

**Decision**: Yes, and with the same guard feature 025 already established: compare before writing,
and return whether anything was written.

**Rationale**: A `Catalog::persist()` rewrites one project's state file, which holds that project's
session records. Two things bound how often that happens:

- `drain_signals` has already debounced against `last_title`, so only a genuine text change reaches
  the caller.
- The catalog method compares the incoming name against the stored `SessionLabel` and returns
  `Ok(false)` without writing when they match — the shape `remember_foreground`
  (`micold-daemon/src/catalog.rs:403`) uses, and for the same reason it gives: a reconnect,
  a re-attach, or a restart re-observing a title the catalog already holds must not rewrite a file
  holding every one of that project's records.

Between them, the steady state for an idle session is zero writes, and for an active one it is a
write per actual re-title — a handful over a conversation's life.

**Alternatives considered**: A time-based debounce (coalesce writes into one per N seconds).
Rejected as unjustified complexity: it buys nothing the two comparisons above do not already buy,
and it adds a window in which a crash loses a name change that the simpler design would have kept.

---

## R4 — Recovering the name of a session that was never recorded (FR-006, FR-010)

**Decision**: A recovery pass in the attach-time blocking hop, beside the existing FR-014 discovery,
filling only labels that are `Pending` and only from that session's own provider records.

**Bugfix**: 2026-09-26 — [BUG-002](./bugs/BUG-002.md). What `read_title` returns for `claude` was too
narrow: the latest `{"type":"ai-title"}` record. `claude` keeps re-emitting the **pre-rename**
`ai-title` after a user's `/rename`, and records the chosen name as `{"type":"custom-title"}`, mirrored
in `{"type":"agent-name"}` — its own resolved display name. So the pass must read the CLI's *current*
name for the conversation: the latest `custom-title` if there is one — that kind alone ranks above
position, because a rename is sticky in those records — else the latest of `agent-name` or `ai-title`
**by position**, which is what stays correct if a later `claude` stops writing `agent-name`
(contract C16.1). Nothing else
about R4 changes — same pass, same hop, same inputs, same cost, same precedence against the live
terminal title.

**Rationale**: `refresh_worktrees_off_runtime` (`micold-daemon/src/server.rs:1516`) already does
exactly the surrounding work in one `spawn_blocking`: it refreshes the project's worktrees and then
runs `discover_external_sessions`, which enumerates the same location list, resolves each cwd, and
calls `provider.read_title` per candidate id (`state.rs:911`). Recovery needs the identical inputs
and produces the identical kind of value; putting it anywhere else would mean re-deriving the
location list and taking a second blocking hop to read the same directories.

It runs on **every** project open, not just the first — matching the documented behaviour of the
discovery pass — so a name that only appears in the CLI's records later is still picked up.

**Why it is bounded** (SC-007): the existing pass holds a per-*location* cost rule, and recovery
cannot honour that literally — a name is per conversation. What bounds it instead is FR-007: a
recovered name is persisted, so a session costs one transcript read *once*, and thereafter its label
is `Named` and the pass skips it. The residual cost is one read per project open per session that
has **no** name anywhere — a session created and never used — which is a small, self-limiting set,
and the same read the existing pass already performs for a newly discovered id.

**Alternatives considered**:

- *Recover at `Catalog::load`.* Rejected: load runs before any project's worktrees are known, so the
  location list — and therefore every cwd — is not available yet, and it would put provider I/O in
  the daemon's startup path for projects the user may never open.
- *Recover lazily, when a session is selected.* Rejected: it fixes the row only after the user has
  clicked it, which is the state the bug report is about ("shown as `New Session`" *before* it is
  active). US2's independent test is explicitly "without opening it".
- *Widen `discover_external_sessions` to stop subtracting known ids.* Rejected: that subtraction is
  load-bearing for the documented per-location cost rule and for the invariant that a known
  session's provider is never re-derived from disk. Recovery is a separate pass reading the same
  inputs, not a relaxation of that one.

---

## R5 — Keeping a recorded name after the AI CLI's records go away (FR-008)

**Decision**: Falls out of the design; no code defends it, and one test pins it.

**Rationale**: The recovery pass only ever *fills* a `Pending` label. Nothing in the design moves a
label from `Named` back to `Pending`, and `read_title` returning `None` is a no-op rather than a
clear. The pre-existing behaviour around a vanished conversation already agrees: a resume of a
session whose transcript is gone is refused with a message naming the CLI (`state.rs:1218`), rather
than silently starting fresh under the old id — the codebase's own note calls out that reusing the
id would "put the user in an empty session wearing the old one's title". Keeping the name on the row
while refusing to resume into it is the coherent pair.

The risk this creates is the one FR-005 guards: a name that is never revised. It is not real here,
because the live path (R1/R2) keeps overwriting the record whenever the conversation is re-titled,
and the live path is the one that runs whenever there is a conversation to re-title.

---

## R6 — Failure to persist must not change the display (FR-009)

**Decision**: Update in memory first, persist second, log a warning on failure, and never surface it
as a session error.

**Rationale**: This is the established convention in this file — `adopt_discovered_sessions`
(`catalog.rs:727`) writes `tracing::warn!(%err, ...)` and carries on, with the reasoning spelled out:
the value is in memory, the client will see it, and the next pass re-derives it. The same holds
exactly for a name. Surfacing it would also be actively wrong: a read-only data directory is not a
session failure, and `WireLifecycle::Failed` is about the session's process.

---

## R7 — Client, protocol, and schema: what does *not* change

**Decision**: Nothing.

**Rationale**, in the order a reviewer would check it:

- **Wire**: `SessionSummary.title` is already `SessionLabel` (`micold-core/src/protocol/messages.rs:772`).
  A persisted `Named` label travels on the existing field.
- **Client**: the snapshot reconciler already adopts `title` for a session it knows, and already
  builds one from the summary for a session it does not — the behaviour its own test
  `an_existing_sessions_cli_is_not_rewritten_by_a_later_snapshot`
  (`micold-client/src/shell/daemon_sync.rs:2019`) documents as *"adopts `lifecycle`, `activity` and a
  real `title`"*. Once the daemon sends a name, the client shows it.
- **Storage**: `StoredSession::title` exists and round-trips (R0). No new field, so no
  `schema_version` bump, and an older reader tolerating an unknown field is not even at issue.
- **`SessionMsg::TitleUpdated`** stays as it is. It is unreached in production (R0), but it is the
  client's own reducer message and removing it is a separate cleanup with its own tests to delete;
  doing it here would enlarge the diff for a bug fix without changing any behaviour.

---

## R8 — Cross-platform (Principle VI)

**Decision**: No platform-conditional code, and no platform-specific test.

**Rationale**: Every path involved is already platform-neutral — `JsonFileStore`'s temp-and-rename
write, `SessionLocation::cwd`, and the providers' own path derivation. The one place a test could
accidentally become Unix-only is FR-009's "a failed write changes nothing", if it were written by
`chmod`-ing a directory read-only. It is written instead against a non-persisting catalog
(`Catalog::ephemeral`), which reaches the same branch with no filesystem permissions involved and
runs identically on all three platforms. Recorded here because it is the kind of thing that passes
locally and stops CI on Windows.
