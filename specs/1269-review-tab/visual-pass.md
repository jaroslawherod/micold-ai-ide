# Visual pass: 1269 (M4, T041)

Quickstart Part B, run by hand on the M4 branch (`1269-m4`): debug client on Xvfb `:77`,
`ICED_BACKEND=tiny-skia` (CPU rasteriser), driven with XTest. Project `repo` with worktrees `W`
(changes in two files, two drafts), `X` (one session) and `Z` (no session). Window 1280×800, light
then dark, and 640×480.

Screenshots are in the project files under `issue-1269/` (`/mnt/project-files/issue-1269/m4-*.png`),
not in `specs/1269-review-tab/visual-pass/`: the cloud container keeps them out of the repository.

| # | Result | Screenshots |
|---|---|---|
| B1 | pass: the Review tab is last in the strip, shown and underlined; header and strip above the view; Changes drawn selected, tooltip *The review is shown*; split buttons and restart disabled | `m4-b1-review-shown-light-1280.png`, `m4-b1-review-shown-dark-1280.png` |
| B2 | pass: file selected, diff scrolled, composer typed, lines picked | `m4-b2-review-before-switch-light-1280.png` |
| B3 | pass: the session tab shows the terminal; the Review tab brings the review back as left | `m4-b3-session-shown-light-1280.png` |
| B4 | pass: **Changes** while the review is shown changes nothing (selected file kept) | `m4-b4-changes-while-shown-dark-1280.png` |
| B5 | pass: hidden tab reads *Review* with badge `2` and the dot; hover *Review · 2 drafts / Changed since you looked* | `m4-b5-hidden-badge-mark-tooltip-light-1280.png`, `m4-b5-hidden-badge-mark-tooltip-dark-1280.png` |
| B6 | pass: the edited file re-read, dot gone, file, composer and pick kept | `m4-b6-review-back-kept-light-1280.png` |
| B7 | pass: `X` shows no Review tab; **Review changes** on `W` shows `W` with its Review tab shown, kept | `m4-b7-review-changes-on-w-light-1280.png` |
| B8 | pass: `Z`'s own header, strip of only *Review* and **New session**, review shown | `m4-b8-sessionless-z-light-1280.png`, `m4-b8-sessionless-z-dark-1280.png` |
| B9 | pass for the Overview: opened and closed, the review returns unchanged; Settings not run by hand (the same cover path, `uncovered`, is under test) | `m4-b9-overview-open-dark-1280.png`, `m4-b9-overview-closed-review-kept-dark-1280.png` |
| B10 | pass: × on `W` removes the tab and shows the session shown before; **Changes** then starts fresh (no file selected) with the two drafts and badge kept. On sessionless `Z` the × removes the tab and shows the project page | `m4-b10-hidden-on-w-dark-1280.png`, `m4-b10-closed-returns-to-session-dark-1280.png`, `m4-b10-reopened-fresh-badge-kept-dark-1280.png`, `m4-b10-close-sessionless-dark-1280.png` |
| B11 | pass after two fixes (findings 1 and 2): at 640×480 the strip scrolls; the Review tab's label, badge, dot and × are whole with the sidebar collapsed; with the 300 px sidebar the viewport (~150 px) is narrower than any tab, which now scrolls to its leading edge. Twelve tabs at 1280×800 not opened by hand: `strip_with` covered states and `session_area_fits` gate them | `m4-b11-hidden-badge-mark-light-640.png`, `m4-b11-strip-scrolled-light-640.png`, `m4-b11-sidebar-collapsed-light-640.png`, `m4-b11-reveal-after-location-change-dark-640.png` |
| B12 | not run by hand: chords with Review shown are covered by `terminal_focus.rs`, `keyboard.rs`, `navigation.rs` and the app chord tests (SC-004) | — |
| B13 | pass: B1, B5, B8, B10 and B11 (640) in dark, all text legible | as above, `-dark-` |

## Findings

1. **A tab wider than the strip's viewport was revealed at its trailing end** (640×480 with the
   sidebar open). `scroll_into_view` brought the trailing edge into view, which left the leading
   slot — where the Review tab's changed dot and a session tab's stopped mark are drawn — under the
   leading fade. Fixed: a member wider than the viewport is revealed at its leading edge
   (`session_strip::scroll_into_view`, unit test extended).
2. **After a location change the marked tab was not revealed** (X → W by **Review changes**, or
   from the project page): the reveal measured from `tab_strip_scroll_offset`, which still held the
   left strip's offset, so `scroll_into_view` found nothing to do and the new strip kept the old
   one's scroll (a leading accent fade on `Z`). Fixed: `follow_header` arms the reveal with
   `tab_reveal_from_start` when the shown location changes, and `main.rs::tab_reveal_scroll` then
   measures from the start and always scrolls (`tab_reveal.rs`
   `a_location_change_reveals_from_the_strips_start`, red by mutation).
3. The 640×480 window with the sidebar open leaves the strip narrower than one tab (feature 1126's
   layout, not this feature's); the tab scrolls and is whole with the sidebar collapsed. Left as a
   follow-up.
