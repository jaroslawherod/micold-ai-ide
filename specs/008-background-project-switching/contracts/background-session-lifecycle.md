# Behavior Contract: Background Sessions Across a Project Switch

Defines the guaranteed behavior of terminal sessions when the active project changes. This is a behavioral contract over the core `State`/`Workspace` transitions and the gui poll loop; it is what the core unit tests and gui-gated tests assert.

## Invariants

- **BS-1 (no teardown on switch)**: Switching the active project MUST NOT kill any PTY or change any session's `lifecycle`. A session that was `Running` before a switch is still `Running` after (FR-001, FR-002). Process kill happens only on explicit per-session close (`Message::SessionClosed`) or app exit (`impl Drop for App`).
- **BS-2 (background streaming continues)**: While a project is inactive, each of its running sessions continues to consume PTY output and update its VT grid (the poll + subscription iterate the whole `terminals` map by id). Output produced while inactive is visible on return, up to the existing per-session scrollback cap (FR-012, SC-003).
- **BS-3 (foreground restore)**: Returning to a project sets `active_session` to the session that was in the foreground when the user left it, if it still exists and is running; otherwise the project's first running session; otherwise `None`. Other running sessions remain background (FR-003).
- **BS-4 (isolation)**: Concurrent background sessions across projects remain isolated — each is bound to its own worktree cwd and routed by its own `SessionId`; no filesystem/in-memory/config state leaks between them (FR-010, Constitution II).
- **BS-5 (no cap)**: Any number of projects may hold running background sessions simultaneously; the system imposes no limit beyond available resources (FR-013).

## Crash handling while inactive (FR-011, SC-007)

- **BS-6 (project-aware restart)**: When a background session of an **inactive** project exits unexpectedly, the poll loop MUST resolve it via the across-all-projects lookup (`Workspace::find_session_mut`) and apply the same crash-loop guard used for the active project (`Session::on_unexpected_exit`, `MAX_RESTART_ATTEMPTS = 3`). It MUST NOT be silently dropped.
- **BS-7 (notify on return)**: If such a session is auto-restarted while its project is inactive, its id is recorded in `restarted_while_inactive`. On the next switch **to** that project, a return notice is shown and the ids are cleared. State MUST NOT change silently.
- **BS-8 (exhausted restarts)**: If restarts are exhausted, the session becomes `Failed` (existing behavior) and is shown as `Failed` in the sidebar on return, not removed.
- **BS-9 (intentional stop is not a crash)**: A session in `Idle` (explicitly stopped) that exits is not auto-restarted (existing guard preserved).

## Switch acceptance (FR-008)

- **BS-10 (reject unavailable)**: Switching to a project whose folder is unavailable/missing MUST leave the current active project and all sessions unchanged (`Workspace::activate` returns `false`). Background sessions of still-available projects are unaffected.

## App restart boundary (2026-07-17 clarification)

- **BS-11**: Live processes do not survive an app restart. After restart, sessions are restored `Idle` and resumed lazily via `claude --resume` on selection (existing behavior). "Background" guarantees (BS-1..BS-8) apply only within a single app run. No new persistence is introduced.

## Test hooks

- Core (`cargo test --no-default-features`): BS-1, BS-3, BS-5, BS-6 (guard decision), BS-7 (marker set/clear), BS-10 asserted against `State::switch_active` / `Workspace::find_session*` with fake sessions — no processes.
- GUI-gated: BS-1 (terminals map retained across a switch), BS-2 (bytes pumped for an inactive project's session), BS-6 end-to-end (a killed background PTY is respawned by the poll), switcher indicators reflect `running_session_count`.
