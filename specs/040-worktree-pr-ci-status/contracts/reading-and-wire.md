# Contract: when a reading runs, what it writes, and the wire change

**Feature**: [spec.md](../spec.md) | **Research**: [R5–R8, R11–R13](../research.md) |
**Data**: [data-model §3, §5, §6](../data-model.md)

---

## 1. Start events and the source gate

A reading may start only when **all** hold: the switch is on, the window **holds** its active
project, that project's listing has arrived from the daemon, and `now ≥ pause_until`.

**Holding** (R6). The window holds its active project from `DaemonMsg::Attached { project }` for it
until `DaemonMsg::Displaced` or `Refused { ProjectBusy }` for it, a switch to another project, or a
disconnect — the facts `app.displaced` and `app.disconnected` already record. `Released` is sent
exactly where `app.displaced.insert` runs: the two early returns for a window's own superseded
connection (010 BUG-022) record no displacement and send none. A refused or
displaced window stays on the project read-only; it does not hold it, reads nothing and shows no
indicator.

A reading is started by exactly these events:

| # | Event | Message that carries it | Requirement |
|---|---|---|---|
| S1 | the listing arrives after the window came to hold the project | the first `DaemonMsg::CatalogChanged` after `DaemonMsg::Attached` for the active project (`awaiting_listing`, data-model §3) | FR-018 "opened in a window" |
| S2 | the switch turns on while the project is held and listed | `SettingsChanged` with `pr_status_enabled: true` after `false` | FR-030 |
| S3 | the interval elapses | `Message::PrStatusTick` from `iced::time::every(300 s)` | FR-018 "every 5 minutes" |
| S4 | a list refresh ends | `WorktreeMsg::RefreshFinished` or `RefreshTimedOut` | FR-018, FR-018a |
| S5 | a reading ends with `again` set | `PrStatusMsg::Finished` | FR-022 |

- **S1 covers opening, switching, reconnecting and taking over**: each makes the client send
  `ClientMsg::Attach`, and the daemon answers `Attached` and then one `CatalogChanged` with the
  freshly discovered worktrees (`refresh_worktrees_and_send`). `Attached` sets `awaiting_listing`;
  the `CatalogChanged` arm of `shell/daemon_sync.rs`, after `reconcile_catalog`, clears it and
  sends `Msg::ListingArrived`, which starts the reading. Every later `CatalogChanged` finds the flag clear and starts nothing.
  `WorktreeMsg::Loaded` is not used: production code never sends it.
- **S2 and start-up do not overlap**: `Welcome` carries the settings before any `Attached`, so
  `EnabledChanged { true }` then finds `held == false` and starts nothing; S1 follows. While
  `awaiting_listing` is set, `EnabledChanged { true }` also starts nothing, for the same reason: the
  listing is about to arrive and S1 reads it. So one of S1 and S2 fires per opening, never both, and
  no reading runs over the listing of a project the daemon has not yet answered for.
- **S4 is the end of the list refresh**, successful or not. The daemon broadcasts the new listing
  before it acknowledges the refresh, so the branches read are those the updated listing shows
  (FR-018a). The refresh control's busy state and its notice are settled by the existing
  `RefreshFinished` handling before the reading starts and are never touched by it (FR-027).
- **What a reading covers**: every entry of `state.worktree.worktrees` with `branch: Some`,
  deduplicated, in listing order. A reading never sends `ClientMsg::WorktreeRefresh` and never
  asks for a listing.
- **Not start events**: any `CatalogChanged` but S1's, worktree create / delete / rename / include,
  hover, selection, focus, a session event.

**The interval subscription** exists only while the switch is on and the window holds its active
project (`shell/subscriptions.rs`). `tests/idle_subscriptions.rs` holds: switch off ⇒ the timer is
absent; a read-only (displaced) window ⇒ the timer is absent.

**Source gate** `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs`: the
function that turns a start into a task (`shell::pr_status::start`) is called from exactly the
lines that handle S1 to S5, counted as `issues_are_requested_only_on_named_events.rs` counts its
own; `Msg::ListingArrived` is sent from exactly one line, inside the `CatalogChanged` arm, and only
the reducer's `awaiting_listing` row turns it into a reading; and `PullRequestSource::read` is called from one place.

---

## 2. The schedule reducer (`features/pr_status.rs`)

Render-free, no clock of its own: every message that needs the time carries `now` (Unix seconds).

```rust
pub enum Msg {
    Trigger { cause: Cause, now: u64 },          // S1–S4
    Finished { seq: u64, outcome: Outcome, now: u64 },
    EnabledChanged { enabled: bool, now: u64 },
    Held,                                         // `Attached` for the active project
    ListingArrived { now: u64 },                  // `CatalogChanged`; S1 when `awaiting_listing`
    Released,                                     // project switched or closed, displaced, refused, disconnected
}
pub enum Cause { Interval, Refresh }
pub enum Effect { None, Read { seq: u64 } }
pub fn update(state: &mut State, msg: Msg) -> Effect
```

`Outcome` is `Ok { statuses, removable, started_at }` or `Err(ReadingFailure)`.

| State | Message | New state | Effect |
|---|---|---|---|
| any | `Trigger` while `!enabled`, `!held`, `awaiting_listing` or paused | unchanged | `None` |
| `Idle` | `Trigger` | `Reading { seq: next_seq, again: false }` | `Read { seq }` |
| `Reading` | `Trigger { Refresh }` | `again = true` | `None` |
| `Reading` | `Trigger { Interval }` | unchanged | `None` |
| any | `Held` | `held = true`, `awaiting_listing = true` | `None` |
| `awaiting_listing` | `ListingArrived` | `awaiting_listing = false`; then as `Trigger` in `Idle` (refused while off or paused) | `Read { seq }` or `None` |
| not `awaiting_listing` | `ListingArrived` | unchanged | `None` |
| `Reading` | `Held` | `held = true`, `awaiting_listing = true`; the reading goes on | `None` |
| `Reading`, `awaiting_listing` | `ListingArrived` | `awaiting_listing = false`, `again = true` | `None` (one further reading when this one ends) |
| `Reading { seq }` | `Finished { seq }` | data-model "State transitions"; then `Idle` | `Read` when `again` and not paused, else `None` |
| any | `Finished` with another `seq` | unchanged | `None` |
| any | `EnabledChanged { false }` | data-model §3 invariant 3 | `None` |
| `Idle`, was off | `EnabledChanged { true }`, `held` and not `awaiting_listing` | `Reading` | `Read { seq }` |
| was off | `EnabledChanged { true }`, `!held` or `awaiting_listing` | `enabled = true` | `None` (S1 follows) |
| any | `EnabledChanged` with the value it already has | unchanged | `None` |
| any | `Released` | data-model §3 invariant 4: statuses cleared, `Idle`, `held = false`; `pause_until` kept | `None` (S1 follows a new `Attached`) |

Invariants, each a test:

- **One at a time** (FR-022): no sequence of messages yields two `Read` effects without a
  `Finished` (or a reset) between them.
- **Any number of refreshes ⇒ one further reading**: three `Trigger { Refresh }` during a reading
  and its `Finished` yield exactly one `Read`.
- **Paused**: after `RateLimited { until }`, every `Trigger` and `ListingArrived` with `now < until`
  yields `None`, a pending `again` is dropped, and the first `Trigger` with `now ≥ until` reads
  (FR-024, story 4 scenario 7). A refresh pressed during the pause is not remembered. The pause
  survives `Released`: a project opened during it is first read by the interval tick that follows
  the pause's end, at most 5 minutes after it.
- **Only the holder reads and shows** (SC-007): after `Released` no message but `Held` followed by
  `ListingArrived` yields a `Read`, and `statuses` is empty; `Held` + `ListingArrived` (a take-over)
  yields exactly one `Read`.
- **A lost task cannot stop the schedule for ever**: `Reading` records its start time; a `Trigger`
  that arrives 60 s or more after it treats the reading as abandoned (`Passing`) and starts a new
  one with a new `seq`, so a late answer of the old one is dropped. 60 s is above the worst case of
  a real reading (§3) for up to 50 worktrees.
- **FR-020**: `update` takes only its own `State`. The shell's handling of a reading's messages
  writes nothing but that state; a reducer test over the application state asserts selection,
  expansion, scroll, filter and sessions are equal before and after a `Finished`.

**What one reading does** (`shell/pr_status.rs`, the pattern of `shell/issues.rs`):

0. The shell handles `Effect::Read { seq }` by collecting the branches (§1) and starting the task.
1. `ClientMsg::RemoteList` to the daemon → `choose_remote`. No github.com remote ⇒ `Unavailable`,
   nothing further (FR-026).
2. `locate_gh`. Not found ⇒ `Unavailable`.
3. `spawn_blocking`: `PullRequestSource::read` ([pull-request-source §1](./pull-request-source.md)).
4. For the branches whose status is `Merged`, and only when there is one:
   `ClientMsg::MergedBranchCheck` (§3) → `removable`.
5. `PrStatusMsg::Finished`.

Each daemon step (1 and 4) is bounded by 10 s in the client. Step 1 without an answer ⇒ `Passing`.
Step 4 without an answer, or failed ⇒ the statuses are applied with `removable` empty. Nothing in
steps 1 to 5 runs on the update loop (FR-021, SC-005).

---

## 3. `MergedBranchCheck` RPC

```rust
// ClientMsg
MergedBranchCheck { req: u64, project: PathBuf, checks: Vec<MergedBranchQuery> }
// OperationResult
MergedBranchCheck { answers: Vec<BranchContainment> }
```

Daemon arm, a copy of `RemoteList`'s (`project_repo`, `reject_non_repo`, `spawn_blocking`). Per
query, through the `Git` trait:

| Step | Git | Outcome |
|---|---|---|
| tip | `Git::branch_tip(repo, branch)` = `git rev-parse --verify --quiet refs/heads/<branch>^{commit}` | none ⇒ `Unknown` |
| equal | tip = `head` | `Contained` |
| ancestor | `Git::is_ancestor(repo, tip, head)` = `git merge-base --is-ancestor <tip> <head>` | exit 0 ⇒ `Contained`; exit 1 ⇒ `Beyond`; any other exit (`head` unknown locally) ⇒ `Unknown` |

- **Read-only**: no fetch, no ref is written, no worktree is touched. A source test asserts the arm
  calls only `branch_tip` and `is_ancestor`.
- `head` is used only when it is 40 or 64 hexadecimal characters; `branch` is passed as a ref name
  under `refs/heads/`, never as a free argument. Anything else ⇒ `Unknown` without running git.
- `answers.len() == checks.len()`, same order. At most 50 queries are answered; a longer list is
  refused as a bad request (the client never sends more than its reading's merged branches, and
  chunks at 50).
- A non-repository project is refused exactly as `RemoteList` refuses it; the client treats any
  refusal as "no answer".
- `FakeGit` gains scripted `branch_tip` and `is_ancestor`.

---

## 4. The setting on the wire

Exactly the path of `tool_server_enabled`:

| Place | Change |
|---|---|
| `micold_core::settings::Settings` | `pr_status_enabled: bool`, `#[serde(default)]` |
| `protocol::DaemonSettings` | `pr_status_enabled: bool` (in `Welcome` and `SettingsChanged`) |
| `ClientMsg::SettingsSet` | `pr_status_enabled: Option<bool>` |
| daemon `SettingsSet` arm, `catalog.rs`, `state.rs` | store, persist, broadcast `SettingsChanged` to every client |
| client `shell/daemon_sync.rs` settings mirror | `PrStatusMsg::EnabledChanged` on a changed value |
| client `shell/persist.rs` | sends the field when the Settings draft changed it |

Every window follows the broadcast: off clears all indicators in all windows at once (FR-029); on
starts a reading in every window that shows a project (FR-030).

---

## 5. Protocol version

`PROTOCOL_VERSION` 21 → 22 (feature 039 took 21 first; planned as 20 → 21), **one** bump for §3 and §4 together, in the milestone that ships them,
with `crates/micold-core/tests/schema_hash.rs` re-pinned and `protocol_roundtrip.rs` given an
example of `MergedBranchCheck` (request and result) and of `SettingsSet` / `DaemonSettings` with
the new field. If another feature has taken 21 on `main` by then, the next free number is used.
A client and a daemon of different versions refuse each other at the handshake, as today.
