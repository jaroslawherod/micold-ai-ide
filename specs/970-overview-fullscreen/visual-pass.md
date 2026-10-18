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
