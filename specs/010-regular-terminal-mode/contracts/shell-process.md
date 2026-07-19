# Contract: shell process — command resolution + spawn

Governs FR-003, FR-004, spec Assumptions ("platform's standard interactive default shell") and
the Edge Case "the platform's default shell cannot be determined or fails to launch."

## Command resolution

```rust
// src/terminal.rs (pure core)
pub fn default_shell_command(shell_env: Option<&str>, comspec_env: Option<&str>) -> String
```

| Platform | Source                          | Fallback   |
|----------|----------------------------------|------------|
| Unix (Linux, macOS) | `$SHELL` (passed as `shell_env`) | `/bin/sh`  |
| Windows  | `%COMSPEC%` (passed as `comspec_env`) | `cmd.exe`  |

An empty-string env value is treated the same as absent (falls back), matching
`ClaudeProvider::config_dir`'s existing `!dir.is_empty()` guard on `CLAUDE_CONFIG_DIR`.

No shell arguments are added (no `-l`/`-i` login/interactive flags) — the shell is invoked the
same way `portable-pty` invokes any command: attached to a PTY, which is what makes most shells
behave interactively without needing an explicit `-i`. If a chosen shell requires an explicit
flag to behave as expected under a PTY, that is a task-level detail to verify manually against
the quickstart, not a spec-level branch.

## Spawn

```rust
// src/ui/terminal.rs
pub fn spawn_shell_pty(
    cwd: &Path,               // the session's worktree directory — same cwd source as the
                               // AI CLI launch (`launch_spec`'s `cwd` in main.rs)
    env: &[(String, String)], // same TERM=xterm-256color convention as the AI CLI launch
    scrollback_lines: usize,  // same Settings-configured value (feature 006) as the AI CLI Term
) -> std::io::Result<RuntimeTerminal>
```

Shares PTY-open + `Term::new(Config { scrolling_history: scrollback_lines, .. })` +
reader-thread-spawn with `spawn_pty` via a private helper (research R4) — same PTY size
(`INIT_ROWS`/`INIT_COLS`), same resize path, same `TerminalHandle`-shaped `kill`/`write_input`
surface. The only difference from `spawn_pty` is the `CommandBuilder` source: no `LaunchMode`,
no `session_id`, no `claude_args`.

## Failure

`spawn_shell_pty`'s `io::Result::Err` (unresolvable/unlaunchable shell) is surfaced the same way
`spawn_pty`'s launch failure already is today (`SessionStartRequested`'s `Err(err) =>
app.core.worktree_error = Some(format!("Could not start session: {err}"))`) — a user-visible
error, not a silent no-op or a panic. `Session.shell_lifecycle` stays whatever it was before the
attempt (typically `NotStarted`/`Exited`), so the restart affordance (contracts/
terminal-mode-lifecycle.md) remains available to retry.
