# Contract: when a reading runs, what it writes, and the wire change

**Feature**: [spec.md](../spec.md) | **Research**: [R5–R8, R11–R13](../research.md) |
**Data**: [data-model §3, §5, §6](../data-model.md)

---

## 1. Start events and the source gate

A reading may start only when **all** hold: the switch is on, the window shows a project, the
daemon connection is up, and `now ≥ pause_until`. It is started by exactly these events:

| # | Event | Message that carries it | Requirement |
|---|---|---|---|
| S1 | the shown project's first listing arrives after it is opened or switched to | `WorktreeMsg::Loaded` for a project that was not the shown one | FR-018 "opened in a window" |
| S2 | the switch turns on | `Welcome` / `SettingsChanged` with `pr_status_enabled: true` after `false` | FR-030 |
| S3 | the interval elapses | `Message::PrStatusTick` from `iced::time::every(300 s)` | FR-018 "every 5 minutes" |
| S4 | a list refresh ends | `WorktreeMsg::RefreshFinished` or `RefreshTimedOut` | FR-018, FR-018a |
| S5 | a reading ends with `again` set | `PrStatusMsg::Finished` | FR-022 |

- **S1 also covers reconnection and takeover**: a window that reconnects or takes a project over
  receives a fresh listing for it and reads once.
- **S2 with a project already shown** reads at once; a `Welcome` that carries `true` at start-up
  starts nothing by itself — S1 follows when the listing arrives. One of S1 and S2 fires per
  opening, never both.
- **S4 is the end of the list refresh**, successful or not. The daemon broadcasts the new listing
  before it acknowledges the refresh, so the branches read are those the updated listing shows
  (FR-018a). The refresh control's busy state and its notice are settled by the existing
  `RefreshFinished` handling before the reading starts and are never touched by it (FR-027).
- **What a reading covers**: every entry of `state.worktree.worktrees` with `branch: Some`,
  deduplicated, in listing order. A reading never sends `ClientMsg::WorktreeRefresh` and never
  asks for a listing.
- **Not start events**: `CatalogChanged`, worktree create / delete / rename / include, hover,
  selection, focus, a session event.

**The interval subscription** exists only while the switch is on and a project is shown
(`shell/subscriptions.rs`). `tests/idle_subscriptions.rs` holds: switch off ⇒ the timer is absent.

**Source gate** `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs`: the
function that turns a start into a task (`shell::pr_status::start`) is called from exactly the
lines that handle S1 to S5, counted as `issues_are_requested_only_on_named_events.rs` counts its
own; and `PullRequestSource::read` is called from one place.

---

## 2. The schedule reducer (`features/pr_status.rs`)

Render-free, no clock of its own: every message that needs the time carries `now` (Unix seconds).

```rust
pub enum Msg {
    Trigger { cause: Cause, now: u64 },          // S1–S4
    Finished { seq: u64, outcome: Outcome, now: u64 },
    EnabledChanged { enabled: bool, now: u64 },
    ProjectChanged,                               // shown project switched or closed
}
pub enum Cause { Opened, SwitchedOn, Interval, Refresh }
pub enum Effect { None, Read { seq: u64 } }
pub fn update(state: &mut State, msg: Msg, shown: bool) -> Effect
```

`Outcome` is `Ok { statuses, removable, started_at }` or `Err(ReadingFailure)`.

| State | Message | New state | Effect |
|---|---|---|---|
| any | `Trigger` while `!enabled`, `!shown` or paused | unchanged | `None` |
| `Idle` | `Trigger` | `Reading { seq: next_seq, again: false }` | `Read { seq }` |
| `Reading` | `Trigger { Refresh }` | `again = true` | `None` |
| `Reading` | `Trigger { Interval \| Opened \| SwitchedOn }` | unchanged | `None` |
| `Reading { seq }` | `Finished { seq }` | data-model "State transitions"; then `Idle` | `Read` when `again` and not paused, else `None` |
| any | `Finished` with another `seq` | unchanged | `None` |
| any | `EnabledChanged { false }` | data-model §3 invariant 3 | `None` |
| `Idle`, was off | `EnabledChanged { true }` and `shown` | `Reading` | `Read { seq }` |
| any | `ProjectChanged` | data-model §3 invariant 4 | `None` (S1 follows) |

Invariants, each a test:

- **One at a time** (FR-022): no sequence of messages yields two `Read` effects without a
  `Finished` (or a reset) between them.
- **Any number of refreshes ⇒ one further reading**: three `Trigger { Refresh }` during a reading
  and its `Finished` yield exactly one `Read`.
- **Paused**: after `RateLimited { until }`, every `Trigger` with `now < until` yields `None`, a
  pending `again` is dropped, and the first `Trigger` with `now ≥ until` reads (FR-024, story 4
  scenario 7). A refresh pressed during the pause is not remembered.
- **A lost task cannot stop the schedule for ever**: `Reading` records its start time; a `Trigger`
  that arrives 60 s or more after it treats the reading as abandoned (`Passing`) and starts a new
  one with a new `seq`, so a late answer of the old one is dropped. 60 s is above the worst case of
  a real reading (§3) for up to 50 worktrees.
- **FR-020**: `update` takes only its own `State`. The shell's handling of a reading's messages
  writes nothing but that state; a reducer test over the application state asserts selection,
  expansion, scroll, filter and sessions are equal before and after a `Finished`.

**What one reading does** (`shell/pr_status.rs`, the pattern of `shell/issues.rs`):

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

`PROTOCOL_VERSION` 20 → 21, **one** bump for §3 and §4 together, in the milestone that ships them,
with `crates/micold-core/tests/schema_hash.rs` re-pinned and `protocol_roundtrip.rs` given an
example of `MergedBranchCheck` (request and result) and of `SettingsSet` / `DaemonSettings` with
the new field. If another feature has taken 21 on `main` by then, the next free number is used.
A client and a daemon of different versions refuse each other at the handshake, as today.
