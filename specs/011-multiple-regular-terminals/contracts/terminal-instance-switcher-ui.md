# Contract: bottom-bar "open instance" affordance + instance-switcher row

GUI-only (`src/ui/terminal.rs::pane`). Governs FR-001, FR-004–FR-007, FR-011–FR-013.

## Placement

Both controls live in the same bottom status bar `pane()` already renders (session name, status
text, restart control, release-focus control, primary AI-CLI/Regular mode toggle — feature 010).
The primary mode-toggle `IconButton` keeps its existing position, anchoring the bar's bottom-right
corner (unchanged, per the feature 010 spec Clarification); the new controls are inserted before
it in the bar's `row!`, alongside the restart/release-focus controls.

## "Open a new instance" control

- **Visible** whenever the active session's `mode == TerminalMode::Regular` — regardless of
  `shells.len()` (0, 1, or many). This is what lets a user go from zero or one instance to two
  (spec Assumptions) even though the list/switcher portion below is hidden until there are two.
- **Never visible** while `mode == TerminalMode::AiCli` — there is nothing to open a sibling
  instance of when no Regular Terminal is even displayed.
- On press: dispatches `Message::ShellInstanceOpenRequested` (same message the
  `Ctrl+Shift+T`/`Cmd+Shift+T` chord dispatches — contracts/keyboard-shortcut.md).
- Built from `IconButton::new(Icon::AddTerminalInstance, r)` + `Tooltip`, the same builder
  components the mode toggle and restart control already use in this bar — no new shared
  component (research.md R3).

## Instance-switcher row

- **Visible** only when the active session's `shells.len() > 1` (FR-005). At 0 or 1 instances it
  renders nothing — pixel-identical to today's (feature 010) single-instance experience, matching
  User Story 2 Scenario 1.
- **Contents**: one small entry per element of `session.shells`, in list order (append-on-open —
  data-model.md), each labeled with its `ShellInstanceId`'s numeric value (the spec's own
  "sequentially numbered" assumption — no separate title source exists for a shell instance).
- **Active entry** (`session.active_shell == Some(entry.id)`) is visually distinguished
  (highlight/selected style), mirroring `TreeItem::selected` visually even though this row is not
  a `TreeItem` (research.md R3) — a user must be able to tell which instance is active from this
  row alone (FR-004, SC-004), without opening it.
- Clicking an entry's body dispatches `Message::ShellInstanceSelected(entry.id)`.
- Each entry carries its own close action (a small trailing icon/button, reusing `Icon::Delete`
  — the same icon feature 008's delete affordance already uses) dispatching
  `Message::ShellInstanceCloseRequested(entry.id)` (FR-011).
- Each entry that is individually "restartable" (its own `lifecycle ∈ {NotStarted, Exited}`,
  contracts/shell-instance-lifecycle.md) shows its own restart affordance, dispatching
  `Message::ShellInstanceRestartRequested(entry.id)` — **independent** per entry; the existing
  session-level restart control in this same bar continues to reflect only the *currently
  attached* process's overall restartability (AI CLI in `AiCli` mode; the active shell instance
  in `Regular` mode) and is unaffected by a background sibling instance's state.

## Interaction with the primary mode toggle

Unchanged from feature 010: the primary toggle only flips `Session.mode`. It never touches
`shells`/`active_shell` — switching into Regular mode shows whichever instance `active_shell`
already names (FR-007), lazily calling `open_shell_instance()` only if `shells` is currently
empty (first-ever switch for that session, or the state just after the last instance was closed
and `mode` fell back to `AiCli`, FR-013).

## Session status text (bar's left/center text, feature 010, extended)

In `Regular` mode, the status text reflects the **currently active instance's** lifecycle only
(`session.active_shell_lifecycle()`, data-model.md) — not an aggregate of all open instances'
states. A background sibling instance exiting does not change this text; only its own row entry
in the switcher (above) reflects that.
