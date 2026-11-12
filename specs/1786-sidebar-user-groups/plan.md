# Implementation Plan: Sidebar user groups

**Branch**: `feat/1786_feat-sidebar-let-the-user-create-groups` | **Date**: 2026-11-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1786-sidebar-user-groups/spec.md`

## Summary

Add user-created groups to the sidebar. A new render-free core module `sidebar_layout` owns the
stored **Sidebar layout** (groups, ordered members, ungrouped order) and every pure operation on it
(create, rename, delete, move, reorder, collapse, reconcile against the live worktree list). It is
stored per project in `layouts/<project id>.json`, beside `runs/`, with the same atomic write and
quarantine-on-corrupt as `runs`. The client's `State::sidebar_entries()` applies the layout to the
stable-order entries (after `arrange_groups`), producing groups as a new `SidebarEntry::UserGroup`
header followed by its member rows, and the ungrouped rows after. Drag is a pointer-tracked
reducer in the client (`features/sidebar_drag.rs`): press on a row arms it, window cursor events
move it, a hit-test over `row_heights` yields a `DropTarget`, release commits through the core
operation, Escape or an off-list release cancels. The menu path ("Move to group", "Move up/down",
group Rename/Delete) reuses the worktree context menu (`WorktreeContextMenu`) and a new
group-header menu built the same way. Decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable (via `mise`)

**Primary Dependencies**: iced 0.14 (`mouse_area`, window event subscription); serde/serde_json already in `micold-core`; no new crate

**Storage**: `layouts/<project id>.json` under the app state directory, JSON, atomic write; Principle IV

**Testing**: `micold-core` unit tests (`mise run test-core`) for `sidebar_layout`; client render-free
tests in `crates/micold-client/tests/` (`sidebar_groups.rs` new, `sidebar_state.rs`,
`sidebar_tree.rs`, `navigation_apply.rs`, `gates/*`, `layout_snapshot.rs`); quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows; no platform branch, drag uses iced pointer events only

**Project Type**: Rust workspace, desktop GUI

**Performance Goals**: layout applied once per frame over the existing entry list (O(rows))

**Constraints**: no row position depends on card state (FR-013); corrupt file never crashes (FR-018)

**Scale/Scope**: 1 new core module, 2 new client modules, ~6 edited client files, ~8 test files, 2 doc files, 1 snapshot

## Constitution Check

| Principle | Status | Note |
|---|---|---|
| I Test-first | Pass | Core operations and drag reducer are pure and tested red first; `ui/sidebar.rs` glue only calls them (Principle I exception), covered by quickstart §B |
| II Multi-session | Pass | Layout holds worktree names only, never session state; sessions follow the worktree; two windows: last whole-file write wins, load de-duplicates (FR-018) |
| III Worktree integration | Pass | No worktree, branch or session is created, changed or removed by any group operation (FR-003, FR-014); Default stays first, unassignable (FR-015) |
| IV Local-first (NN) | Pass | One local JSON file per project; no network |
| V Rust + iced | Pass | `LayoutItem`/`DropTarget` closed enums; a worktree in two groups is made unrepresentable by `reconcile` and by operations that move, not copy |
| VI Cross-platform | Pass | Pure logic shared; drag on mouse/trackpad everywhere; the menu path is the fallback |
| VII Documentation | Pass | `docs/user-guide/worktrees-and-sessions.md`, `docs/development/layout-snapshot.md`, showcase poses, module docs |
| VIII Reusable UI | Pass | Group header is a new shared `GroupHeader` builder in `ui/material/` (`GroupHeader::new(name, on_toggle).collapsed(b).count(n).roles(r)` into `Element`); insertion marker is `DropMarker`; both consumed by the sidebar and the showcase |

## Complexity Tracking

None. Feature 1718 removed `StateGroup` headers; user group headers are a different component
(`GroupHeader`) and do not revive `TypeRole::SidebarGroup`'s state meaning (research R7).

Re-checked after design: unchanged.

## Project Structure

```text
specs/1786-sidebar-user-groups/   plan.md research.md data-model.md quickstart.md contracts/sidebar-groups.md
crates/micold-core/src/sidebar_layout.rs   # new: SidebarLayout, UserGroup, LayoutError, ops, reconcile, to_json/from_json
crates/micold-core/src/lib.rs              # module line
crates/micold-core/src/store.rs            # layouts_path, load_layout, save_layout (JsonFileStore + Store trait defaults like runs)
crates/micold-client/src/features/sidebar.rs # SidebarEntry::UserGroup(GroupHeaderNode); sidebar_entries applies layout; row_heights, row_index, reveal_ready expand a collapsed group
crates/micold-client/src/features/sidebar_drag.rs # new: DragState, DropTarget, hit_test, reducer
crates/micold-client/src/features/sidebar_groups.rs # new: Msg (New, Rename, Delete, Move, MoveUp/Down, Toggle), naming draft, apply+save outcome
crates/micold-client/src/features/navigation.rs # location_nodes skips headers; keyboard reaches headers (803)
crates/micold-client/src/features/worktree.rs # menu items: Move to group submenu, Move up/down
crates/micold-client/src/ui/sidebar.rs     # header row, drop highlight, marker, drag wiring (glue)
crates/micold-client/src/ui/material/group_header.rs # new shared GroupHeader, DropMarker builders
crates/micold-client/src/showcase/sections/surfaces.rs # group header, marker, highlight poses
crates/micold-client/src/main.rs           # Escape and window cursor events route to drag (glue)
crates/micold-client/tests/{sidebar_groups.rs,sidebar_drag.rs,sidebar_layout_store.rs} # new
docs/user-guide/worktrees-and-sessions.md, docs/development/layout-snapshot.md, tests/layout_snapshot.rs
```

## Requirement map

| Req | Where |
|---|---|
| FR-001, FR-002, FR-004 | `SidebarLayout::{create,rename,delete}` + `LayoutError::{Blank,Duplicate}`; sidebar_groups Msg; naming draft shown with the error (contract C2) |
| FR-003, FR-014 | `delete` returns members to ungrouped; tests assert worktree/session lists unchanged |
| FR-005, FR-016 | `LayoutItem` keyed by location key (worktree dir name or run group id); `move_item` removes before insert |
| FR-006, FR-008, FR-009, FR-019 | `sidebar_drag` reducer + `hit_test` (contract C3, C4); `DropMarker`; Escape and off-target cancel |
| FR-007 | Layout holds no session data; entries built from `active_sessions()` as today |
| FR-010 | Menu items from `sidebar_groups::move_menu_items(state, key)`; keyboard via feature 803 menu path |
| FR-011, FR-017 | `UserGroup.collapsed`; `reveal_ready` expands the holding group; header is a navigation stop |
| FR-012, FR-018 | `layouts/<id>.json`; `from_json` + quarantine; `reconcile` drops stale, appends new, first-group-wins |
| FR-013 | Layout never reads a state; test cycling five card states (SC-005) |
| FR-015 | Default is never in `items`; `sidebar_entries` puts it first |
| SC-007, docs | quickstart §B, docs, snapshot, showcase |

## Test strategy

- **Core unit** (`sidebar_layout`): every operation and error; round trip; corrupt/old/missing file
  loads empty; two-group duplicate on load; stale names dropped; new worktrees append to ungrouped;
  property-style loop over all outcomes keeps each item in exactly one list (SC-002).
- **Client render-free**: `sidebar_entries` with a layout (order, collapsed hides rows, empty group
  header kept, tag filter hides but keeps order, run group moves as one unit, Default first); card
  state cycling leaves position (SC-005); drag reducer (press without move is a click, hit-test,
  drop outcomes, Escape, vanished worktree); `reveal_ready` expands; menu items; keyboard reaches
  every action without a pointer (SC-006).
- **Store**: save/load/quarantine in `sidebar_layout_store.rs`, as the runs store tests.
- **Geometry gates / snapshot**: `row_heights` with headers; layout snapshot re-recorded.
- **Quickstart §B**: visual pass of layout, drop highlight and marker, both themes (SC-007).
