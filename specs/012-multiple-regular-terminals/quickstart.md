# Quickstart: Validating Multiple Regular Terminal Instances per Session

Runnable validation for feature 012. Proves a session can run several independent Regular
Terminal instances at once, that switching/closing/restarting one never disturbs a sibling
instance or the AI CLI process, and that the switcher control appears only when it's needed.
Maps each step to Success Criteria (SC-00x).

## Prerequisites

- `claude` CLI installed and on `PATH`.
- A git repository open as a project, with a worktree or "Default" session already running in
  Regular Terminal mode (see feature 010's quickstart to get there).
- Toolchains: `cargo` (stable). GUI run needs the `gui` feature.

## Automated checks (fast, no GUI)

```bash
# Pure logic: ShellInstanceId allocation, open_shell_instance/close_shell/select_shell
# transitions (including the FR-012/FR-013 fallback rules), is_new_terminal_chord — must pass.
cargo test --no-default-features

# Full suite incl. gui-gated SessionTerminals multi-shell map + switcher-row wiring tests.
cargo test --features gui
```

Expected: `session_shell_instances` tests pass (ids never reused across opens/closes,
`close_shell` reassigns `active_shell` to the next instance or falls back to the previous/last
one, closing the last instance flips `mode` to `AiCli`); `keymap` tests pass (the new chord is
detected per-platform and takes precedence over plain `t`/`T` typing).

## Manual end-to-end (GUI)

```bash
cargo run --features gui
```

Open the project, select a session already in Regular Terminal mode (one instance running).

### 1. Open a second, independent instance — SC-001, FR-001–FR-003 (US1)

- Note the current shell's `pwd`, run `cd` into a subdirectory.
- Click the "+" (open new instance) control in the bottom bar. **Expect**: a second shell starts
  immediately (<500ms), its `pwd` is the session's working directory (not the first instance's
  `cd`-adjusted one), and the first instance's process/output is untouched.
- Run a long-lived command in the first instance (e.g. `sleep 30 && echo done`), then switch to
  the second instance and run ordinary commands. **Expect**: the first instance's command keeps
  running in the background, unaffected.
- Confirm exactly one `claude` (AI CLI) process is associated with the session throughout.

### 2. See and switch between instances — SC-002, SC-004, FR-004–FR-007 (US2)

- With only one instance open, confirm no switcher row is shown (matches today's single-terminal
  look).
- Open a second and third instance (via the "+" control and/or `Ctrl+Shift+T` /
  `Cmd+Shift+T` — while the terminal pane has focus). **Expect**: a switcher row now appears,
  listing all three instances, with the currently-visible one clearly highlighted.
- Click each entry in turn. **Expect**: the pane switches to that instance's process/output with
  no perceptible delay (<500ms), and the previously-visible instance keeps running unattended.
- Toggle to AI CLI mode, then back to Regular mode via the primary toggle. **Expect**: the pane
  shows whichever instance was last active before the toggle-away, not an arbitrary one.

### 3. Close instances without disturbing siblings — SC-003 (US3)

- With three instances open, close a background (non-visible) one via its row's close action.
  **Expect**: that instance's process ends, its row disappears, and the visible instance plus the
  remaining sibling are completely unaffected.
- Close the currently-visible instance. **Expect**: per the resolved clarification, the pane
  automatically shows the next instance in the list (or the new last one, if the closed instance
  was last), and the remaining sibling instance is unaffected.
- Close the one remaining instance. **Expect**: the session falls back to AI CLI mode (today's
  single-terminal close behavior) — matching feature 010's existing close behavior exactly.

### 4. Independent lifecycle and restart — SC-005 (US4)

- Open two instances. In one, type `exit`. **Expect**: only that instance shows a not-running
  state with its own restart affordance; the other instance and the AI CLI process are
  unaffected.
- Press that instance's restart affordance. **Expect**: only that instance starts a fresh shell;
  the sibling instance is not restarted, the AI CLI process is not restarted.

### 5. The keyboard shortcut and its mode gating — FR-019

- While the terminal pane is focused and the session is in Regular Terminal mode, press
  `Ctrl+Shift+T` (`Cmd+Shift+T` on macOS). **Expect**: a new instance opens, identical in effect
  to pressing the on-screen "+" control.
- Switch to AI CLI mode and press the same shortcut. **Expect**: nothing happens — no new
  instance is created, and the session does not switch back to Regular mode on its own.

### 6. Per-session independence — SC-006

- Open a second session (same or different worktree/Default). Open multiple Regular Terminal
  instances in it. **Expect**: the first session's instances, active selection, and AI CLI
  process are completely unaffected.

### 7. Reopen after a restart — Edge Cases, FR-017

- Leave a session with three Regular Terminal instances open, quit the app, relaunch it, and
  reopen that session. **Expect**: it reopens in whatever mode (`AiCli`/`Regular`) it was last
  in, with **at most one** freshly-started instance if `Regular` — the prior instance count is
  not restored (same restart behavior feature 010 already established for the single-instance
  case).

### 8. The switcher reads as a tab strip — FR-004a, FR-011a, SC-007, SC-008 (BUG-001)

Scenario 2 above checks that you can tell *which* instance is active. These check that the row
looks like a tab strip while you do — the half no automated gate can see, and the half BUG-001
shipped without. Run each in **both** the light and the dark theme; the original defect was far
worse in one of them.

With two or more instances open:

- **Every entry is a tab.** Each one sits in a container of the same shape and size — active and
  inactive alike. No entry is bare text with a close glyph floating beside it. The active tab
  differs by *emphasis* (a filled background) and by nothing structural.
- **Label centred, close trailing.** Within each tab, the number is horizontally centred and the
  close control sits at the tab's right edge — not immediately beside the number.
- **Nothing reflows.** Select a different tab. Every tab keeps its position and size; only the
  emphasis moves. Nothing shifts under the pointer, so a second press lands where you aimed the
  first. Open a tenth instance if you can, and confirm a two-digit label does not resize its tab
  either.
- **The close control is visible on the *active* tab.** This is the one to look at hardest: the
  close glyph on the highlighted tab must read at the same strength as that tab's own number. If
  it is a faint ghost against the fill, that is the original bug — the glyph kept the bar's
  foreground colour instead of the tab's (FR-011a).
