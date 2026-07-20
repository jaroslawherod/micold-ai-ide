# Implementation Plan: Multiple Regular Terminal Instances per Session

**Branch**: `011-multiple-regular-terminals` (feature branch `feat/allow-multiple-regular-terminals`) | **Date**: 2026-07-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/011-multiple-regular-terminals/spec.md`

## Summary

Feature 010 gave each session two independent background processes — `claude` (AI CLI) and
**one** plain shell — with `SessionTerminals { ai_cli: Option<RuntimeTerminal>, shell:
Option<RuntimeTerminal> }`. This feature removes the "one shell" ceiling: a session's Regular
Terminal side becomes a small, ordered collection of independent shell instances plus a record
of which one is currently active, mirroring the existing `Vec<Session>` + `active_session:
Option<SessionId>` pattern already used at the project level for switching between sessions
(`src/app.rs`). The AI CLI side is untouched — still exactly one process, one `TerminalMode`
enum, one primary toggle button.

**Technical approach**: `Session.shell_lifecycle: ShellLifecycle` (`src/session.rs`) becomes
`Session.shells: Vec<ShellInstance>` + `Session.active_shell: Option<ShellInstanceId>`, mutated
only through three new pure methods (`open_shell_instance`, `close_shell`, `select_shell`) that
keep the "`active_shell` always points at an element of `shells`, or is `None` iff `shells` is
empty" invariant true by construction. `SessionTerminals.shell: Option<RuntimeTerminal>`
(`src/ui/terminal.rs`) becomes `shells: HashMap<ShellInstanceId, RuntimeTerminal>`. A new bottom
status-bar control (built from the same `IconButton`/`Tooltip` primitives already used for the
mode toggle and restart button in that exact bar) adds an always-visible "open a new instance"
affordance in Regular mode, plus a compact numbered switcher row shown only once
`shells.len() > 1` (FR-005). A new `Ctrl+Shift+T`/`Cmd+Shift+T` chord in `src/keymap.rs` mirrors
the existing `is_release_chord` platform-split pattern. No persistence schema change: exactly as
today, only the `TerminalMode` enum is persisted (`StoredSession.mode`) — the instance
list/count is never written to disk (FR-017), matching the non-goal explicitly.

## Technical Context

**Language/Version**: Rust, edition 2021, no MSRV change. No new crate — reuses the same
`portable-pty` + `alacritty_terminal::Term` stack `RuntimeTerminal`/`spawn_shell_pty` already
wrap (feature 010).

**Primary Dependencies**: Reused only — `portable-pty 0.9`, `alacritty_terminal =0.25`, `iced
0.13`, `serde`/`serde_json`. No new runtime dependency.

**Storage**: Local-first (Principle IV). **No `StoredSession` schema change.** Only
`StoredSession.mode` (`StoredTerminalMode`, feature 010) is persisted; the set/count of open
Regular Terminal instances is intentionally never written (FR-017, spec non-goal) — reopening a
session in Regular mode always starts with exactly one freshly-spawned instance, the same
`ensure_attached_process`-style spawn-if-absent path feature 010 already uses.

**Testing**: `cargo test --no-default-features` covers all new pure logic: `ShellInstanceId`
allocation (monotonic, never reused), `Session::open_shell_instance` / `close_shell` /
`select_shell` transitions (including the FR-012 "next-in-list, else previous" fallback and the
FR-013 "last instance closed → mode reverts to `AiCli`" edge), and the new
`is_new_terminal_chord` platform-split keymap predicate. GUI-gated tests (`--features gui`)
cover `SessionTerminals`'s `HashMap<ShellInstanceId, RuntimeTerminal>` slot (spawn/attach/kill
per id), the bottom-bar "open new instance" + switcher row's visibility/wiring, and the new
`KeyOutput::NewTerminalInstance` match arm in `TerminalPane`. Manual end-to-end validation via
`quickstart.md`.

**Target Platform**: Desktop — Linux, macOS, Windows (Principle VI, CI on all three). The one
platform-varying addition, the `Ctrl+Shift+T` (macOS: `Cmd+Shift+T`) chord, is isolated behind a
single pure predicate in `src/keymap.rs`, `cfg(target_os = "macos")`-split exactly like the
existing `is_release_chord`.

**Project Type**: Desktop application (single Rust project; render-free lib core + gui binary) —
unchanged from every prior feature.

**Performance Goals**: Opening a new instance or switching among instances completes with no
perceptible delay (SC-001/SC-002, <500ms) — same class of operation as feature 010's mode
switch (a `HashMap` insert/lookup plus a `TerminalPane` borrow swap), no new I/O on the hot path.
Background instances are pumped every `TerminalTick` exactly like today's single backgrounded
shell (feature 010 research R6), just iterating one more collection.

**Constraints**: App functionality stays fully offline/local-first. `ShellInstanceId` is a
per-session monotonic counter (never reused, matching the spec's "closed instances' positions
are not reused" assumption) so a stale id cannot silently alias a different, later instance.
`active_shell: Option<ShellInstanceId>` is mutated only by the three controlled `Session`
methods, keeping "points at a live element, or `None`" true by construction (Principle V) rather
than needing a runtime existence check at every read site. No artificial cap on concurrent
instances per session (spec Assumptions) — bounded only by host resources, same posture feature
006/010 already took for concurrent sessions.

**Scale/Scope**: A handful of concurrent shell instances per session in practice (same
"handful of concurrent background processes" scale assumption feature 006/010 made) — this
feature turns the existing "0 or 1 shell children" per session into "0 to N," not a change to
the number of concurrently open sessions or AI CLI processes.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: `ShellInstanceId`/`ShellInstance`, the three
  `Session` mutators, and `is_new_terminal_chord` are pure and land in the render-free core
  (`src/session.rs`, `src/keymap.rs`) — written and reviewed as failing tests first, exactly like
  `TerminalMode`/`ShellLifecycle` were for feature 010. The gui-side `SessionTerminals` multi-slot
  map and the new bottom-bar controls are thin, gui-gated wiring around already-tested pure
  decisions.
- [x] **II. Multi-Session Support**: `shells`/`active_shell` are per-`Session` fields; the gui
  `HashMap<ShellInstanceId, RuntimeTerminal>` is keyed per `SessionId` exactly like today's
  single-shell slot. Opening/switching/closing one session's instances touches only that
  session's map entry — no shared/global state, no new cross-session leakage surface (FR-015).
- [x] **III. Worktree Integration**: Every new instance's cwd is the same
  `session_cwd_for_location` resolution already used for the session's first shell and its AI
  CLI process — no new worktree/location resolution path.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: No persistence schema change at all (see
  Technical Context/Storage) — strictly less new persisted state than a naive design might add,
  fully honoring the FR-017 non-goal.
- [x] **V. Rust + iced Stack**: `ShellInstanceId` (a per-session monotonic newtype) and the
  invariant that `active_shell` only ever names a live element of `shells` are enforced by
  routing every mutation through three total, controlled methods — an "active id with no
  matching instance" state is never constructed, not just avoided by convention.
- [x] **VI. Cross-Platform Parity**: The only OS-varying addition — the new-instance chord — is
  isolated behind one pure predicate (`is_new_terminal_chord`), covered by CI on all three
  platforms; PTY spawning itself is unchanged, already `portable-pty`-abstracted.
- [x] **VII. Documentation First-Class**: The user guide's terminal section gains multiple
  Regular Terminal instances (opening, switching, closing, the keyboard shortcut) in the same
  change; verified by the CI docs build.
- [x] **VIII. Reusable UI Component Foundation**: The new "open instance" button and switcher row
  are composed from the already-shared, builder-API `IconButton`/`Tooltip` primitives
  (`src/ui/material/`) — the same composition style the mode-toggle/restart/release-focus
  controls in this exact bottom bar already use (`src/ui/terminal.rs::pane`), not a new one-off
  widget. One new `Icon` variant (`AddTerminalInstance`) is added to the existing shared `Icon`
  vocabulary (`src/icons.rs`) rather than a feature-local icon hack. `TreeView`/`TreeItem`
  (`src/ui/material/tree_view.rs`) was considered for the switcher row and rejected — it is
  purpose-built for the sidebar's indented, expandable hierarchy, and forcing an unrelated visual
  shape (a slim horizontal bottom-bar row) onto it would be the misuse Principle VIII's reuse
  posture warns against in the other direction; the compact row instead reuses the same atomic
  primitives already used for the bar's other controls (see research.md R2).

**Result: PASS — no violations. Complexity Tracking left empty.**

## Project Structure

### Documentation (this feature)

```text
specs/011-multiple-regular-terminals/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md         # Phase 1 output
├── quickstart.md         # Phase 1 output
├── contracts/            # Phase 1 output
│   ├── shell-instance-lifecycle.md      # ShellInstanceId/ShellInstance + Session mutators
│   ├── terminal-instance-switcher-ui.md # bottom-bar "+" affordance + switcher row
│   └── keyboard-shortcut.md             # Ctrl+Shift+T / Cmd+Shift+T chord contract
├── checklists/
│   └── requirements.md   # (from /speckit-specify + /speckit-clarify)
└── tasks.md               # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── session.rs        # extend: Session.shell_lifecycle: ShellLifecycle replaced by
│                      #   shells: Vec<ShellInstance>, active_shell: Option<ShellInstanceId>;
│                      #   new ShellInstanceId/ShellInstance types; new
│                      #   open_shell_instance/close_shell/select_shell methods
│                      #   (contracts/shell-instance-lifecycle.md)
├── keymap.rs          # extend: KeyOutput gains NewTerminalInstance; is_new_terminal_chord()
│                      #   (Ctrl+Shift+T / Cmd+Shift+T, contracts/keyboard-shortcut.md)
├── icons.rs           # extend: one new Icon variant (AddTerminalInstance) + Icon::ALL +
│                      #   glyph() arm
├── app.rs             # extend: Message gains ShellInstanceOpenRequested,
│                      #   ShellInstanceSelected(ShellInstanceId),
│                      #   ShellInstanceCloseRequested(ShellInstanceId),
│                      #   ShellInstanceRestartRequested(ShellInstanceId),
│                      #   ShellInstanceRunning(SessionId, ShellInstanceId),
│                      #   ShellInstanceExited(SessionId, ShellInstanceId); replaces the feature
│                      #   010 ShellSessionRunning(SessionId)/ShellSessionExited(SessionId)/
│                      #   implicit-single-slot TerminalRestartRequested handling for the shell
│                      #   side; pure reducers call the new Session methods for the addressed
│                      #   session
├── main.rs            # extend: spawn-on-open logic for a new shell instance (mirrors today's
│                      #   ensure_attached_process's Regular branch, now id-addressed);
│                      #   handle_process_exits scans every entry of the shell HashMap instead
│                      #   of a single Option slot
└── ui/
    ├── terminal.rs    # extend: SessionTerminals.shell: Option<RuntimeTerminal> becomes
    │                  #   shells: HashMap<ShellInstanceId, RuntimeTerminal>; attached()/
    │                  #   attached_mut() take the session's active_shell id for the Regular
    │                  #   arm; each_mut()/kill_all() iterate every shell entry; pane() gains
    │                  #   the "open instance" IconButton (visible whenever mode == Regular) and
    │                  #   the switcher row (visible whenever shells.len() > 1)
    │                  #   (contracts/terminal-instance-switcher-ui.md)
    └── material/
        └── terminal_pane.rs  # extend: one new KeyOutput::NewTerminalInstance match arm,
                               #   publishing Message::ShellInstanceOpenRequested

tests/
├── session_shell_instances.rs   # NEW (pure): open_shell_instance/close_shell/select_shell
│                                 #   transitions, id monotonicity, FR-012/FR-013 fallback rules
├── keymap.rs                    # extend (pure, existing file): is_new_terminal_chord platform
│                                 #   split, precedence over plain-`t`/`T` typing
└── (session_terminal_mode.rs, pty_routing.rs, store_terminal_mode.rs extended where the shape
    they assert changed — e.g. pty_routing.rs's single-shell-slot assertions become per-id)

docs/user-guide/
└── worktrees-and-sessions.md   # extend: opening/switching/closing multiple Regular Terminal
                                 #   instances, the Ctrl+Shift+T/Cmd+Shift+T shortcut,
                                 #   independent per-instance restart (Principle VII)
```

**Structure Decision**: Preserve the render-free-core + gui-binary layout unchanged. All new
pure logic (`ShellInstanceId`, `ShellInstance`, the three `Session` mutators, the new keymap
chord) lands in the same core modules that already own the single-shell-instance concept it
replaces, so nothing moves to a new module. The only gui-side structural change is
`SessionTerminals.shells` becoming a small `HashMap` keyed by `ShellInstanceId` instead of a bare
`Option`, and one new bottom-bar control composed from already-shared primitives. No new crate,
no new workspace member, no new shared UI component (research.md R2 records why `TreeView` was
considered and rejected for the switcher row).

## Complexity Tracking

*No constitution violations — no entries.*
