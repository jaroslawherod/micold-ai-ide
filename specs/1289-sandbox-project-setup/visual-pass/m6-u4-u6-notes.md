# Visual pass M6 (U4, U6; U3 Setup output entry) — 2026-10-26

Ran on Xvfb :77 (1600x1400) + Mesa lavapipe (WGPU_BACKEND=vulkan), not a real display, driven with xdotool.
Binaries: micold-ai-ide + micold-daemon built together (debug) from claude/project-thread-d7q05f @ 07a50ad79
(source identical to 3e4881d6f; only autopilot.md differs); pair confirmed by "client attached to daemon".
Host daemon (spawned by the client), private XDG_DATA_HOME/XDG_RUNTIME_DIR, git project "demo-app".
States produced by setting Settings › Project › Setup command and creating worktrees from the form.
The Default record was seeded on disk (`setup/<id>/default.json|log`, copied from a failed run) because
**Run setup** on Default is a later milestone. Window sizes: 1600x1000 ("large") and 640x480.

- U3 menu (light+dark): **Setup output** with the terminal icon directly above Delete on a worktree,
  last on Default (`m6-u3-*-menu.png`).
- U4 title "Setup — <name>" uses the app's friendly name (Running, Quiet, Lots, Default), the same
  name the delete confirmation uses; the sidebar row shows the branch-style "feat/<name>".
- U4 header: command monospace, clamped to 3 lines (a 4-line command shows 3, cut on a line boundary);
  hovering it shows the full text in a tooltip (`m6-u4-light-command-tooltip.png`). State line in words:
  "Setup running since 2026-10-26 07:56", "Setup failed at …: Exited with status 2", "Setup finished at …".
- U4 empty: "No output yet." (`sleep 900`). Failed (`echo before; exit 2`): "before" then
  "Exited with status 2.". Dropped (`seq 1 12000`): first row "2001 earlier lines not kept", then 2002…
  (12,000 lines + the exit line, 10,000 kept). Running (`for i in $(seq 1 200) …; sleep 0.3`): follows
  the tail at a fixed window size (frames 12 s apart: 28–51 → 68–91); scrolling back to the bottom
  resumes the follow (`m6-u4-light-640-follow-resumed.png`).
- Output lines monospace, one per row, not wrapped (clipped at the right edge), no overlap.
  Copy output and Close (last, filled) inside the dialog at 640x480; nothing spills outside.
  Both themes readable.
- U6 (light+dark, both sizes): "Setup is running here. It will be stopped first." between the warning
  and the branch checkbox while running; absent once setup ended. Confirming the delete stopped the
  `sleep 900`, removed the worktree and its setup record and log.

Findings
- F1 MAJOR — follow lost on window resize: resizing the window (1600x1000 → 640x480) while the dialog
  is open and following jumps the list to line 1 and it stops following while lines keep arriving
  (`m6-u4-light-640-resize-lost-follow.png`, `m6-u4-dark-640-resize-lost-follow.png`, frames 6 s apart).
  Seen 3 times out of 4 (a 12,001-line view stayed at the tail). Suspect the scrollable is rebuilt on
  relayout and its `on_scroll(0, …)` detaches the follow.
- F2 MINOR — the row menu stays open: choosing **Setup output** leaves the worktree/Default menu
  open under the scrim, and it is still open after Close (`m6-u3-light-menu-left-open-after-close.png`;
  at 640x480 its Delete item shows below the dialog, `m6-u4-*-640-empty.png`). Choosing Delete closes it.
- F3 MINOR — the output list's scrollbar is drawn over the last visible text column (clipped lines run
  under the thumb); cosmetic.

Not covered: Stop setup in the dialog (contract U4; later milestone T058); Copy output's clipboard
contents (no xclip/xsel in the container); reconnect resume; animation/smoothness.

## Round 2 (scoped re-check of F1–F3) — 2026-10-26

Same recipe: Xvfb :77 + lavapipe (WGPU_BACKEND=vulkan), xdotool, client + host daemon built together
(debug, CARGO_PROFILE_DEV_DEBUG=0) from claude/project-thread-d7q05f @ fe8cfd113 (source = a6b1f4d97;
only autopilot.md differs), pinned copies, "client attached to daemon" in the daemon log. Private XDG
dirs, git project "demo-app"; setup command and theme seeded in the private settings.json, worktrees
created from the form. Default record seeded on disk from a failed run, as in round 1.

- F1 (follow on resize) — fixed. Light (400-line `echo line $i; sleep 0.3`) and dark (3,000 lines of
  300 chars): 5 resizes each, 1600x1000 ↔ 900x700 ↔ 640x480, both directions; after every resize the
  list sat on the tail and two frames 3 s apart showed advancing numbers (e.g. 127→135 / 138→146 at
  640, 121→153 / 130→163 at 1600); repeated in light with long lines (4 more resizes, 645→716).
  Scrolling up stops the follow (two frames identical, 208–231 / 193–216) and scrolling back to the
  bottom resumes it (289–312 → 299–322; 273–296 → 284–307).
  `m6-r2-{light,dark}-resize-follow*.png`, `m6-r2-{light,dark}-scroll-stop-resume.png`.
- F2 (row menu left open) — fixed. Choosing **Setup output** from a worktree menu and from Default's
  menu closes the menu: nothing under the scrim, nothing after Close, no menu item below the dialog at
  640x480. `m6-r2-light-worktree-menu-closed-on-open.png`, `m6-r2-*-after-close-no-menu.png`,
  `m6-r2-dark-default-*.png`, `m6-r2-light-default-menu-failed-close-empty.png`.
- F3 (scrollbar over text) — fixed. Long lines clip before the scrollbar rail at both sizes and in
  both themes; no glyph under the thumb (`m6-r2-{light,dark}-scrollbar-clip.png`, 4x crops: large left,
  640 right).
- Sanity: empty ("No output yet.", `sleep 900`) and failed ("before" / "Exited with status 2.", Default)
  render as in round 1, light and dark, large and 640x480 (`m6-r2-*-640-*.png`, `m6-r2-dark-empty-large.png`).

No new findings. Observation only (not a regression, out of scope): with ~700 lines at 640x480 the
scrollbar thumb is a sliver of about 2–3 px.
