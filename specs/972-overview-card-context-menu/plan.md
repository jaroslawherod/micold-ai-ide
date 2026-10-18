# Implementation Plan: Overview Card Context Menu

**Branch**: `claude/project-thread-a1yb7j` | **Date**: 2026-10-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/972-overview-card-context-menu/spec.md`

## Summary

A right-click (or Menu key / Shift+F10 on a focused card) opens a context menu on an Overview card:
*Open* (the card's activation) followed by exactly the location's sidebar row-menu entries, built
by one function shared with the row menu so the two cannot drift. Entries run the row menu's own
messages, so confirmations, navigation and in-place behaviour are the existing handlers'. The menu
is a registered `ContextMenu`-layer surface, so Escape, outside click, scroll-beneath and
displacement are the shared rules. The target card shows a selected-style mark; the shared menu
gains a keyboard highlight; focus returns to the target card after a keyboard-opened menu.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (already in the workspace); no new crate; one new glyph from
the bundled `assets/fonts/MaterialSymbolsOutlined.ttf`

**Storage**: none; menu and focus-return state are per-window memory (Principle IV)

**Testing**: `cargo test` via `mise run gate` (client state tests in `crates/micold-client/tests/`,
in-crate pixel/geometry gates in `ui/material/`, registry pair tests, icons, docs); quickstart §B
visual pass for the `main.rs` chains

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`); this feature touches `micold-client` only

**Performance Goals**: menu opens within 100 ms (SC-003): items are built per render from state
already held, O(entries)

**Constraints**: no new network use; row menus' entries and behaviour unchanged; the menu key and
Shift+F10 unchanged outside a focused card

**Scale/Scope**: 1 feature module extended, 1 registered surface, 1 item builder extracted,
6 shared-component builder additions, 1 icon, 1 user-guide section

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision is in tested code written red first: `overview::update` for the new `Msg`s (open/replace, highlight wrap, dismissal arms focus return, entry closes); `ui::overview::card_menu_items`, `card_menu_entry`, `settle_card_menu`, `focus_return_target`, the keyboard anchor inset; `location_menu_items` parity with the row menu; registry displacement pairs; `TakesTheKeyboard::chord`; `OverviewCard` targeted/secondary press and `MenuOverlay::highlighted` pixel tests. Only the `main.rs` chains (focused bounds → open; focus-return drain; inner dispatch) are glue, covered by quickstart §B B10–B12. |
| II. Multi-Session | PASS | The menu targets a location, not a session; session entries are the row menu's for that location (several sessions: the same entries the row offers); another window's changes close a menu whose target is gone (R8). No per-session state is added. |
| III. Worktree Integration | PASS | Worktree operations (rename, delete, stop showing) are the row menu's existing app-owned operations; the Default card offers only the Default row's entries, so nothing new can act on the project root as a worktree. |
| IV. Local-First | PASS | No persistence and no network; *Open pull request* is the row menu's existing, user-chosen browser open. |
| V. Rust + iced | PASS | iced only; `MenuOrigin` enum instead of a bool; target by `SessionLocation`, not an index, so a reorder cannot retarget the menu. |
| VI. Cross-Platform | PASS | Secondary press as iced reports it (incl. macOS Ctrl+click); Shift+F10 and `Named::ContextMenu` are platform-neutral; no OS branch; CI on three platforms; quickstart §C. |
| VII. Documentation | PASS | `docs/user-guide/overview.md` gains *Acting on a card* (right-click, keys, entries, dismissal); `docs/user-guide/icons.md` row for *Open*; same PR. |
| VIII. Components | PASS | Builders on shared components: `OverviewCard::{on_secondary_press, on_menu_key, targeted}`, `CardGrid::on_scroll`, `MenuOverlay::highlighted`, `TakesTheKeyboard::chord`; showcase entries for the targeted card and the highlighted menu. No feature-local widget. |

Post-design re-check: unchanged, all PASS. Known limitation (not a deviation): iced 0.14 has no
accessibility tree, so the highlight is visual and keyboard-driven only (as in 813).

## Design

### D1 — Shared item builder — R1

`ui/mod.rs`: extract `pub fn location_menu_items(state, &SessionLocation) -> Vec<MenuItem<Message>>`
from the `worktree_menu` closure (Default branch and worktree branch, session items, bind entry);
the worktree row menu calls it. `worktree_menu_items` stays as its inner part (its tests in
`tests/worktree_menu_pull_request.rs` keep passing).

### D2 — Overview state and reducer — R2, R4, R8, R9, data-model

`features/overview.rs`: `MenuOrigin`, `CardMenu`, `State::{card_menu, focus_return}`, the `Msg`
variants in [data-model.md](./data-model.md), and `OverviewCardMenu` (`floating_surface!` +
`Registered`). `update` returns `SurfaceOpened` on open. `overlay/registry.rs`: register it with
the displacement lists of R2; `tests/popover_displacement.rs` gains the new ordered pairs.

### D3 — Composition (client, testable from `tests/`) — R1, R4, R6, R8, R9

`ui/overview.rs`: `card_id(&SessionLocation)` (replaces `FIRST_CARD_ID`), `card_menu_items`,
`card_menu_entry`, `keyboard_anchor(bounds)`, `settle_card_menu(&mut State)`,
`focus_return_target(&State, &SessionLocation) -> Option<SessionLocation>`. `app.rs`: call
`settle_card_menu` at the outermost-update epilogue; route `GridScrolled` to
`dismiss_on_scroll_beneath`.

### D4 — Shared components — R3, R5, R6, R7, R10

- `ui/material/keyboard_focus.rs`: `TakesTheKeyboard::chord`.
- `ui/material/overview_card.rs`: `on_secondary_press` (wraps in `cdk::ContextArea`),
  `on_menu_key`, `targeted` (outline in `primary` + `state::SELECTED` layer; drops key claims).
- `ui/material/card_grid.rs`: `on_scroll`.
- `ui/material/menu.rs`: `MenuOverlay::highlighted`.
- `icons.rs`: `Icon::Open` (`login`).
- `showcase/`: targeted card, highlighted menu; `tests/showcase_completeness.rs`.

### D5 — Wiring (glue) — R3, R5, R6, R9

- `ui/overview.rs::view`: per card `.id(card_id)`, `.on_secondary_press`, `.on_menu_key`,
  `.targeted`; the grid's `.on_scroll`; the container in a `ContextArea` whose secondary press
  sends `CardMenuDismissed` (empty-space right-click).
- `ui/mod.rs::view_with`: push the card menu surface (`MenuOverlay::new(card_menu_items, ..)
  .anchor(clamped).highlighted(..)`) beside the row menus.
- `ui/mod.rs::subscription`: while a card menu is open, the card-menu key listener replaces the
  Overview arrow listener (named `fn`s).
- `main.rs::overview_message`: `CardMenuRequested` → `focused_bounds_task` →
  `CardMenuOpened { origin: Keyboard, anchor: keyboard_anchor(bounds) }`; `CardMenuActivated` →
  `card_menu_entry`; `CardMenuEntryChosen(inner)` → reducer then `update_inner(inner)`. A
  `card_focus_return` drain beside `tab_reveal_scroll`; Overview opening focuses the first card's
  `card_id`.

### D6 — Docs

`docs/user-guide/overview.md` §Acting on a card; `docs/user-guide/icons.md` *Open* row.

## Requirement map

| FR | Where | Research |
|---|---|---|
| FR-001 | D4 `on_secondary_press`, D5 view + clamped anchor | R3 |
| FR-002 | D1 `location_menu_items`, D3 `card_menu_items` | R1 |
| FR-003 | D3 *Open* = `CardActivated(activation)` | R4 |
| FR-004 | D3 wrapped row-menu messages, D5 `update_inner` | R4 |
| FR-005 | existing handlers (`ChangesRequested`, `MainAreaTaken`, dialogs) | R4 |
| FR-006 | D4 `targeted`, D5 view | R7 |
| FR-007 | D2 registered surface, Escape via registry | R2 |
| FR-008 | backdrop (primary), D5 empty-space `ContextArea`, displacement (secondary) | R3 |
| FR-009 | D4 `chord`/`on_menu_key`, D5 focused bounds, D3 `keyboard_anchor` | R5 |
| FR-010 | D2 `CardMenuMoved`, D4 `highlighted`, D5 key listener, D3 `card_menu_entry` | R6 |
| FR-011 | D2 `focus_return`, D3 `focus_return_target`, D5 drain | R9 |
| FR-012 | D3 `settle_card_menu`, D4 `CardGrid::on_scroll` | R8 |
| FR-013 | D4 `targeted` only drops claims while a menu is open; no other card change | R6 |
| FR-014 | D2 registry displacement | R2 |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Client state | `crates/micold-client/tests/overview_card_menu.rs` (new) | `update`: open sets target/anchor/origin and highlight (`Some(0)` keyboard, `None` pointer); opening on B replaces A; `CardMenuMoved` wraps both ways and from `None`; dismissal clears and arms focus return only for keyboard; `CardMenuEntryChosen` closes and arms likewise; `settle_card_menu` closes on target gone, on Overview closed (`Closed`, `MainAreaTaken`, `ChangesRequested`, project switch), keeps it on a target whose state changed or that reordered; `GridScrolled` closes it; `card_menu_items` = *Open* + row items for Default, plain, included, with PR, with session, no daemon, orphan (labels equal `location_menu_items`); *Open* wraps the card's activation; `card_menu_entry` with highlight, none, stale index; `focus_return_target` listed → target, gone → first card; `keyboard_anchor` lies inside the bounds; an in-place entry (`RenameStarted`, `DeleteRequested`) leaves `overview.open` true and opens its dialog; *Review changes* and a session select close it | 001–005, 009–012 |
| Client state | `tests/popover_displacement.rs`, `tests/overlay_registry.rs`, `tests/overlay_registration.rs` | the new surface's ordered pairs; Escape reaches it as topmost; scroll-beneath reaches it | 007, 012, 014 |
| Client state | `tests/worktree_menu_pull_request.rs` (unchanged, still green) | the row menu built through `location_menu_items` is unchanged | 002, Out of Scope |
| Widget unit | `ui/material/keyboard_focus.rs` tests | `chord` answers Shift+F10, not F10 alone or Ctrl+F10; Menu key | 009 |
| Widget unit | `ui/material/overview_card.rs` tests | secondary press publishes with the press point; Menu/Shift+F10 publish while focused; targeted card draws the outline and layer (pixel), unlike focus and hover; targeted card ignores Enter/Space | 001, 006, 009, 013 |
| Widget unit | `ui/material/menu.rs` tests (`menu_anatomy`) | the highlighted item draws the focus layer and ring; others do not; size unchanged | 010 |
| Geometry gate | `tests/gates/context_menu_anchor.rs` | card menu clamped inside the window at edges and minimum size | 001, Edge: window edge |
| Contrast gate | `ui/material/composition_contrast.rs` | `primary` outline against each card fill, both schemes | 006 |
| Icons | `tests/icons.rs`, `tests/icons_font.rs` | `Icon::Open` codepoint pinned, distinct, in font | 002 |
| Showcase | `tests/showcase_completeness.rs` | targeted card and highlighted menu listed | 006 |
| Visual pass | quickstart §B (visual-pass skill) | look, dismissals, keys, focus return, edges, scroll, two windows | all; SC-001–005 |
| Platforms | CI on Linux, macOS, Windows; quickstart §C | parity | Principle VI |
| Docs | gate docs checks; review | user-guide section | Principle VII |

## Project Structure

### Documentation (this feature)

```text
specs/972-overview-card-context-menu/
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
crates/micold-client/src/
├── features/overview.rs              # CardMenu, MenuOrigin, Msg variants, OverviewCardMenu surface
├── overlay/registry.rs               # register + displacement
├── app.rs                            # settle_card_menu epilogue, GridScrolled
├── main.rs                           # keyboard-open chain, entry dispatch, focus-return drain
├── icons.rs                          # Icon::Open
├── ui/mod.rs                         # location_menu_items (extracted), card menu surface, key listener
├── ui/overview.rs                    # card_id, card_menu_items, card_menu_entry, settle, focus target, anchor
├── ui/material/{overview_card.rs,card_grid.rs,menu.rs,keyboard_focus.rs,composition_contrast.rs}
└── showcase/{catalogue.rs,sections/{floating.rs,…}}
crates/micold-client/tests/{overview_card_menu.rs (new),popover_displacement.rs,overlay_registry.rs,overlay_registration.rs,icons.rs,icons_font.rs,showcase_completeness.rs,worktree_menu_pull_request.rs,context_menu_anchor_call_sites.rs,layout_snapshot.rs + gates/context_menu_anchor.rs}
docs/user-guide/{overview.md,icons.md}
```

**Structure Decision**: existing workspace layout; client only. Render-free state in
`features/overview.rs`, composition and its decisions in `ui/overview.rs` (reachable from
`tests/`), drawing in the shared `ui/material` library, wiring in `ui/mod.rs` and `main.rs`.

## Complexity Tracking

No violations.
