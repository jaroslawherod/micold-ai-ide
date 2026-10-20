# Visual pass — M1 (Overview full screen), quickstart Part B items 1, 2, 4, 5, 8

Run 2026-10-18 on a private Xvfb (1400x1000), `ICED_BACKEND=tiny-skia` (no Vulkan in the cloud
container), driven with XTest through ctypes. Fixture: one project, one location (the project root),
so the grid has a single card. Items 3, 6, 7 belong to M2–M4 (zoom, anchor, reduce motion): skipped.

**Limits of this harness.** Software rendering draws about one frame per input event, so the
slide timing (≤ 500 ms) and mid-motion frames cannot be judged; a slide took 3–6 s of wall clock
(10). Partial redraws leave stale grey tint over the sidebar (13) that a forced full redraw clears
(12); that is the backend, not judged. A second project was not available, so "switch project" in
item 4 was not run. The grid has one card, so "no overlapping cards" was not judged.

| Item | Result |
|---|---|
| 1 Full screen: sidebar fully retracts, Overview fills width | PASS: no gap or strip at rest (02) |
| 2 Button text and icon flip, tooltip names Ctrl+Shift+F | PASS (02, 03); Tab reaches it with a focus ring (04) |
| 2 Space toggles it | NOT CONFIRMED: see finding 1 |
| Ctrl+Shift+F chord enters and leaves | PASS |
| 4 Settings open/close keeps full screen | PASS (05) |
| 5 Exit by top-bar button, Ctrl+Shift+O, card | PASS: sidebar returns as it was: expanded (06, 07), rail (08), 420 px (09) |
| 5 Exit by Escape | FAIL: see finding 2 |
| 8 Showcase drawer "retracted"/"retracting" | FAIL: nothing renders, see finding 3 |

## Findings

1. Tab order — in full screen it took 5 Tabs to reach the header button (04), so the retracted
   sidebar's four controls still take focus; the sixth Tab plus Space then pressed the Default row's
   "+" in the invisible sidebar (opened its agent menu and left the Overview). Retracted drawer
   content should not be focusable. (Evidence 04; the count is from one run.)
2. Escape does not leave the Overview, in full screen or windowed, with a card focused. FR-010 and
   813 FR say it must. `grep` finds no Overview handler for Escape (`EscapePressed` only reaches the
   overlay registry, which holds the card menu only). It may predate 970, but the spec claims it.
3. Showcase NavigationDrawer section (`showcase/sections/surfaces.rs:704`) draws no poses: the
   heading and caption are followed by an empty gap, then TerminalPane (11). "open", "closed",
   "retracted" and "retracting" are all absent.

## Screenshots
`specs/970-overview-fullscreen/visual-pass/` 01–13.

## Resolution (M1 verify unit)

1. Fixed: `NavigationDrawer::operate` now reaches no child from the moment `retracted` is set, not
   only once settled (`a_retracting_drawer_takes_no_focus`). The software renderer stretched the
   slide to seconds, so the Tabs landed mid-retraction, before the drawer was `gone`.
2. Fixed (pre-existing on main, not 970 code): `Message::EscapePressed` asked only the overlay
   registry, so with nothing floating Escape neither closed the Overview nor dropped a Settings
   draft. `App::dismiss_topmost` now applies `on_escape`'s whole answer
   (`pressing_escape_closes_the_overview`).
3. Not fixed here: the section's pre-existing "open" and "closed" poses are blank too, and the
   drawer's `size()` (`Shrink` × `Fill`) is unchanged from main, so the blank section predates 970.
   Recorded as a follow-up in the ledger; the retraction poses are covered by the drawer's geometry
   tests (`a_retracted_drawer_settles_…`, `a_retracting_drawer_narrows_…`).

# Visual pass — M2 (cards zoom with the sidebar), quickstart Part B items 1, 3, 8

Run 2026-10-19 on a private Xvfb, `ICED_BACKEND=tiny-skia`, binaries copied out of `target-shared/`
(client, daemon, showcase built from HEAD 11cf7bf6). Fixture: one git project with 12 worktrees under
`.claude/worktrees/` plus the project root = 13 cards. Window 1400x1000 (3 columns windowed, 4 columns
full screen), and 900x600 (2 -> 1 column) to get more frames out of the slow renderer. Items 2, 4-7 are
M1 (passed) or later milestones (anchor, reduce motion): skipped.

**Limits of this harness.** The software renderer draws about one frame per input event or a few per
second, so the 500 ms zoom shows 0-3 intermediate frames. In none of them did a row hold a fractional
number of cards: the first changed frame after the click already shows whole columns. The ribbon's
"card split at a row end drawn in two pieces" was therefore never seen on screen (covered by the
`ribbon` unit tests only). Grey veils and a stale strip of the retracted sidebar appear until a
redraw (known from M1, not judged).

| Item | Result |
|---|---|
| 1 At rest in full screen the rows span the freed width, no gap | PASS: 4 columns, margins left about 25 px and right about 22 px, no strip after a redraw (21) vs 3 columns windowed (20) |
| 1 Mid-zoom: no overlap, tearing, card blinking out | PASS on the frames obtained (22, 23, 24): cards sit on an even grid, grid edge follows the sidebar edge with no gap, 13 cards present in every frame. Ribbon sliding itself not observed (limits above) |
| 1 After the zoom the layout equals the at-rest layout | PASS: exit settles on the 3-column windowed layout of 20 (26); entry settles on 21 |
| 3 Ctrl+Shift+F mid-zoom reverses | PASS in 3 of 4 runs: reversal settles on the right end state (25). One run showed an extra toggle (Full screen, Exit, Full screen) that two repeats with the same protocol did not reproduce; chord effects arrive seconds late under this renderer, so not judged a bug |
| 8 Showcase CardGrid "zoom — press it" pose | PASS: renders at rest (drawer 200 px, 3 columns), press retracts the drawer and the grid widens to 4 columns, no overlap, settles clean (27). "retracted"/"retracting" drawer poses: M1 finding 3, not re-run |

## Findings

None that fail the criteria. Open for a real display: record the 3 -> 4 column mid-frames, where the
ribbon splits a card across a row end; this harness cannot show them.

## Screenshots
`specs/970-overview-fullscreen/visual-pass/` 20-27.

# Visual pass — M3 (scroll position survives the zoom), quickstart Part B item 7

Run 2026-10-19 on a private Xvfb, `ICED_BACKEND=tiny-skia`, client and daemon built from the uncommitted
working tree and copied out of `target-shared/`. Fixture: one git repo with 29 worktrees
(`feat/wt-01`..`wt-29`) plus the project root = 30 cards, window 1400x1000 (3 columns windowed, 4 full
screen). Scrolled with the wheel (60 px a notch) to the bottom of the grid; card 20 (`feat/wt-19`, middle
column) sits in the first fully visible row. The wheel cannot put a card at the very top of a grid
whose last rows end the page, so "first visible" is the first full row, not the first card. Entered
full screen with the button, not the chord (the chord was not run for this item).

| Item | Result |
|---|---|
| 7 Card 20 of 30 visible after entering full screen | PASS: after the zoom settles (12 s) `feat/wt-19` is top right of the 4-column grid, still in the first visible row (`30-m3-fullscreen-settled.png`; before: `30-m3-before-scrolled-windowed.png`) |
| 7 No cards overlap | PASS: settled frame is an even 4x grid with no overlap; one mid-zoom frame (`30-m3-fullscreen-mid.png`) shows cards sliding left under the retracting sidebar, none overlapping, card 20 present |
| 7 Exit full screen keeps card 20 visible | not observed: only entering was driven |
| 7 Card 20 as the very first card in the viewport | not observed: see above; the scroll offset in px was not read, only the visible rows |

Screenshots: `specs/970-overview-fullscreen/visual-pass/30-m3-*.png` (downscaled to 60%).

# Visual pass — M4 (Reduce motion), quickstart Part B item 6

Run 2026-10-20 on a private Xvfb (1400x1000), `ICED_BACKEND=tiny-skia`, client and daemon built from the
working tree (HEAD 1c94608e + uncommitted M4) and copied out of `target-shared/`, own `XDG_DATA_HOME`.
Fixture: one git project, 12 worktrees + the root = 13 cards (3 columns windowed, 4 full screen).

**Method for "first frame".** The renderer applies an input seconds after it arrives, so a single
capture right after a key shows the old state. Instead: send the chord, capture 40-60 frames back to
back (about 0.15 s apart), and compare each with a windowed and a full-screen reference
(`compare -fuzz 3%`). One step means every frame is exactly one reference or the other.

| Check | Result |
|---|---|
| (a) Appearance page: Theme select + *Reduce motion* checkbox, note "Switch the Overview's full-screen zoom in one step", unchecked by default, no overlap/clipping | PASS (20) |
| (b) On + Save (`settings.json` gains `"reduce_motion": true`): enter with Ctrl+Shift+F | PASS: 9 frames exactly windowed, then every later frame exactly the full-screen layout (sidebar gone, 4 columns); none in between (28, 29) |
| (b) Leave with Ctrl+Shift+F | PASS: 30 frames full screen, then 10 frames exactly windowed; none in between |
| (b) Entry via the header button with it on | not run (chord only) |
| (c) Off: zoom still animates | PASS, weakly: with it off, the first six captures after the button press (about 1 s) were unchanged, then a capture at 12 s showed a torn mid-zoom frame (cards offset, clipped at the left, 24), then settled (25). With it on, no such frame in 100 captures. Smoothness not judgeable |
| (d) Tick, Cancel, reopen Settings | PASS: Cancel closes Settings, the checkbox is unchecked again (21, 22) |

## Findings

None. Not judged: the sidebar's own hide/show still sliding with it on (item 6, second half), and
whether the zoom is visibly smooth with it off.

## Screenshots
`specs/970-overview-fullscreen/visual-pass/` 20-29 (23-29 downscaled to 50% / 40%).


# Visual pass — close (T042)

Run 2026-10-20, private Xvfb (1400x1000 window), `ICED_BACKEND=tiny-skia`, XTest via ctypes, binaries
copied from `target-shared/debug` (built after the last source edit; no rebuild was possible in this
session). Fixtures: project "fixture" (13 cards) and a second empty project "second" (1 card). Waited
~12 s after each click. Not repeating the M1–M4 passes.

| Item | Result |
|---|---|
| 1 Top bar in full screen (FR-003): project switcher opens, switching project keeps full screen; ⋮ menu opens, About dialog opens/closes over it; Overview button toggles back | PASS (40-close-topbar-fullscreen-project-switched, 40-close-overview-toggle-back) |
| 2 Sidebar and Overview edges touch, no gap (SC-002) | PASS at rest in windowed Overview and on the first frame after leaving full screen: sidebar edge, 1 px divider, page background, no strip (40-close-edge-flush-zoom). Mid-slide frames: see not observed |
| 3 Connection lost in full screen (daemon killed, respawn blocked) | PASS: red banner "Not connected to the session service" sits under the top bar and pushes the Overview down, no overlap; "Exit full screen" and "Full screen" still work and the sidebar returns/leaves; Overview button still toggles (40-close-conn-lost-fullscreen) |
| 4 Very narrow (650 px) windowed | PASS: Overview shows 1 column, no clipping or overlap, top bar fits |
| 4 Very narrow (650 px) full screen | PASS: 2 columns, no clipping, button and top bar fit (40-close-narrow-650-windowed-and-fullscreen, left windowed, right full screen) |
| 5 Showcase NavigationDrawer section | FAIL (still blank): heading and caption ("posed: open, closed (the rail), retracted … retracting") then an empty gap before TerminalPane; none of the four poses draws (40-close-drawer-section-blank). Unchanged after 30 s and many pointer moves, so not render lag |

## Findings

1. The showcase NavigationDrawer section still draws nothing (follow-up from M1 open). All four poses
   are missing, including the long-standing "open" and "closed", so it is not specific to the 970
   `retracted` poses; the section's code is at `surfaces.rs:820`. Cause not diagnosed.
2. Connection-lost banner takes about 40 px of height in full screen; the Overview and the
   Exit/Full-screen button move down by that amount. Expected, noted for completeness.

## Not observed

- Mid-slide frames for SC-002: the renderer drew no intermediate frames in 12 captures 0.6 s apart,
  so "no gap on every frame" is judged on rest frames and the first frame after exit only.
- Narrow window at widths other than 650 px (600 and 700 not tried); the sidebar at 420 px wide in a
  narrow window.
- Reconnect after the daemon returns (it was blocked on purpose; the client was restarted for item 4).
- Grey veils, ghost menu rectangles and stale tint after menus close are the software backend's
  partial redraw (as in M1 finding 13), not judged.

## Screenshots
`specs/970-overview-fullscreen/visual-pass/` 40-close-*.png.
