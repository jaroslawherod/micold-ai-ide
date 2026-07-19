# Contract: `TerminalMode` + `ShellLifecycle` state machines

Pure, in `src/session.rs`. Governs FR-001–FR-007, FR-013, and the spec Clarifications
(2026-07-18).

## `TerminalMode`

| State     | Meaning                                              |
|-----------|-------------------------------------------------------|
| `AiCli`   | The session's `claude` process is attached to the pane (default, FR unchanged behavior). |
| `Regular` | The session's shell process is attached to the pane.  |

Transitions: exactly one — `other()` (`AiCli ↔ Regular`) — fired by `Message::TerminalModeToggled`
for the active session. **Always legal**, regardless of either process's running state (FR-002:
the toggle is reachable in either mode; switching to a not-yet-started process's mode is exactly
how that process gets started, FR-003).

`TerminalMode` never determines whether a process is *running* — only which one is *displayed*.
Query `Session.lifecycle` (AI CLI) / `Session.shell_lifecycle` (shell) separately for that.

## `ShellLifecycle`

```text
NotStarted ──start_shell()──▶ Starting ──mark_shell_running()──▶ Running
     ▲                                                              │
     │                                                              │
     └───────────────────── mark_shell_exited() ◀────────────────── ┘
                                    │
                                    ▼
                                 Exited ──start_shell()──▶ Starting  (manual restart, FR-013)
```

- `start_shell()` is idempotent: a no-op from `Starting`/`Running` (mirrors `Session::start`).
- `mark_shell_exited()` is reachable from `Starting` or `Running` and takes **no** attempt
  counter and returns **no** `RestartDecision` — unlike `Session::on_unexpected_exit`, there is
  no automatic follow-up. The caller (gui `handle_process_exits`) does nothing further; the user
  must trigger `Message::TerminalRestartRequested` to call `start_shell()` again.
- `NotStarted` is the only state a session begins in and the only state besides `Exited` from
  which `start_shell()` does real work — both represent "no live shell process."

## Interaction with `SessionLifecycle` (AI CLI, unchanged enum)

`SessionLifecycle`'s existing five states and `on_unexpected_exit`'s crash-loop guard
(`MAX_RESTART_ATTEMPTS = 3`) are **unchanged** and continue to apply to the AI CLI process
regardless of `TerminalMode` — an AI CLI process backgrounded by `Regular` mode still
auto-restarts on crash (research R6; spec User Story 2, Scenario 2). This asymmetry (AI CLI
auto-restarts, shell never does) is intentional per the 2026-07-18 clarification and is what R2
explains `ShellLifecycle` is a distinct, smaller enum rather than a `SessionLifecycle` reuse.

## Restart affordance (FR-013)

A process is "restartable" (the bottom-bar restart control is shown, research R8) exactly when
the **currently attached** process (per `Session.mode`) is not running:

```text
attached_mode == AiCli   → restartable ⟺ lifecycle ∈ { Idle, Failed }
attached_mode == Regular → restartable ⟺ shell_lifecycle ∈ { NotStarted, Exited }
```

`Message::TerminalRestartRequested` is only meaningful when this predicate holds; sending it
otherwise is a no-op (both `start()` and `start_shell()` are idempotent no-ops when already
active).
