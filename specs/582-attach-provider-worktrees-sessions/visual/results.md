# Feature 582 visual pass: B2/B3 (worktrees only)

Date 2026-10-05. Xvfb (JetBrains-bundled binary) + lavapipe, 1400x900 window, not a real display. Client/daemon built from ae5d9a56, pinned copies in ~/vp/bin; handshake confirmed (`client attached to daemon`). Scratch XDG_DATA_HOME, scratch repo /tmp/vp77repo with worktrees alpha/beta/gamma under .claude/worktrees/ (beta has an uncommitted change), empty catalog. Driven with python-xlib XTEST (xdotool not installed). Sessions and banner not exercised. Animation not covered.

| Check | Light | Dark | Evidence |
|---|---|---|---|
| Entry button: folder glyph in sidebar header, tooltip "Attach existing..." readable, no overlap | PASS | PASS | b2-sidebar-header-{light,dark}.png (light shows tooltip) |
| Button is left of the add-worktree button | NOTE | NOTE | It sits in the header row (refresh, attach, tree, collapse). The "+" add-worktree button is in the Default row below, not adjacent. |
| Dialog "Attach existing worktrees", checkbox list, Attach selected / Attach all / Cancel, readable | PASS | PASS | b2-dialog-{light,dark}.png |
| Dialog lists 3 worktrees (alpha, beta, gamma) | PASS | PASS | same |
| Attach all: 3 worktrees in sidebar, "Show agent worktrees" untouched | PASS | PASS | b3-sidebar-after-attach-{light,dark}.png |
| Notification "Attached 3 worktrees." | PASS (seen, with Dismiss) | NOT SEEN (screenshot taken 8 s after click; likely already timed out) | light only, in a transient frame, not saved |
| Uncommitted change survives (git status in beta) | PASS | PASS | ` M a.txt`; `git worktree list` still 4 entries |
| Second press: nothing left to attach | PASS | PASS | b3-second-press-{light,dark}.png ("No unattached worktrees found.", both buttons disabled; crops cut off the button row) |

## Defects and observations
1. Checkboxes render as hollow circles, which read as radio buttons. Visually unclear for multi-select (not tested whether clicking toggles).
2. The dialog is sized for the list state with a large empty gap (about 100 px) under 3 rows before the buttons. Loading and empty states are shorter, so the dialog resizes between states.
3. After "Attach all" the dialog stayed up for about 3 s with the stale list and disabled buttons while the notification was already showing; it closed afterwards. Possibly frame lag under lavapipe, not confirmed.
4. Entry button placement differs from the description (see NOTE above).
5. In light theme the disabled "Attach selected" label is low contrast (expected for a disabled state).
