# Research: Overview with larger cards, five per row

No `NEEDS CLARIFICATION` remained in the Technical Context. The two decisions below fix how the
layout is built.

## R1 — Where the cap lives

- **Decision**: In the shared `CardGrid` (`crates/micold-client/src/ui/material/card_grid.rs`): a
  `MAX_COLUMNS = 5` default, a `max_columns(usize)` builder, and a `max` argument on the pure
  `columns` function: `clamp(floor((w + gap) / (min + gap)), 1, max)`.
- **Rationale**: FR-011 wants the Overview and the showcase to show the same layout; both build
  `CardGrid::new(..)` with the defaults, so a default in the component reaches both with no call
  site edit. A pure `columns` keeps the rule unit-testable without a layout pass (Principle I). The
  builder keeps the component general (Principle VIII) and lets a test prove the bound is honoured.
- **Alternatives considered**:
  - Cap at the Overview call site (`ui/overview.rs` passes `.max_columns(5)`): the showcase would
    then show a different layout unless it repeated the value; rejected by FR-011.
  - Derive a larger minimum width so that only five fit (e.g. `min = w/5`): changes the reflow
    threshold below five columns, which FR-003 keeps at 280 px.
  - A fixed card width (wider cards, but still as many as fit): would not give "five per row" on
    wide panes and leaves dead space; rejected by FR-002 / FR-004.

## R2 — Card width and partial rows

- **Decision**: Keep feature 813's `rows`: each card is `Length::Fill` in a `row` with `gap` spacing,
  and a partial row is padded with `Space` of `FillPortion(1)` up to the column count.
- **Rationale**: With `n` columns every card already gets `(w − (n − 1)·gap) / n` and a partial row
  (the last row, or fewer cards than columns) keeps that width at the start of the row, which is
  exactly FR-004, Story 1 #3–#4 and the "One location" edge case. Height comes from
  `OverviewCard`'s fixed `HEIGHT`, so FR-006 holds with no change.
- **Numbers** (min 280, gap 16; the grid gets the main pane less the Overview's 2 × 24 px padding):
  five-column threshold `5·280 + 4·16 = 1464` px of grid width. At 2400 px of grid width feature
  813 shows `floor(2416 / 296) = 8` cards of `(2400 − 112) / 8 = 286` px; this feature shows 5 of
  `(2400 − 64) / 5 = 467.2` px, 1.63× (SC-002 asks ≥ 1.5×). At a 2400 px *main pane* (2352 px grid)
  the ratio is 457.6 / 280 = 1.63×.
- **Alternatives considered**: centring a partial row, or stretching fewer than five cards to fill
  the row; both rejected by the spec (Story 1 #3–#4, Assumptions).

## R3 — Re-layout, keyboard order, platforms

- **Decision**: No new code. iced `responsive` re-runs the closure on every size change (window
  resize, sidebar resize or collapse: FR-008). Focus order follows the widget tree, which is card
  order row by row (FR-009). iced lays out in logical pixels on every platform and scale factor
  (Edge "Display scaling").
- **Rationale**: Feature 813 already relies on all three; the cap does not touch them.
