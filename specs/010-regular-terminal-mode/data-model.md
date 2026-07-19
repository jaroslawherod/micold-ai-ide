# Phase 1 Data Model: Switchable Regular Terminal Mode

Only the deltas over the current implementation (features 005/006/008) are listed. "Pure" =
render-free core, testable under `--no-default-features`. "GUI" = compiled only with the `gui`
feature (`src/main.rs`, `src/ui/`).

## Pure core (`src/`)

### `TerminalMode` (NEW, pure) — `src/session.rs`

Which of a session's two processes is currently attached to its visible terminal pane.
Persisted (FR-011).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalMode {
    #[default]
    AiCli,
    Regular,
}
```

- **`fn other(self) -> TerminalMode`** — the mode a single toggle press switches *to*
  (`AiCli ↔ Regular`). Pure, total, used by the toggle reducer and by tests.

### `ShellLifecycle` (NEW, pure) — `src/session.rs`

Runtime state of a session's shell process. Deliberately **not** a copy of `SessionLifecycle`
(research R2) — no crash-loop, no `Failed`, restart is always manual (spec Clarifications,
2026-07-18). Never persisted (mirrors `SessionLifecycle`).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellLifecycle {
    #[default]
    NotStarted,
    Starting,
    Running,
    Exited,
}
```

- **`fn is_active(self) -> bool`** — `Starting | Running` (mirrors `Session::is_active`, used to
  decide whether "start" is a no-op per FR-003/FR-004).

### `Session` (extended) — `src/session.rs`

```rust
pub struct Session {
    pub id: SessionId,
    pub worktree_dir: String,
    pub label: SessionLabel,
    pub lifecycle: SessionLifecycle,     // unchanged in shape; now specifically describes the
                                          // AI CLI process (FR-005/FR-006/FR-014)
    pub mode: TerminalMode,              // NEW, persisted (FR-011)
    pub shell_lifecycle: ShellLifecycle, // NEW, runtime-only (mirrors `lifecycle`'s non-
                                          // persistence, FR-021-style)
}
```

New/changed methods:

- **`Session::start_new`** / **`Session::restored`** — both gain `mode: TerminalMode` defaulting
  to `TerminalMode::AiCli` for `start_new` (a brand-new session always starts attached to the AI
  CLI, matching today's only behavior) and taking the persisted value for `restored`.
  `shell_lifecycle` starts `NotStarted` in both.
- **`fn set_mode(&mut self, mode: TerminalMode)`** (NEW) — sets `self.mode` unconditionally (no
  guard: switching is always allowed per FR-002, regardless of either process's running state).
- **`fn start_shell(&mut self)`** (NEW) — `NotStarted | Exited → Starting`; no-op if already
  `Starting | Running` (mirrors `Session::start`'s idempotency, FR-003/FR-004: "if one is not
  already running").
- **`fn mark_shell_running(&mut self)`** (NEW) — `→ Running` (mirrors `mark_running`).
- **`fn mark_shell_exited(&mut self)`** (NEW) — `→ Exited`, unconditionally, no restart decision
  (FR-013, no `RestartDecision`-shaped return value — this is the type-level difference from
  `on_unexpected_exit` that makes "the shell never auto-restarts" true by construction).

**Validation / invariants**: `mode` and `shell_lifecycle` are independent of `lifecycle` — all
four combinations of (`lifecycle` running-or-not) × (`shell_lifecycle` running-or-not) are valid
and exercised by Story 2 Scenario 2 (AI CLI running while backgrounded, Regular mode displayed).

**FR-015 invariant**: none of `TerminalMode`, `ShellLifecycle`, `set_mode`, `start_shell`,
`mark_shell_running`, or `mark_shell_exited` read or call anything in `src/provider.rs`
(`AiCliProvider`, `ClaudeProvider::transcript_path`/`parse_title`/`read_title`). The shell process
has no transcript, no title, and never touches `SessionLabel`/`session.label` — only
`session.lifecycle` (AI CLI) is ever written by `sync_session_titles`/`read_title`, and this
feature adds no new caller of those functions. This is what makes FR-015 ("Regular Terminal mode
MUST NOT alter the AI CLI session's identity, transcript, or sidebar label") true by construction
rather than by a runtime guard.

### `StoredSession` (extended) — `src/store.rs`

```rust
struct StoredSession {
    id: uuid::Uuid,
    worktree_dir: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    mode: StoredTerminalMode,   // NEW — see contracts/persistence-schema.md
}
```

`StoredTerminalMode` is a serde-mapped mirror of `TerminalMode` (`AiCli` default via
`#[derive(Default)]` + `#[serde(default)]`, so files written before this feature load unchanged
— no `schema_version` bump, per research R5). `StoredCatalog::from_workspace` /
`into_workspace` round-trip it alongside `id`/`worktree_dir`/`title`.

### `default_shell_command` (NEW, pure) — `src/terminal.rs`

```rust
pub fn default_shell_command(shell_env: Option<&str>, comspec_env: Option<&str>) -> String
```

Pure, argument-driven shell-command resolution (research R3); the impure `std::env::var` reads
happen at the call site (`src/main.rs` / `src/ui/terminal.rs`).

### `Message` (extended) — `src/app.rs`

New variants:
- **`TerminalModeToggled`** — the bottom-bar toggle was pressed for the active session. Pure
  reducer: `session.set_mode(session.mode.other())` for `active_session`.
- **`TerminalRestartRequested`** — the restart affordance was pressed for the active session's
  currently-attached, not-running process (research R8). Pure reducer is a no-op (this message
  only triggers binary-side spawn logic — the pure core has no process to mark running yet); the
  gui-side handler follows up with `SessionRunning`/`ShellSessionRunning` once the process is
  actually up, exactly like `SessionStartRequested` already does.
- **`ShellSessionRunning(SessionId)`** — the shell process for `id` is up. Pure reducer:
  `session.mark_shell_running()` (mirrors `SessionRunning`).
- **`ShellSessionExited(SessionId)`** — the shell process for `id` exited (intentional or
  crash). Pure reducer: `session.mark_shell_exited()`.

No existing `Message` variant is removed.

## GUI (`src/ui/`, `src/main.rs`)

### `SessionTerminals` (NEW) — `src/ui/terminal.rs`

```rust
#[derive(Default)]
pub struct SessionTerminals {
    pub ai_cli: Option<RuntimeTerminal>,
    pub shell: Option<RuntimeTerminal>,
}

impl SessionTerminals {
    pub fn attached(&self, mode: TerminalMode) -> Option<&RuntimeTerminal> { .. }
    pub fn attached_mut(&mut self, mode: TerminalMode) -> Option<&mut RuntimeTerminal> { .. }
}
```

Replaces `RuntimeTerminal` as the value type of `App.terminals: HashMap<SessionId,
SessionTerminals>` (`src/main.rs`). Every existing single-process call site (`TerminalTick` pump
loop, `pane()`'s render borrow, `TerminalBytes` write-through, `handle_process_exits`,
`SessionCloseRequested`'s kill) is updated: pump/exit-check iterate **both** slots; render/write
go through `attached(session.mode)` / `attached_mut(session.mode)`.

### `spawn_shell_pty` (NEW) — `src/ui/terminal.rs`

```rust
pub fn spawn_shell_pty(
    cwd: &Path,
    env: &[(String, String)],
    scrollback_lines: usize,
) -> std::io::Result<RuntimeTerminal>
```

Shares the PTY-open + `Term`-construction body with `spawn_pty` via a private helper (research
R4); builds its `CommandBuilder` from `default_shell_command(...)` with no extra arguments (no
`LaunchMode`/`session_id` — those are `claude`-specific concepts that don't apply to a shell).

### `App.terminals` call-site changes — `src/main.rs`

- **`Message::TerminalModeToggled`**: `core.update(TerminalModeToggled)` flips `session.mode`
  (pure); the binary then checks the new mode's slot in `app.terminals[id]` — if empty, spawns
  it (`spawn_pty(..., Resume)` for AI CLI, `spawn_shell_pty(...)` for Regular), inserts into the
  slot, and follows up with `SessionRunning`/`ShellSessionRunning`; `persist(&app.core)` (mode
  is persisted).
- **`Message::TerminalRestartRequested`**: same spawn logic as above, addressed at whichever
  slot the current mode selects, gated on that slot's lifecycle actually being not-running
  (idempotent no-op otherwise).
- **`handle_process_exits`**: extended to scan `st.ai_cli` (unchanged crash-loop branch — still
  calls `session.on_unexpected_exit()` and auto-restarts on `Resume`, regardless of whether AI
  CLI mode is currently attached, research R6) and `st.shell` (new branch: on exit, remove the
  slot and `core.update(ShellSessionExited(id))` — **no** restart decision, no auto-respawn).

**Structural invariant** carried by this shape: a session can have 0, 1, or 2 live child
processes; `Session.mode` says which one is *displayed*, independent of which are *running* —
exactly the four-combination validation note above.
