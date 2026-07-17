# Phase 0 Research: Background Project Switching

All spec clarifications were resolved in `/speckit-clarify` (session 2026-07-17). No `NEEDS CLARIFICATION` markers remained in Technical Context. This document records the design decisions that shape Phase 1, each grounded in the existing code.

## R1 — Keep sessions running across a switch: remove teardown, retain the terminals map

**Decision**: On a project switch, do **not** kill PTYs or mark sessions `Idle`. Retain the global `App.terminals: HashMap<SessionId, RuntimeTerminal>` across switches. Delete the `app.stop_active_project_sessions()` calls from the `Message::FolderChosen` (`main.rs:431`) and `Message::KnownProjectReopened` (`main.rs:445`) handlers.

**Rationale**: `stop_active_project_sessions()` (`main.rs:166`) is the *only* thing that ends a project's sessions on switch — it drains `terminals` (killing each `claude`) and calls `Session::stop_for_project_change()` (`session.rs:170`) → `Idle`. Process kill also lives in `impl Drop for App` (`main.rs:181`, app exit) and per-session `Message::SessionClosed` (`main.rs:521`); both stay. Removing only the switch-time teardown is the minimal change that satisfies FR-001/FR-002 without altering shutdown or explicit-close behavior.

**Key finding — the streaming path is already project-agnostic**: `subscription()` (`main.rs:677`) enables the terminal poll whenever `!app.terminals.is_empty()`, and both the poll (`Message::TerminalTick`, `main.rs:650` → `for rt in app.terminals.values_mut() { rt.pump() }`) and exit detection iterate the **whole** map by `SessionId`, not the active project. So once we stop draining the map, background sessions of inactive projects keep pumping bytes into their VT emulators and preserving scrollback with no further wiring (satisfies FR-012 / SC-003 up to the existing scrollback cap).

**Alternatives considered**: (a) Suspend/park background PTYs — rejected: adds a new lifecycle state and buffering machinery for no requirement (FR-013 wants them simply running); the existing bounded channel + capped scrollback already bound cost. (b) Persist live processes across app restart — rejected: out of scope by the 2026-07-17 clarification ("background" is within one run) and infeasible for child processes.

## R2 — Restore the foreground session on return: per-project in-memory memory

**Decision**: Add `foreground_by_project: BTreeMap<PathBuf, SessionId>` to core `State` (in-memory, not persisted). On switching **away**, record `foreground_by_project[outgoing] = active_session`. On switching **in**, set `active_session` to the stored id **iff** that session still exists in the target project and is running; else the target's first running session; else `None`.

**Rationale**: `State.active_session: Option<SessionId>` (`app.rs:304`) is a single foreground pointer and `view()` (`main.rs:664`) renders exactly that session's terminal. Switching currently nulls it (`stop_active_project_sessions` → `active_session = None`). A per-project map makes "restore the prior foreground, others stay background" (FR-003) a pure state transition that is unit-testable with no GUI. In-memory is sufficient because live processes don't survive restart (R1), so there is nothing to restore to after a restart.

**Alternatives considered**: persisting the foreground id in `projects.json` — rejected: on restart every session is restored `Idle` and resumed lazily, so a persisted foreground would point at a non-running session; adds schema surface for no behavior.

## R3 — Project-aware background crash handling (a real gap today)

**Decision**: Make `handle_process_exits()` resolve an exited session across **all** projects. Add a total core lookup on `Workspace` — `find_session(id) -> Option<(&Path, &Session)>` and a `_mut` variant — and derive the session's cwd from its owning project path + `worktree_dir`. Apply the existing crash-loop guard (`Session::on_unexpected_exit`, `MAX_RESTART_ATTEMPTS = 3`, `session.rs:153`) regardless of which project is active.

**Rationale**: `handle_process_exits()` (`main.rs:690`) routes exited sessions through `with_session` (`main.rs:720`) and `session_cwd`, both of which start from `core.workspace.active`. A background session that belongs to an inactive project is therefore **not found** on crash → silently removed from `terminals`, never restarted. That directly violates FR-011. Resolving by id across all projects fixes it and keeps the fix in the testable core.

**Alternatives considered**: iterate every project's session list inline in the gui — rejected: duplicates lookup logic in the binary and is untestable without the GUI; the core lookup is reused by the switcher's running-count as well (R6).

## R4 — Restart-while-inactive notification: mark in core, surface on return

**Decision**: Add `restarted_while_inactive: BTreeSet<SessionId>` to `State`. When `handle_process_exits` restarts a session whose owning project is **not** the active one, insert its id. When the user switches **to** a project, if any of its sessions are in the set, set a transient `notice: Option<String>` (e.g. "A background session was restarted while you were away.") and clear those ids. Render `notice` in the shell using the existing banner pattern (`worktree_error` is rendered similarly in `shell::view`).

**Rationale**: The 2026-07-17 clarification chose "auto-restart **and notify on return**, never change state silently" (FR-011, SC-007). Recording the event in core keeps the trigger unit-testable; reusing the existing banner avoids a new bespoke widget (Principle VIII). A failed session (restarts exhausted) is already visible as `Failed` in the sidebar on return; the notice covers the *restart happened while away* case.

**Alternatives considered**: OS-level notifications — rejected: adds a cross-platform dependency and violates local-first quietness for an in-app event; an in-window notice is sufficient and testable.

## R5 — Top-bar switcher: reuse the menu-trigger/overlay machinery as a shared builder

**Decision**: Add a shared, chainable builder primitive `ProjectSwitcher` in `src/ui/material/project_switcher.rs`, terminating in `impl From<ProjectSwitcher> for Element` so call sites end in `.into()` (Principle VIII builder rule). Its trigger is added to the top bar via `Toolbar::new(...).action(switcher).action(menu_trigger)` so it sits immediately **left** of the existing `MenuTrigger` ("next to the menu button" — FR-004). Its panel floats via the same overlay path as `menu_overlay` (`ui::mod::view`), so opening it never reflows the bar. Rows show: project display name, an active marker, a running-background-session count badge, and an unavailable badge; a trailing "Add project…" row emits `Message::ProjectSelectorOpened` (the existing folder browser).

**Rationale**: `toolbar::view` (`src/ui/toolbar.rs:39`) already composes the bar from `Toolbar` + `MenuTrigger`, and the menu panel already floats as an overlay — the switcher is the same shape with richer rows, so reuse is the constitutional path (Principle VIII forbids a forked one-off). `toolbar::view` currently takes only `ColorScheme`; it must additionally take the data the switcher needs (known projects, active path, per-project running counts) — a signature change, not a new pattern.

**Row badges**: if `MenuItem`/`menu_overlay` cannot express a trailing count/badge, extend those **shared** primitives (still builder-style) rather than inlining a private row widget — keeping the extension in the shared library.

**Alternatives considered**: a modal dialog for switching — rejected: slower (>2 interactions, fails SC-002) and duplicates the folder-browser modal's role; the lightweight floating panel matches the menu idiom users already know.

## R6 — Running-session indicator source: core lifecycle, not the gui terminals map

**Decision**: Compute each project's "running background sessions" count from core session lifecycle — count sessions in `workspace.sessions[path]` whose lifecycle is running/restarting (`Session::is_active`) — exposed by a `State`/`Workspace` helper. Do **not** read the gui `terminals` map for the indicator.

**Rationale**: With R1, a backgrounded session stays `Running` in core, so core lifecycle is an accurate, GUI-free source for the count (FR-007) and is unit-testable. Keeping the indicator out of the `terminals` map avoids leaking a gui-only structure into the view's data contract and keeps the switcher renderable from `State` alone.

**Alternatives considered**: counting live entries in `terminals` — rejected: only available in the binary, untestable in the core suite, and redundant with lifecycle.

## R7 — Concurrency cost of many background projects (no cap)

**Decision**: Impose no cap (FR-013 / 2026-07-17 clarification). Rely on the existing per-session resource shape: one reader + one bounded channel + one capped-scrollback VT grid per live session, polled on the shared `TERMINAL_POLL` tick.

**Rationale**: A background session of an inactive project costs exactly what a background session of the *active* project already costs today (the app already supports multiple concurrent sessions per project). No requirement asks for throttling; adding one would be unjustified complexity (Principle: added complexity must be justified). If resource pressure ever becomes real, a soft-warning threshold is a future, separately-specified increment.

**Alternatives considered**: hard cap / soft warning — both explicitly rejected by the clarification for this feature.
