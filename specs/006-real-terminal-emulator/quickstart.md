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
- Press the reserved chord **Ctrl+Shift+E** (macOS **Cmd+Shift+E**) ~~— or click outside the pane,
  or use the header "release focus" affordance~~. *(Both alternatives are gone: click-outside by
  feature 023 FR-005/FR-006, the affordance by 023 FR-021b — `012-multiple-regular-terminals`
  BUG-001. The chord is the only explicit release, and this step now validates it alone.)*
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

## 10. Bugfix verification — BUG-001 (auto-focus) and BUG-002 (scroll)

Added 2026-07-20. These cover the two patched bugs and the paths their unit tests cannot reach.
`on_event` needs a GPU-backed renderer, so every step below is manual by necessity — the routing
helpers (`press_routing`, `wheel_routing`, `select_kind`) are unit-tested, but nothing proves the
events actually arrive and are dispatched except running the app.

Mark each ✅/❌ as you go.

### 10a. Touchpad scrolling — SC-010, FR-016b
1. Fill the terminal past one screen (`yes | head -n 500`, or a long `git log`).
2. Two-finger scroll **up** on the pane, slowly.
   - **Expect**: the viewport moves through the scrollback. ✅ *Verified 2026-07-20 (Wayland/GNOME).*
3. Scroll up in the smallest increments you can manage — barely-moving flicks.
   - **Expect**: still scrolls. Each flick is under one line; they must accumulate, not vanish.
     This is the exact BUG-002 defect.
4. Scroll **up**, then immediately reverse to **down** without lifting.
   - **Expect**: the reversal responds promptly and in the right direction. Banked upward travel
     must not cancel the first downward movement (residual reset on direction change).
5. Scroll back to the very bottom.
   - **Expect**: parks cleanly at the live bottom, no overshoot or stickiness.

### 10b. Scrollbar — FR-016a
6. While scrolled back, look at the right edge of the pane.
   - **Expect**: a scrollbar is visible; the thumb sits proportional to position and to how much
     history exists. *(This was the visible face of BUG-002 — the branch is named after it.)*
     ✅ *Verified 2026-07-20.*
7. Return to the live bottom.
   - **Expect**: the scrollbar disappears. Hidden at the bottom is correct, not a defect.
     ✅ *Verified 2026-07-20.*
8. Drag the thumb up and down.
   - **Expect**: the view follows the grabbed point smoothly; no flicker or drift.
9. Click the scrollbar track above and below the thumb.
   - **Expect**: pages through the history.
10. Toggle app theme while the scrollbar is visible.
    - **Expect**: it follows Material light/dark (FR-022).

### 10c. Touchpad inside a mouse-reporting program — FR-013a
> The branch I could unit-test but never run. A mouse-reporting TUI gets wheel *reports* instead
> of local scrolling, and before BUG-002 a touchpad generated none at all.
11. Focus the terminal and run a mouse-mode TUI — `htop`, or `less -X --mouse` on a long file.
12. Two-finger scroll over it.
    - **Expect**: the *program* scrolls its own view. The pane's scrollback must not move and no
      scrollbar should appear. ✅ *Verified 2026-07-20 with `htop` — the FR-013a wheel-report path
      works from a touchpad, which it did not before BUG-002.*
13. Sub-line flicks again inside that program.
    - **Expect**: still scrolls it — accumulation applies on this path too.
14. Quit the TUI, scroll again.
    - **Expect**: back to local scrollback + scrollbar.

### 10d. Discrete wheel — regression check
15. If you have a mouse, wheel up/down over the pane; repeat inside `htop`.
    - **Expect**: unchanged from before the fix. Line-based deltas pass straight through.

### 10e. Auto-focus on select/start — SC-009, FR-010/FR-010a (BUG-001)
16. Start a **new** session. Without clicking anything, type.
    - **Expect**: characters reach `claude` immediately. No click needed.
17. With session A focused, click session **B** in the sidebar, then type without clicking the pane.
    - **Expect**: keys go to B. The sidebar click is a click *outside* the pane, which would
      normally release focus — the auto-focus must win (FR-010a).
      ✅ *Verified 2026-07-20 — the precedence rule holds in the real event ordering.*
18. Press **Ctrl+Shift+E**, then type.
    - **Expect**: keys drive the app, not the terminal. The session keeps running.
19. Close a session / switch project.
    - **Expect**: focus is cleared; keys drive the app.

### 10f. Selection granularity — FR-013 (T057)
20. **Single**-click and drag across some text. **Expect**: character-level selection.
21. **Double**-click a word. **Expect**: the word is selected.
22. **Triple**-click a line. **Expect**: the whole line is selected.
23. With `htop` running, **Shift+drag**. **Expect**: selection works despite mouse mode (FR-013b).

### 10g. Scrollback limit interaction — SC-007
24. Set a small scrollback limit in Settings, start a new session, flood output past it, scroll up.
    - **Expect**: history is bounded at the limit; scrolling stops there rather than misbehaving.

## Cross-platform — SC-006

Repeat the manual flow on Linux, macOS, and Windows (CI builds/tests all three). Note the
platform copy/paste chords (Cmd on macOS). Behavior should be equivalent.
