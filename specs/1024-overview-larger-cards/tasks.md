---

description: "Tasks: Overview with larger cards, five per row"
---

# Tasks: Overview with larger cards, five per row

**Input**: Design documents from `/specs/1024-overview-larger-cards/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ui.md, quickstart.md

**Tests**: Constitution Principle I: every test task below is written first and seen failing (red)
for the stated reason before the implementation task it covers. Tests that pin behaviour feature
813 already has (US2, US3) are characterization gates: they compile against the new
`columns(.., max)` signature, so they fail to compile until T006 and must pass once T007 adds the cap.

**Documentation**: Principle VII: the user-guide task T008 ships in the same change as the cap.

**Cross-platform**: Principle VI: widths are iced logical pixels; the in-crate gates run on Linux,
macOS and Windows CI unchanged.

**Organization**: grouped by user story. All three stories are served by one change in the shared
`CardGrid` component (`crates/micold-client/src/ui/material/card_grid.rs`, plan D1); US2 and US3
add gates only.

Every test below lives in the `#[cfg(test)] mod tests` of
`crates/micold-client/src/ui/material/card_grid.rs` and reuses its `cards(n)`, `grid(n)` and
`card_boxes(node)` helpers and `test_support::{laid_out, leaves}`. Widths are **grid widths** (the
main pane less the Overview's 2 × 24 px padding, plan *Test strategy by layer*). Run them with
`cargo test -p micold-client card_grid`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, module, dependency or fixture. The existing test helpers in `card_grid.rs`
suffice.

---

## Phase 2: Foundational (Blocking Prerequisites)

None: the change is one bound inside an existing component.

---

## Phase 3: User Story 1 - Five cards per row on a wide pane (Priority: P1) 🎯 MVP

**Goal**: A full row of the Overview holds at most five cards; from the five-card threshold
(1464 px of grid) up, exactly five, each sharing the width equally; a partial row and a grid of
fewer than five cards keep the five-card width and start at the left.

**Independent Test**: `cargo test -p micold-client card_grid`: twelve cards at 2400 px lay out
5 / 5 / 2, every card the same width, the last row starting at x = 0.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these first and run them: they must FAIL, first to compile (T005), then on column counts
> once T006 adds the signature without the rule.

- [X] T001 [US1] [U1] Unit test `columns_are_capped_at_max` in `crates/micold-client/src/ui/material/card_grid.rs`: with `min = 280.0`, `gap = 16.0`, `max = MAX_COLUMNS`, `columns` returns 4 at 1463 px, 5 at 1464 px, 5 at 2400 px, 5 at 3840 px and 5 at 10 000 px; with `max = 0` it returns 1 at 2400 px ("`0` is treated as `1`", data-model.md). Update the existing `columns_follow_the_width_and_never_drop_below_one` calls to pass `MAX_COLUMNS` as the fourth argument, its expected values unchanged (FR-001, FR-002, Edge "Exactly at the threshold")
- [X] T002 [US1] [U2] Laid-out test `full_rows_hold_five_cards_from_the_threshold_up` in `crates/micold-client/src/ui/material/card_grid.rs`: `grid(12)` laid out at 1464, 2400 and 3840 px wide (800 px tall): `card_boxes` finds 12 cards in rows of 5, 5 and 2 (group by `y`); every card's width equals `(w − 4 · 16) / 5` within 0.01; the last row's first card has `x` within 0.01 of the first row's first card (FR-002, FR-004, US1 scenarios 1, 2, 3 and 5, Edge "Very wide pane")
- [X] T003 [US1] [U3] Laid-out test `fewer_cards_than_columns_keep_the_five_card_width` in `crates/micold-client/src/ui/material/card_grid.rs`: `grid(3)` and `grid(1)` at 2400 px: every card's width is `(2400 − 4 · 16) / 5` within 0.01 and the first card starts at the grid's left edge (FR-004, US1 scenario 4, Edge "One location")
- [X] T004 [US1] [U4] Builder test `max_columns_sets_the_cap` in `crates/micold-client/src/ui/material/card_grid.rs`: `CardGrid::new(cards(6), roles()).max_columns(3)` laid out at 2400 px puts 3 cards in the first row; `CardGrid::new(cards(12), roles())` with no builder call puts 5 there (the default is `MAX_COLUMNS`, so the Overview and the showcase share it, FR-011, Principle VIII)
- [X] T005 [US1] [U1] [U2] [U3] [U4] [U5] Run `cargo test -p micold-client card_grid` and record the first red reason (does not compile: `columns` takes 3 arguments, no `max_columns` method, no `MAX_COLUMNS`)

### Implementation for User Story 1

- [X] T006 [US1] [U1] [U2] [U3] [U4] [U5] Plumbing without the rule, in `crates/micold-client/src/ui/material/card_grid.rs`: add `pub const MAX_COLUMNS: usize = 5;` beside `MIN_WIDTH` with a doc comment; change the signature to `pub fn columns(width: f32, min: f32, gap: f32, max: usize) -> usize` (confirm its one caller with LSP `findReferences` first) but keep the old body, ignoring `max`; add the `max_columns: usize` field to `CardGrid` (default `MAX_COLUMNS` in `new`), the chainable builder `pub fn max_columns(mut self, n: usize) -> Self`, and pass it to `columns` in the `From<CardGrid>` impl. Run `cargo test -p micold-client card_grid` and record the behavioural red: T001–T004 and T009 fail on column counts and widths (e.g. 8 columns at 2400 px), the existing tests pass
- [X] T007 [US1] [U1] [U2] [U3] [U4] [U5] [U6] [U7] The rule, in `crates/micold-client/src/ui/material/card_grid.rs`: `columns` returns `floor((width + gap) / (min + gap)).clamp(1, max.max(1))`; update its doc comment and the module documentation ("at most `MAX_COLUMNS` (five, feature 1024) columns"); leave `rows`, `ui/overview.rs` and `showcase/sections/surfaces.rs` unchanged (plan D1, contracts/ui.md, FR-011). T001–T004, T009–T011 now pass
- [X] T008 [P] [US1] In `docs/user-guide/overview.md` add a `## Layout` section after the opening paragraph (before *Opening and closing it*): the grid shows up to five cards per row and the cards share the width; a narrower window shows fewer per row, down to one; the grid scrolls down, never sideways (FR-012, plan D2)

**Checkpoint**: Five per row on wide panes; the user guide says so.

---

## Phase 4: User Story 2 - Larger cards that stay readable (Priority: P1)

**Goal**: At 2400 px each card is at least 1.5 × as wide as feature 813's, at the same height.

**Independent Test**: `cargo test -p micold-client card_grid` and the existing `overview_card`
text gates.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T009 [US2] [U5] Laid-out test `cards_at_2400_are_half_again_as_wide_as_before_and_as_tall` in `crates/micold-client/src/ui/material/card_grid.rs`: `grid(12)` at 2400 px: every card's width is ≥ 1.5 × `(2400 − 7 · 16) / 8` (feature 813's eight columns there, SC-002) and every card's height equals `overview_card::HEIGHT` within 0.01 (FR-005, FR-006, US2 scenarios 1 and 2). Written with T001–T004, red until T006–T007

### Implementation for User Story 2

None beyond T006–T007: card height, text and ellipsis come from `OverviewCard`, unchanged
(FR-007, FR-010). Its existing 280 px overlap and ellipsis gates in
`crates/micold-client/src/ui/material/overview_card.rs` stay green, and wider cards only add room.

**Checkpoint**: Cards are wider, the same height, text still clipped with an ellipsis.

---

## Phase 5: User Story 3 - Narrow panes still work (Priority: P2)

**Goal**: Below the threshold the grid reflows to fewer columns down to one, with no overlap and no
sideways scroll; every card stays reachable.

**Independent Test**: `cargo test -p micold-client card_grid`.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T010 [US3] [U6] In `crates/micold-client/src/ui/material/card_grid.rs` extend `the_laid_out_grid_has_the_columns_its_width_gives` with `(1463.0, 4)` (one pixel under the threshold, using `grid(6)`), and extend `no_card_overlaps_another` to widths 279, 600, 1000, 1464, 2400 and 3840 px, also asserting every card's right edge is ≤ the width + 0.01 (FR-003, SC-003, US3 scenarios 1 and 2, Edge "Exactly at the threshold" and "Pane narrower than the minimum card width")
- [X] T011 [US3] [U7] In `crates/micold-client/src/ui/material/card_grid.rs` extend `thirty_cards_scroll_vertically` to also lay out `grid(30)` at 2400 × 400 px: 30 cards, content bottom > 400, node height ≤ 400, and node width ≤ 2400 (no sideways growth) (SC-004, US3 scenario 3)

### Implementation for User Story 3

None beyond T006–T007: the fit rule below the cap, `responsive` re-layout (FR-008) and the
keyboard order in `crates/micold-client/tests/overview.rs` (FR-009) are feature 813's, unchanged.

**Checkpoint**: All three stories hold; `cargo test -p micold-client` green.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T012 Visual pass per quickstart §B (steps 1–6) with the `visual-pass` skill on the component showcase's `CardGrid` entry and the client Overview; record the result and screenshots in `specs/1024-overview-larger-cards/visual-pass.md` (FR-007, FR-008, FR-009, FR-011); the record names the resize and sidebar-collapse re-layout (FR-008) and the keyboard focus order (FR-009, SC-004) checks explicitly
- [ ] T013 Full gate (`mise run gate`, in this container `scripts/gate.sh`) green, including `crates/micold-client/tests/showcase_completeness.rs` still finding the four `CardGrid` poses (quickstart §A)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup, Foundational**: empty.
- **US1 (Phase 3)**: T001–T004 (with T009–T011) → T005 (compile red) → T006 (behavioural red) → T007 (green); T008 any time (different file).
- **US2 (Phase 4)**, **US3 (Phase 5)**: their tests are written with US1's (same file, same red
  phase) and turn green with T006–T007.
- **Polish**: T012 after T007 (needs the cap built); T013 last.

### User Story Dependencies

- **US1**: none.
- **US2**, **US3**: depend on US1's T006–T007; no code of their own.

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Constitution Principle I).
- User-guide documentation accompanies US1 in the same change (Principle VII).

### Parallel Opportunities

- T008 (`docs/user-guide/overview.md`) runs in parallel with every `card_grid.rs` task.
- All other tasks edit `card_grid.rs` and run in order.

## Parallel Example: User Story 1

```bash
# While T001–T007 edit card_grid.rs:
Task: "T008 Layout section in docs/user-guide/overview.md"
```

## Implementation Strategy

### MVP First

US1 alone is the issue: T001–T008. US2 and US3 add only gates over the same change, so all three
ship together in one milestone (below).

### Incremental Delivery

One PR: cap, its gates, user guide, visual pass. No later milestone.

## Notes

- `columns` has one caller, `CardGrid`'s `From` impl (plan D1); confirm with LSP `findReferences`
  before changing its signature.
- Never point `CARGO_TARGET_DIR` elsewhere; run tests through the shared target.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Five cards per row, larger cards 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: the Overview (and the showcase's `CardGrid` entry) shows at most five cards per
  row, exactly five from 1464 px of grid up, each card sharing the width and wider than before;
  narrower panes reflow to fewer columns down to one; the user guide describes the layout
- **Satisfies**: US1 acceptance scenarios 1–5; US2 scenarios 1–3; US3 scenarios 1–3; FR-001–FR-012;
  SC-001–SC-004; edge cases threshold, very wide pane, one location, narrow pane
- **Verify**: `cargo test -p micold-client card_grid`; `scripts/gate.sh`; quickstart §B (visual
  pass recorded in `specs/1024-overview-larger-cards/visual-pass.md`)
- **Depends on**: —
- **Tier**: full
