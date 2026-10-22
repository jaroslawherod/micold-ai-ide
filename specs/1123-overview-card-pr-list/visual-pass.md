# Visual pass: pull request rows (quickstart §B)

The whole quickstart §B result, from the visual passes of T019 (M1), T028 (M2) and T037 (M3). Each ran
the real client against a scripted pull request source (`FakePullRequestSource`) through an
`autopilot-worker` with the `visual-pass` skill. Evidence is in `visual-pass/m1/`, `visual-pass/m2/`
and `visual-pass/m3/`. Round folders `r2/` and `r3/` hold the re-runs after fixes. Verdicts are quoted
in full in `autopilot.md`, *Review rounds*.

| Step | What was checked | Result | Rounds | Evidence |
|---|---|---|---|---|
| §B1 | `feat-1123_x`: "#12 Open checks failing", "#9 Merged", counts "1 open · 1 merged" | PASS | M1 r1 | `m1/*-b1-*` |
| §B2 | `feat-1123_y`: two rows and "+2 more" while collapsed, four rows and no "+N more" when expanded, collapsed again after the pointer leaves; `feat-1123_z`: no row and no counts | PASS | M1 r1 | `m1/*-b2-*` |
| §B3 | A click on "#12" or "#11" opens exactly that address with no ripple and no selection, and Overview stays. A click on the pill activates the card. A failing opener shows a toast. The top pixel of a number opens it, and 3 px above it activates the card | PASS | M2 r1 (one MINOR: a hover underline repaints late under software rendering, not a defect), M2 r2 (number hit area after review A F2) | `m2/`, `m2/r2/` |
| §B4 | Keyboard only. Tab and Shift+Tab walk every number in order and leave the card at its ends, and Shift+Tab from the next card lands on #11, the expanded card's last number. Enter or Space opens a number. Arrows step from a number to the neighbouring card. Escape collapses the card, and the card keeps the keyboard. The Menu key or Shift+F10 opens the card menu, and closing it returns to the number | PASS | M3 r1 FAIL (Tab skipped #11, Right from #12 stayed on its own card), M3 r2 FAIL (Shift+Tab from z landed on #13), M3 r3 PASS after the fixes in *Fixes M3 r1/r2* | `m3/`, `m3/r2/`, `m3/r3/` |
| §B5 | While rate-limited and while offline the rows stay. Unavailable or switched off shows the dash with no rows, and its reason is the tooltip on both the collapsed and the expanded card (D13) | PASS | M1 r1 (one MINOR: no tooltip on the expanded card, fixed test-first), M1 r2 | `m1/light-1-b5-*`, `m1/r2-*` |
| §B6 | At 280, 380 and 480 px, light and dark, scale 1.0 and 2.0, numbers and pills are drawn whole | PASS | M1 r1, plus 380 px in M2 r1 | `m1/*-b6-*`, `m2/*-380px-*` |

Partly run: the Page Up and Page Down keys on a number (§B4, c7) had no overflow to scroll in the fixture
window, so they were only partly exercised (`m3/light-1-b4-c7-*`). The keys' wiring on a number is covered by
`overview_card.rs` `keys_on_a_focused_number_open_it_its_menu_and_page_the_card`.
