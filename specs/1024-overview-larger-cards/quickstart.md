# Quickstart: Overview with larger cards, five per row

## A — Automated

```bash
mise run gate            # in this container: scripts/gate.sh
cargo test -p micold-client card_grid   # just the grid's gates
```

Expected: the `card_grid.rs` tests listed in [contracts/ui.md](contracts/ui.md) pass, and
`tests/showcase_completeness.rs` still finds the four `CardGrid` poses.

## B — Visual pass (visual-pass skill)

Prerequisite: a project with at least twelve locations (worktrees plus Default), or the component
showcase's `CardGrid` entry.

1. Showcase, `CardGrid`, "30 cards" at a window wide enough for ≥ 1464 px of grid: rows of five
   equal cards, wider than before; "3 cards": three cards at the left, each as wide as a card in a
   five-card row; "narrow": one column.
2. Client, Overview open, window so the grid is ~2400 px: five per row; a long task summary shows
   more text before the ellipsis than on `main`; no overlap; all cards the same height.
3. Shrink the window step by step to ~1460 px grid width: four per row appears at the threshold;
   down to ~300 px: three, two, one per row; never a sideways scrollbar; no overlap.
4. Collapse and re-open the sidebar with the Overview open: the grid re-lays out at once.
5. Tab and the arrow keys from the first card: focus walks left to right along a row, then to the
   next row.
6. Scroll to the end with 30 locations (or the showcase's 30 cards): every card reachable.

Record the pass in `specs/1024-overview-larger-cards/visual-pass.md`.

## C — Platforms

CI runs the gate on Linux, macOS and Windows. Widths are logical pixels, so the thresholds are the
same at any display scale factor.
