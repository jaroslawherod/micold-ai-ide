# Visual pass — feature 972, milestone M1

Run 2026-10-18 by the `visual-pass` skill on Xvfb (tiny-skia software renderer), input via xdotool
and XTest, at branch head 5dbd5098. Scope: quickstart §B B1, B3–B9, B14–B16, B19; the target mark
(M2) and keyboard steps (M3) are out of scope. Screenshots: [visual/](visual/).

| Step | Result | Seen |
|---|---|---|
| B1 | PASS (light, dark) | Menu at the pointer: Open, Review changes, Copy name, Rename, Delete; order and icons match the sidebar row menu plus Open |
| B3 | PASS | Rename dialog over the Overview; card and sidebar show `feat-42_beta`; Overview stays open |
| B4 | PASS | After Cancel the Overview is unchanged |
| B5 | PASS | Default card: Open, Review changes (the Default row menu's entries) |
| B6 | PASS | Escape, empty-space click, other-card click each close the menu, run nothing, Overview stays; clicked card not activated |
| B7 | PASS | A's menu closes, B's opens at the new point (a blank ghost rectangle where A's menu was cleared on a forced redraw: renderer damage, not the app) |
| B8 | PASS | Right edge and 640×480: panel inside the window |
| B9 | PASS | Scrolling the grid closes the menu |
| B14 | PARTIAL | Review changes opens the Changes view and closes the Overview; Open closes the Overview and selects the worktree, but the fixture had no session, so the agent picker showed |
| B15 | not run | Needs a second window to delete the worktree |
| B16 | PASS | Ctrl+Shift+O closes the Overview and the menu together |
| B19 | PARTIAL | With `misc` bound to an unknown daemon (seeded), *Bind to daemon...* shows the dialog over the Overview; Cancel leaves it unchanged |

Observation: in one B3 run the Rename dialog did not take keyboard focus, and Return reached the
focused card behind it. The dialog never auto-focuses its field on any path (pre-existing); recorded
as a follow-up in the ledger.
