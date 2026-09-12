# Phase 1 Data Model: Refresh the Worktree List on Demand

**Feature**: 029-refresh-worktrees-list | **Date**: 2026-08-31

This feature introduces **no new persisted entity and no new domain type**. What it adds is one bit
of transient client state and the transitions that write it. The worktree listing itself
(`micold_core::worktree::Worktree`) is unchanged — this feature changes *when* it is re-read, never
what it is.

---

## 1. State

### 1.1 `features::worktree::State` — one new field

```rust
/// Whether a user-requested re-read of the worktree listing is in flight (feature 029, FR-006).
///
/// Owned here rather than by the sidebar because the thing being re-read is the *listing*, and
/// the listing is this feature's (research R4) — the button's location in the sidebar header
/// confers nothing.
///
/// Transient and never persisted: a refresh that was in flight when the app closed did not
/// happen, and the next start re-reads on attach anyway.
pub refreshing: bool,
```

Default `false` (derived, matching the struct's existing `Default`).

**Why a `bool` and not `Option<u64>` holding the request id**: the id that must be matched belongs
to the shell's `pending_ops` map, which is where correlation already lives (`daemon_sync.rs:156`).
Putting a wire request id in a render-free feature's state would give this module a second
vocabulary — the protocol's — that no other feature module carries.

### 1.2 `shell::daemon_sync::PendingOp` — one new variant

```rust
/// A `WorktreeRefresh` (feature 029). Carries nothing: the refreshed listing arrives on its own
/// as the `CatalogChanged` broadcast, and the only thing the reply adds is "it finished".
WorktreeRefresh,
```

`describe()` gains `PendingOp::WorktreeRefresh => "refresh the worktree list".into()`, which is what
the disconnect drain (`daemon_sync.rs:240`) puts into its "may or may not have taken effect" notice.
For this operation that sentence is unusually true and unusually harmless: a refresh that may or may
not have happened costs nothing, because the reconnect's fresh welcome catalog re-reads anyway.

### 1.3 What is deliberately *not* state

- **The refreshed listing.** It is not carried by the reply. It arrives through `CatalogChanged` →
  `catalog_sync::reconcile_catalog` → `State::set_worktrees`, the path every other worktree
  operation already uses. FR-004 requires the user to be unable to tell which trigger produced a
  listing; the way to guarantee that is to have one path, not two that agree.
- **A timestamp of the last refresh.** Nothing in the spec asks the UI to show one, and a field
  written on every refresh that nothing reads is a field that will be wrong later.
- **A retry count.** FR-008 leaves the previous listing in place and returns the control to idle;
  the user retries by pressing again.

---

## 2. Messages

### 2.1 `features::worktree::Msg` — three new variants

| Variant | Raised by | Meaning |
|---------|-----------|---------|
| `RefreshRequested` | the sidebar header control | The user asked for the listing to be re-read |
| `RefreshFinished` | the shell, on `OperationOk` or `OperationError` | The request reached a terminal outcome |
| `RefreshTimedOut(u64)` | the shell's armed timer | The bounded wait for request `u64` elapsed |

`RefreshTimedOut` carries the request id even though `State::refreshing` does not, because the
*shell* arm that receives it must distinguish a timer for the live request from one left over by a
request that already replied. The reducer ignores the payload; the shell reads it. This is the one
place the two vocabularies meet, and it meets in the shell, which is where correlation lives.

### 2.2 Transitions

```
                    ┌──────────────────────────────────────────┐
                    │                                          │
                    ▼                                          │
              ┌───────────┐   RefreshRequested          ┌──────────────┐
              │  refreshing│──────────────────────────▶ │  refreshing  │
   (initial)─▶│  = false   │   (project active)         │   = true     │
              │            │                            │              │
              └───────────┘                             └──────────────┘
                    ▲                                          │
                    │  RefreshFinished                         │
                    │  RefreshTimedOut(req)  ◀─────────────────┘
                    │
                    └── RefreshRequested while true: IGNORED (single-flight, FR-006)
```

Written as rules, each of which is a test:

| # | Given | Message | Then |
|---|-------|---------|------|
| T1 | `refreshing == false` | `RefreshRequested` | `refreshing == true`; no outcome emitted |
| T2 | `refreshing == true` | `RefreshRequested` | **unchanged** — the second request is dropped (FR-006) |
| T3 | `refreshing == true` | `RefreshFinished` | `refreshing == false` |
| T4 | `refreshing == false` | `RefreshFinished` | **unchanged** — an outcome for a refresh nobody started is not an error, it is a late reply |
| T5 | `refreshing == true` | `RefreshTimedOut(_)` | `refreshing == false` |
| T6 | any | any of the three | the worktree **listing** is untouched — the flag is the only field written |

T6 is the one that matters for FR-009 and US3: none of these transitions may touch `worktrees`,
`expanded`, `filters`, `hovered`, or any menu state. The refreshed listing changes those, and it
does so through `set_worktrees`, which already has that reconciliation tested.

---

## 3. The derived predicate

```rust
impl State {
    /// Whether the worktree listing can be re-read right now (feature 029, FR-005/FR-006).
    ///
    /// Pure and unit-tested, so the view's single `if` over it is the "thin glue invoking
    /// already-tested pure logic" Principle I's exception describes rather than a decision of
    /// its own (research R7).
    pub fn can_refresh_worktrees(&self) -> bool {
        self.workspace.active.is_some() && !self.worktree.refreshing
    }
}
```

| # | Given | `can_refresh_worktrees()` | Requirement |
|---|-------|---------------------------|-------------|
| P1 | no active project | `false` | FR-005 |
| P2 | active project, idle | `true` | FR-003 |
| P3 | active project, refreshing | `false` | FR-006 |

The view renders the control with `on_press` **only** when this is true. That is what makes
single-flight structural: while a refresh runs, no press can produce a message, so there is nothing
for a guard to catch. The reducer's T2 remains as the second line of defence, because a structural
guarantee that lives only in a render is one refactor away from not existing.

Deliberately **not** in the predicate: whether the client is connected. `send_op` already raises the
"Not connected to the session service — can't refresh the worktree list right now." notice
(`daemon_sync.rs:145-152`), and telling the user why is better than a control that is silently inert
for a reason it does not explain.

---

## 4. Wire

One new `ClientMsg` variant; no new `DaemonMsg`, no new `OperationResult`. See
[`contracts/worktree-refresh.md`](./contracts/worktree-refresh.md) for the full contract.

---

## 5. Validation rules

| Rule | Where enforced |
|------|----------------|
| At most one refresh in flight per client | View (no `on_press`) + reducer T2 |
| A refresh names the *active* project only (FR-011) | The shell reads `workspace.active` when building the message; there is no project parameter the caller could get wrong |
| A refresh never empties the listing on failure (FR-008) | Structural: the failure path writes only `refreshing = false` and raises a notice. `set_worktrees` is not on it |
| The busy state always ends (FR-007) | Three exits: `OperationOk`, `OperationError`, and the timer — plus the disconnect drain, which is a fourth |
| Nothing is persisted | `refreshing` is not in any `Persisted*` type; the persistence gate in `shell/persist.rs` sees no new field |
