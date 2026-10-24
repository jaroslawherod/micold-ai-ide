# M1 visual pass — quickstart Part B 2 and 6 (header half)

- **Date**: 2026-10-24
- **Where**: cloud container, Xvfb `1600x1400x24`, `ICED_BACKEND=tiny-skia`, driven with xdotool;
  client, daemon and showcase pinned from one debug build of this branch at 31fe07e6 (daemon log:
  `client attached to daemon`). A scratch git project with *Default* and three managed worktrees
  (`.claude/worktrees/`), no sessions. Light, then dark (`"theme": "dark"` in the private data
  home's `settings.json`).
- **Harness caveat**: tiny-skia under Xvfb repaints only damaged regions, so after a press the
  sidebar showed grey patches; a 1 px window resize forced a full repaint, and only those frames
  were judged.

| Step | Check | Verdict | Evidence |
|---|---|---|---|
| — | Showcase TreeView: "group header, open" (label "Working · 2", twisty, its rows below) and "group header, closed" (no rows) | PASS (both themes) | `m1-showcase-group-header-{light,dark}.png` |
| B2 | App: one "Idle · 4" header above *Default* and the three worktrees, *Default* first | PASS (both themes) | `m1-app-idle-header-open-closed-{light,dark}.png` (left) |
| B6 | Pressing the header collapses the group; the count stays; pressing again expands it | PASS (both themes) | same files (right) |
| B2 | Needs action, Working and Done groups, Done collapsed at start | NOT SEEN live (no agent session, permission prompt or merged PR here; `gh` is unauthenticated) | `sidebar_state`, `features_sidebar` (`state_groups`) tests |
| B6 | Enter/Space on a header; a chord into collapsed Done | NOT RUN live | `tree_view` `enter_and_space_toggle_a_heading_that_has_the_keyboard`; `navigation_apply` `into_a_closed_group` |

The twisty glyphs (an up arrow open, a folder closed) are `TreeView`'s existing twisty for every
expandable row, not this milestone's.
