# Contract: `TerminalBackend` I/O Boundary

**Feature**: 005-worktree-session-terminal | trait in `src/terminal.rs`; real impl gui-gated in
`src/ui/terminal.rs`.

Isolates PTY / process spawning from the pure core (research R1, R4, R8) so session lifecycle
is unit-testable with no spawned processes. The pure core drives a `SessionLifecycle` state
machine and consumes `PtyOutput`/`PtyExited` messages it is *given*; the backend performs the
effects.

## Trait

```rust
/// One handle per running session. `Send` so its reader lives on a worker thread (R4).
pub trait TerminalBackend: Send {
    /// Spawn `claude` for a session: cwd = worktree path, args include `--session-id <uuid>`
    /// for a fresh session or `--resume <uuid>` when resuming (R6). Returns a live handle.
    fn spawn(&self, spec: LaunchSpec) -> io::Result<Box<dyn TerminalHandle>>;
}

pub trait TerminalHandle: Send {
    fn write_input(&mut self, bytes: &[u8]) -> io::Result<()>;  // keystrokes → PTY writer
    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()>;
    fn kill(&mut self) -> io::Result<()>;                        // terminate + reap child
}

pub struct LaunchSpec {
    pub cwd: PathBuf,          // the worktree directory (R6: cwd scopes the claude session)
    pub session_id: uuid::Uuid,
    pub mode: LaunchMode,      // Fresh | Resume
    pub env: Vec<(String, String)>,   // e.g. TERM=xterm-256color
}
pub enum LaunchMode { Fresh, Resume }
```

## Runtime effects (binary side, gui-gated)

- **Spawn** (`portable-pty` 0.9): `native_pty_system().openpty(size)`, `CommandBuilder::new("claude")`
  with `cwd`, env, and the `--session-id`/`--resume` args; `slave.spawn_command`; drop the slave.
- **Read loop**: dedicated thread reads 8 KiB chunks from `master.try_clone_reader()` →
  `tokio::mpsc` → `Subscription::run_with_id(session_id, …)` yields `Message::PtyOutput{id,chunk}`;
  EOF → `Message::PtyExited{id, status}` (R4). One subscription per session via `batch` (R5).
- **Grid**: chunks feed a per-session `alacritty_terminal::Term`; only the active session's grid
  is rendered (`iced_term` 0.6 widget or a `canvas` renderer), redraws coalesced ≤1/frame (R3).
- **Input/resize**: `update` writes keystrokes to the handle; pane resize → `resize()` + `Term::resize()`.
- **Close/shutdown**: `kill()` + reap on session close (FR-015a) and for all sessions on app
  exit (avoid zombies, R5).

## Message surface consumed by the pure core

```rust
Message::PtyOutput  { id: SessionId, chunk: Vec<u8> }   // → feed Term, mark dirty
Message::PtyExited  { id: SessionId, status: ExitStatusKind }  // → lifecycle transition
Message::SessionStartRequested  { worktree_dir: String }
Message::SessionSelected        { id: SessionId }        // switch visible terminal (FR-015)
Message::SessionCloseRequested  { id: SessionId }        // FR-015a
```

`update` maps `PtyExited` to `SessionLifecycle`: unexpected exit → `Restarting{attempts+1}`
(auto `--resume`, FR-022) → `Failed` past the guard (FR-022a); intentional stop (close /
project switch) → `Idle` with no auto-restart (FR-023).

## Fake (tests)

`FakeTerminalBackend`: `spawn` records the `LaunchSpec` (assert cwd + `--session-id`/`--resume`)
and returns a handle capturing `write_input`/`resize`/`kill`. Pure-core tests inject `PtyOutput`
(assert it is routed to the correct per-session sink — routing/isolation only, no `Term`) and
`PtyExited` (assert lifecycle: restart, guard→Failed, idle on close). No real process,
deterministic; runs under `--no-default-features`. VT `Term` grid rendering is gui-side and
covered by a separate gui-gated test.

## Constitution mapping

Principle II (isolated concurrent sessions), I (trait + fake → TDD, no processes in tests),
V (`SessionLifecycle` enum), VI (portable-pty cross-platform).
