# Implementation Plan: Overview with larger cards, five per row

**Branch**: `claude/project-thread-iq9j8q` | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1024-overview-larger-cards/spec.md`

## Summary

Cap the Overview's card grid at five columns. Feature 813's `CardGrid`
(`crates/micold-client/src/ui/material/card_grid.rs`) already picks
`max(1, floor((w + gap) / (min + gap)))` columns, shares the width equally between them with
`Length::Fill` cards, and pads a partial last row with `FillPortion(1)` spacers so its cards keep the
full-row width. The change is one bound on that count: `columns = clamp(fit, 1, max)` with
`max = MAX_COLUMNS = 5` as the grid's default, set through a new `max_columns` builder method
(Principle VIII). The shared default makes the Overview (`ui/overview.rs`) and the showcase entry
show the same layout (FR-011) with no change at either call site. Card height, content and order are
untouched (FR-005, FR-006, FR-010). The user guide gains a short *Layout* section (FR-012).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`responsive`, `row`, `column`, `Space`); no new crate

**Storage**: none; the layout is derived from the width on every layout pass

**Testing**: in-crate geometry gates in `card_grid.rs` (`cargo test -p micold-client`, run by
`mise run gate`, here `scripts/gate.sh`); showcase completeness test; visual pass (quickstart §B)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: no change; the column count is O(1) per layout pass, as today

**Constraints**: never more than five columns; never sideways scroll; minimum card width 280 px and
gap `spacing::MD` (16 px) unchanged (spec Assumptions)

**Scale/Scope**: 1 component (`CardGrid`: 1 const, 1 builder method, 1 function signature), its
tests, 1 contract line, 1 user-guide section. No core, daemon or wire change.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | The new bound is written red first in `card_grid.rs` tests: `columns` with a cap (unit), and laid-out grids at 1463 / 1464 / 2400 / 3840 px (five per row, equal widths, partial row at the start, never more than five) before `columns` and `CardGrid` change. The showcase only calls the component (glue exception, quickstart §B). |
| II. Multi-Session | PASS | No session state; the grid lays out whatever cards it is given; a card moving on a state change only changes order (spec Edge Cases). |
| III. Worktree Integration | PASS | No worktree or VCS operation. |
| IV. Local-First | PASS | No storage, no network. |
| V. Rust + iced | PASS | iced `responsive` layout only; the count is a pure function. |
| VI. Cross-Platform | PASS | Widths are iced logical pixels on every platform and scale factor; no platform code. CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/overview.md` gains *Layout* in the same PR (FR-012). |
| VIII. Components | PASS | Built in the shared `CardGrid` with a chainable `max_columns(usize)` builder; Overview and showcase reuse it unchanged (FR-011). |

Post-design re-check: unchanged, all PASS.

## Design

### D1 — Column cap in `CardGrid` (R1, R2)

- `pub const MAX_COLUMNS: usize = 5;` beside `MIN_WIDTH`.
- `columns(width, min, gap, max) -> usize` returns
  `floor((width + gap) / (min + gap)).clamp(1, max.max(1))`. Its one caller is `CardGrid`'s
  `From` impl (no other reference in the workspace).
- `CardGrid` gains a `max_columns: usize` field, default `MAX_COLUMNS`, and a builder
  `max_columns(mut self, n: usize) -> Self`. The `From` impl passes it to `columns`.
- `rows` is unchanged: `Length::Fill` cards plus `FillPortion(1)` spacers already give every card
  `(w − (n − 1)·gap) / n` and put a partial row's cards at the start (FR-004, Story 1 #3–#4, Edge
  "One location").
- Module documentation names the cap and feature 1024.

### D2 — Contract and docs (FR-011, FR-012)

- [contracts/ui.md](contracts/ui.md) supersedes the `CardGrid` row and U4 column rule of
  `specs/813-worktree-overview/contracts/ui.md`.
- `docs/user-guide/overview.md`: a *Layout* section after the opening paragraph: up to five cards
  per row, cards share the width; a narrower window shows fewer per row down to one; the grid
  scrolls down, never sideways.

## Requirement map

| FR | Where | Proved by |
|---|---|---|
| 001 | D1 clamp | unit: `columns` never above 5 at 2400 / 3840 / 10 000 px; layout at 3840 px |
| 002 | D1 | layout: 1464 px → 5 per row; 2400 px → 5 |
| 003 | D1 (unchanged fit) | unit: 1463 → 4, 279 → 1; existing narrow test; vertical-only `Scrollable` |
| 004 | `rows` (unchanged) | layout: 12 cards at 2400 px → rows 5/5/2, all widths equal, last row starts at x = 0; 3 cards → width of a five-card row |
| 005 | D1 | layout: 2400 px card width ≥ 1.5 × feature 813's (SC-002); height = `overview_card::HEIGHT` |
| 006 | `OverviewCard` (unchanged) | existing card height gate; `card_boxes` finds every card at `HEIGHT` |
| 007 | `OverviewCard` (unchanged) | existing 280 px overlap and ellipsis gate; widths grow from there; visual pass §B |
| 008 | `responsive` (unchanged) | visual pass §B (resize, sidebar collapse) |
| 009 | widget order (unchanged) | existing keyboard tests in `tests/overview.rs`; visual pass §B |
| 010 | no change to card or order code | existing 813 tests stay green |
| 011 | D1 default in `CardGrid::new` | showcase pose "30 cards" shows 5 per row (visual pass) |
| 012 | D2 | review of the docs diff |
| SC-004 | `rows` + `Scrollable` (unchanged) | layout: 30 cards scroll at 2400 px; keyboard: `arrow_keys_move_the_focus_and_other_keys_do_not` in `tests/overview.rs` (unchanged); quickstart §B steps 5–6 |

## Test strategy by layer

The laid-out gates use the **grid width** (what `responsive` gives `CardGrid`). The main pane is
48 px wider (the Overview's 2 × 24 px padding): the five-card threshold of 1464 px of grid is a
1512 px main pane. The spec's pixel widths (thresholds, edge cases, SC-001–SC-004) are grid
widths (spec *Terms*), so the gates test them as written.

| Layer | Where | What it proves | FR / SC |
|---|---|---|---|
| Unit | `card_grid.rs` `columns` tests | threshold 1463 / 1464 px, cap at 2400 / 3840 px, floor 1 at 0 / 279 px, `max = 0` treated as 1 | 001–003, SC-001 |
| Geometry gate | `card_grid.rs` laid-out tests | 5 per row and equal widths at 1464 / 2400 / 3840 px; 12 cards → 5/5/2 start-aligned; 3 cards keep the five-card width; no overlap and every card inside the width at 279 / 600 / 1000 / 1464 / 2400 / 3840 px; 2400 px width ≥ 1.5 × 813's; 30 cards scroll at 2400 px | 001–006, SC-001–SC-004 |
| Builder | `card_grid.rs` | `max_columns(3)` caps at three (the builder is honoured) | 011 |
| Showcase | `tests/showcase_completeness.rs` | poses unchanged | 011 |
| Visual pass | quickstart §B | look at 2400 / 1464 / 1000 / 300 px, resize, sidebar collapse, keyboard order | 007–009, 011 |
| Platforms | CI Linux, macOS, Windows | parity | Principle VI |

## Project Structure

### Documentation (this feature)

```text
specs/1024-overview-larger-cards/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── ui.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-client/src/ui/material/card_grid.rs   # MAX_COLUMNS, columns(.., max), max_columns builder, tests
docs/user-guide/overview.md                         # + Layout section
```

**Structure Decision**: existing workspace layout; the change lives entirely in the shared
`ui/material` component, so `ui/overview.rs` and `showcase/sections/surfaces.rs` need no edit.

## Complexity Tracking

No violations.
