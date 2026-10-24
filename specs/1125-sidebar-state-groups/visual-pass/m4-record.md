# M4 visual pass — quickstart Part B 1 (chip-bar half), 5 and 8

- **Date**: 2026-10-24
- **Where**: cloud container, Xvfb, `ICED_BACKEND=tiny-skia`, driven with xdotool; client and daemon
  pinned from one debug build of this branch at 3eedd2e8 (`client attached to daemon` in the private
  data home's log). A scratch git project with *Default* and three managed worktrees
  (`fix/1117_ci-red`, `feat/chip-bar`, `docs/guide`), no sessions. Light, then dark (`"theme":
  "dark"` in the private data home's `settings.json`).
- **Harness caveat**: as in M1–M3, frames were judged after a window resize forced a full repaint.

| Step | Check | Verdict | Evidence |
|---|---|---|---|
| B1 | Under the Overview row, the chip bar: **All**, the type chips (feat, fix, docs, issue), then add, hide, ⋯ on the first line; chips wrap with a gap between lines | PASS | `m4-app-chip-bar-light.png` |
| B5 | ⋯ pressed: the menu hangs under its trigger; items *Refresh the worktree list*, *Attach existing…*, *Run in parallel…* in that order | PASS (both themes) | `m4-app-more-menu-pointer-light.png`, `m4-app-more-menu-filtered-dark.png` |
| B5 | Escape on the open menu closes it | PASS | (frame not kept) |
| B5 | Choosing *Attach existing…* closes the menu and opens the attach dialog, as the header button did | PASS | `m4-app-attach-chosen-light.png` |
| B5 | Choosing *Refresh the worktree list* closes the menu | PASS (a refresh of an unchanged listing shows nothing) | `sidebar_state` refresh tests |
| B5 | A type chip filters: *docs* active leaves Default and `docs/guide` (Idle · 2), **All** unselected | PASS (dark) | `m4-app-more-menu-filtered-dark.png` |
| B5 | ⋯ reached with Tab (ring on the trigger) and opened with Enter: the menu hangs from the sidebar's trailing edge at the chip bar's first line | PASS | `m4-app-more-menu-keyboard-light.png` |
| B8 | At 180 px: All and feat on the first line beside add, hide, ⋯; fix, docs, issue wrap one per line with gaps; rows ellipsize; nothing overlaps; the menu still opens under ⋯ | PASS | `m4-app-chip-bar-180px-light.png`, `m4-app-more-menu-180px-light.png` |

Observed, not a defect: while the pointer rests on ⋯ after opening the menu, the trigger's "More
actions" tooltip draws over the first item's icon until the pointer moves (the tooltip layer is
above the menu layer, as for every trigger).
