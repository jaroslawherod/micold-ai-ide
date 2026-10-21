# Quickstart: validating the daemon overlay

Prerequisites: `mise trust`, then `mise run gate`.

## Part A: automated

1. `mise run test` (or `cargo test -p micold-client` through mise): `features_daemons`, `overlay_registry`,
   `popover_displacement`, `surface_registration_cost`, `daemons_section_geometry` and the new dialog tests pass.
2. Page-save independence: the dedicated test saves a daemon with a page-level edit pending and asserts the page's draft and its save-enabled state are unchanged (FR-007, SC-004).

## Part B: visual pass (`visual-pass` skill, real GUI)

1. Settings > Daemons: no form inline; Add daemon opens a dialog with a scrim; the page does not react to clicks.
2. Edit a daemon: fields filled; change the name; Save daemon: dialog closes, row shows the new name; page Save untouched (SC-001).
3. Edit then Cancel, Escape, outside click in turn: stored values unchanged on reopen (SC-002).
4. Empty the name, Save daemon: dialog stays, error beside the field.
5. Edit a running container daemon's runtime: Save daemon shows the confirmation inside the dialog; No (and Escape) keeps edits; Yes closes.
6. Details: dialog with errors/log, a new log line appears live; Close, Escape and outside click close it; the row has no Hide details.
7. Keyboard: Tab cycles inside the dialog; focus returns to the opening button on close.
8. Resize to the smallest window: form scrolls, Save daemon and Cancel stay visible (SC-005).
9. Light and dark theme: scrim, surface and action row look right.

## Part B results (close unit, 2026-10-21)

Headless sway, private display, real client and daemon built from the worktree. Earlier passes: `visual-pass-m1.md` (steps 1-5, 8, 9) and the M2/M3 passes (steps 6-7 partly). Judged here:

- Step 6, live log line: Pass. With Details open, killing the daemon added "connection lost" and `state: Reconnecting` lines; the reconnect added "connected".
- Step 7, Tab: Pass for the Add dialog (Name through Cancel, then wraps; no ring ever on the page behind). Not tested on the Edit dialog. The checkbox, Save daemon and one step after Cancel show no clear focus ring.
- Step 7, focus after Escape on Details: Pass. Opened by keyboard, Escape shows the ring on Details; opened by mouse, no ring is drawn but Return reopens Details (focus is there; ring shows for keyboard use only).
- Stale "This field is required" on a reopened Add dialog (blur, Escape, reopen): Pass, the dialog reopens clean.
- Harness note: headless sway has no keyboard; a long-running `wtype` was needed for Escape and Tab to reach the app.
