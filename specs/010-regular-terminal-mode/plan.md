# Implementation Plan: Switchable Regular Terminal Mode

**Branch**: `010-regular-terminal-mode` | **Date**: 2026-07-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/010-regular-terminal-mode/spec.md`

## Summary

Let a session's embedded terminal carry **two** independent background processes instead of
one: the existing `claude` process, and a new plain-shell process scoped to the same worktree.
A single icon-button toggle in the terminal's existing bottom status bar switches which of the
two is *attached* to the visible pane (rendered + receiving keystrokes); the other keeps
running untouched. Neither process is ever killed as a side effect of switching (spec
Assumptions) — this is what makes the `claude` conversation "survive round-trips" and lets
shell state (cwd, scrollback) persist.

**Technical approach**: Extend `App.terminals` (currently `HashMap<SessionId, RuntimeTerminal>`,
`src/main.rs`) to `HashMap<SessionId, SessionTerminals>` where `SessionTerminals` holds
`ai_cli: Option<RuntimeTerminal>` and `shell: Option<RuntimeTerminal>`. A new persisted
`TerminalMode` field on the pure `Session` (`src/session.rs`) records which one is attached; a
new, deliberately simpler `ShellLifecycle` (no crash-loop) tracks the shell process's runtime
state, mirroring `SessionLifecycle` only where behavior actually matches (per spec
clarification: the shell never auto-restarts). The existing `TerminalTick` poll loop, the
`RuntimeTerminal`/`spawn_pty` machinery, `TerminalPane`, and `handle_process_exits` are all
extended in place rather than replaced — this feature adds a second process slot to
infrastructure feature 006 already built, it does not redesign that infrastructure.

## Technical Context

**Language/Version**: Rust, edition 2021, no MSRV change. No new crate: the shell process reuses
the same `portable-pty` + `alacritty_terminal::Term` stack `RuntimeTerminal` already wraps.

**Primary Dependencies**: Reused only — `portable-pty 0.9`, `alacritty_terminal =0.25`, `iced
0.13`, `serde`/`serde_json`. No new runtime dependency.

**Storage**: Local-first (Principle IV). Extend the existing `StoredSession` record
(`src/store.rs`) with a persisted `mode` field (serde-defaulted to `AiCli` for backward
compatibility, no `schema_version` bump needed — same pattern feature 008 used for
`worktree_display_names`). Shell process state itself is never persisted (mirrors
`SessionLifecycle`, feature 005 FR-021) — only *which mode a session was last in* is.

**Testing**: `cargo test --no-default-features` covers all new pure logic: `TerminalMode`
transitions, `ShellLifecycle` transitions (no restart-decision branch, unlike
`on_unexpected_exit`), the extended `StoredSession` mode field's serde default/roundtrip, and
`Session::start_new`/`restored` defaulting to `TerminalMode::AiCli`. GUI-gated tests
(`--features gui`) cover `SessionTerminals` attach/detach selection and the new bottom-bar
toggle button's message wiring. Manual end-to-end validation via `quickstart.md`.

**Target Platform**: Desktop — Linux, macOS, Windows (Principle VI, CI on all three). Shell
command resolution is the one genuinely platform-varying piece (`$SHELL` vs `%COMSPEC%`),
isolated behind a single pure function (research R3).

**Project Type**: Desktop application (single Rust project; render-free lib core + gui binary) —
unchanged from every prior feature.

**Performance Goals**: Mode switch completes with no perceptible delay (SC-001, <500ms) when
the target process is already running — this is a pure state flip plus a `TerminalPane` borrow
swap, no I/O on the hot path. Background pumping of the non-attached process reuses the
existing coalesced `TerminalTick` cadence (feature "keep background terminal poll alive at a
coarser cadence") — no new polling loop.

**Constraints**: App functionality stays fully offline/local-first. `TerminalMode` and
`ShellLifecycle` are enums so an invalid combination (e.g. "attached to a process that was
never spawned") is guarded by `Option` at the type level, not a runtime flag. Every session may
now hold up to two live child processes instead of one — sized for "a handful of concurrent
sessions" per feature 006's existing scale assumption; no new resource cap is introduced (spec
Assumptions — the shell process is lazily started only on first switch to Regular mode, FR-003).

**Scale/Scope**: Same as feature 006 — a handful of concurrent background sessions. This
feature doubles the worst-case process count per session that has ever used Regular Terminal
mode (two children instead of one), not the number of sessions.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: `TerminalMode`/`ShellLifecycle` transitions, the shell
  command resolution function, and the `StoredSession.mode` serde default/roundtrip are pure
  and land in the render-free core (`src/session.rs`, `src/store.rs`, a small shell-resolution
  helper) — tested first under `--no-default-features`, exactly like `SessionLifecycle` and
  `on_unexpected_exit` were for feature 005. The gui-side `SessionTerminals` attach/detach and
  the toggle button are thin, gui-gated wiring around already-tested pure decisions.
- [x] **II. Multi-Session Support**: `TerminalMode` and `ShellLifecycle` are per-`Session`
  fields; `SessionTerminals` is keyed per `SessionId` exactly like today's single-process map.
  Switching one session's mode touches only that session's map entry — no shared/global state.
  No new cross-session leakage surface is introduced.
- [x] **III. Worktree Integration**: The shell process's cwd is the session's worktree
  directory, the same `cwd` already computed for the `claude` launch — no new worktree
  resolution path, no manual git steps.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Only the mode enum is persisted, to the
  existing local JSON store. No process state, no shell output, and nothing else leaves the
  device.
- [x] **V. Rust + iced Stack**: Rust + iced only. `TerminalMode`/`ShellLifecycle` are enums
  precisely so an unrepresentable state (e.g. "Regular mode with no shell slot and no way to
  start one") cannot compile — `SessionTerminals` uses `Option<RuntimeTerminal>` per slot rather
  than a boolean + separate handle that could disagree.
- [x] **VI. Cross-Platform Parity**: The only OS-varying behavior — resolving the default shell
  command — is isolated behind one pure function (research R3), covered by CI on all three
  platforms; PTY spawning itself is already `portable-pty`-abstracted and unchanged.
- [x] **VII. Documentation First-Class**: The user guide's terminal section gains the mode
  toggle (what it does, what survives a switch, how to restart an exited shell) in the same
  change; verified by the CI docs build.
- [x] **VIII. Reusable UI Component Foundation**: The toggle reuses the existing
  `IconButton` builder (`src/ui/material/icon_button.rs`) and `Tooltip` builder
  (`src/ui/material/mod.rs`), both already builder-API/`.into()` components — no new one-off
  widget. Two new `Icon` variants are added to the existing shared `Icon` vocabulary
  (`src/icons.rs`) rather than a feature-local icon hack. No forked component.

**Result: PASS — no violations. Complexity Tracking left empty.**

## Project Structure

### Documentation (this feature)

```text
specs/010-regular-terminal-mode/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md         # Phase 1 output
├── quickstart.md         # Phase 1 output
├── contracts/            # Phase 1 output
│   ├── terminal-mode-lifecycle.md   # TerminalMode + ShellLifecycle state machines
│   ├── shell-process.md             # shell command resolution + spawn contract
│   ├── mode-toggle-ui.md            # bottom-bar toggle button: placement, icons, messages
│   └── persistence-schema.md        # StoredSession.mode addition (back-compat)
├── checklists/
│   └── requirements.md   # (from /speckit-specify + /speckit-clarify)
└── tasks.md               # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── session.rs        # extend: Session gains `mode: TerminalMode`, `shell_lifecycle:
│                      #   ShellLifecycle`; new TerminalMode/ShellLifecycle enums + transition
│                      #   methods (start_shell/mark_shell_running/mark_shell_exited/set_mode)
├── store.rs           # extend: StoredSession gains `#[serde(default)] mode: StoredTerminalMode`;
│                      #   StoredCatalog to/from Workspace round-trips it
│                      #   (contracts/persistence-schema.md)
├── icons.rs           # extend: two new Icon variants (AI CLI glyph, plain-terminal glyph) +
│                      #   Icon::ALL + glyph() match arms
├── app.rs             # extend: Message gains TerminalModeToggled, TerminalRestartRequested,
│                      #   ShellSessionRunning(SessionId), ShellSessionExited(SessionId); pure
│                      #   reducers flip Session.mode / ShellLifecycle for the addressed session
├── terminal.rs         # extend (pure core): a small pure shell-command-resolution function
│                      #   (env-value-in, command-out) alongside the existing claude_args() seam
├── main.rs             # extend: App.terminals value type becomes SessionTerminals; spawn/attach
│                      #   logic on TerminalModeToggled/TerminalRestartRequested; handle_process_
│                      #   exits scans both slots (shell branch: mark Exited, never auto-restart)
└── ui/
    ├── terminal.rs     # extend: SessionTerminals struct + attached()/attached_mut() accessor,
    │                   #   spawn_shell_pty() (factors PTY-open+Term-construction out of the
    │                   #   existing spawn_pty so both share it), pane() gains the toggle
    │                   #   IconButton + restart affordance in the bottom bar
    └── material/       # reused as-is: IconButton, Tooltip — no new component

tests/
├── session_terminal_mode.rs   # NEW (pure): TerminalMode/ShellLifecycle transitions, defaults
├── store_terminal_mode.rs     # NEW (pure): StoredSession.mode serde default/roundtrip
├── shell_command.rs           # NEW (pure): shell command resolution per platform input
└── (existing session/terminal/store tests extended where the shape they assert changed)

docs/user-guide/
└── worktrees-and-sessions.md   # extend: the mode toggle, what survives a switch, restarting an
                                 #   exited shell (Principle VII)
```

**Structure Decision**: Preserve the render-free-core + gui-binary layout unchanged. All new
pure logic (`TerminalMode`, `ShellLifecycle`, shell-command resolution, persistence roundtrip)
lands in `src/` core modules already responsible for the analogous `claude`-process concept, so
each new enum sits next to the one it parallels rather than in a new module. The only gui-side
structural change is `SessionTerminals` replacing the bare `RuntimeTerminal` as `App.terminals`'
value type, and one new bottom-bar control built from existing shared components. No new crate,
no new workspace member.

## Complexity Tracking

*No constitution violations — no entries.*

---

## Bugfix notes

**BUG-001 note (2026-08-04)**: this plan predates the daemon. When the mode toggle ran in the
client, it owned the terminal map, so "spawn the shell" and "register the shell" could not come
apart — feature 012's T009 could reasonably treat them as one act. The daemon port split them
across a lock acquisition and made the registration conditional on the session already having a
live entry (`if let Some(live) = inner.sessions.get_mut(&session)`), which silently discarded the
just-spawned PTY for any session whose primary was not running.

Two consequences the plan should carry forward for anything else moved across the client/daemon
seam:

- **A spawn and its registration must not be separable.** If the registration can fail, the spawn
  must be undone or reported; dropping the handle kills the child through `Drop` and looks exactly
  like success from every call site.
- **A fire-and-forget command must not be able to fail meaningfully.** `SessionOpenShell` has no
  reply, so `open_shell` returning `Ok(())` having done nothing was unobservable to the client, the
  logs, and the tests alike. Either the operation is infallible once accepted, or it needs a reply.

**Bugfix**: 2026-08-04 — BUG-001 Updated from bugfix patch
