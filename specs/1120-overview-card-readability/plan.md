# Implementation Plan: Overview cards, readable layout and content

**Branch**: `claude/project-thread-7041o6` | **Date**: 2026-10-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1120-overview-card-readability/spec.md`

## Summary

Redo what an Overview card says and how it reads. The text rules move into a new render-free core
module `micold_core::overview_text` (`CardText::of(&CardModel)`: title, short branch, sessions line,
progress row, activity line) and a markup sanitiser `micold_core::markup`, both unit-tested in
`micold-core` (FR-018). `CardModel.summary` becomes `CardModel.title` (issue title, else branch, else
display name). `PrDisplay::footer` hides 0 · 0 and shows a single count alone. `HookActivity::apply`
keeps the whole prompt, sanitised to one line, instead of its first raw line. On the client,
`OverviewCard` (`ui/material/overview_card.rs`) gets a tinted header row (state left, type right) over
a neutral `surface_container_low` body, a 17 px card-scoped title role, 14 px body lines and 12 px
header labels. A new layout-only `cdk::EvenRow` makes every card in a grid row as tall as the tallest,
never under 172 px (`CardGrid`). Contrast, overlap and row-height gates, a new Overview layout-snapshot
state, the showcase and the user guide follow. Design decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`advanced::Widget` for `EvenRow`, `container`, `row`, `column`,
`stack`); no new crate (R3, R7)

**Storage**: none. The prompt kept in the service's in-memory `HookActivity` (daemon `state.rs`) is
now sanitised and one line; nothing is persisted (FR-020)

**Testing**: `micold-core` unit tests (`mise run test-core`) for every text rule; in-crate client
geometry and pixel tests in `overview_card.rs`, `card_grid.rs`, `cdk/even_row.rs`; contrast in
`composition_contrast.rs`; type scale in `type_role_mapping.rs` and `micold-core/tests/tokens_scales.rs`;
`tests/layout_snapshot.rs` (+ text-overflow and containment gates) with a new covered state; visual
pass (quickstart §B). Whole gate: `scripts/gate.sh`

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`, `micold-daemon`)

**Performance Goals**: card text derived per render as today, O(text length); `EvenRow` lays each card
out twice per layout pass (≤ 5 per row)

**Constraints**: 4.5:1 for all card text in both themes and five states; no overlap at 280/380/480 px;
title 17 px, body 14 px, nothing under 12 px; minimum card height 172 px; prompts never logged

**Scale/Scope**: core: 2 new modules, 1 changed function (`PrDisplay::footer`), 1 changed field
(`CardModel.title`), 1 changed hook rule, 1 type token; client: `OverviewCard`, `CardGrid`, new
`cdk::EvenRow`, 1 type role, `ui/overview.rs::card`, showcase sample, 1 covered state; docs: user
guide Overview section

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule is a pure core function written red first in `micold-core` (`overview_text`, `markup`, `PrDisplay::footer`, `HookActivity::apply`). Client: `EvenRow` layout, card geometry at three widths, palette contrast and the type role are red-first in-crate tests. Only the card's composition of already-tested values is glue, checked by geometry tests and quickstart §B. |
| II. Multi-Session | PASS | The activity line still comes from the location's main session only (`micold_core::occupancy::main_session`, called from `ui/overview.rs::card`); a session whose prompt sanitises to nothing shows no line and falls back to no other session (R8). No new session state. |
| III. Worktree Integration | PASS | No worktree or VCS operation; the title reads the existing issue-title lookup. |
| IV. Local-First | PASS | No new network call; issue titles come from the existing lookup; prompts stay in memory and are never logged (FR-020). |
| V. Rust + iced | PASS | iced only; `EvenRow` is an `advanced::Widget`. `Option` returns make "no row" unrepresentable as an empty string. |
| VI. Cross-Platform | PASS | No platform code. Text is drawn in the bundled Roboto on all platforms and laid out in logical px, so geometry gates measure what every platform draws (R14); CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/overview.md` updated in the same PR; layout snapshot regenerated (FR-019). |
| VIII. Components | PASS | `OverviewCard`, `CardGrid` extended in the shared library; `EvenRow` is a new shared cdk primitive with a builder (`new(children, columns).spacing().min_height()` → `.into()`); every colour from `Roles` (FR-017). |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### Core (`crates/micold-core`)

- **`src/markup.rs`** (new, `pub mod markup` in `lib.rs`): `readable(&str) -> Option<String>`, R7.
- **`src/overview_text.rs`** (new): `card_title`, `short_branch`, `branch_line`, `sessions_line`,
  `progress_row` (moved from the client's `progress_words`), `activity_text`, `activity_line`, and
  `CardText::of(&CardModel)`. Contract: [contracts/card-text.md](contracts/card-text.md).
- **`src/overview.rs`**: `CardModel.summary` → `title`; `PrDisplay::footer` new rule (R6).
- **`src/hook_activity.rs`**: `apply` sanitises the whole prompt, then clips; unreadable → `None` (R8).
- **`src/tokens/typography.rs`**: `OVERVIEW_CARD_TITLE` and `SCOPED` (R10).

### Client (`crates/micold-client`)

- **`src/ui/overview.rs::card`**: title from `overview_text::card_title` with the `TitleLookup::Known`
  text; no `summary`. `features/overview.rs::summary` removed.
- **`src/ui/material/overview_card.rs`**: new `Palette` (R9), header row + body (R13), `MIN_HEIGHT`
  replaces `HEIGHT`, all text from `CardText`; `words()` keeps only icons and header words.
  Contract: [contracts/overview-card.md](contracts/overview-card.md).
- **`src/ui/cdk/even_row.rs`** (new) and **`src/ui/material/card_grid.rs`**: equal row height,
  minimum (R12).
- **`src/ui/material/text.rs`**: `TypeRole::CardTitle`; `type_role_mapping.rs` admits `SCOPED`.
- **`src/ui/material/composition_contrast.rs`**: header and body contrast per state and scheme (R11).
- **`src/showcase/sections/surfaces.rs`**: sample model with title, typed branch, sanitised activity;
  five states in a `CardGrid` (R15).
- **`tests/support/covered_states.rs`**, **`tests/fixtures/layout_snapshot.txt`**: `overview-open`
  state (R14).

### Docs

`docs/user-guide/overview.md` (R15). `docs/development/layout-snapshot.md` *What is covered* gains
the Overview state if it lists states.

## Requirement map

| FR | Where | Test layer |
|---|---|---|
| FR-001 header row, state left / type right | `overview_card.rs` header, R13 | client geometry (header spans the width; state cluster left of type cluster); showcase §B |
| FR-002 neutral body | `Palette.body`, R9 | client: palette body equal for all states; pixel sample in body |
| FR-003 on-container / on-surface, 4.5:1 | `palette`, R9, R11 | `composition_contrast.rs` (10 state-theme pairs, header and body) |
| FR-004 17 / 14 / ≥12 px | `TypeRole::CardTitle`, `Body`, `Label`, R10 | `type_role_mapping.rs`, `tokens_scales.rs`; client test that every role the card uses is ≥ 12 |
| FR-005 Idle de-emphasised | Idle header `surface_variant`, R9 | contrast test asserts Idle header differs from body and from the four coloured headers |
| FR-006 title rule + tooltip | `overview_text::card_title`, `CardText.title_tooltip` | core unit |
| FR-007 short branch + tooltip, not repeated | `short_branch`, `branch_line`, R3 | core unit (all spec examples and edge cases) |
| FR-008 sessions line | `sessions_line`, R4 | core unit |
| FR-009 no empty progress row | `progress_row`, R5 | core unit; client bar test kept |
| FR-010 pull request row | `PrDisplay::footer`, R6 | core unit |
| FR-011 strip tags | `markup::readable`, R7 | core unit |
| FR-012 slash command sentence | `activity_text`, R7 | core unit |
| FR-013 no line rather than markup | `readable` → `None`, `activity_line`, R7, R8 | core unit (hook `apply` and card text) |
| FR-014 minimum height, equal row height | `cdk::EvenRow`, `MIN_HEIGHT`, R12 | client: `even_row.rs`, `card_grid.rs` geometry |
| FR-015 no overlap, ellipsis + tooltip | `Ellipsized` lines + tooltips, R14 | client geometry at 280/380/480; layout snapshot + text-overflow + containment gates |
| FR-016 behaviour unchanged, outline/ring/ripple cover header | R13 | existing focus, menu, ripple tests kept (pixel probes moved where the header changed them) |
| FR-017 shared components, roles, showcase | R9, R15 | `showcase_completeness`; quickstart §B |
| FR-018 rules in core | R1 | `mise run test-core` |
| FR-019 user guide, layout snapshot | R14, R15 | `layout_snapshot`; docs gate (`check-user-guide-updated.sh`) |
| FR-020 no logging | `markup`, `overview_text` are pure (no `tracing`) | mechanical: the quickstart §A grep finds no `tracing`/`log`/`println` call in `markup.rs`, `overview_text.rs`, `hook_activity.rs`; no unit test (absence of logging is not observable from a test) |

## Delivery order

1. **Core text rules** (no GUI): `markup`, `overview_text`, `PrDisplay::footer`, `HookActivity::apply`,
   typography token. `CardModel.title` rename touches the client call sites mechanically.
2. **Card and grid**: palette, header, type roles, `EvenRow`, `CardGrid`, contrast and geometry
   tests, showcase.
3. **Snapshot and docs**: covered state, regenerated fixture, user guide, visual pass.

## Project Structure

### Documentation (this feature)

```text
specs/1120-overview-card-readability/
├── spec.md
├── plan.md              # this file
├── research.md          # R1–R15
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── card-text.md     # core text rules
│   └── overview-card.md # card layout, palette, grid
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── lib.rs                     # + pub mod markup; pub mod overview_text;
├── markup.rs                  # new
├── overview_text.rs           # new
├── overview.rs                # CardModel.title, PrDisplay::footer
├── hook_activity.rs           # apply sanitises the whole prompt
└── tokens/typography.rs       # OVERVIEW_CARD_TITLE, SCOPED
crates/micold-core/tests/tokens_scales.rs
crates/micold-client/src/
├── features/overview.rs       # summary() removed
├── ui/overview.rs             # card(): title
├── ui/cdk/{mod.rs,even_row.rs}   # new EvenRow
├── ui/material/
│   ├── overview_card.rs       # header, palette, MIN_HEIGHT, CardText
│   ├── card_grid.rs           # rows of EvenRow
│   ├── text.rs                # TypeRole::CardTitle
│   ├── type_role_mapping.rs   # SCOPED admitted
│   └── composition_contrast.rs
└── showcase/sections/surfaces.rs
crates/micold-client/tests/
├── support/covered_states.rs  # overview-open
└── fixtures/layout_snapshot.txt
docs/user-guide/overview.md
```

**Structure Decision**: Existing workspace layout; rules in `micold-core`, rendering in
`micold-client`'s `ui/material` and `ui/cdk`.

## Complexity Tracking

None.
