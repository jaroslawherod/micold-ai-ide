# Implementation Plan: Sidebar keeps a stable order; state shows on the row

**Branch**: `feat/1718_sidebar-stop-grouping-worktrees-sessions-by-state` | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1718-sidebar-stable-order/spec.md`

## Summary

Remove feature 1125's state grouping. The sidebar lists `State::sidebar_entries()` as is (the
stable order); no header, no group toggle, no `group_choices`. Each location row shows its **card
state** (five states) through the existing `StateIcon`, which today draws feature 905's
`LocationState` (three glyphs, Unknown empty). The plan finds that 905's icon cannot meet FR-004 or
SC-002 (it has no Done or Needs attention glyph, and Needs action and Needs attention would read
alike), so `StateIcon` is re-keyed on `CardState` and reuses `overview_card::state_icon`, whose five
icons have five code points (`StateRunning` e037, `CardNeedsAction` e645, `CardNeedsAttention`
e8f4, `CardDone` e86c, `StateIdle` e034). Decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable (via `mise`)

**Primary Dependencies**: iced 0.14; no new crate

**Storage**: none. `group_choices` was in memory only; settings and state files unchanged (FR-008)

**Testing**: `micold-core` unit tests (`mise run test-core`); client render-free tests in
`crates/micold-client/tests/` (`sidebar_state.rs`, `navigation_apply.rs`, `features_sidebar.rs`,
`layout_snapshot.rs`, `gates/*`); `ui/material/state_icon.rs` unit tests; contrast in
`ui/material/composition_contrast.rs`; quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows; all logic is pure core/client logic (Principle VI)

**Project Type**: Rust workspace, desktop GUI

**Performance Goals**: none beyond today's; the list is built once per frame as before

**Constraints**: no colour-only state (FR-004); no on-disk change (FR-008)

**Scale/Scope**: ~10 source files, ~12 test files, 3 doc files, one layout snapshot

## Constitution Check

| Principle | Status | Note |
|---|---|---|
| I Test-first | Pass | Tests first per [quickstart.md](quickstart.md) test map; red before green |
| II Multi-session | Pass | Order independent of any session state; no row moves in any window (FR-003) |
| III Worktree integration | Pass | Worktree and Default rows, run groups unchanged apart from grouping |
| IV Local-first (NN) | Pass | No remote service, no stored field |
| V Rust + iced | Pass | Closed enums kept; `ListRow` collapses to entries |
| VI Cross-platform | Pass | Pure logic shared; no platform branch |
| VII Documentation | Pass | FR-009: user guide, layout-snapshot doc, showcase, module docs updated |
| VIII Reusable UI | Pass | `StateIcon` stays the one shared component; reuses `overview_card::state_icon` |

No violation; Complexity Tracking is empty. Re-checked after design: unchanged.

## Project Structure

```text
specs/1718-sidebar-stable-order/   plan.md research.md data-model.md quickstart.md contracts/sidebar-flat-list.md
crates/micold-core/src/sidebar_groups.rs      # delete StateGroup, Group, group_by_state, header_text, group_open; keep RunCard + run_group_state (move to overview.rs or rename module sidebar_state)
crates/micold-core/src/lib.rs                 # module line
crates/micold-client/src/features/sidebar.rs  # drop SidebarGroup, GroupHeaderRow, ListRow::Header, group_choices, GroupToggled, open_group_of_*; sidebar_list = entries
crates/micold-client/src/features/navigation.rs # location_nodes over sidebar_entries
crates/micold-client/src/ui/sidebar.rs        # drop group_header, Header row key; pass card state to StateIcon
crates/micold-client/src/ui/material/state_icon.rs # re-key on CardState
crates/micold-client/src/showcase/sections/{surfaces,atoms}.rs # drop group header poses; five-state icon pose
docs/user-guide/worktrees-and-sessions.md, docs/development/layout-snapshot.md, tests/layout_snapshot.rs
```

## Requirement map

| Req | Where |
|---|---|
| FR-001, FR-003 | `sidebar_list()` returns `sidebar_entries()` unchanged; contract G1; core/client test cycling states |
| FR-002, FR-007 | Remove `group_header`, `GroupToggled`, `StateGroup`; contract G2; showcase poses removed |
| FR-004 | `StateIcon` on `CardState` (research R1, R2); `row_second_line` unchanged; icon distinctness test (SC-002) |
| FR-005 | Rows, hover, selection, menus, tags, run groups untouched; existing tests keep passing |
| FR-006 | `location_nodes` over `sidebar_entries`; `open_group_of_*` and their calls in `reveal_ready` removed; row heights over entries |
| FR-008 | Nothing stored; test that old state files load (no `group_choices` key was ever stored) |
| FR-009 | Docs, `layout_snapshot` re-record, showcase |

## Test strategy

- **Core unit** (`micold-core`): `run_group_state` kept and tested; card-state to icon mapping
  distinctness is a client test (Icon lives in the client crate).
- **Client render-free**: stable order across all five states for Default, worktree and run group
  (SC-001, SC-003); no header in `sidebar_list` (FR-002); `location_order` / reveal / keyboard over
  the flat list (FR-006); `row_heights` has no header row.
- **Unit in `state_icon.rs`**: five states give five different glyph code points (SC-002) and
  equal-width slots; each state's role colour is in `composition_contrast.rs`.
- **Geometry gates**: layout snapshot and `gates/*` re-recorded for the flat list.
- **Quickstart §B**: visual pass of the flat sidebar with all five states.
