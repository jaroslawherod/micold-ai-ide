---
feature: 1024-overview-larger-cards
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 11 # US1 1–5, US2 1–3, US3 1–3
planned_at: 384f526b
updated_at: 384f526b
suite_baseline: green # scoped: `cargo test -p micold-client --lib card_grid`, see cycle-log.md
---

# Test List: Overview larger cards, five per row

The feature's real entry point is the shared `CardGrid` element (the Overview and the showcase
both build it, FR-011). Its laid-out tests (`test_support::laid_out`) are the highest level the
repository tests a view at, so the outer loop runs through them; what only a display shows
(text, focus order, live resize) is the visual pass, T012.

## Outer loop: acceptance behaviors

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | A grid of 1464 px or wider holds five cards in each full row, never more, cards sharing the width; a partial row and a short grid keep the five-card width at the left | US1 1–5, FR-001, FR-002, FR-004, SC-001 | example | DONE | U2, U3 |
| A2  | At 2400 px each card is at least 1.5× feature 813's card width and all cards share one height | US2 1–2, FR-005, FR-006, SC-002 | example | DONE | U5 |
| A3  | Text ellipsis, no text overlap, live re-layout and focus order at the new widths | US2 3, FR-007–FR-009 | example (visual pass) | PENDING | T012 visual-pass.md |
| A4  | Below 1464 px the grid reflows to as many columns as fit, down to one, with no overlap, no sideways growth, every card reachable by scrolling | US3 1–3, FR-003, SC-003, SC-004 | example | DONE | U6, U7 |

## Inner loop: unit behaviors

### `crates/micold-client/src/ui/material/card_grid.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | `columns` returns 4 at 1463 px and 5 at 1464, 2400, 3840 and 10 000 px with `max = 5`; `max = 0` acts as 1 | FR-001, FR-002, Edge threshold | example | DONE | `card_grid.rs::tests::columns_are_capped_at_max` |
| U2  | 12 cards at 1464, 2400 and 3840 px lay out in rows of 5, 5, 2, each `(w − 64) / 5` wide, last row left-aligned | US1 1, 2, 3, 5, FR-002, FR-004 | example | DONE | `card_grid.rs::tests::full_rows_hold_five_cards_from_the_threshold_up` |
| U3  | 3 cards and 1 card at 2400 px keep the five-card width and start at the left edge | US1 4, FR-004 | example | DONE | `card_grid.rs::tests::fewer_cards_than_columns_keep_the_five_card_width` |
| U4  | `max_columns(3)` caps the first row at 3; without it the default is `MAX_COLUMNS` (5) | FR-011 | example | DONE | `card_grid.rs::tests::max_columns_sets_the_cap` |
| U5  | 12 cards at 2400 px are each ≥ 1.5 × `(2400 − 112) / 8` wide and `overview_card::HEIGHT` tall | US2 1, 2, FR-005, FR-006, SC-002 | example | DONE | `card_grid.rs::tests::cards_at_2400_are_half_again_as_wide_as_before_and_as_tall` |
| U6  | 6 cards at 1463 px put 4 in the first row; at 279–3840 px no card overlaps another or passes the right edge | US3 1, 2, FR-003, SC-003 | example | DONE | `card_grid.rs::tests::the_laid_out_grid_has_the_columns_its_width_gives, no_card_overlaps_another` |
| U7  | 30 cards at 2400 × 400 px all lay out, the content is taller than 400 and the node is no taller than 400 nor wider than 2400 | US3 3, SC-004 | example | DONE | `card_grid.rs::tests::thirty_cards_scroll_vertically` |

## Invariants and edge cases still to place

None.

## Out of scope

- Taller cards, larger text, a two-line summary: spec Out of scope (larger means wider only).
- A user setting for the column count: spec Out of scope.
- Card content, order, states, activation (FR-010): unchanged code, feature 813's tests.

## Verification commands

- Single test: `scripts/build-lock.sh cargo test -p micold-client --lib card_grid::tests::<name> -- --exact`
- File: `scripts/build-lock.sh cargo test -p micold-client --lib card_grid`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Coverage: none (profile: no coverage tool)
- Mutation: none (profile: no mutation tool)
