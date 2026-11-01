# Contract: effective sidebar state

## Reducer (render-free, `tests/sidebar_state.rs`, `tests/features_window.rs`)

Each row is a sequence applied to a fresh `State`; expected is `sidebar_hidden()` after the last step
and `(hidden, width)` unchanged unless the sequence has a toggle at >= 720.

| Sequence | Expected |
|---|---|
| resize 1000x700 | shown |
| resize 640x480 | collapsed |
| resize 1000, 640, 720 | collapsed then shown (US2 s1, s2) |
| hide at 1000, resize 640, resize 1000 | collapsed at both (US2 s3) |
| first report 640 | collapsed, `hidden == false` (US2 s4) |
| resize 640, toggle | shown (US3 s1); resize 650 still shown |
| resize 640, toggle, resize 720 | shown as `hidden` says; `compact_open == false` (US3 s2) |
| ... then resize 640 | collapsed (fresh crossing) |
| resize 640, toggle, toggle | collapsed (US3 s3) |
| report 0x0 | shown (not compact) |
| toggle at 1000 | flips `hidden` |
| resize 640 then toggle in one update order | toggle sees the compact flag |
| resize 1000x700 -> 719 -> 720 | boundary: 719 compact, 720 not |
| two `State`s, 640 and 1000 | independent |

## Full screen (`tests/overview.rs`)

Full screen at 640: `drawer_flags().retracted`, closing it: `sidebar_hidden()` true. Full screen
at 1000 with the sidebar expanded: closing returns expanded; `hidden`/`width` unchanged.

## Geometry (`tests/gates/review_tab_fits.rs`)

At `MIN_WINDOW_SIZE`: the sidebar drawer is the 32px strip and the main area is 608px wide; at
`WINDOW` the sidebar is expanded. Per state, no escapes beyond the declared overhangs
(`SMALLEST_WINDOW` has no `#1474` entries, SC-001); the tree filter box and rows lie inside the tree
pane (FR-010); `drawer::width` stays at most the area and, for areas >= 2 x 360, equals today's.
