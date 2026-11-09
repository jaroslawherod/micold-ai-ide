# Contract: sidebar flat list

- **G1** `State::sidebar_list()` yields one row per entry of `sidebar_entries()`, same order, for any
  combination of card states. Cycling one location through the five card states leaves the sequence
  of row identities unchanged.
- **G2** No header row, toggle message or open/closed state exists in the list, view, showcase or docs.
- **G3** `location_order` and reveal walk `sidebar_entries()`; nothing needs expanding to make a row visible.
- **G4** `row_heights(rows)` has one entry per location row (plus its session rows when shown).
- **G5** `StateIcon::new(CardState, Roles)`: the five states map to five different `Icon`s; the slot
  width is the same for all; the glyph never depends on colour alone.
- **G6** A row's second line stays `type/number · session state` (`row_second_line`).
- **G7** `row_tint(CardState, Roles) -> Option<Rgb>`: NeedsAction `error_container`, NeedsAttention
  `tertiary_container`, Done `surface_container_high`, Working and Idle `None`. Pure; depends only on
  the card state, so it can never affect order (G1).
- **G8** The tint is the row's resting fill; hover and selected state layers are drawn over it and
  selection stays distinguishable. The glyph (G5) and the Done tooltip state line "Done (merged)" are always
  present, so the tint is never the only state cue.
- **G9** Text and glyph on each tint meet 4.5:1 / 3:1 in the light and dark themes.
