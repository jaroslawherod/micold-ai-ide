# Visual pass — feature 1024 (quickstart §B, steps 1–6)

Date: 2026-10-18. Branch `claude/project-thread-iq9j8q`. Run on **Xvfb :77 (1600×1400) with the
tiny-skia software renderer** (no Vulkan ICD, no xdotool, no sway in this container), not a real
display. Input was driven with a small Python XTest script (pointer, wheel, keys, window resize).
Binaries were built from this worktree and copied out of `target-shared/` before launch; the client
ran against a private data home and a private daemon (`client attached to daemon` logged).
Sources: showcase `micold-showcase` (CardGrid entry) and the real client `micold-ai-ide` with a
throw-away git repo of **29 worktrees + Default = 30 locations** (long names, in `.claude/worktrees`).

Grid width = window width − 56 px in the showcase; in the client with the sidebar open it is
window − ~330 px (sidebar ≈ 296 px), hidden ≈ window − 56 px. Column thresholds from `columns()`:
2 ≥ 576, 3 ≥ 872, 4 ≥ 1168, 5 ≥ 1464 px.

| § | Check | How | Result |
|---|---|---|---|
| 1 | Showcase "30 cards" at ≥ 1464 px grid: five equal cards per row | Window 1600 → grid 1544; card edges measured per pixel row: 28–324, 340–636, 652–948, 964–1260, 1276–1572 (five × 296 px, 16 gap) | PASS (`01`) |
| 1 | "3 cards": three at the left, same width as a five-row card | 3 cards at x 28…, same 296 px | PASS (`01`) |
| 1 | "narrow" (279 px): one column | one column, name ellipsised ("worktree-…") | PASS |
| 1 | Threshold at 1464 | Window 1530 (grid 1474): 5 cols of 282; window 1500 (grid 1444): 4 cols of 349 | PASS |
| 3 | Reflow 4→3→2→1, showcase | Window 1230 (grid 1174): 4; 1200 (1144): 3; 930 (874): 3; 900 (844): 2; 640 (584): 2; 620 (564): 1; 340 (284): 1. Equal widths in every row, no overlap seen; no horizontal scrollbar at 340 (bottom strip uniform) | PASS |
| 2 | Client Overview, 30 cards, sidebar hidden, grid ≈ 1544 px | 5 cols: 56–347, 363–654, 670–961, 977–1268, 1284–1576 (equal 291); six equal-height rows; long titles and paths ellipsised, no overlap | PASS (`05`) — see Not covered for 2400 px |
| 2 | Client, sidebar open (grid ≈ 1274 px) | 4 cols (`04`); 3 cols at window 1250, 2 at 1000, 1 at 700 (card edges measured; `07`) | PASS |
| 3 | Client shrink, never a sideways scrollbar | 1250/1000/700/450 px windows: no horizontal scrollbar; at 450 the sidebar leaves a 96 px content area, below the 280 px minimum card, one column still (this width is below the supported size) | PASS |
| 4 | **FR-008 — resize and sidebar-collapse re-layout** | Hid the sidebar with the Overview open: grid went from 4 to 5 columns; showing it again went back to 4. Window resize re-laid out the grid at every width above (showcase and client). Software rendering animates the sidebar slowly, so frames mid-slide were seen; settled frames are as stated. | PASS (`04`, `05`, `07`) |
| 5 | **FR-009 / SC-004 — keyboard focus order** | Overview open, 4 columns, keyboard on the cards: Tab walked Default → number-1 → number-10 → number-11 (along row 1, left to right), then number-12 → number-13 (row 2, left to right). Right arrow moved number-13 → number-14, then a following Down/Right frame showed number-15. Card order is the grid's visual order. Focus ring visible on each. | PASS for Tab and Right; Down arrow not isolated (lagging frames), see Findings (`06`) |
| 6 | **SC-004 — all 30 reachable by scrolling** | Showcase "30 cards": scrolled the grid to the end, rows worktree-20…24 and worktree-25…29 shown (30 cards, 6 rows of 5). Client: all 30 cards (Default + 29) visible as 6 rows in the 1400-px-tall window. | PASS (`03`) |

## Not covered

- Client at a **~2400 px grid**: the screen is 1600 px wide. Five columns were checked at 1544 px (client
  and showcase); "more text before the ellipsis than on `main`" was not compared against a `main` build.
- Down/Up arrow focus movement in isolation (the software renderer lags a frame behind input).
- Animation smoothness of the sidebar slide and a real GPU: not judgeable here.
- Showcase cards have no `on_press`, so are not focusable there; focus order was checked in the client only.

## Screenshots (this directory)

- `01-showcase-1600-grid.png` — showcase CardGrid, 1 / 3 / 30 cards at 1600 px (five columns).
- `03-showcase-30-cards-end-reachable.png` — last rows of the 30-card grid after scrolling (worktree-20…29).
- `04-client-overview-sidebar-open-4col.png` — client Overview, sidebar open, four columns, first card focused.
- `05-client-overview-sidebar-hidden-5col.png` — sidebar hidden, five columns.
- `06-client-overview-keyboard-focus-order.png` — focus ring walking Default, 1, 10, 11, 12, 13, then 14.
- `07-client-overview-narrow-1250-1000-700-450.png` — narrower client windows (top to bottom).

## Findings

- None blocking. One cosmetic remark: in the showcase's 320 px-tall 30-card container the grid's own
  vertical scrollbar overlays the right edge of the fifth column; it is the container's scrollbar, not the grid.
