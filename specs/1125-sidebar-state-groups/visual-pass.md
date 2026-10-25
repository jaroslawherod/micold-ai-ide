# Visual pass — quickstart Part B (T060)

The pass ran milestone by milestone, each against that milestone's debug build in a cloud
container (Xvfb, `ICED_BACKEND=tiny-skia`, xdotool), in light and, where marked, dark. Screenshots
and the per-run notes are under [visual-pass/](visual-pass/) (`m1-record.md` … `m5-record.md`).
This file collects the results against Part B's steps. The close unit (2026-10-25) re-ran no live
session; it compared the kept screenshots with the mockup board (B10).

| # | Result | Evidence |
|---|---|---|
| B1 | PASS: Overview row first with its icon; badge in the error container colours (showcase); chip bar under it: **All**, type chips, then add, hide, ⋯, wrapping with a gap. Live badge not seen (no agent session here); badge equality is tested | `m3-app-overview-row-first-light.png`, `m3-showcase-overview-rows-light.png`, `m4-app-chip-bar-light.png`, `m4-app-chip-bar-180px-light.png` |
| B2 | PASS for Idle (header "Idle · n", *Default* first) and Working (M5); Needs action and Done not seen live (no permission prompt, failing check or merged PR without GitHub); order and Done collapsed are tested | `m1-app-idle-header-open-closed-{light,dark}.png`, `m5-app-keyboard-moves-with-row-light.png` |
| B3 | PASS: two lines, branch then "no sessions"; a rename leads; *Default* reads "Default" / "project root · …"; no type or issue chip. An issue title as first line not seen live (no GitHub) | `m2-app-two-line-rows-light.png`, `m2-record.md` |
| B4 | PASS: revealed row and Overview row in the primary container, legible; while the Overview is open no location row looks selected. A selected session's row not seen live | `m2-app-revealed-row-selected-light.png`, `m3-app-overview-row-selected-{light,dark}.png`, `m3-app-revealed-row-overview-unselected-light.png` |
| B5 | PASS: **All** and type chips filter; ⋯ menu by pointer and by keyboard, its three items, Escape closes it, *Attach existing…* opens the dialog | `m4-app-more-menu-*.png`, `m4-app-attach-chosen-light.png` |
| B6 | PASS: Tab order Overview row, **All**, chips, actions, header, rows; Enter/Space toggle a header; chords move the ring. A chord into a collapsed Done not seen live (no Done row without a merged PR); tested in `navigation_apply` | `m5-app-tab-order-light.png`, `m5-app-header-enter-space-light.png` |
| B7 | PASS: a row with the keyboard moves to Working and back with the ring on it; counts update; a moved row drops its hover action | `m5-app-keyboard-moves-with-row-light.png`, `m5-app-hover-drops-on-move-light.png` |
| B8 | PASS: at 180 px both lines ellipsize, chips wrap, nothing overlaps; widest shows full titles | `m2-app-sidebar-narrow-light.png`, `m2-app-sidebar-wide-light.png`, `m4-app-chip-bar-180px-light.png` |
| B9 | PASS in dark for B1 (chip bar, menu), B2, B3/B4 (selected rows, Overview row) | `m1-…-dark.png`, `m2-app-rows-selected-dark.png`, `m3-app-overview-row-selected-dark.png`, `m4-app-more-menu-filtered-dark.png` |
| B10 | Matches the mockup's structure; differences below | the board's sidebar (`Main.dc.html`, "Proposal: session view") |

## B10: against the mockup board

Same: the Overview row first with its badge chip ("2 need action", error container); a chip row of
**All** plus type chips with add and hide at its end; sentence-case headers "Needs action · n",
"Working · n", "Idle · n"; two-line rows, title then `fix/1117 · 1 session running`; *Default*
reading "Default" / "project root"; the selected row in the primary container.

Different, each by the spec:

- **Done** is a collapsed group in the list, not a footer pinned to the sidebar's foot (research R8).
- A ⋯ action follows add and hide: it holds refresh, attach and run-in-parallel, which the old
  header's buttons held (FR-015).
- The second line reads branch and sessions only. The board's extras (`PR #1112 · 6/10 tasks`,
  `writing tests`, `waiting for you`) come from feature 1120's text, which has no task or activity
  phrase for the sidebar (FR-011).
- *Default* comes first in its group (FR-003 keeps today's order); the board lists it after a
  worktree.
- The Overview row's fill is the primary container only while the Overview is open (FR-009); the
  board draws it tinted at rest.

Not seen live in any run, for want of GitHub or an agent here: an issue title, a Needs action or
Done row, a live badge, a session's own selected row. Each is covered by the tests named in the
records.
