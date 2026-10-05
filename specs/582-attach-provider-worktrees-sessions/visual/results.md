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

## M2 sessions pass

Date 2026-10-05. Xvfb :78 (LD_LIBRARY_PATH from the JetBrains bundle) + lavapipe, window forced to 1400x900 via Xlib, python-xlib XTEST driver (venv). Client/daemon built from the working tree (HEAD 2a84d8d8), pinned copies, handshake confirmed. Scratch XDG_DATA_HOME, CLAUDE_CONFIG_DIR/COPILOT_HOME/PI_CODING_AGENT_DIR; scratch repo /tmp/vp78repo with worktrees alpha, beta. Claude transcripts (repo root "fix the build", alpha "refactor the parser in alpha", vanished "ghost session in a removed worktree") written after the project was open. Note case: a candidate worktree dir with an unreadable transcript. Animation not covered; the empty-store note was not exercised (the missing Copilot and Pi stores correctly produced no note while the list was non-empty).

| Check | Light | Dark | Evidence |
|---|---|---|---|
| "Stored sessions" section legible: title, provider, status line | PASS | PASS | b2s-dialog-note-{light,dark}.png |
| Statuses: "Ready to resume" / "Resuming also attaches its worktree" / "Cannot resume: no worktree of this project matches where it ran" | PASS | PASS | same |
| Resume button per row; unresumable row disabled and shows its reason | PASS | PASS | same |
| Rows not overlapping | NOTE (rows and Resume buttons are tightly stacked, about 4 px between buttons, status text small) | NOTE | same |
| Footer note readable ("Claude Code has a session entry that could not be read.") | PASS (muted, small) | PASS (muted, small) | same |
| Dialog fits 1400x900 with worktrees and sessions | PASS (about 750 px tall, margins above and below) | PASS | same |
| Resume on alpha: session in sidebar, alpha attached, toast "Attached 1 session." | PASS (session row under Alpha with provider chip and x; daemon logged session started launch=Resume; no provider CLI so it did not run, nothing shown as an error) | PASS | b2s-sidebar-after-resume-{light,dark}.png |
| Worktree list in the dialog untouched by Resume (git worktree list still 3 entries) | PASS | PASS | git |

## M2 sessions pass: defects and observations
1. The dialog has a large empty gap (about 150 px) between the 2 worktree rows and "Stored sessions" (fixed-height worktree list); it grows by one footer line when a note appears.
2. After Resume the dialog stays up about 5 s with all Resume buttons disabled and stale rows, then closes (same lag as defect 3 of the first pass; possibly lavapipe).
3. Light theme: after the dialog closed the sidebar stayed dimmed grey (scrim remnant) in screenshots up to 10 s later, and the new session row in the sidebar renders as a white block with its label bottom-aligned and offset from the Alpha label. Not seen as dimmed in dark (a dim is hard to tell there). Could be a stale frame under lavapipe; the session row layout looks odd in both themes.
4. A project's first click on the header attach button after closing the dialog was ignored twice (tooltip only); a second click opened it. Probably focus or hover under XTEST with no window manager.
5. An unreadable Copilot store (sessions dir chmod 000) produced no note, and a file in place of session-state neither; only the Claude entry case was verified to produce a note.
M4 visual pass (B1, B4, B2 first half): banner and dismiss pass in light and dark; evidence visual/b1-offer-{light,dark,dismissed-light,dismissed-dark}.png (Xvfb, python-xlib XTEST, worktrees only, no transcripts seeded).
