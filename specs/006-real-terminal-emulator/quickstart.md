# Quickstart: Validating Real Terminal Behavior

Runnable validation for feature 006. Proves the embedded terminal renders like a real terminal,
propagates keys only when focused, supports interactive `claude`, resizes, scrolls, copies, and
exposes a configurable scrollback in Settings. Maps each step to Success Criteria (SC-00x).

## Prerequisites

- `claude` CLI installed and on `PATH`.
- A git repository to open as a project, with at least one worktree + session available (see
  feature 005 quickstart to create one).
- Toolchains: `cargo` (stable). GUI run needs the `gui` feature.

## Automated checks (fast, no GUI)

```bash
# Pure logic: key encoding, focus routing, scrollback settings — must pass.
cargo test --no-default-features

# Full suite incl. gui-gated widget tests.
cargo test --features gui
```

Expected: `keymap` tests pass (incl. `Ctrl+U == 0x15`, arrows in/out of app-cursor mode,
reserved focus-out chord, copy/paste chords); `terminal_focus` routing tests pass;
`settings_scrollback` roundtrip/default/validation tests pass.

## Manual end-to-end (GUI)

```bash
cargo run --features gui
```

Open the project, expand a worktree, and start (or select) a session so its terminal shows.

### 1. Colored, faithful rendering — SC-002 (US1)
- In the terminal, run something colorful, e.g. `git -c color.ui=always diff` or `ls --color=always`,
  and use the interactive `claude` UI.
- **Expect**: ANSI colors (incl. 256/truecolor), bold/dim/italic/underline/reverse render the
  same as a standalone terminal; a full-screen (`claude`) UI redraws with no leftover artifacts;
  the cursor is visible.
- Toggle app theme (toolbar menu). **Expect**: default fg/bg follow light/dark; ANSI colors
  unchanged.

### 2. Focus gate — SC-003 (US3)
- **Without** focusing the terminal, press app shortcuts / type. **Expect**: the app responds;
  nothing appears in the terminal; `claude` is not driven.
- Click the terminal. **Expect**: a visible focus indicator appears.
- Type. **Expect**: characters reach `claude` immediately (no line buffering).

### 3. Interactive claude — SC-001 (US2)
- With the terminal focused: navigate a `claude` menu with **arrow keys**; trigger slash-command
  autocomplete with **Tab**; enter a **multi-line** prompt; press **Ctrl+C** to interrupt a
  running turn.
- **Expect**: every interaction behaves exactly as in a standalone terminal.

### 4. Move focus out — SC-005 (US3)
- Press the reserved chord **Ctrl+Shift+E** (macOS **Cmd+Shift+E**) — or click outside the pane,
  or use the header "release focus" affordance.
- **Expect**: focus returns to the app; subsequent keys drive the app; the `claude` session keeps
  running uninterrupted. Confirm **Esc** while focused reaches `claude` (does not close overlays).

### 5. Copy / paste — (US2/US3, FR-013)
- Select terminal text by dragging (Shift+drag if a mouse-mode TUI is active); copy with
  **Ctrl+Shift+C** (macOS **Cmd+C**), auto-copy-on-select + **middle-click** paste, or the
  **right-click** menu. Paste with **Ctrl+Shift+V** (macOS **Cmd+V**).
- **Expect**: selection copies; paste inserts text into `claude` without auto-submitting each
  line (bracketed paste honored).

### 6. Sizing, resize, scrollback — SC-004 (US4)
- Note the `claude` UI fits the pane. Resize the window / drag the sidebar handle.
- **Expect**: the terminal and `claude`'s UI reflow to the new size within a redraw; no truncated
  or mis-wrapped content.
- Produce more output than fits; scroll the wheel / PageUp. **Expect**: earlier output is shown,
  up to the configured scrollback limit.

### 7. Non-Running input — (FR-012a)
- Close/stop a session's process (or catch it mid-restart). With the terminal focused, type.
- **Expect**: keystrokes are discarded (not buffered); the pane header shows the session status
  (starting…/restarting…/failed); scrolling/selection/copy still work.

### 8. High-output responsiveness — SC-008 (FR-005a)
- Run a command that floods output (e.g. `yes | head -n 100000` or a large `cat`).
- **Expect**: input and scrolling stay responsive (≤~100 ms perceived); memory stays bounded by
  the scrollback limit; the final screen matches the process output.

### 9. Settings — configurable scrollback — SC-007 (US5)
- Open the toolbar overflow menu (three dots) → **Settings**. Change the scrollback limit, save.
- **Expect**: a session started after the change honors the new limit.
- Quit and relaunch the app, reopen Settings. **Expect**: the changed value persists.
- Provide an out-of-range value. **Expect**: a clear validation message; not saved.

## Cross-platform — SC-006

Repeat the manual flow on Linux, macOS, and Windows (CI builds/tests all three). Note the
platform copy/paste chords (Cmd on macOS). Behavior should be equivalent.
