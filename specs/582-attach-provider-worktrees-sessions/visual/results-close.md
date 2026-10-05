# Close-out visual pass (quickstart Part B, items never seen before)

Date 2026-10-05. Client and daemon release-built from HEAD 8947038b (no local changes), pinned copies in ~/vp/bin80. Xvfb :80 (JetBrains-bundled binary, 1400x900) + lavapipe, python-xlib XTEST driver (no xdotool). Private data: XDG_DATA_HOME, CLAUDE_CONFIG_DIR, COPILOT_HOME and PI_CODING_AGENT_DIR under ~/.cache/vp80, repo /tmp/vp80repo with worktrees alpha, beta, gamma, empty catalog.
Seeded Claude transcripts: 2 at the repo root, 2 in alpha, 1 in gamma (all adopted by feature 026 at open, count=5 in the daemon log), 2 in worktrees git no longer lists (vanished, gone: the only ones that stay "resumable" candidates), 2 for another repo (never shown). Animation not covered.

Note on fixtures: 026's open-time adoption already takes every transcript that sits in the root or a listed worktree, so the banner's "sessions" count can only come from stored sessions of removed worktrees. Those are unresumable ("no worktree of this project matches where it ran").

| Item | Result | Evidence |
|---|---|---|
| Start-up banner with seeded sessions | SEEN, dark and light: "Found 3 worktrees and 2 sessions / This project has none attached yet." with Dismiss and Attach all | c1-offer-dark.png, c1-offer-light.png |
| Banner "Attach all" | SEEN: toast "Attached 3 worktrees."; banner gone; 3 worktrees in sidebar, no agent chip; no session launched (0 "session started" in daemon log); `git worktree list` unchanged | c2-banner-attach-all-dark-0s.png, -3s.png |
| B3 "sessions show idle" | SEEN (dark and light): Alpha expands to its 2 adopted sessions "refactor the parser" and "add parser tests" with a claude chip and a close x, muted styling, nothing running. Idle has no separate label in the sidebar | c3-alpha-expanded-dark.png, c3-after-attach-light.png |
| Dialog after attach | Lists only the 2 ghost sessions (Resume disabled, reason shown), "No unattached worktrees found." | c3-dialog-after-attach-dark.png |

## M2 oddities

- Gap in the dialog between the list and the buttons: about 110 px blank in the empty-worktrees case (c3-dialog-after-attach-dark.png). Still present (spacing only).
- Dialog lingering after Resume: not retested (no resumable session in the fixtures; Resume was disabled for both).
- Sidebar dimmed or stale after a state change: REPRODUCED, also after the banner's Attach all and after expanding a worktree (dark: sidebar nearly black, only hovered rows repaint at full brightness; c2-*.png shows it with a ghost hover tooltip). A sidebar collapse and expand restored a correct frame every time, so this looks like a presentation/damage problem of lavapipe + Xvfb + no window manager rather than wrong state, but that is not proven and no real display was available. Unresolved.
