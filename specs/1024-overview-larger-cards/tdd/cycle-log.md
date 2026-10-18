# Cycle Log: Overview larger cards, five per row

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite (scoped, the file the feature changes): `scripts/build-lock.sh cargo test -p micold-client --lib card_grid`
  -> `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 740 filtered out`
- commit: `384f526b` (origin/main)
- recorded: cycle 0, before any change. The full workspace suite runs in the gate (T013).

## Cycle 1: U1–U7, the five-column cap in `CardGrid` (T001–T007, T009–T011)

- tests (all in `crates/micold-client/src/ui/material/card_grid.rs`, `mod tests`):
  `columns_are_capped_at_max` (new, U1), `full_rows_hold_five_cards_from_the_threshold_up` (new,
  U2), `fewer_cards_than_columns_keep_the_five_card_width` (new, U3), `max_columns_sets_the_cap`
  (new, U4), `cards_at_2400_are_half_again_as_wide_as_before_and_as_tall` (new, U5),
  `the_laid_out_grid_has_the_columns_its_width_gives` + `no_card_overlaps_another` (extended, U6),
  `thirty_cards_scroll_vertically` (extended, U7)
- red 1 (T005, compile): `scripts/build-lock.sh cargo test -p micold-client --lib card_grid`
  -> `error[E0061]: this function takes 3 arguments but 4 arguments were supplied`,
  `error[E0425]: cannot find value MAX_COLUMNS in this scope`,
  `error[E0599]: no method named max_columns found for struct card_grid::CardGrid`
- red 2 (T006, behavioural; `MAX_COLUMNS`, the 4-argument `columns` ignoring `max`, the
  `max_columns` builder): same command -> `test result: FAILED. 4 passed; 5 failed`:
  `columns_are_capped_at_max`: `assertion left == right failed: 2400 / left: 8 / right: 5`;
  `full_rows_hold_five_cards_from_the_threshold_up`: `left: [8, 4] / right: [5, 5, 2]`;
  `fewer_cards_than_columns_keep_the_five_card_width`: `3 cards: 286 != 467.2`;
  `max_columns_sets_the_cap`: `left: 6 / right: 3`;
  `cards_at_2400_are_half_again_as_wide_as_before_and_as_tall`: `286 < 1.5 × 286`
- green (T007): `columns` now `floor((width + gap) / (min + gap)).clamp(1, max.max(1))`. Same
  command -> `test result: ok. 9 passed; 0 failed`
- mutant (U6, U7 passed before the rule): `columns` returning `max.max(1)` (always five) ->
  `columns_follow_the_width_and_never_drop_below_one`, `columns_are_capped_at_max`,
  `the_laid_out_grid_has_the_columns_its_width_gives` FAILED; code restored. U7 asserts the
  scroll and no-sideways-growth invariants, which no column-count mutant breaks (they come from
  `rows` and `Scrollable`, unchanged).
- refactor: `cargo fmt`; test helpers `row_width` and `row_counts` shared by U2–U5. No production
  refactor needed (one expression changed).
- commit: `a383c665`

## Notes and deviations

- One cycle for seven behaviors: tasks.md (Decision 5 in autopilot.md) batches the tests of one
  rule ahead of its single-line implementation, with a compile red (T005) and a behavioural red
  (T006) recorded before the rule (T007). Each behavior's own failure is in red 2 above, except
  U6 and U7, which pin the unchanged below-threshold and scroll behavior and are covered by the
  mutant run instead.
