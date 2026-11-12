# Research: Sidebar user groups

## R1 Where the layout lives
- **Decision**: new core module `sidebar_layout` with its own `layouts/<project id>.json`, mirroring `runs/`.
- **Rationale**: pure, iced-free, unit-testable (Principle I); `runs` already has atomic write and quarantine helpers (`write_atomic`, `read_or_quarantine`) to reuse; a corrupt layout must not touch session state (FR-018).
- **Rejected**: a field in the per-project state file (couples corruption of layout to sessions, and pre-split files must keep loading); settings.json (global, not per project).

## R2 What identifies a member
- **Decision**: a worktree by `dir_name`, a run group by its `GroupId`; `LayoutItem::{Worktree(String), Run(GroupId)}`.
- **Rationale**: both are the keys `arrange_groups` already uses; a run group moves as one unit (FR-016). Names survive restarts; absent ones are dropped on `reconcile`.
- **Rejected**: path (breaks on rename/move of the project); session id (sessions follow worktrees, FR-007).

## R3 Applying the layout
- **Decision**: apply after `arrange_groups` in `sidebar_entries`: Default first, then each group's header and (when expanded) its rows in stored order, then ungrouped rows in saved order, unknown rows appended in 1718 stable order.
- **Rationale**: 1718's order and run grouping stay the base; the layout is a pure function over them, so card state cannot reach it (FR-013).
- **Rejected**: reordering inside `arrange_groups` (mixes two features, 483 tests churn).

## R4 Drag mechanism
- **Decision**: client reducer `sidebar_drag`: `mouse_area` on_press arms (row key), window cursor-moved and button-released events from the existing subscription feed it, `hit_test(y, row_heights, entries)` returns a `DropTarget`; a move under a small threshold is a click, preserving selection (edge case).
- **Rationale**: iced 0.14 has no native drag-and-drop widget; the sidebar already computes `row_heights` for scrolling, so a hit-test needs no new layout query; works with mouse and trackpad on all three platforms.
- **Rejected**: OS drag-and-drop (platform-specific, Principle VI); a per-row widget with its own state (cannot see other rows).

## R5 Keyboard and menu path
- **Decision**: extend `WorktreeContextMenu` with "Move to group" (flat list: groups, "Ungrouped", "New group…"), "Move up", "Move down"; new `GroupContextMenu` (Rename, Delete, Move up, Move down) built like it; headers are tab stops of the 803 navigation.
- **Rationale**: feature 803 already opens the row menu by keyboard (US2-4, SC-006); a flat list avoids a submenu widget the menu primitive lacks.
- **Rejected**: nested submenu (new primitive, no gain); keyboard drag (heavier than Move up/down).

## R6 Naming input and errors
- **Decision**: reuse the rename dialog pattern (`ui/worktree_rename.rs`, `rename_dialog.rs`): a draft with inline error from `LayoutError`; names compared trimmed, case-insensitive for duplicates.
- **Rationale**: one existing dialog idiom (Principle VIII).
- **Rejected**: inline edit in the header (new text-edit-in-row behaviour, focus traps).

## R7 Header component
- **Decision**: new shared `GroupHeader` and `DropMarker` builders in `ui/material/group_header.rs`; no revival of `TypeRole::SidebarGroup` semantics, a new role name `UserGroupHeader`.
- **Rationale**: Principle VIII builder API; 1718 deleted the state header and its tests, so a differently-named role avoids implying state grouping (FR-013, US7-2).
- **Rejected**: restoring the 1718-removed header builder.

## R8 Concurrent windows
- **Decision**: last whole-file save wins; load runs `reconcile`, which keeps a duplicated worktree in the first group.
- **Rationale**: matches spec edge case; no locking needed, no worktree lost.
- **Rejected**: merge on save (needs a change log, out of scope).

## R9 Save timing
- **Decision**: save after every committed operation (drop, menu, toggle) off the render path, like `save_runs`; a failed save keeps the in-memory layout and surfaces the usual status message.
- **Rejected**: save on exit (lost on crash, SC-003).
