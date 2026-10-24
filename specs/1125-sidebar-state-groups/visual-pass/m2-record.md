# M2 visual pass — quickstart Part B 3, 4 (row half) and 8

- **Date**: 2026-10-24
- **Where**: cloud container, Xvfb `1600x1400x24`, `ICED_BACKEND=tiny-skia`, driven with xdotool;
  client and daemon pinned from one debug build of this branch at bf2c21df. A scratch git project
  with *Default* and three managed worktrees on conventional branches (`fix/1117_ci-red`,
  `feat/42_crash-when-opening-an-empty-project-with-a-very-long-name`, `chore/77_tidy-up`, the last
  renamed "my pet name" in the private data home), no sessions. Light, then dark (`"theme": "dark"`
  in the private data home's `settings.json`).
- **Harness caveat**: as in M1, tiny-skia under Xvfb repaints only damaged regions; frames were
  judged after a window resize forced a full repaint. The showcase's long page scrolled
  inconsistently under this harness, so its "two-line row" entries were not captured; the
  `tree_view` and `layout_snapshot` tests cover them.

| Step | Check | Verdict | Evidence |
|---|---|---|---|
| B3 | Each row has two lines: the branch (no issue title is known: `gh` is unauthenticated) then "No sessions"; a renamed worktree reads its rename, then `chore/77 · no sessions`; *Default* reads "Default" / "project root · no sessions"; no type or issue chip | PASS | `m2-app-two-line-rows-light.png` |
| B3 | First line an issue title | NOT SEEN live (no GitHub access here) | `features_sidebar` `two_line_rows`, `overview_text` tests |
| B4 | The row the Overview revealed is filled with the primary container, both lines legible in `on_primary_container` | PASS (both themes) | `m2-app-revealed-row-selected-light.png`, `m2-app-rows-selected-dark.png` |
| B4 | A selected session's row | NOT RUN live (no agent session here) | `tree_view` `primary_selection_*` tests |
| B8 | Narrowest sidebar: both lines ellipsize, chips and actions stay, nothing overlaps; widest: full titles, chips at the row's end | PASS | `m2-app-sidebar-narrow-light.png`, `m2-app-sidebar-wide-light.png` |

The Overview card of the renamed worktree still reads its branch (feature 1120's `card_title`);
recorded in the ledger's *Follow-ups not done*.
