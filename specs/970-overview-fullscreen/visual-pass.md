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
