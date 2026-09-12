# Phase 0 Research: Refresh the Worktree List on Demand

**Feature**: 029-refresh-worktrees-list | **Date**: 2026-08-31

Every unknown the Technical Context raised is resolved below against the code as it stands, with
file:line evidence. Nothing here is a preference poll — each decision names what the codebase
already does and why this feature should or should not follow it.

---

## R1 — How the worktree listing is produced today, and why it goes stale

**Finding**: git is the single source of truth and the daemon caches it. `DaemonState::worktrees`
is described in `crates/micold-daemon/src/state.rs:80` as "a cache refreshed at well-defined
points", filled by `DaemonState::refresh_worktrees` (`state.rs:799`), which reads the repo path
under the lock, runs `worktree::discover` **unlocked**, then stores the result. The catalog
snapshot overlays it (`state.rs:422-434`) and every client sees it as `CatalogChanged`.

The "well-defined points" are exactly six call sites in `crates/micold-daemon/src/server.rs`:
attach (`:498`, per-client), worktree delete (`:924`), include/exclude (`:1205`, `:1340`,
`:1377`), and project add (`:1400`). **Nothing watches the repository.** There is no timer, no
`notify` watcher on the git dir — `event_log.rs` is the only `notify` consumer and it watches a
session's event log, not the repo.

**Consequence, and it is the whole feature**: a worktree that appears by any other route is
invisible until one of those six moments recurs. The only user-reachable one is attach, which is
why "switch project away and back" is the current workaround.

**Decision**: the feature adds a seventh trigger and changes nothing about how discovery works.
`refresh_worktrees_and_broadcast` (`server.rs:1489`) already composes the refresh with a broadcast
and is called by four of the six sites; this feature reuses it verbatim.

---

## R2 — Reuse an existing RPC, or add one?

Two candidates were examined for reuse before deciding to add a message.

**`ClientMsg::ProjectAdd`** already ends in `refresh_worktrees_and_broadcast` (`server.rs:1398-1401`)
and `Catalog::add_project` is idempotent (`catalog.rs:481`, `open_or_activate`). Sending it for a
project that is already open would refresh the list with **zero** protocol change.

**Rejected.** `open_or_activate` does not only refresh — it activates, which writes the workspace's
active project and its recency ordering. A control whose job is "re-read the list" would silently
reorder the user's known-projects list as a side effect, and the next person to read the call would
have no way to tell that the reorder was unintended. Reusing a message for its side effect rather
than its meaning is the kind of thing that is correct until someone changes `add_project`.

**`ClientMsg::Attach`** was rejected for a stronger reason: attach is exclusive and per-client
(`server.rs:473`, the `Displaced`/`ProjectBusy` machinery). Re-attaching to a project this client is
already attached to, to get a side effect, puts a takeover path on the refresh button.

**Decision**: add `ClientMsg::WorktreeRefresh { req, project }`. It is correlated like every other
mutating RPC even though it mutates nothing durable, because its reply is what ends the button's
busy state — see R5. Its daemon arm is two lines: `refresh_worktrees_and_broadcast(state, project)`
then `send_ack(state, id, req)`.

**Reply**: `OperationResult::Ack` (`messages.rs:941`) — no new result variant. The refreshed
listing arrives on its own as the `CatalogChanged` broadcast the refresh already sends; the ack
says only "the re-read finished". Two channels, each carrying one fact, which is how include/exclude
already work.

---

## R3 — Protocol version

`PROTOCOL_VERSION` is 9 (`crates/micold-core/src/protocol/version.rs`). Its doc comment records a
hard-won rule from features 026 and 027: **one bump per feature, in one edit**, because
`SCHEMA_HASH` is generated over `messages.rs` by `build.rs` and `tests/schema_hash.rs` fails if the
hash moves twice within a feature.

**Decision**: 9 → 10, in the same edit that adds the variant, with a doc line in the same style as
its predecessors. This feature has exactly one wire-visible change, so there is no risk of the
double-bump 026's note warns about.

---

## R4 — Which feature module owns the in-flight state

Feature 028's contract is "own your messages, own your state". The candidates:

- `features/sidebar.rs` owns the panel — "what is expanded", "what is shown", "the panel's own
  geometry" (`sidebar.rs:26-36`). Its module doc is explicit that **the tree is not here**: which
  worktrees the sidebar lists is derived from `state.worktree` on every view.
- `features/worktree.rs` owns "the discovered listing and the surfaces over it"
  (`worktree.rs:34-45`), including `Msg::Loaded` — "the binary discovered/re-discovered the active
  project's worktrees" (`worktree.rs:490`).

**Decision**: the worktree feature. The button lives in the sidebar's header, but a button's
location does not confer ownership — the thing being re-read is the listing, and the listing is the
worktree feature's by its own module doc. Putting `refreshing` in `sidebar::State` would make the
sidebar hold a fact about an operation on data it explicitly does not own, which is the exact
cross-feature write feature 028 exists to remove.

The button's *view* code stays in `ui/sidebar.rs` beside its two neighbours; a view reading another
feature's state is ordinary (the sidebar already reads `state.worktree.hovered`, `sidebar.rs:508`).

---

## R5 — What ends the busy state, and what "bounded wait" means here

**Finding**: the client has **no RPC timeout anywhere**. A pending op is resolved by exactly two
things: a terminal reply (`OperationOk` at `daemon_sync.rs:439`, `OperationError` at `:619`), or a
disconnect, which drains every pending op and resolves each to an explicit *unknown* outcome
(`daemon_sync.rs:240-256`). `grep` for `timeout` in `src/shell/` and `src/features/` returns only
the *environment-include* setting, which is a daemon-side subprocess timeout, not a wire one.

So there are two readings of FR-007's "bounded wait":

**(a) The connection is the bound.** Ship nothing new; a hung daemon leaves the button greyed until
the connection drops, at which point the existing drain resolves it. Cheapest, and consistent.

**(b) An explicit timer.** Arm a delayed message when the request is sent; if the op is still
pending when it fires, resolve it as unknown.

**Decision: (b), scoped to this one operation.** (a) is not good enough for *this* operation
specifically: the daemon's refresh runs `git` in `spawn_blocking` (`state.rs:804`), and a `git
worktree list` blocked on an index lock hangs with the connection perfectly healthy. Every other
correlated op in the client is one the user initiated from a modal that shows its own failure; this
one leaves a control inert in a panel the user keeps looking at. FR-007 was written with that case
in mind.

**Implementation**: `Task::future` over `tokio::time::sleep` — the client already depends on tokio
timers (`daemon.rs:182`, `daemon.rs:316`) — mapped to `Msg::RefreshTimedOut(req)`. The handler is a
shell function that only acts if `app.pending_ops.remove(&req)` finds the op still there, so a
timer that fires after a normal reply is a no-op. **This is the codebase's first wire timeout**, and
that is recorded here deliberately: if a second operation wants one, the right move is to lift this
into a shared helper rather than copy it.

**Duration**: 30 seconds. Long enough that a slow repository finishes normally (R9), short enough
that a user does not conclude the control is broken. Named as a `const` beside the handler.

---

## R6 — How "in progress" is shown, without a new component

`IconButton` renders disabled — greyed and inert — when given no `on_press`
(`ui/material/icon_button.rs:22`). That is already how FR-005's "no project open" case is expressed.

Three options for the in-progress state:

1. **A spinner replacing the glyph.** Material's own answer. **Rejected**: there is no circular
   progress component in `ui/material/` (only `StageProgress`, a linear indeterminate bar built for
   the create dialog, `progress.rs:1-21`). Adding one means a new component, which means a
   `showcase/catalogue` entry (`tests/showcase_completeness.rs` fails in both directions), an
   `anatomy_size` figure, and a `tests/idle_requests_no_frames.rs` interaction — an indeterminate
   indicator holds the render loop awake by design
   (`tests/indeterminate_stops_with_its_operation.rs`). That is a large, gated change for a state
   that normally lasts under a second.
2. **Disabled + a changed tooltip.** Immediate and free, but the tooltip needs a hover, and a greyed
   button is ambiguous with FR-005's "no project" greying.
3. **Disabled, plus a transient notice on completion** through the existing notifications queue
   (`app.rs:253`, `notify_info`).

**Decision: 2 + 3 together.** While refreshing, the button is inert with its tooltip reading
"Refreshing worktrees…"; on completion a snackbar says so. This satisfies FR-006 (a state is shown),
FR-007 (it returns to idle), and US2's real requirement — that a refresh which changes nothing is
distinguishable from a control that did nothing — using only surfaces the spec's Assumptions already
sanctioned. It adds no component and trips no gate.

Recorded as a **known limitation**: the inert-greyed state is a weaker in-progress cue than a
spinner. If the visual pass (quickstart §B) reads it as broken rather than busy, the follow-up is
option 1 as its own change, with the component and its gates done properly.

---

## R7 — Single-flight, expressed structurally rather than as a guard

FR-006 forbids a second concurrent refresh. The obvious implementation is an `if` in the shell — but
`src/shell/` is **not** covered by Constitution Principle I's GUI-glue exception, which names
`src/main.rs`, `src/ui/` and `src/showcase/` only. Shell logic needs tests, and a guard is decision
logic wherever it sits.

**Decision**: make it structural. `ui/sidebar.rs` attaches `on_press` only when the listing is
refreshable — a project is active and no refresh is in flight — so while one runs there is no
message a press could emit. The reducer *also* ignores a second `RefreshRequested` while
`refreshing` is true, and that guard is unit-tested; the view is then the belt and the reducer the
braces, and neither is the only thing standing between the user and a doubled request.

The predicate itself (`State::can_refresh_worktrees()`) is a pure method on the core state, tested
directly — so the view's use of it is the "thin glue invoking already-unit-tested pure logic" the
exception describes, not a decision of its own.

---

## R8 — The icon

No refresh glyph exists: `Icon` (`crates/micold-client/src/icons.rs:25-100`) has 31 variants and
none of them means "re-read".

The shipped font is the **full** Material Symbols Outlined static instance, not a per-codepoint
subset (900 KB, 4277 mapped codepoints). Parsing its `cmap` directly confirms the candidates are all
present: `refresh` U+E5D5, `sync` U+E627, `autorenew` U+E863, `cached` U+E86A.

**Decision**: `Icon::Refresh = '\u{e5d5}'` — Material's `refresh`, the circular arrow that is the
universal "re-read this" mark. `sync` implies two-way exchange with a remote (this touches no
remote), and `autorenew` implies something automatic, which FR-012 says this is explicitly not.

Three places must be updated together, each with an existing gate: the variant, the `ALL` list, and
`glyph()` (all in `icons.rs`), pinned by `tests/icons.rs`'s codepoint table and verified against the
real font by `tests/icons_font.rs::every_icon_codepoint_has_a_glyph`.

---

## R9 — Performance, and what SC-002's 2 seconds is measured against

The re-read is `git worktree list --porcelain` plus a filesystem stat per record
(`micold_core::worktree::discover`). On a repository with 50 worktrees this is one subprocess and
~50 stats — milliseconds, not seconds. It runs in `spawn_blocking` and the daemon never holds its
state lock across it (`state.rs:799-808`), so FR-010's "interface remains responsive" and FR-011's
"other projects undisturbed" are properties the existing code already has; this feature must simply
not break them by, say, calling `refresh_worktrees` on the async runtime.

**Decision**: no new performance work. The gate is that the new call site uses the existing
`refresh_worktrees_and_broadcast` helper, which is already `spawn_blocking`-correct.

---

## R10 — The header is already width-constrained (the plan's main risk)

`ui/material/icon_button.rs`'s `glyph_size` doc records feature 018's **FR-045 deviation**: the
sidebar's controls keep the small 14dp glyph rather than §7.3's 24dp, and the evidence given is
`tests/layout_text_overflow.rs` reporting "the expanded sidebar's header squeezing 'Worktrees' below
the width it needs — four controls at 24dp take 40dp more than four at 14dp, out of a ~260dp panel."

The sidebar's minimum width is 180 px (`app.rs:22`) and `layout_text_overflow.rs:363-376` asserts
that the minimum-width sidebar paints no text past its clip. A fourth header control consumes that
headroom.

**This is the one thing in the feature that can fail for a reason unrelated to its logic**, so it is
verified first rather than last (see plan Phase 2 ordering). Fallbacks, in preference order:

1. **Ellipsize the header title.** `ui/material/ellipsized.rs` exists and is the in-idiom answer —
   a `Fill`-width title that shortens is exactly what it is for, and it removes the squeeze
   permanently rather than for one more control.
2. **Record a widened FR-045-style deviation** if the title cannot shorten without reading badly.
3. **Move the control** — rejected in advance unless 1 and 2 both fail, because it changes what the
   user asked for.

**Decision**: attempt nothing pre-emptively. Add the button, run the gate, and take fallback 1 if it
fires. Fixing an overflow that has not been observed would be a change with no test behind it.

---

## Summary of decisions

| # | Decision | Rationale in one line |
|---|----------|----------------------|
| R1 | Add a seventh refresh trigger; discovery itself unchanged | The cache and its refresh already exist and are correct |
| R2 | New `ClientMsg::WorktreeRefresh`, replying `Ack` | Reusing `ProjectAdd` would reorder the user's projects as a side effect |
| R3 | `PROTOCOL_VERSION` 9 → 10, one edit | The schema hash may move only once per feature |
| R4 | In-flight state lives in `features/worktree.rs` | The listing is that feature's; the button's location confers nothing |
| R5 | An explicit 30 s timeout, the codebase's first | A hung `git` leaves the connection healthy and the button inert forever |
| R6 | Inert button + tooltip + completion snackbar; no spinner | A new indeterminate component drags in three gates for a sub-second state |
| R7 | Single-flight structurally (no `on_press`), guarded again in the reducer | `src/shell/` is outside Principle I's glue exception |
| R8 | `Icon::Refresh` = U+E5D5, verified present in the shipped font | `sync` means remote, `autorenew` means automatic; both are wrong here |
| R9 | No performance work | The existing helper is already `spawn_blocking`-correct |
| R10 | Add the button, then run the overflow gate; ellipsize the title if it fires | The header's headroom is already spent; do not pre-emptively fix an unobserved defect |
