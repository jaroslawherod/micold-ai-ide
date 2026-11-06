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
