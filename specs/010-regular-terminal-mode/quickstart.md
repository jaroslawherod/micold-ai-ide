# Quickstart: Validating Switchable Regular Terminal Mode

Runnable validation for feature 010. Proves a session's terminal can switch between the `claude`
CLI and a plain shell without losing the AI CLI conversation or the shell's state, that both
processes survive being backgrounded, and that the active mode is always visible. Maps each step
to Success Criteria (SC-00x).

## Prerequisites

- `claude` CLI installed and on `PATH`.
- A git repository open as a project, with a worktree + session running (see feature 005/006
  quickstarts to create one).
- Toolchains: `cargo` (stable). GUI run needs the `gui` feature.

## Automated checks (fast, no GUI)

```bash
# Pure logic: TerminalMode/ShellLifecycle transitions, shell command resolution,
# StoredSession.mode serde default/roundtrip — must pass.
cargo test --no-default-features

# Full suite incl. gui-gated SessionTerminals attach/detach + toggle button wiring tests.
cargo test --features gui
```

Expected: `session_terminal_mode` tests pass (mode defaults to `AiCli`, `other()` toggles,
`start_shell`/`mark_shell_running`/`mark_shell_exited` transitions, no restart-decision return
value on exit); `store_terminal_mode` roundtrip/default tests pass (an old catalog file with no
`mode` key loads as `AiCli`); `shell_command` tests pass (env value used when present/non-empty,
platform fallback otherwise).

## Manual end-to-end (GUI)

```bash
cargo run --features gui
```

Open the project, expand a worktree, and start (or select) a session so its `claude` terminal
shows.

### 1. Switch to Regular Terminal and run a command — SC-001, SC-004, FR-008 (US1)

- Press the mode toggle in the bottom status bar. **Expect**: the icon changes immediately
  (<500ms, no relaunch flicker) and the pane now shows a plain shell prompt.
- Run `pwd`. **Expect**: it prints the session's worktree directory.
- Run `cd ..` then `pwd` again, run a couple of ordinary commands (`git status`, `ls --color=always`).
- **Expect** (FR-008 — real-terminal behavior identical to AI CLI mode, per feature 006): ANSI
  colors render, keystrokes stream live with no line buffering, scrollback works, text
  selection/copy/paste work, and the focus-release chord (Ctrl+Shift+E / Cmd+Shift+E) still
  releases focus — all exactly as they do in AI CLI mode.
- Press the toggle again (back to AI CLI), then press it once more (back to Regular). **Expect**:
  the shell prompt still shows the `cd ..`-adjusted directory and scrollback from before — the
  shell process was never killed by switching (SC-004).

### 2. The `claude` conversation survives round-trips — SC-002, FR-015 (US2)

- Before switching anything, note the session's sidebar label and (optionally) the path of its
  `claude` transcript file (`~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`).
- While in AI CLI mode, send `claude` a message and let it start responding.
- Immediately press the toggle to switch to Regular Terminal mode while `claude` is still
  generating.
- Wait a few seconds, then toggle back to AI CLI mode. **Expect**: the response that was
  generating is now fully visible, with no new/duplicate session and no restarted conversation.
- Repeat the switch a few more times. **Expect**: full conversation history is intact every time
  (SC-002).
- **Expect** (FR-015): the sidebar label is unchanged (unless `claude` itself renamed it via a
  new message, same as it always could), and the transcript file path/session id from before the
  round-trips is still the one being written to — nothing about the shell process created a
  second transcript or session id.

### 3. Mode is always visible — SC-003 (US3)

- With the terminal in each mode, glance at the bottom bar without typing anything. **Expect**:
  the toggle button's icon/tooltip alone tells you unambiguously which process is attached.

### 4. Shell exit and manual restart — Edge Cases, FR-013

- In Regular Terminal mode, type `exit`. **Expect**: the pane shows a not-running state and a
  restart control appears in the bottom bar; the pane does NOT switch back to AI CLI mode on its
  own.
- Press the restart control. **Expect**: a fresh shell starts in the worktree directory.
- Confirm no automatic retry happened on its own before you pressed restart (contrast with AI CLI
  crash-loop behavior, which does retry automatically) — this is the intended asymmetry from the
  2026-07-18 clarification.

### 5. Per-session independence — SC-005

- Open a second session (same or different worktree). Toggle its mode independently. **Expect**:
  the first session's mode, processes, and indicator are completely unaffected.

### 6. Mode persists across a restart — FR-011

- Leave a session in Regular Terminal mode, quit the app, relaunch it, and reopen that session.
  **Expect**: its terminal reopens already in Regular Terminal mode (a fresh shell process, per
  spec Assumptions — OS processes cannot survive a restart, so it's a new process, but the
  *mode* the pane shows is the one you left it in).
