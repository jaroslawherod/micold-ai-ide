# Visual pass, M1 (T013)

**Date**: 2026-10-05. **Where**: Linux container, Xvfb `:77` 1600x1400, Mesa lavapipe (software Vulkan), no window manager; not a real display or GPU. **Binary**: `micold-showcase` built from HEAD `acc37dfa` (code tree identical to `d8ad1e5b`), pinned to a private run directory before launch.

| Row | Verdict | Evidence |
|---|---|---|
| B10 | PASS (light and dark) | `b10-location-rows-light-dark.png` (light, red border, above dark, blue border; identical crop geometry), `b10-count-light-dark-2x.png` (the count column at 2x, light left, dark right). The collapsed and the expanded worktree row each end in `● 2`; the row with none shows nothing; the location name keeps the same weight in all three; each child session row keeps its own mark; the dot and the numeral do not collide; both read clearly against the surface in both schemes. |
| B1-B5, B7, B8 | NOT RUN | These need the real client and daemon with a seeded project whose agent sessions finish turns (a stub provider), a second window (B7) and a restart (B8). Not attempted in this unit; the behaviour they check is covered by the automated `crates/micold-client/tests/sidebar_attention.rs` state tests and the tree_view geometry tests, not by eyes. Left for the close unit or a person (quickstart §B). |

Not covered by any screenshot here: hover fade-in of row actions beside the count (B3), the ellipsis at the minimum sidebar width (B4), live update latency (B1, B5, B7).
