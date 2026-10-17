# Data Model: Worktree State Icon

**Feature**: 905 | **Plan**: [plan.md](./plan.md) | Research: [research.md](./research.md)

Nothing new is stored. Every entity is derived, or a runtime-only field beside `activity`.

## TurnClock (existing, `crates/micold-core/src/attention.rs`) — one new reader

| Member | Meaning |
|---|---|
| `paused_on_user(&self) -> bool` | **New.** `true` exactly in `TurnState::Paused`: the session was working, stopped mid-turn to ask for a permission or an answer, and has not resumed or finished (R1). |

## SessionSummary (existing, `crates/micold-core/src/protocol/messages.rs`) — one new field

| Field | Type | Rule |
|---|---|---|
| `paused_on_user` | `bool`, `#[serde(default)]` | Runtime-only; overlaid from the live turn clock in `overlay_live_summaries`; `false` with no live entry. Read only when `activity == AwaitingInput`. Contract: [wire.md](./contracts/wire.md). |

## Session (existing, `crates/micold-core/src/session.rs`) — one new field

| Field | Type | Rule |
|---|---|---|
| `paused_on_user` | `bool` | Runtime-only, like `activity`; `false` from every constructor; adopted from each summary in `crates/micold-client/src/catalog_sync.rs` at both places `activity` is. |

## LocationState (new, `crates/micold-core/src/location_state.rs`)

The spec's *worktree state*, for any location (a worktree or Default).

```text
enum LocationState { Idle, Unknown, Running, Blocked(BlockedReason) }   // ascending precedence
enum BlockedReason { Resumable, Failed, NeedsPermission }               // ascending precedence
```

`Ord` is derived in that order, so the location's state is the `max` of its sessions' states
(FR-004, R5).

### `session_state(&Session) -> LocationState` — the spec's Per-session rules, first match wins

| # | lifecycle | activity | `paused_on_user` | State |
|---|---|---|---|---|
| 1 | `Failed { .. }` | any | any | `Blocked(Failed)` |
| 1 | `InterruptedResumable` | any | any | `Blocked(Resumable)` |
| 2 | `Starting`, `Restarting { .. }` | any | any | `Running` |
| 3 | `Idle` | any | any | `Idle` |
| 4 | `Running` | `Working` | any | `Running` |
| 5 | `Running` | `AwaitingInput` | `true` | `Blocked(NeedsPermission)` |
| 5 | `Running` | `AwaitingInput` | `false` | `Idle` |
| 6 | `Running` | `Ended { .. }` | any | `Idle` |
| 7 | `Running` | `Unknown` | any | `Unknown` |

### `location_state(sessions) -> LocationState`

- Skips `archived` sessions; no session left → `Idle` (FR-003).
- Otherwise the maximum `session_state` (Blocked > Running > Unknown > Idle; among blocked,
  NeedsPermission > Failed > Resumable).

### Words (FR-009) — `LocationState::tooltip_line(self) -> &'static str`

| State | Line |
|---|---|
| `Running` | `Running` |
| `Blocked(NeedsPermission)` | `Blocked: needs permission` |
| `Blocked(Failed)` | `Blocked: failed` |
| `Blocked(Resumable)` | `Blocked: resumable` |
| `Idle` | `Idle` |
| `Unknown` | `State unknown` |

### Glyph (client, `StateIcon`) — contract [state-icon.md](./contracts/state-icon.md)

| State | Glyph | Role |
|---|---|---|
| `Running` | `Icon::StateRunning` (`play_arrow`) | `primary` |
| `Blocked(_)` | `Icon::StateBlocked` (`report`) | `error` |
| `Idle` | `Icon::StateIdle` (`pause`) | `on_surface_variant` |
| `Unknown` | nothing; slot reserved | — |

## Transitions

Derived, never stored: recomputed on every render from the sessions the last catalog snapshot
gave. A snapshot is broadcast on every lifecycle change and, after R3, on every activity or
`paused_on_user` change. While disconnected the client keeps its last sessions, so every row keeps
its last state (spec Edge Cases).
