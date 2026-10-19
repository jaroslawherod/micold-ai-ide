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

# Visual pass — feature 972, milestone M3

Run 2026-10-18 by the `visual-pass` skill on Xvfb (tiny-skia software renderer, `ICED_BACKEND=tiny-skia`),
input via xdotool/XTest, against the uncommitted M3 tree (client and daemon built together,
private data dir, fixture project P with `feat-42_alpha`, `inc`, `misc`). Scope: quickstart §B B10–B13,
B18 and two extra checks. Card order in the grid: Default, Alpha, Inc, Misc. Screenshots: [visual/](visual/).

| Step | Result | Seen |
|---|---|---|
| B10 | PASS (light, dark) | Tab, Tab to `inc`; Shift+F10 opens the menu inside the card with *Open* highlighted ([light](visual/b10-menu-light.png), [dark](visual/b10-menu-dark.png)); Down moves the highlight to *Review changes*; Enter closes the Overview and shows the Changes view for `inc` ([ran](visual/b10-review-changes-ran.png)) |
| B11 | PASS | Menu key on Alpha opens its menu, *Open* highlighted ([open](visual/b11-menu-key.png)); Escape closes it and the ring is back on Alpha ([ring](visual/b11-escape-ring-back.png)). Same with Enter on *Copy name*: menu closes, ring stays on Alpha |
| B12 | PASS (re-run after the `focus_landed` fix) | Shift+F10 on the last card, Down x4, Enter opens the Delete confirm; after Delete (confirmed with the mouse, as the dialog takes no keyboard focus, pre-existing) the card disappears, the worktree is removed on disk and the ring is on the first card (Default); Tab then moves it to Alpha ([after delete, then Tab](visual/b12-after-delete-first-card.png)). Done for two targets (`misc2`, then `misc`). The first run, before the fix, left no card focused |
| B13 | PASS | After clicking empty Overview space, Shift+F10 and the Menu key open nothing ([shot](visual/b13-no-menu.png)) |
| B18 | PASS | Overview closed, a regular terminal running `cat -v`: Shift+F10 arrives as `^[[21~` and no card menu opens ([shot](visual/b18-terminal-keys.png)); the Menu key prints nothing (the terminal does not encode it) and opens nothing |
| Extra: Enter in the open menu does not activate the card | PASS | Enter on *Copy name* runs only that entry: Overview stays open for 4 s (no 300 ms delayed activation), ring stays on Alpha. Control: Enter on the focused Alpha card with no menu activates it (agent picker for Alpha, Overview closes), so activation would have been visible. The ripple itself is not capturable under software rendering (grabs at 0.25 s showed none, also in the control) |
| Extra: Menu key / Shift+F10 start no ripple or activation | PASS (partial evidence) | After each, the Overview stays open and no card activates; no ripple seen in grabs taken straight after the key, with the same capture limit as above |

Observation: with the software renderer a menu showed some rows dimmed after Down (renderer damage, as in M1's ghost rectangle); a forced redraw cleared it.

# Visual pass — feature 972, close unit (B2, B6, B17)

Run 2026-10-19 on Xvfb (tiny-skia software renderer, `ICED_BACKEND=tiny-skia`), input via xdotool,
at origin/main 12619e44 (client, daemon and showcase built together, pinned copies, private data dir,
fixture project P: Default, `feat-42_alpha`, `inc`, `misc`). Scope: only the steps the M1 and M3 passes
left open: B2, the mark part of B6, B17. Screenshots: [visual/](visual/).

| Step | Result | Seen |
|---|---|---|
| B2 | PASS (mark); hover not capturable | Menu opened on Alpha: Alpha has the 2dp primary outline and selected layer. Pointer over the menu ([shot](visual/b2-pointer-over-menu.png)): mark stays on Alpha only, *Open* row shows hover. Pointer moved onto Inc with the menu open ([shot](visual/b2-pointer-on-other-card.png)): mark still on Alpha only, Inc carries no outline or layer. Inc's own hover is not visible: the card draws no hover layer of its own (its button style is empty; only the keyboard-focus indicator tops one up), and pixel samples of Inc under the pointer and of Misc were identical, with and without the menu. The Default card's heavy outline in the shots is its keyboard focus ring from Ctrl+Shift+O, not a mark |
| B6 | PASS | Escape ([shot](visual/b6-escape-closed.png)), click on empty Overview space ([shot](visual/b6-empty-space-closed.png)) and click on Inc ([shot](visual/b6-other-card-closed.png)) each closed the menu, left the Overview open with all four cards, removed Alpha's mark, opened no dialog and ran no entry; Inc was not activated (no agent picker, Overview still shown). Frames were taken after a window resize nudge to force a redraw (see observation) |
| B17 | PASS (light, dark, greyscale) | Showcase reached by scrolling with wheel clicks (slow: the page redraws late, wait 10-15 s per scroll). `OverviewCard` *targeted* next to an untargeted card: light [colour](visual/b17-card-light.png), [grey](visual/b17-card-light-gray.png); dark [colour](visual/b17-card-dark.png), [grey](visual/b17-card-dark-gray.png). In all four the outline and the darker fill separate it from its neighbour; in greyscale the outline is a clear dark (light) or light (dark) line. `MenuOverlay` with a keyboard-highlighted item (*Rename*): light [colour](visual/b17-menu-light.png), [grey](visual/b17-menu-light-gray.png); dark [colour](visual/b17-menu-dark.png), [grey](visual/b17-menu-dark-gray.png). Ring plus state layer on the one row are visible in both schemes and in greyscale |

Observation: after the pointer moved, the open menu's rows below the hovered one were drawn dimmed until a
forced redraw (same renderer damage as in M1 and M3). Not an app finding.

# Quickstart §C — macOS and Windows (T030)

Not run on a desktop: no macOS or Windows machine is available to the autopilot.

**Covered by CI** (`.github/workflows/ci.yml`): the `test` job builds and runs the whole workspace
suite on `ubuntu-latest`, `macos-latest` and `windows-latest` (the matrix drops Ubuntu only on a
push its pull request proved), and `windows-11-arm` builds the ARM64 client when packaging
changes. The feature's tests (reducer, card menu composition, key mapping, root dispatch, overlay
registry, popover displacement, pixel tests of the mark) run on all three, so the logic is
platform-checked. The release workflow's `macos` and `windows` jobs build the shipped artifacts.

**Still for a human on real desktops** (Principle VI; the same results as the Linux pass):

- B1 with a right-click on Windows, and with Ctrl+click and a two-finger click on macOS.
- B6: Escape, empty-space click and other-card click close the menu with no entry run.
- B10: the keyboard path (Shift+F10 / the Menu key on Windows; Shift+F10 or the context-menu key on
  macOS keyboards that have one), Up/Down, Enter, and focus back on the card.
