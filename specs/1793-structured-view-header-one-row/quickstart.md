# Quickstart: feature 1793

## Part A: automated
`scripts/build-lock.sh cargo test -p micold-core header_bar`; `-p micold-client features_header_bar`; gates `header_row_fits`, `top_bar_collapse`; then `mise run gate`.

## Part B: visual pass (`visual-pass` skill)
1. Wide window, session selected with 3 hidden events: one 40 px row, `3 hidden` chip, no title row; top bar order ring, status, Changes, `⋮`, settings, menu.
2. Hover ring: tooltip with tokens, cost, turns.
3. `+`: menu with two entries; terminal opens a new tab each press; Escape closes.
4. Worktree `⋮`: raw-events toggle and view control react.
5. Narrow window: ring text hides, then `Changes` label; tabs scroll; no overlap.
6. Overview: none of the four items.
