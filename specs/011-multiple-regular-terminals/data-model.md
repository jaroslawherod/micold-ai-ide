# Phase 1 Data Model: Multiple Regular Terminal Instances per Session

Only the deltas over the current implementation (features 005/006/008/010) are listed. "Pure" =
render-free core, testable under `--no-default-features`. "GUI" = compiled only with the `gui`
feature (`src/main.rs`, `src/ui/`).

## Pure core (`src/`)

### `ShellInstanceId` (NEW, pure) — `src/session.rs`

Identifies one Regular Terminal instance within a single session (research R1). Unique only
within its owning session, for the lifetime of the running app — never persisted, never
compared across sessions.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShellInstanceId(pub u32);
```

Doubles as the switcher row's display label (the spec's own Assumption: instances are identified
by creation order) — no separate "display name" field.

### `ShellInstance` (NEW, pure) — `src/session.rs`

```rust
pub struct ShellInstance {
    pub id: ShellInstanceId,
    pub lifecycle: ShellLifecycle,   // unchanged enum + methods from feature 010
}
```

`ShellLifecycle` itself (`NotStarted | Starting | Running | Exited`, manual-restart-only) is
**unchanged** — it now describes one instance's state instead of the session's single (former)
shell slot; its existing `start_shell()`/`mark_running()`/`mark_exited()`/`is_active()` methods
are reused as-is, called through the id-addressed `Session` wrappers below.

### `Session` (extended) — `src/session.rs`

```rust
pub struct Session {
    pub id: SessionId,
    pub location: SessionLocation,
    pub label: SessionLabel,
    pub lifecycle: SessionLifecycle,      // unchanged (AI CLI)
    pub mode: TerminalMode,               // unchanged (persisted, FR-011 from feature 010)
    pub shells: Vec<ShellInstance>,       // REPLACES shell_lifecycle: ShellLifecycle
    pub active_shell: Option<ShellInstanceId>,  // NEW — which instance is attached/last-active
    pub next_shell_id: u32,               // NEW — monotonic counter, never reused (research R1)
}
```

New/changed methods, all total (never panic) and the **only** legal way to mutate
`shells`/`active_shell`/`next_shell_id`:

- **`fn open_shell_instance(&mut self) -> ShellInstanceId`** (NEW) — allocates
  `ShellInstanceId(self.next_shell_id)`, increments the counter, pushes a new `ShellInstance`
  with `lifecycle` already advanced `NotStarted → Starting` (mirrors `start_shell`'s effect),
  appends it to `shells` (append-on-open ordering, spec Assumptions), sets `active_shell =
  Some(id)`, and returns `id` (FR-001, FR-007's "start a first instance if the session has never
  had one" reuses this same method).
- **`fn restart_shell_instance(&mut self, id: ShellInstanceId)`** (NEW) — finds the instance by
  `id` and calls its `lifecycle.start_shell()` (idempotent no-op unless `NotStarted`/`Exited`,
  same rule as feature 010's single-slot restart, FR-010). No-op (not found) if `id` no longer
  exists — closing races a restart press harmlessly.
- **`fn mark_shell_running(&mut self, id: ShellInstanceId)`** (NEW) — finds the instance by `id`,
  calls `lifecycle.mark_running()`. No-op if not found.
- **`fn mark_shell_exited(&mut self, id: ShellInstanceId)`** (NEW) — finds the instance by `id`,
  calls `lifecycle.mark_exited()` (no restart decision, FR-008). No-op if not found.
- **`fn select_shell(&mut self, id: ShellInstanceId)`** (NEW) — sets `active_shell = Some(id)`
  only if `id` names an element of `shells`; otherwise a no-op (guards against a stale id from a
  race with a concurrent close). Drives the switcher row (FR-004) and the primary toggle's
  "whichever instance was last active" behavior (FR-007).
- **`fn close_shell(&mut self, id: ShellInstanceId)`** (NEW) — removes the instance at `id` from
  `shells` (its position is never reused, research R1). If it was `active_shell`, reassigns to
  the element now at the removed position (the former *next* instance, FR-012) or, if none (the
  closed instance was last), the new last element (`shells.last()`); both branches naturally
  yield `None` when `shells` is now empty. If `shells` is empty after removal, additionally sets
  `self.mode = TerminalMode::AiCli` (FR-013 — closing the last instance falls back to today's
  single-terminal close behavior). No-op (not found) if `id` doesn't name a live instance.
- **`fn active_shell_lifecycle(&self) -> Option<ShellLifecycle>`** (NEW, convenience) — the
  currently-active instance's lifecycle, or `None` if `shells` is empty; replaces direct
  `session.shell_lifecycle` field reads in `src/ui/terminal.rs`'s `session_status`/
  `attached_process_restartable`.

`Session::start_new`/`Session::restored` both initialize `shells: Vec::new()`, `active_shell:
None`, `next_shell_id: 1` (a session always starts with zero Regular Terminal instances,
matching FR-017's "at most one, freshly started" — the first instance is created lazily by
`open_shell_instance` on first switch to Regular mode, same laziness feature 010 already had).

**Invariant** (Principle V — enforced by construction, not a runtime check): `active_shell` is
either `None` (iff `shells.is_empty()`) or `Some(id)` where `id` names a live element of
`shells`. All five mutators above preserve this; no other code path writes `shells` or
`active_shell` directly.

**FR-016/FR-015-style invariant** (carried over from feature 010, restated for the new shape):
none of the methods above read or call anything in `src/provider.rs` — a Regular Terminal
instance has no transcript, no title, and never touches `session.label`; only the AI CLI side
(`lifecycle`, unchanged) is ever written by `sync_session_titles`/`read_title`.

### `Message` (extended) — `src/app.rs`

New variants (replacing feature 010's single-slot `ShellSessionRunning(SessionId)` /
`ShellSessionExited(SessionId)`, and the shell branch of `TerminalRestartRequested`):

- **`ShellInstanceOpenRequested`** — the "+" affordance or the `Ctrl+Shift+T`/`Cmd+Shift+T`
  chord fired for the active session. Binary-side: no-op if that session's `mode !=
  TerminalMode::Regular` (spec edge case — the shortcut does nothing, and does not also switch
  modes); otherwise calls `session.open_shell_instance()` directly (mirrors how
  `SessionStartRequested` constructs its `Session` directly rather than round-tripping through a
  pure reducer) to obtain the new `ShellInstanceId` before spawning its PTY.
- **`ShellInstanceSelected(ShellInstanceId)`** — a switcher-row entry was clicked. Pure reducer:
  `session.select_shell(id)` for `active_session` (mirrors `SessionSelected`, scoped to a
  sub-session-level selection instead of the sidebar's session-level one).
- **`ShellInstanceCloseRequested(ShellInstanceId)`** — an instance's close action was pressed.
  Pure reducer: `session.close_shell(id)` (may also flip `mode` back to `AiCli`, FR-013 — see
  `close_shell` above); binary-side follow-up kills and removes that one `RuntimeTerminal` from
  the gui-side map (mirrors `SessionCloseRequested`'s `kill_all`, scoped to one id).
- **`ShellInstanceRestartRequested(ShellInstanceId)`** — replaces the shell branch of feature
  010's mode-generic `TerminalRestartRequested` (which remains, unchanged, for the AI CLI
  branch) — now id-addressed since more than one instance may need independent restart.
- **`ShellInstanceRunning(SessionId, ShellInstanceId)`** — the shell process for `id` in session
  `SessionId` is up. Pure reducer: `session.mark_shell_running(id)`.
- **`ShellInstanceExited(SessionId, ShellInstanceId)`** — that instance's shell process exited.
  Pure reducer: `session.mark_shell_exited(id)`.

No existing `Message` variant is removed other than the two feature-010 shell variants these
directly replace.

## GUI (`src/ui/`, `src/main.rs`)

### `SessionTerminals` (extended) — `src/ui/terminal.rs`

```rust
#[derive(Default)]
pub struct SessionTerminals {
    pub ai_cli: Option<RuntimeTerminal>,
    pub shells: std::collections::HashMap<ShellInstanceId, RuntimeTerminal>,  // was: Option<RuntimeTerminal>
}

impl SessionTerminals {
    pub fn attached(&self, mode: TerminalMode, active_shell: Option<ShellInstanceId>)
        -> Option<&RuntimeTerminal> { .. }          // Regular arm: active_shell.and_then(|id| self.shells.get(&id))
    pub fn attached_mut(&mut self, mode: TerminalMode, active_shell: Option<ShellInstanceId>)
        -> Option<&mut RuntimeTerminal> { .. }
    pub fn each_mut(&mut self) -> impl Iterator<Item = &mut RuntimeTerminal> { .. }  // ai_cli + every shell entry
    pub fn kill_all(&mut self) { .. }                 // kills ai_cli + every shell entry, clears the map
    pub fn close_shell(&mut self, id: ShellInstanceId) { .. }  // NEW: kill + remove exactly one entry
}
```

Every call site that reads `session.mode` alone to pick a slot (`attached`/`attached_mut`) now
also threads that session's `active_shell` through — the two together (not `mode` alone) name
which single `RuntimeTerminal`, if any, is attached to the pane, exactly as `mode` alone did when
there was only ever one shell slot to pick between.

### `App.terminals` call-site changes — `src/main.rs`

- **`Message::ShellInstanceOpenRequested`**: gated on the active session's `mode == Regular`
  (edge case above); calls `session.open_shell_instance()` for the new id, spawns via
  `spawn_shell_pty` (unchanged from feature 010), inserts into
  `app.terminals[id].shells[shell_id]`, follows up with `ShellInstanceRunning`, persists (only
  `mode` is ever persisted — unchanged from feature 010, so this is a no-op write unless `mode`
  itself also changed).
- **`Message::ShellInstanceCloseRequested(shell_id)`**: `app.terminals.get_mut(&id)
  .map(|st| st.close_shell(shell_id))` (kills + removes just that `RuntimeTerminal`), then
  `core.update(Message::ShellInstanceCloseRequested(shell_id))` (pure `close_shell`, may flip
  `mode` to `AiCli` per FR-013 — if so, `ensure_attached_process`-style logic reattaches the AI
  CLI process exactly as toggling the primary button already would).
- **`Message::ShellInstanceRestartRequested(shell_id)`** / selecting a different instance: same
  spawn-if-absent shape as feature 010's `ensure_attached_process`, now looked up by
  `(session_id, shell_id)` instead of `(session_id,)`.
- **`handle_process_exits`**: the shell-exit scan iterates every entry of
  `st.shells` instead of a single `Option` (each independently detected, each independently
  marked `Exited` via `session.mark_shell_exited(id)` — still no restart decision, no auto-
  respawn, per FR-008/unchanged feature 010 policy).

**Structural invariant** carried by this shape: a session can have 0..N live shell processes (in
addition to its 0/1 AI CLI process); `Session.active_shell` says which shell instance, if any, is
*displayed* when `mode == Regular` — independent of how many instances exist or are currently
running, exactly the same "displayed ≠ running" split feature 010 established, now applied
across a collection instead of a single slot.
