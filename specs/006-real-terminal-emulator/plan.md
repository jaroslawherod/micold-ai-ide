# Implementation Plan: Real Terminal Behavior for Embedded Session Terminals

**Branch**: `006-real-terminal-emulator` | **Date**: 2026-07-16 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/006-real-terminal-emulator/spec.md`

## Summary

Turn feature 005's plain-text embedded terminal into a real terminal emulator: render the `alacritty_terminal` grid with full ANSI color and text styling on an iced `canvas`; deliver keystrokes live to the `claude` PTY only while the terminal is focused (encoded as a real terminal would — arrows, function keys, control chords, escape sequences); support mouse reporting, text selection, copy/paste, scrollback, and pane-driven resize; and add a Settings form (opened from the toolbar overflow menu) exposing a persisted, configurable scrollback limit.

**Technical approach**: Keep everything feature 005 built — `RuntimeTerminal` (`portable-pty` + `alacritty_terminal::Term`), the `TerminalBackend` seam, `SessionRouter`, session lifecycle, persistence, and crash-restart. Replace only the *view* and *input* layer. The current text pane becomes a custom iced `advanced::Widget` (`TerminalPane`) that borrows the active session's `Term` for a colored canvas render and, **only when focused**, translates iced key/mouse events into terminal actions applied to that session's PTY. Key→bytes encoding lives in a new **pure** module `src/keymap.rs` (unit-tested under `--no-default-features`), and focus-routing is a pure predicate in core `State`. We do **not** adopt `iced_term`'s `Terminal`/`Backend` (its `BackendSettings` exposes no cwd/env, so it cannot run `claude` in the session's worktree, and it owns the process lifecycle we already implement) — instead we adapt its MIT-licensed `view.rs`/`bindings.rs`/`theme.rs` as the rendering/encoding blueprint and **drop the now-unused `iced_term` dependency**. Scrollback is `Config.scrolling_history` on each session's `Term`, sourced from a new `scrollback_lines` field in the persisted `Settings`. A `Settings` overlay + a toolbar `Settings` menu item reuse the existing form pattern and shared Material components.

## Technical Context

**Language/Version**: Rust, edition 2021, `rust-version` unchanged (no new crate raises the MSRV — this feature *removes* `iced_term` and reuses `alacritty_terminal 0.25` + `portable-pty 0.9` already present).

**Primary Dependencies**:
- Reused: `iced 0.13` (features `canvas`, `advanced`, `lazy`, `tokio` — all already enabled by 005), `alacritty_terminal =0.25` (grid model + VT parser + scrollback + selection + `TermMode`), `portable-pty 0.9` (PTY, already used by `RuntimeTerminal`), `serde`/`serde_json`, `directories`.
- **Removed**: `iced_term =0.6.0` — unused after this feature (005 listed it as "swappable for iced_term" but rendered text directly). Removing it is dependency hygiene under Constitution Principle V. Its `view.rs`/`bindings.rs`/`theme.rs` are retained only as an adaptation reference (MIT; attributed in the adapted modules).
- No new runtime dependency is added. Clipboard access uses iced's built-in `Clipboard` (available in `advanced::Widget::on_event`).

**Storage**: Local-first (Principle IV). Extend the existing `Settings` JSON document with `scrollback_lines: usize` (serde-defaulted for backward compatibility). No new store; reuse `JsonFileSettingsStore`. Live scrollback stays in memory (not persisted), consistent with 005.

**Testing**: `cargo test --no-default-features` exercises the new pure logic without a GUI: `keymap::encode` (every key/modifier/`app_cursor` combination → exact bytes/actions), focus-routing predicate, `Settings` scrollback serde roundtrip + default + validation/clamp. GUI-gated tests cover the widget's event→action mapping (adapting iced_term's `view.rs` unit tests) and the Settings overlay reducer. Manual end-to-end validation via `quickstart.md`.

**Target Platform**: Desktop — Linux, macOS, Windows (Principle VI, CI on all three).

**Project Type**: Desktop application (single Rust project; render-free lib core + gui binary).

**Performance Goals**: 60 fps UI; terminal output coalesced to ≤1 redraw/frame via a per-session `canvas::Cache` invalidated only when new PTY bytes arrive (FR-005a); input/scroll perceived latency ≤~100 ms under sustained output (SC-008); memory bounded by the configured scrollback (default 10 000 lines).

**Constraints**: App functionality stays fully offline/local-first; `claude`'s own network use is external tool behavior. Focus, lifecycle, and terminal-mode states expressed as enums/bools so invalid combinations are unrepresentable (Principle V). Keystrokes reach only the displayed, focused, *Running* session (FR-012, FR-012a).

**Scale/Scope**: A handful of concurrent background sessions, each with one `Term` + PTY + reader thread. One focused terminal at a time.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: New behavior is factored so its logic is pure and tested first: `keymap::encode` (key+modifiers+`app_cursor`+`alt_screen` → bytes/`Copy`/`Paste`/`ReleaseFocus`/`Ignore`) and the focus-routing predicate live in the render-free core and are covered exhaustively under `--no-default-features`; `Settings` scrollback serde/default/clamp is unit-tested. The gui widget is a thin translator (iced event → `keymap` → `TermAction`) with gui-gated tests mirroring iced_term's `view.rs` suite.
- [x] **II. Multi-Session Support**: Focus and input are scoped to the *displayed* session (FR-012); each session keeps its own `Term` + scrollback; switching sessions never routes input to a background session, and background PTYs are untouched. No cross-session leakage.
- [x] **III. Worktree Integration**: Unchanged and reaffirmed — sessions still run `claude` with cwd = the session's worktree. This is precisely why `iced_term`'s cwd-less backend is rejected. No manual git steps introduced.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: The scrollback preference persists in the existing local settings file; live scrollback is in-memory only. Nothing leaves the device.
- [x] **V. Rust + iced Stack**: Rust + iced 0.13 only; the terminal view is a native iced `advanced::Widget`. A dependency is *removed*, not added (`iced_term`); retained crates (`alacritty_terminal`, `portable-pty`) were already vetted in 005. Adapted MIT code is attributed. Enums make focus/lifecycle/term-mode states explicit.
- [x] **VI. Cross-Platform Parity**: Color rendering, key encoding, resize, scroll, and copy/paste are platform-agnostic; platform-varying copy/paste chords (Cmd on macOS vs Ctrl+Shift elsewhere) are isolated in `keymap` behind `cfg`, mirroring iced_term's `platform_keyboard_bindings`. `portable-pty` and iced's clipboard cover all three OSes. CI builds/tests all three.
- [x] **VII. Documentation First-Class**: The user guide gains terminal-usage docs (colors, focus in/out, control keys, copy/paste, mouse, scrollback) and the Settings form, in the same change; verified by the CI docs build.
- [x] **VIII. Reusable UI Component Foundation**: `TerminalPane` is a reusable primitive in the shared `src/ui/material/` library, exposed as a chainable **builder** terminating in `.into()` per the Constitution v1.2.0 builder-API rule (`TerminalPane::new(rt, palette).focused(b).into()`); the Settings form and toolbar `Settings` entry (a builder `MenuItem`/`Toolbar`) reuse the shared components; a shared `Icon::Settings` is added. The pre-existing `material/` components were migrated to the builder form (Phase 3b). No forked one-off widgets.

**Result: PASS — no violations. Complexity Tracking left empty.**

## Project Structure

### Documentation (this feature)

```text
specs/006-real-terminal-emulator/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── terminal-render-input.md   # TerminalPane widget: render + focus-gated event → TermAction
│   ├── key-encoding.md            # keymap::encode contract (pure): key/mods/mode → bytes/action
│   ├── focus-model.md             # focus state + app/terminal key routing + reserved release chord
│   └── settings-schema.md         # scrollback_lines added to the settings document (back-compat)
├── checklists/
│   └── requirements.md  # (from /speckit-specify + /speckit-clarify)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── lib.rs                  # add: pub mod keymap
├── keymap.rs               # NEW (pure): Key/Mods/TermFlags enums + encode() → KeyOutput
│                           #   (Bytes | Copy | Paste | ReleaseFocus | Ignore). Adapted from
│                           #   iced_term bindings.rs; Ctrl+U fixed to \x15. Fully unit-tested.
├── app.rs                  # extend State/Message/update: terminal_focused flag, TermAction
│                           #   routing, Settings overlay + draft; on_escape unchanged for overlays
├── settings.rs             # extend: Settings.scrollback_lines + StoredSettings (serde default),
│                           #   clamp/validate; bump SETTINGS_VERSION to 2 (doc only)
├── terminal.rs             # extend: TerminalBackend/handle stays; add resize/scroll/select/write
│                           #   surface to the TerminalHandle trait if needed (pure seam)
├── session.rs, worktree.rs, git.rs, store.rs, project.rs, workspace.rs,
│   theme.rs, tokens.rs, icons.rs (+ Icon::Settings), metadata.rs   # existing (extended minimally)
├── main.rs                 # pass active &RuntimeTerminal into the pane; apply TermActions to the
│                           #   focused Running session's PTY/Term; scrollback from Settings on spawn
└── ui/                     # gui-gated iced layer
    ├── mod.rs              # view(): terminal pane now the TerminalPane widget; subscription()
    │                       #   gates app keyboard shortcuts on !terminal_focused
    ├── components/
    │   ├── terminal_pane.rs   # NEW: advanced::Widget rendering the Term grid (canvas) +
    │   │                      #   focus-gated on_event → TermAction; canvas::Cache coalescing.
    │   │                      #   Adapted from iced_term view.rs (MIT, attributed)
    │   └── (tree_view.rs, icon_button.rs from 005)
    ├── terminal.rs         # RuntimeTerminal: dynamic rows/cols + resize(Term+PTY), scrollback
    │                       #   from Config.scrolling_history, colored-cell theme palette,
    │                       #   selection/scroll passthrough; pane() builds TerminalPane
    ├── settings_form.rs    # NEW: Settings modal (scrollback field) — reuses form/modal pattern
    ├── theme.rs (or in terminal.rs)  # ansi::Color → iced Color palette from app light/dark theme
    ├── toolbar.rs          # add Settings MenuItem to overflow_items
    ├── shell.rs, sidebar.rs, style.rs, about.rs, project_selector.rs, rename.rs, worktree_form.rs
    └── material/           # reused primitives (MenuItem, modal, toolbar, ...)

tests/                      # pure-core tests (--no-default-features) + gui-gated tests (--features gui)
├── keymap.rs               # NEW (pure): exhaustive key/modifier/mode → bytes/action encoding
├── terminal_focus.rs       # NEW (pure): focus-routing predicate (app vs terminal; release chord)
├── settings_scrollback.rs  # NEW (pure): scrollback serde default/roundtrip/clamp
├── terminal_palette.rs     # NEW (gui): ansi::Color → iced Color mapping incl. theme defaults
├── terminal_mouse.rs       # NEW (gui): TerminalPane selection vs mouse-report + focus gate
├── terminal_resize_scroll.rs # NEW (gui): layout→(cols,rows) + wheel→scroll/alt-screen forward
└── (existing tests reused, incl. session/terminal seams)

docs/user-guide/
├── worktrees-and-sessions.md   # extend: real terminal — colors, focus in/out, keys, copy/paste,
│                               #   mouse, scrollback
├── settings.md                 # NEW (Principle VII): the Settings form + scrollback limit
└── (existing guides)
```

**Structure Decision**: Preserve the render-free-core + gui-binary layout from 005. The only new *pure* module is `src/keymap.rs` (key encoding), keeping the high-value logic testable under `--no-default-features`. All rendering/input is a single reusable gui widget `src/ui/material/terminal_pane.rs`; the session runtime (`RuntimeTerminal`) is extended in place for dynamic size + scrollback. Settings reuse the existing store and form/menu patterns. No new crates, no new workspace.

## Complexity Tracking

*No constitution violations — no entries.*

**Bugfix**: 2026-07-17 — BUG-001 Updated from bugfix patch. Auto-focus the displayed session's terminal on start/select: the `SessionSelected` and `SessionStarted` reducer branches in `src/app.rs` set `terminal_focused = true` (previously focus was only click-acquired and `SessionSelected` cleared it). The pure `route_key` gate, the `Running`-only write-gate (`src/main.rs`), and the release mechanisms (`Message::TerminalFocusReleased`) are unchanged. The auto-focus of the selected session must take precedence over the click-outside release produced by the same sidebar click (`contracts/focus-model.md`, BUG-001). Covered by pure tests in `tests/terminal_focus.rs` / `tests/app_state.rs`.
