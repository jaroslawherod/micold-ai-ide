# Visual pass — 968 quick-switcher, M1 (locations only)

Date 2026-10-18, branch claude/project-thread-ftdy5c. Xvfb + lavapipe (software Vulkan), not a real display.
Showcase poses ("Quick-switcher dialog", "…nothing matches") and the real client (fixture project P: Default,
five worktrees incl. one 90-char name, one `missing`; no sessions, no terminal pane).
Not run: B1's terminal focus / `^P` check (no session in the fixture), "current row marked" (no current location
in the fixture), B2 with other projects/sessions (M2/M3), dark theme (showcase toggle did not respond to xdotool).

| Section | Verdict | Screenshot |
|---|---|---|
| Showcase poses | PASS (D1 fixed, re-checked) | showcase-light.png |
| B1 open (chord, scrim, focused field, rows, first highlighted) | PASS (D1/D2 fixed, re-checked) | b1-open.png |
| B2 `logpg` (Login page only, letters emphasised) | PASS | b2-logpg.png |
| B4 `logout` Enter (row selected, ring, no session) | PASS | b4-logout-selected.png |
| B7 `zzzz` ("No matches", Enter keeps open) | PASS | b7-no-matches.png |
| B8 Escape / scrim click / chord close | PASS | b8-close-sequence.png |
| B9 very long name | PASS (D2 fixed, re-checked) | b1-open.png |
| B10 chord with project panel open | PASS | b10-chord-over-project-panel.png |

Defects
- D1 (FIXED, re-checked in showcase-light.png and b1-open.png: `Typeahead::in_flow`, the list is laid out inside the card
  and the card grows to hold it; caption and next pose are clear): the dialog card wraps only the filter field; the results list (and "No matches") floats below it, outside the
  card, with its own shadow, so the card's rounded bottom cuts across the list's top edge. In the showcase the list
  overflows the pose: it covers the pose caption ("Quick-switcher dialog" reads "Quic…") and overlaps the next pose.
- D2 (FIXED, re-checked in b1-open.png: `Row::single_line`, the long name is one line ending in "…"): a long worktree name wraps to two lines instead of one line cut with "…" (b1-open.png, row "This is an
  extremely long worktree name … in the row"); B9 expects one line with "…".
- Note: the hovered row and the highlighted row share one grey fill (Alpha hovered while Default highlighted in
  b1-open.png), so the keyboard highlight is ambiguous under the pointer.
