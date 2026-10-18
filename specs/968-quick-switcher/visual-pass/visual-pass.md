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

## M3 (other projects in the switcher)

Date 2026-10-18, Xvfb + lavapipe, real client with a private data dir. Fixture: P active (Default and Login page each with a
session, six other worktrees, one `missing`), Q available (1 running: a fake `claude` CLI on PATH that echoes and sleeps; the
second running session could not be made, the `+` in the terminal strip opens a plain shell tab, not a session), R unavailable
(folder moved away). Opened via Ctrl+Shift+P; mouse parked away from the list so hover does not mimic the highlight.

| Item | Verdict | Screenshot |
|---|---|---|
| B2 `logpg` with a session present | PASS: Login page highlighted, "Log" "p" "ag" emphasised, its session sub-row follows | b2-logpg-sessions.png |
| B3 Down, Enter on a session sub-row | PASS (reduced): Enter selected the Login page session and the terminal got the keyboard ring. `helper-a`/`helper-b` not built, so "Down twice, helper-b" not run | none |
| B5 Down to the end | PASS: highlight stops on Q after 28 Downs; R reads "Project · 0 running · 0 unread · unavailable", dimmed, never highlighted; click on R did nothing, switcher stayed open | b5-q-highlight-r-unavailable.png |
| B6 Enter on Q | PASS: Q active, its last view (shell tab) restored; reopening lists P "2 running", unchanged (Default and Login page sessions) | b6-switcher-after-q.png |
| B11 session started elsewhere while open | not run: a second window on the same project is read-only (or takes the project over), so no session could be started from it | none |
| B12 dark and light | PASS (real client): rows, project rows, highlight, current-session tint, disabled R legible in both | b12-light.png, b12-dark.png |
| B12 showcase pose with the `Archive` row | not run | none |

Defects
- D3 (FIXED, rechecked 2026-10-18 with the fixed binary): 28 Downs now scroll the list so the highlighted row (Q) sits in view
  at the bottom (b5-scroll-follows.png). Typing a query after scrolling down puts the highlight on the first row, visible
  at the top; the fixture's queries only left 2-5 rows, so a query leaving an overflowing list was not exercised.
- Note: opening the switcher puts the highlight on Default, but the current location (Login page) also carries a grey fill
  similar to the highlight when the pointer is over it; with the pointer away it is clear.
