# Contract: `ShellInstanceId`/`ShellInstance` + `Session` instance mutators

Pure, in `src/session.rs`. Governs FR-001–FR-004, FR-007–FR-013, FR-017.

## Identity

`ShellInstanceId(u32)` is allocated from `Session.next_shell_id`, a per-session counter starting
at 1, incremented on every `open_shell_instance()` call and **never** decremented or reused —
even after every instance is closed, the next one opened gets a fresh, higher id (spec
Assumptions: "closed instances are removed from the list and their position is not reused").

## Collection invariant

`Session.active_shell: Option<ShellInstanceId>` is either:

- `None`, if and only if `Session.shells` is empty, or
- `Some(id)` where `id` names a live element of `Session.shells`.

Every mutator below preserves this; there is no other write path to `shells`/`active_shell`.

## State machine — one instance

Unchanged from feature 010's `ShellLifecycle` (`NotStarted | Starting | Running | Exited`,
manual restart only, no crash-loop) — now scoped to one `ShellInstance` instead of the session's
former single shell slot:

```text
NotStarted ──start_shell()──▶ Starting ──mark_running()──▶ Running
     ▲                                                          │
     │                                                          │
     └───────────────────── mark_exited() ◀──────────────────── ┘
                                    │
                                    ▼
                                 Exited ──start_shell()──▶ Starting  (manual restart)
```

`Session::restart_shell_instance(id)` / `mark_shell_running(id)` / `mark_shell_exited(id)` are
thin, id-addressed wrappers around this unchanged state machine — they look up the instance by
`id` and call its existing `ShellLifecycle` method. A wrapper is a no-op if `id` no longer names
a live instance (the instance was closed in a race).

## Collection transitions

### `open_shell_instance() -> ShellInstanceId`

Always succeeds (FR-001: reachable at any instance count, including zero or one). Effect:

1. `id := ShellInstanceId(next_shell_id)`; `next_shell_id += 1`.
2. Push `ShellInstance { id, lifecycle: Starting }` (i.e. `NotStarted` immediately advanced via
   `start_shell()`) to the **end** of `shells` (append-on-open order, FR-001/spec Assumptions).
3. `active_shell := Some(id)`.

Used both for "open an additional instance" (FR-001) and for lazily creating the session's
*first* instance on the first-ever switch into Regular mode (FR-007's "start a first instance if
the session has never had one" — the same method, no special-cased zero-instance path).

### `select_shell(id: ShellInstanceId)`

`active_shell := Some(id)` **only if** `id` names an element of `shells`; otherwise a no-op.
Drives switching the visible pane among open instances (FR-004) and, indirectly, which instance
the primary AI-CLI/Regular toggle shows when switching into Regular mode (FR-007 — the toggle
itself doesn't call this; it just reads whatever `active_shell` already is).

### `close_shell(id: ShellInstanceId)`

No-op if `id` doesn't name a live instance. Otherwise:

1. Record `pos`, the index of `id` in `shells`; remove it.
2. If `active_shell == Some(id)` (the closed instance was the visible one, FR-012):
   `active_shell := shells.get(pos).map(|s| s.id)` — the element now sitting at the removed
   position, i.e. what was the *next* instance in list order — `.or_else(|| shells.last().map(|s|
   s.id))` if there is no such element (the closed instance was last in the list; fall back to
   the new last instance instead).
   If `active_shell != Some(id)` (a background, non-visible instance was closed), `active_shell`
   is untouched — the visible instance and every other sibling stay exactly as they were
   (FR-011, User Story 3 Scenario 1).
3. If `shells` is now empty: `active_shell` is already `None` from step 2's fallback chain, and
   additionally `mode := TerminalMode::AiCli` (FR-013 — closing the last remaining instance
   reverts the session to today's single-terminal close behavior).

This single method implements both the spec's resolved clarification (2026-07-20: "next in list,
else previous") and the FR-013 last-instance fallback — there is no separate code path for "was
it the last one," it falls out of the same position-based reassignment.

## Interaction with `TerminalMode` and `SessionLifecycle` (both unchanged)

`TerminalMode::Regular` means "the pane shows whatever `active_shell` currently names" — it says
nothing about how many instances exist or which ones are running, exactly as `TerminalMode`
already meant nothing about the single shell's running state in feature 010.
`SessionLifecycle`/AI CLI crash-loop behavior is entirely untouched by any of the above — no
mutator here reads or writes `Session.lifecycle`.

## Restart affordance, per instance

A given instance is "restartable" (its individual restart control is shown) exactly when its own
`lifecycle ∈ { NotStarted, Exited }` — identical predicate to feature 010's single-slot rule,
just evaluated per `ShellInstance` in `shells` rather than once for the session.
