# Implementation Plan: Sidebar grouped by state, with Overview as its first row

**Branch**: `claude/project-thread-22fhnj` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1125-sidebar-state-groups/spec.md`

## Summary

The sidebar lists its locations under state groups (Needs action, Working, Idle, Done), each headed
"Name · count", with Done collapsed at start; its first row is **Overview**, carrying the top bar's
attention badge; under it a chip bar shows **All** and the tag filter chips with add, hide and a ⋯
menu (refresh, attach existing, run in parallel); every location row has two lines (title; short
branch · sessions line) and the selected row is filled with the primary container colour.

The rules go into a new render-free core module `micold_core::sidebar_groups` (state → group,
order, stable partition, counts, header text, open rule, run group state) and one new
`overview_text::sidebar_second_line`, all unit-tested in `micold-core` (FR-020). A location's card
state comes from one render-free function shared with the Overview (`location_card_state`), so the
two never disagree. The client's render-free `features/sidebar.rs` builds `sidebar_groups()` and the
drawn `sidebar_list()` (headers and rows), remembers per-project group choices for the run, and
re-counts the scroll metrics with headers; navigation steps in grouped order and reveals open a
closed group. The view extends shared components (`TreeItem` supporting line, heading, key;
`TreeView` selection; `KeyedColumn` promoted to `cdk`) and reuses `MenuOverlay`, `ToggleChip`, `Tag`.
The filter accordion and its popover go. Design decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`row().wrap()`, `container`, `column`; `advanced::Widget` for the
promoted `KeyedColumn`); no new crate

**Storage**: none on disk. Group choices live in `sidebar::State` for the run (R4)

**Testing**: `micold-core` unit tests (`mise run test-core`) for every grouping and text rule;
client render-free tests in `crates/micold-client/tests/` (`features_sidebar.rs`, `sidebar_tree.rs`,
`sidebar_state.rs`, `features_navigation.rs`, `navigation*.rs`, `overlay_registry.rs`,
`overlay_dispatch_ordering.rs`) and in-crate modules (`features/overview.rs`); geometry and pixel
tests in `ui/material/tree_view.rs`, `ui/cdk/keyed_column.rs`; contrast in
`ui/material/composition_contrast.rs`; `tests/layout_snapshot.rs`, `tests/layout_text_overflow.rs`,
`tests/gates/containment.rs`; `tests/showcase_completeness.rs`; `tests/icons.rs`; visual pass
(quickstart §B). Whole gate: `mise run gate`

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: grouping per render is O(locations) over state the view already reads; the
card state is computed once per location per render for the sidebar and once for the Overview's
cards as today

**Constraints**: 4.5:1 for every new text pair in both themes; no overlap from 180 to 600 px sidebar
width; scroll metrics derived from data must equal the drawn rows (feature 024); nothing new logged

**Scale/Scope**: core: 1 new module, 1 new function; client render-free: `features/sidebar.rs`
(groups, list, choices, more menu surface, metrics), `features/overview.rs` (`location_card_state`,
`Opened`, `overview_tooltip`), `features/navigation.rs` (grouped order), `features/worktree.rs`
(hover index); client view: `ui/sidebar.rs` rewrite of the header/list composition, `ui/mod.rs` one
overlay surface; shared components: `TreeItem`/`TreeView`, `KeyedColumn` move, one type role, one
icon; docs: two user-guide pages, layout-snapshot doc

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule is red first: core `sidebar_groups` and `sidebar_second_line` (G1–G6, T1); client render-free `sidebar_groups`/`sidebar_list`/choices/metrics/`hover_holds`/tooltip/navigation/`Opened`/more-menu registry in `tests/` and in-crate modules. Shared component changes (`supporting_text`, `heading`, `Selection::Primary`, keyed rows) are red-first geometry and pixel tests in their files. `ui/sidebar.rs` composes tested answers only (glue exception), checked by the layout snapshot and quickstart §B. |
| II. Multi-Session | PASS | A location's group comes from its card state over all its un-archived sessions (`location_card_state`, unchanged rule); a location is in exactly one group (G1, G3). No session state is added; group choices are per project, not per session. |
| III. Worktree Integration | PASS | No worktree or VCS operation. *Default* stays its own row, labelled "Default" / "project root", never styled as a worktree (no branch chip). |
| IV. Local-First | PASS | No network call; issue titles come from the existing lookup; group choices are in memory; nothing logged (FR-023). |
| V. Rust + iced | PASS | iced only. `ListRow` keeps headers out of `SidebarEntry`, so location walks cannot meet one; `StateGroup` is a closed enum and `StateGroup::of` is total; `Option<bool>` makes "never chosen" distinct from "chose open". |
| VI. Cross-Platform | PASS | No platform code; the chord label in the Overview row's tooltip comes from `keymap::overview_chord_label()` (Cmd on macOS). Logical px throughout; CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/worktrees-and-sessions.md` and `docs/user-guide/overview.md` updated in the same change; layout snapshot regenerated and `docs/development/layout-snapshot.md` updated (FR-022). |
| VIII. Components | PASS | Extends `TreeItem`/`TreeView` (builders), promotes `KeyedColumn` to `cdk` rather than copying it, reuses `MenuOverlay`/`MenuTrigger`/`ToggleChip`/`Tag`/`IconButton`; every colour from `Roles`; showcase entries for the new variants (FR-021). |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### Core (`crates/micold-core`)

- **`src/sidebar_groups.rs`** (new, `pub mod sidebar_groups` in `lib.rs`): `StateGroup` (`ALL`,
  `of`, `label`, `starts_open`), `Group<T>`, `group_by_state`, `header_text`, `group_open`,
  `RunCard`, `run_group_state`. Contract: [contracts/sidebar-groups.md](contracts/sidebar-groups.md)
  G1–G6.
- **`src/overview_text.rs`**: `RowKind`, `sidebar_second_line` (T1), composed from `branch_line` and
  `sessions_line`.

### Client render-free (`crates/micold-client/src/features`)

- **`overview.rs`**: `location_card_state` and `pr_signal` (moved out of `ui/overview.rs::card`, R2);
  `Msg::Opened { settings_open }` (R9); `overview_tooltip` (moved from `ui/toolbar.rs`).
- **`sidebar.rs`**: `SidebarGroup`, `ListRow`, `GroupHeaderRow`; `State::sidebar_groups()`,
  `State::sidebar_list()`; `group_choices`, `Msg::GroupToggled`; `open_group_of_reveal_target()`
  (R5); `row_heights`/`row_index`/`location_row`/`current_session_row` over `&[ListRow]` (S7);
  `worktree_tooltip` title and branch lines (FR-012); `hover_holds` (R14); `SidebarMoreMenu`
  surface, `more_menu_open`/`more_menu_anchor`, `Msg::MoreMenuToggled`/`MoreMenuAnchored`, the more
  menu's items with enabled rules (R11); `HEADER_ACTIONS` becomes the chip bar's and the menu's
  action lists. Removed: `filter_open`, `Msg::FilterMenuToggled`, `SidebarFilterPanel`.
- **`navigation.rs`**: `location_nodes` walks `sidebar_groups()` in order, closed groups included
  (R5).
- **`worktree.rs`**: `hovered: Option<(String, usize)>`, `Msg::Hovered(String, usize)` (R14).
- **`../overlay/registry.rs`**: `SidebarMoreMenu` replaces `SidebarFilterPanel` in the surface
  list and displacement sets.

### Client view (`crates/micold-client/src/ui`)

- **`sidebar.rs`**: the Overview row (R9), the chip bar (R11), the list from `sidebar_list()` with
  heading rows and two-line rows (R6, R8), `Selection::Primary`, row keys (R13); the header row,
  `filter_bar` and the accordion removed.
- **`mod.rs`**: the more menu's `MenuOverlay` surface beside the switcher's.
- **`overview.rs`**: `card` calls `features::overview::location_card_state`.
- **`toolbar.rs`**: calls the moved `overview_tooltip`; nothing else changes.
- **`material/tree_view.rs`**: `supporting_text`, `leading_chip` (replaces `chip_after_tag`),
  `heading`, `key`, `Selection`. **`material/text.rs`**: `TypeRole::SidebarGroup`;
  **`material/type_role_mapping.rs`** admits it. **`material/composition_contrast.rs`**: R12 pairs.
- **`cdk/keyed_column.rs`**: moved from `transcript/keyed.rs` (R13); the transcript imports it.
- **`../icons.rs`**: `Icon::MoreHorizontal` (U+E5D3); `assets/fonts/PROVENANCE.md` table row.
- **`../showcase/sections/atoms.rs`**, **`../showcase/catalogue.rs`**: R15 samples and entries.
- **`main.rs`**: `reveal_scroll` and `revealed_scroll` call `open_group_of_reveal_target()` first
  (R5); `WorktreeMsg::Hovered` sites pass the index.

### Tests and fixtures

`tests/support/covered_states.rs` (`main-shell-sidebar-grouped`, and the sidebar states' new shape),
`tests/fixtures/layout_snapshot.txt` regenerated; the filter-accordion exemptions removed from the
layout gates.

### Docs

`docs/user-guide/worktrees-and-sessions.md` (*Browsing worktrees*, *Reading a worktree: name &
tags*, *Refreshing the list*, *Filtering worktrees by tag*, *Resizing and hiding the sidebar*),
`docs/user-guide/overview.md` (opening section), `docs/development/layout-snapshot.md` (covered
states, exemptions), `docs/user-guide/icons.md` (the Overview row, hide and ⋯ locations),
`docs/development/architecture.md` (where it names the sidebar's filter panel).

## Requirement map

| FR | Where | Test layer |
|---|---|---|
| FR-001 groups, order, attention in Needs action | `sidebar_groups::StateGroup::of`, `ALL`; `State::sidebar_groups` | core unit (G1, G2); client render-free (`sidebar_groups` over one location per state) |
| FR-002 header text, counts, empty groups hidden | `header_text`, `group_by_state`; `sidebar_list` | core unit (G3, G4); client render-free (filtered-to-empty group absent; run group counts one) |
| FR-003 order within a group | `group_by_state` stable partition over `sidebar_entries()` order | core unit (G3); client render-free (Default before worktrees, run group placement) |
| FR-004 moves in the same update | groups derived per render from `location_card_state`, the Overview's own state function (R2) | client render-free: a state change flips both the card state and the group in one `State` |
| FR-005 Done collapsed, every group collapsible, choice holds per project | `group_open`, `group_choices`, `GroupToggled` (R4) | core unit (G5); client render-free (toggle, re-read, project switch and back) |
| FR-006 reveal and chords open a closed group; grouped chord order | `open_group_of_reveal_target`, `navigation::location_nodes` (R5) | client render-free (`features_navigation`, `navigation_apply`, reveal drain) |
| FR-007 Overview row first, opens, inert | `overview::Msg::Opened`; `ui/sidebar.rs` row (R9) | client in-crate (`Opened` reducer: closed→open, open→open, Settings, pending activation); layout snapshot (row first) |
| FR-008 same badge as the top bar | one `badge(&cards(state))` and `overview_tooltip` (R9, R10, D7) | client in-crate: row badge and tooltip equal the button's for none / attention / action / both (SC-002) |
| FR-009 Overview selected excludes rows | `ui/sidebar.rs` selection from `overview.open` | client render-free helper `row_selected(state, …)` unit-tested; quickstart B4 |
| FR-010 primary container fill | `TreeView::selection(Selection::Primary)` (R7) | `tree_view.rs` pixel test (fill and text colour); contrast (R12) |
| FR-011 two lines | `sidebar_second_line`, `TreeItem::supporting_text` (R6) | core unit (T1); `tree_view.rs` geometry (second line under the label) |
| FR-012 ellipsis, tooltip with full title and branch | `Ellipsized` second line; `worktree_tooltip` | `tree_view.rs` geometry; client render-free tooltip test; `layout_text_overflow` |
| FR-013 chips kept, type/issue tags not drawn | `ui/sidebar.rs` chip list; `leading_chip` | client render-free test of the row's chip labels (no type/issue); layout snapshot |
| FR-014 chip bar, All, filters unchanged, accordion removed | `ui/sidebar.rs` chip bar; `FiltersCleared`; removals (R11) | client render-free (All active iff no filter, clears all; existing filter tests kept); overlay tests migrated |
| FR-015 add, hide, more menu (shared `Menu`) | `SidebarMoreMenu`, `MenuOverlay` (R11) | `overlay_registry`, `overlay_dispatch_ordering` (open, Escape, outside, displacement); render-free item list with enabled rules |
| FR-016 chips wrap, nothing cut off | `row().wrap()` left of the actions | layout snapshot + containment gate at 180/300/600 px |
| FR-017 keyboard order, Enter/Space toggles, focus stays on moved row | keyed rows (R13), `TakesTheKeyboard` on every control | `keyed_column.rs` (focus follows key); `tree_view.rs` heading toggles on Enter/Space; quickstart B6–B7 |
| FR-018 ripple and context menu unchanged | rows keep `on_right_press`, `Ripple` over the whole row | existing sidebar menu tests kept; `tree_view.rs` ripple bounds cover both lines |
| FR-019 4.5:1 | R12 pairs | `composition_contrast.rs` |
| FR-020 rules in core | `sidebar_groups`, `sidebar_second_line` | `mise run test-core` |
| FR-021 shared components, showcase | R15 | `showcase_completeness`; quickstart §B |
| FR-022 user guide, layout snapshot, visual pass | R15 | `layout_snapshot`; docs gate (`scripts/check-user-guide-updated.sh`); quickstart §B |
| FR-023 nothing logged | `sidebar_groups.rs` pure | mechanical: quickstart §A grep; no unit test (absence of logging is not observable) |

## Delivery order

1. **Core rules** (no GUI): `sidebar_groups`, `sidebar_second_line`.
2. **Render-free client**: `location_card_state` move, `sidebar_groups`/`sidebar_list`, group
   choices, metrics over `ListRow`, reveal and navigation, `Opened`, tooltip lines, hover index.
3. **Shared components**: `KeyedColumn` to `cdk`, `TreeItem`/`TreeView` builders, type role, icon,
   contrast pairs, showcase.
4. **Sidebar view**: Overview row, chip bar and more menu (with the filter accordion's removal and
   the overlay tests' migration), grouped list.
5. **Snapshot and docs**: covered state, regenerated fixture, user guide, layout-snapshot doc,
   visual pass.

## Shared layout code with #1124, #1126, #1127

This plan changes the sidebar column only (R16). It touches files the parallel layout threads may
also touch: `ui/mod.rs` (one overlay surface added; the window composition is not changed),
`ui/toolbar.rs` (only `overview_tooltip` moves out; #1127 owns the title bar and later removes the
Overview button), `tests/support/covered_states.rs` and `tests/fixtures/layout_snapshot.txt`
(regenerate after any rebase, never hand-merge), `ui/material/composition_contrast.rs`, the showcase
catalogue, and `docs/user-guide/overview.md`. It does not touch the terminal area, the session tabs
or a status bar.

## Project Structure

### Documentation (this feature)

```text
specs/1125-sidebar-state-groups/
├── spec.md
├── plan.md              # this file
├── research.md          # R1–R16
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── sidebar-groups.md  # core rules G1–G6, T1
│   └── sidebar-ui.md      # client S1–S8
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── lib.rs                      # + pub mod sidebar_groups;
├── sidebar_groups.rs           # new
└── overview_text.rs            # + RowKind, sidebar_second_line
crates/micold-client/src/
├── features/
│   ├── overview.rs             # location_card_state, pr_signal, Msg::Opened, overview_tooltip
│   ├── sidebar.rs              # groups, ListRow, choices, metrics, more menu, removals
│   ├── navigation.rs           # grouped location order
│   └── worktree.rs             # hovered index
├── overlay/registry.rs         # SidebarMoreMenu
├── icons.rs                    # Icon::MoreHorizontal
├── main.rs                     # reveal drains open the target's group
├── showcase/{catalogue.rs,sections/atoms.rs}
└── ui/
    ├── sidebar.rs              # Overview row, chip bar, grouped list
    ├── mod.rs                  # more menu surface
    ├── overview.rs             # card() uses location_card_state
    ├── toolbar.rs              # overview_tooltip moved out
    ├── cdk/{mod.rs,keyed_column.rs}   # KeyedColumn moved here
    ├── transcript/{mod.rs,keyed.rs}   # keyed.rs removed, import from cdk
    └── material/
        ├── tree_view.rs        # supporting_text, leading_chip, heading, key, Selection
        ├── text.rs             # TypeRole::SidebarGroup
        ├── type_role_mapping.rs
        └── composition_contrast.rs
crates/micold-client/tests/
├── features_sidebar.rs, sidebar_tree.rs, sidebar_state.rs, features_navigation.rs,
│   navigation.rs, navigation_apply.rs, overlay_registry.rs, overlay_dispatch_ordering.rs, icons.rs
├── support/covered_states.rs   # main-shell-sidebar-grouped
└── fixtures/layout_snapshot.txt
assets/fonts/PROVENANCE.md
docs/user-guide/{worktrees-and-sessions.md,overview.md,icons.md}
docs/development/{layout-snapshot.md,architecture.md}
```

**Structure Decision**: Existing workspace layout; rules in `micold-core`, render-free projection
and reducers in `micold-client/src/features`, rendering in `micold-client/src/ui` and its shared
`material`/`cdk` libraries.

## Complexity Tracking

None.
