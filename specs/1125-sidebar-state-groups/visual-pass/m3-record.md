# M3 visual pass — quickstart Part B 1 (Overview-row half) and 4

- **Date**: 2026-10-24
- **Where**: cloud container, Xvfb `1600x1400x24`, `ICED_BACKEND=tiny-skia`, driven with xdotool;
  client, daemon and showcase pinned from one debug build of this branch at 5d5b1577. The M2 scratch
  git project (*Default* and three managed worktrees, no sessions). Light, then dark (`"theme":
  "dark"` in the private data home's `settings.json`).
- **Harness caveat**: as in M1 and M2, frames were judged after a window resize forced a full repaint.

| Step | Check | Verdict | Evidence |
|---|---|---|---|
| B1 | The sidebar's first row is Overview with its icon, above the Worktrees header and the groups | PASS | `m3-app-overview-row-first-light.png` |
| B1 | The row's badge ("2 need action") in the error container colours, unselected and selected | PASS (showcase, light) | `m3-showcase-overview-rows-light.png` |
| B1 | The badge live, equal to the top bar's | NOT SEEN live (no agent session here, so nothing needs action) | `features::overview` tooltip and `ui::overview` tag-filter badge tests |
| B4 | Pressing the Overview row opens the Overview; the row fills with the primary container and no location row looks selected; its tooltip reads "Overview (Ctrl+Shift+O)" | PASS (both themes) | `m3-app-overview-row-selected-light.png`, `m3-app-overview-row-selected-dark.png` |
| B4 | Opening a card closes the Overview: the revealed location row is selected, the Overview row is not | PASS | `m3-app-revealed-row-overview-unselected-light.png` |
| B4 | A selected session's row | NOT RUN live (no agent session here) | `features_sidebar` `row_selected` tests |

The showcase's dark scheme was not captured (its toggle sits at the top of a page this harness
scrolls slowly); the dark app frame and `composition_contrast` cover the row's pairs in dark.
