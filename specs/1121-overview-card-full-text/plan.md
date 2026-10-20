# Implementation Plan: Overview cards, full text without truncation

**Branch**: `claude/project-thread-7041o6` | **Date**: 2026-10-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1121-overview-card-full-text/spec.md`

## Summary

The collapsed card wraps its title to three lines and its activity to two (the existing
`LineClamped` label over `micold_core::tooltip::clamp_to_lines`, promoted from Tooltip-private to a
crate-wide material label). One card at a time expands in place: same top-left and width, drawn
last by `CardGrid` above its neighbours with the menu elevation, grown downward to its full content,
shifted up to stay in the visible area and capped at its height, with the part below header and
title scrolling inside it. Which card is expanded, and why, is a render-free state machine
`micold_core::overview_expansion::Expansion` fed by pointer-rest, pointer-leave, focus, Escape,
window-blur, menu and listing events (FR-016); the expanded text (display name, full branch,
worktree directory, full activity) is `micold_core::overview_text::ExpandedText`; the shift and
cap are `overview_expansion::place`. The app holds the `Expansion` in `features::overview::State`;
`on_escape` asks it first (D1); `CardModel` gains the location's directory, shown by
`overview_text::worktree_dir` with the home folder as `~` (D2). Tooltips on title, branch and
activity go. Showcase, layout snapshot, user guide follow. Design decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`advanced::Widget`, `mouse::Cursor::levitate`,
`Layout::with_offset`, `renderer.with_layer`, `scrollable`); no new crate. Home folder from the
`directories::UserDirs`, the lookup the client already uses (R9).

**Storage**: none. `Expansion` is in-memory and reset when the Overview closes (FR-013); nothing
persisted, paths and prompts never logged (FR-018).

**Testing**: `micold-core` unit tests (`mise run test-core`) for `overview_expansion` (every
trigger of FR-002/005/006/012/013 and each Edge Case), `place`, `page_target`, `ExpandedText`,
`worktree_dir`; client in-crate tests in `overview_card.rs` (line counts at 280/380/480, no clipped
text both forms, focus/ripple/menu behaviour on an expanded card) and `card_grid.rs` (no slot moves,
row heights unchanged, expanded drawn last, events go to the expanded card, shift and cap);
`app.rs` reducer tests for `on_escape` and the message wiring; `tests/layout_snapshot.rs` with a
new covered state; visual pass (quickstart §B). Whole gate: `scripts/gate.sh`.

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`, `micold-daemon`)

**Performance Goals**: expand/collapse within 0.5 s of the trigger (SC-006): rest delay
`EXPAND_REST` = 400 ms, focus and Escape at once; one extra card layout per grid layout (the
collapsed twin of the expanded card, R4)

**Constraints**: no slot or row moves (FR-009); no clipped text at 280/380/480 px (FR-007);
contrast and type scale of feature 1120 (FR-015); at most one card expanded (FR-006)

**Scale/Scope**: core: 1 new module (`overview_expansion`), `overview_text` gains `ExpandedText`
and `worktree_dir`, `CardText` loses three tooltips, `CardModel` gains `dir`; client: `LineClamped`
promoted, `OverviewCard` (`expanded`, focus report, page keys, rest/leave messages), `CardGrid`
(twin, top draw, event priority, rest timer, place), `features::overview` (state, `Msg::Expansion`,
`Msg::CardPaged`), `app.rs::on_escape`, `main.rs` (window blur, page scroll),
`ui/overview.rs` (wiring), showcase, 1 covered state; docs: user guide Overview section

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule is a pure core function, red first: `Expansion::apply`, `place`, `page_target`, `ExpandedText::of`, `worktree_dir`, the 3/2-line clamp through `clamp_to_lines`. Client: grid geometry, event priority and draw order, card line counts and no-clipping, `on_escape` are red-first in-crate tests. Only the card's composition of tested values and `main.rs` task wiring are glue, checked by quickstart §B. |
| II. Multi-Session | PASS | No session state. The activity line is still the main session's (feature 1120 R8). `Expansion` is per window, keyed by `SessionLocation`. |
| III. Worktree Integration | PASS | Read-only: the directory shown is `Worktree::path` or the project root for *Default*; no VCS operation. |
| IV. Local-First | PASS | No network; paths, titles and prompts stay on screen and are never logged (FR-018). |
| V. Rust + iced | PASS | iced only. `Cause` (Pointer/Focus) and `Option<Expanded>` make "two cards expanded" unrepresentable (FR-006). |
| VI. Cross-Platform | PASS | `worktree_dir` takes the home folder as a parameter and keeps the platform's separators; no OS branch in core. Text in the bundled font, logical px. CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/overview.md` in the same PR; layout snapshot gains `overview-expanded` (FR-017). |
| VIII. Components | PASS | `OverviewCard` and `CardGrid` extended in the shared library with builder methods; `LineClamped` promoted to a shared label with a builder (`new(text, role, lines).tint(c)`), not forked; colours from `Roles`, shadow from `tokens::elevation::MENU` (FR-015). |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### Core (`crates/micold-core`)

- **`src/overview_expansion.rs`** (new, `pub mod` in `lib.rs`): `Expansion`, `Event`, `Cause`,
  `EXPAND_REST`, `place`, `Placement`, `page_target`, `PageTarget`. Contract:
  [contracts/expansion.md](contracts/expansion.md) (K1–K4).
- **`src/overview_text.rs`**: `ExpandedText::of(&CardModel, home: Option<&Path>)`, `worktree_dir`;
  `CardText` drops `title_tooltip` and the branch/activity `tooltip` fields (FR-004). Contract K5–K6.
- **`src/overview.rs`**: `CardModel.dir: PathBuf` (R8).

### Client (`crates/micold-client`)

- **`src/ui/material/line_clamp.rs`**: `pub(crate)`, `.tint(Rgb)` and the role's line spacing, so
  the card uses it at `CardTitle` (3 lines) and `Body` (2 lines) (R2).
- **`src/ui/material/overview_card.rs`**: collapsed form with the clamps and without the three
  tooltips; `.expanded(bool)` builds the expanded form (R3); `.on_keyboard_focus`,
  `.on_page`, `.on_rest`, `.on_leave`, `.scroll_id`, `.on_body_scroll` builders. Contract
  [contracts/card-grid.md](contracts/card-grid.md) U1.
- **`src/ui/material/card_grid.rs`**: `.expanded(index, twin)`; layout, draw, update,
  mouse_interaction and the rest timer per R4–R6. Contract U2–U4.
- **`src/ui/material/keyboard_focus.rs`**: `TakesTheKeyboard::on_shown_change`, reporting
  keyboard (shown) focus only, so a pointer press expands nothing (U1, FR-002).
- **`src/features/overview.rs`**: `State.expansion`, `Msg::Expansion(Event)` (the inner scroll's edges
  arrive as `Event::BodyScrolled`), `Msg::CardPaged`; `CardMenuOpened`/`Dismissed`/`EntryChosen` feed it.
- **`src/app.rs`**: `on_escape` returns `Msg::Expansion(Event::Escape)` when a card is expanded,
  before `Msg::Closed` (D1, R7).
- **`src/ui/overview.rs`**: builds each card with its expansion, focus and message wiring and the
  twin; `settle_card_menu` resets the expansion when the Overview is closed (every way of closing)
  and otherwise feeds `Event::Listed` (card removed) before its no-menu early `return`. `card_scroll_id(location)`.
- **`src/main.rs`**: `WindowFocusChanged(false)` → `Event::WindowBlurred`; `Msg::CardPaged` → a
  `scrollable::scroll_by` on the card's scroll id (R6).
- **`src/showcase/sections/surfaces.rs`**: a collapsed and an expanded card (FR-015).
- **`tests/support/covered_states.rs`**, **`tests/fixtures/layout_snapshot.txt`**:
  `overview-expanded` (R10).

### Docs

`docs/user-guide/overview.md`: *One card per location* (wrapped title and activity), a new
*Reading a card in full* section (hover, focus, Escape, scroll inside), *Keyboard* (Escape,
Page Up/Down). Feature 1120's tooltip sentences are removed.

## Requirement map

| FR | Where | Test layer |
|---|---|---|
| FR-001 title 3 / activity 2 lines | `LineClamped` + `clamp_to_lines`, R2 | client: line counts at 280/380/480 for 1–5-line titles and short/long activity; core `clamp_to_lines` tests kept |
| FR-002 expand on rest and on focus | `Expansion::apply(PointerRested / FocusGained)`, rest timer R5 | core unit; client: grid publishes `on_rest` after `EXPAND_REST`, not before |
| FR-003 in place, above, elevated | `CardGrid` R4, `place` | client: expanded node keeps slot x, width, top (unless shifted); drawn last; shadow present |
| FR-004 expanded text, tooltips removed | `ExpandedText::of`, R3 | core unit (each optional line, display name vs title, branch vs title); client: no tooltip on title/branch/activity, header tooltips kept |
| FR-005 collapse triggers, Escape, re-expand | `Expansion::apply`, `on_escape` R7 | core unit per trigger and Edge Case; `app.rs` test: Escape with a card expanded collapses and keeps the Overview open, second Escape closes |
| FR-006 one at a time | `Option<Expanded>` | core unit (rest on B while A focused) |
| FR-007 no clipped text | `LineClamped` WordOrGlyph, expanded `Text` WordOrGlyph | client geometry at 280/380/480, both forms, unbroken branch/path; text-overflow + containment gates |
| FR-008 shift up, cap, inner scroll | `place`, `page_target`, inner `Scrollable` R6 | core unit; client: shifted card inside the view, header and title outside the scrolled part |
| FR-009 no reflow, no scroll change | twin measure R4 | client: row heights and every slot equal expanded vs not |
| FR-010 events to the expanded card | grid levitates others' cursor R4 | client: press over the overlapped neighbour publishes the expanded card's message only |
| FR-011 behaviour unchanged | existing focus, ripple, menu tests run on an expanded card too | client |
| FR-012 menu freezes expansion | `Expansion` menu flag | core unit |
| FR-013 none on open | `Expansion::default` in `settle_card_menu`'s `!open` branch | core unit; app test: activate a card, reopen → none expanded |
| FR-014 directory with `~` | `worktree_dir`, R9 | core unit (inside home, outside, home itself, Windows separators as given) |
| FR-015 contrast, type, components, showcase | palette unchanged, `elevation::MENU` | `composition_contrast.rs` kept; `showcase_completeness`; §B |
| FR-016 rules render-free | `overview_expansion`, `overview_text` | `mise run test-core` |
| FR-017 guide, snapshot | R10 | `layout_snapshot`; docs gate |
| FR-018 not logged | pure modules | quickstart §A grep (no `tracing`/`log`/`println` in the two modules) |

## Delivery order

Superseded by the four milestones in [tasks.md](tasks.md#milestones), cut so each one is observable
and full text is never unreachable on `main` (the tooltips go only when hover expansion ships):

1. **M1** collapsed title on 3 lines, activity on 2.
2. **M2** `ExpandedText`, `worktree_dir`, `CardModel.dir`, `place`; the card's expanded form and the
   grid's twin, top draw, event priority, shift, cap and inner scroll; shown in the showcase only.
3. **M3** `Expansion` and `EXPAND_REST`, the grid's rest timer, app state and hover wiring, menu,
   close, blur, listing; tooltips dropped; covered state; user guide.
4. **M4** `on_shown_change`, focus wiring, `page_target` and Page keys, `on_escape` (D1); user guide.

## Project Structure

### Documentation (this feature)

```text
specs/1121-overview-card-full-text/
├── spec.md
├── plan.md              # this file
├── research.md          # R1–R10
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── expansion.md     # core: K1–K6
│   └── card-grid.md     # client: U1–U4
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── lib.rs                     # + pub mod overview_expansion;
├── overview_expansion.rs      # new
├── overview_text.rs           # ExpandedText, worktree_dir; tooltips dropped
└── overview.rs                # CardModel.dir
crates/micold-client/src/
├── app.rs                     # on_escape (D1)
├── main.rs                    # window blur, page scroll
├── features/overview.rs       # State.expansion, Msg::{Expansion, CardPaged}
├── ui/overview.rs             # wiring, twin, card_scroll_id, Listed
├── ui/material/
│   ├── line_clamp.rs          # pub(crate), tint
│   ├── overview_card.rs       # clamps, expanded form, new builders
│   └── card_grid.rs           # twin, top draw, priority, rest, place
└── showcase/sections/surfaces.rs
crates/micold-client/tests/
├── support/covered_states.rs  # overview-expanded
└── fixtures/layout_snapshot.txt
docs/user-guide/overview.md
```

**Structure Decision**: Existing workspace layout; rules in `micold-core`, rendering in
`micold-client`'s `ui/material`, wiring in `features/overview.rs`, `app.rs`, `main.rs`.

## Complexity Tracking

None.
