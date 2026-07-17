# Implementation Plan: Background Project Switching

**Branch**: `008-background-project-switching` | **Date**: 2026-07-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/008-background-project-switching/spec.md`

## Summary

Make switching the active project non-destructive to running terminal sessions, and add a top-bar project switcher next to the existing overflow-menu button.

Today the app keeps exactly one active project and, on every switch (`Message::FolderChosen`, `Message::KnownProjectReopened`), calls `App::stop_active_project_sessions()` which drains the global `terminals: HashMap<SessionId, RuntimeTerminal>` (killing every live `claude` PTY) and marks each session `Idle`. This feature removes that teardown on switch: the previously active project's sessions stay `Running` in the background, keep streaming into their VT emulators (the terminal poll + subscription already iterate the whole `terminals` map, so they need no change), and are re-displayed when the user returns — restoring the session that was in the foreground.

**Technical approach**: the change is concentrated in the render-free core plus three seams in the gui binary.

- **Stop tearing down on switch.** The switch handlers no longer call `stop_active_project_sessions()`; the `terminals` map is retained across switches. Process kill remains only in `impl Drop for App` (app exit) and in the existing per-session close (`Message::SessionClosed`).
- **Per-project foreground memory (core).** `State` gains an in-memory `foreground_by_project: BTreeMap<PathBuf, SessionId>`. Switching away records the outgoing project's `active_session` **before** the active pointer moves; switching in restores it (falling back to the project's first running session, else `None`). This makes FR-003 a pure, unit-tested state transition. Ordering is load-bearing: the `FolderChosen` handler currently calls `Workspace::open_or_activate` (which mutates `active`) first, so it must capture the previous active path and pass the outgoing foreground into `switch_active` explicitly, or the wrong project gets recorded.
- **Project-aware background crash handling (core + gui).** `handle_process_exits()` currently resolves an exited session through `with_session`/`session_cwd`, which look **only** in `workspace.active`; a background session belonging to an *inactive* project would be silently dropped with no restart. The core gains a total "find session (and owning project) by id across all projects" lookup so the crash-loop guard (`Session::on_unexpected_exit`, `MAX_RESTART_ATTEMPTS = 3`) applies to background sessions too, and a `restarted_while_inactive` marker set so the user is **notified on return** (FR-011, per the 2026-07-17 clarification).
- **Top-bar switcher (gui, shared component).** A new reusable, builder-style `ProjectSwitcher` primitive in `src/ui/material/` renders a trigger placed immediately left of the existing `MenuTrigger` in `toolbar::view`, opening a floating panel (reusing the `menu_overlay` machinery) that lists known projects with an active marker, a running-background-session count, and an unavailable badge, plus an "Add project…" row that opens the existing folder browser. It complements — does not replace — the shell body "Known projects" list and the folder-browser modal (2026-07-17 clarification).

## Technical Context

**Language/Version**: Rust, edition 2021, current `rust-version` as pinned in `Cargo.toml`. No new MSRV pressure (no new dependencies).

**Primary Dependencies**: Existing only — `iced 0.13` (with `canvas`/`advanced`/`lazy`/`tokio`), `serde`/`serde_json`, `directories`, `dark-light`, `portable-pty`, `alacritty_terminal`, `uuid`. **No new crates** (Principle V dependency vetting: nothing to vet).

**Storage**: Local-first (Principle IV). No schema change. Sessions already persist per project (`Workspace.sessions: BTreeMap<PathBuf, Vec<Session>>`) as id + `claude` session name + worktree binding. The new foreground memory and restart-notice markers are **in-memory only** (they describe the current run; "background" means within a single app run — 2026-07-17 clarification), so `projects.json` is unchanged.

**Testing**: `cargo test --no-default-features` exercises the pure core (foreground restore on switch, no-teardown-on-switch invariant, cross-project session lookup, running-session counts per project, restart-while-inactive marker set/clear) against fake backends — no real git, no spawned processes, no GUI. GUI-gated tests cover: `terminals` retained across a switch, background crash restart via the poll loop, and switcher rendering/indicators. Headless VT/logic tests preferred over launching the GUI.

**Target Platform**: Desktop — Linux, macOS, Windows (Principle VI, CI on all three). No platform-specific code added.

**Project Type**: Desktop application (single Rust project; render-free lib core + gui binary).

**Performance Goals**: 60 fps UI. Switching displays the newly selected project within 1 s (SC-005) — a switch is now a cheap state re-point (no process kill/spawn) plus re-render. Background sessions of inactive projects continue to be polled at the existing `TERMINAL_POLL` cadence and coalesced to ≤1 redraw/frame; N concurrent background PTYs each keep one bounded channel + reader, exactly as a foreground session does today.

**Constraints**: Fully offline/local-first. Invalid states kept unrepresentable via the type system (Principle V) — cross-project session lookup is total (returns `Option`), and "backgrounded" is *not* a new lifecycle variant (a session is simply `Running` while not the foreground of its project), so `SessionLifecycle` is unchanged.

**Scale/Scope**: A handful of known projects, each with a handful of sessions; no fixed cap on how many projects hold running background sessions (FR-013 / 2026-07-17 clarification) — bounded only by system resources, exactly like concurrent sessions within one project today.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: Every new behavior lands first as a failing pure-core test — foreground restore on switch, the no-teardown invariant, cross-project session/owner lookup, per-project running counts, and the restart-while-inactive marker lifecycle. GUI seams (terminals retained across switch, background-crash restart, switcher indicators) are covered by gui-gated tests. No production path is planned without a preceding test.
- [x] **II. Multi-Session Support**: This feature is a direct extension of the principle — sessions become independently runnable across projects, not just within the active one. Each stays independently addressable by `SessionId`, keeps its own PTY/`Term`/reader (routing keyed by id), and persists/restores as today. Isolation is preserved and explicitly tested: a background session is bound to its own worktree cwd and leaks no filesystem/in-memory/config state into another project's sessions (FR-010).
- [x] **III. Worktree Integration**: Unchanged. Sessions remain worktree-bound; switching projects does not touch worktree lifecycle. The newly active project's worktrees are re-derived from git on switch (existing `discover_worktrees`), and all file/VCS ops stay worktree-aware. No manual git steps introduced.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: No network. All new state is in-memory or reuses the existing local JSON store; the feature works fully offline. Nothing leaves the device.
- [x] **V. Rust + iced Stack**: Rust + iced 0.13 only; no new GUI framework, no new crates. Invalid states stay unrepresentable: the across-projects session lookup is total, "backgrounded" is expressed as *not the project's foreground* rather than a new enum variant, so `SessionLifecycle` need not grow.
- [x] **VI. Cross-Platform Parity**: No OS branching. PTY/terminal behavior is already cross-platform via existing crates; the switcher is pure iced UI. CI builds + tests all three platforms.
- [x] **VII. Documentation First-Class**: User-guide docs updated in the same change — the top-bar switcher, switching without losing running work, background-session indicators, and the "restarted while you were away" notice. Verified in the CI docs build.
- [x] **VIII. Reusable UI Component Foundation**: The switcher is a shared, chainable **builder** primitive (`ProjectSwitcher::new(...).…().into()`) added to `src/ui/material/`, reusing the existing `MenuTrigger`/`menu_overlay` machinery rather than forking a one-off widget; it honors light/dark theming (via `Roles`/`ColorScheme`) and cross-platform parity. Any row-badge need is met by extending the shared menu primitives, not by a feature-local copy.

**Result: PASS — no violations. Complexity Tracking left empty.**

## Project Structure

### Documentation (this feature)

```text
specs/008-background-project-switching/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── project-switcher-ui.md
│   └── background-session-lifecycle.md
├── checklists/
│   └── requirements.md  # (from /speckit-specify + /speckit-clarify)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── app.rs               # State: add foreground_by_project, restarted_while_inactive,
│                        #   notice; switch reducer restores/records foreground; running
│                        #   counts + notice helpers. (core, tested with fakes)
├── workspace.rs         # add find_session across all projects (id -> (&Path, &Session))
│                        #   + mutable variant; per-project running-session count. (core)
├── session.rs           # unchanged lifecycle; stop_for_project_change no longer invoked
│                        #   on a mere switch (only on explicit close / app exit).
├── project.rs           # unchanged (Project identity/availability reused by switcher).
├── main.rs              # gui seams: drop stop_active_project_sessions() from the switch
│                        #   handlers; retain terminals across switch; make
│                        #   handle_process_exits project-aware (restart bg crashes + mark
│                        #   restarted_while_inactive); restore foreground terminal on switch.
└── ui/
    ├── toolbar.rs       # place the ProjectSwitcher trigger left of the MenuTrigger; take State.
    ├── mod.rs           # float the switcher panel (like menu_overlay) in view().
    ├── shell.rs         # keep the "Known projects" list (complement); render the return notice.
    └── material/
        ├── project_switcher.rs   # NEW shared builder primitive (trigger + panel rows).
        └── menu.rs / menu_overlay.rs  # extend MenuItem/overlay for row badges if needed (shared).

tests/                   # core tests (no-default-features) + gui-gated tests as above
docs/                    # user-guide update (Principle VII)
```

**Structure Decision**: Single Rust project, unchanged. New logic is added to the existing render-free core modules (`app.rs`, `workspace.rs`) so it is unit-testable without the GUI, and the gui binary (`main.rs`, `src/ui/**`) is edited only at the identified seams. The one new file is the shared `ProjectSwitcher` UI primitive under `src/ui/material/`, consistent with Principle VIII and the existing component layout.

## Complexity Tracking

> No constitutional violations. Section intentionally empty.
