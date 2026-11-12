---
description: "Task list for feature 1786: sidebar user groups"
---

# Tasks: Sidebar user groups

**Input**: `specs/1786-sidebar-user-groups/` (spec.md, plan.md, research.md, data-model.md, contracts/sidebar-groups.md, quickstart.md)

**Tests**: mandatory (Constitution I): every implementation task is preceded by the test task that covers it, written red first. `ui/sidebar.rs`, `main.rs` and overlay wiring are glue (Principle I exception) and are covered by quickstart §B.

**Format**: `- [ ] T### [P?] [US?] [A#/U#] description with path`; `[A#]`/`[U#]` are behavior ids of tdd/test-list.md. `[P]` = disjoint files, no dependency on an incomplete task.

## Phase 1: Setup

- [ ] T001 Create `crates/micold-core/src/sidebar_layout.rs` with the types of data-model.md and doc comments, no operations yet: `SidebarLayout { version: u32 (1), groups: Vec<UserGroup>, ungrouped: Vec<LayoutItem> }`, `UserGroup { id: UserGroupId (u64, monotonic, never reused), name: String, collapsed: bool, items: Vec<LayoutItem> }`, `LayoutItem::{Worktree(String), Run(GroupId)}`, `DropTarget::{Group{id,index}, Ungrouped{index}, GroupSlot{index}}`, `LayoutError::{Blank, Duplicate, UnknownGroup, UnknownItem, IsDefault}`; add the `mod` line in `crates/micold-core/src/lib.rs`. Existing `sidebar_groups.rs` (feature 483) stays unchanged.

## Phase 2: Foundational (blocks all stories)

- [ ] T002 [A24][A25][U1][U2][U3][U4][U5][U6] Tests in `crates/micold-core/src/sidebar_layout.rs` (`#[cfg(test)]`): `to_json`/`from_json` round trip; a missing, unparseable, older or newer `version` loads as empty; `reconcile(live)` drops absent items, keeps an item named by two groups in the first group (group order), appends unknown live items to `ungrouped` in live order (live = the 1718 stable order, so a new worktree lands at its stable-order place among the ungrouped, per the spec Assumption), never stores Default (C1, FR-015, FR-018). Red first.
- [ ] T003 [A24][A25][U1][U2][U3][U4][U5][U6] Implement `to_json`, `from_json` (never errors: falls back to empty) and `reconcile` in `crates/micold-core/src/sidebar_layout.rs` (depends on T002).
- [ ] T004 [A24][U7][U8] [P] Tests in `crates/micold-client/tests/sidebar_layout_store.rs`: `JsonFileStore::save_layout`/`load_layout` round trip; missing file gives empty; corrupt file is renamed `.corrupt` and gives empty; write is atomic; modelled on the runs store tests (C7, SC-003). Red first.
- [ ] T005 [A24][U7][U8] Add `layouts_path`, `load_layout`, `save_layout` to `crates/micold-core/src/store.rs` (`JsonFileStore` plus `Store` trait defaults like runs), reusing `write_atomic` and `read_or_quarantine`, file `layouts/<project id>.json` (depends on T003, T004).
- [ ] T006 [U9][U10][U11][U12] [A28] Tests in `crates/micold-client/tests/sidebar_state.rs` and `sidebar_tree.rs`: with an empty layout `sidebar_entries` is unchanged; with a layout, order is Default, then per group `UserGroup(header)` plus its rows (run groups with their runs as one unit) when expanded, then ungrouped rows; collapsed group hides rows but keeps header; empty group keeps header; tag filter hides rows but keeps stored order; `row_heights`/`row_index` count headers; `location_nodes` skips headers; reconcile runs when the live worktree list changes (a worktree added or removed appears in ungrouped or leaves its group); hover, context menu and keyboard navigation (803) work over a grouped list (C5, FR-017). Red first.
- [ ] T007 [U9][U10][U11][U12] [A28] Implement in `crates/micold-client/src/features/sidebar.rs`: `SidebarEntry::UserGroup(GroupHeaderNode)`; the state holds the loaded layout (loaded and reconciled when a project opens, and reconciled again on every change of the live worktree list, so a worktree added or removed by another window or on disk is appended or dropped while the app runs; tested in T006); `sidebar_entries` applies it after `arrange_groups`; update `row_heights`, `row_index`; `reveal_ready` expands a collapsed group holding the target. Edit `crates/micold-client/src/features/navigation.rs` so `location_nodes` skips headers and a header is one stop (depends on T005, T006).
- [ ] T008 [P] Create the shared `GroupHeader::new(name, on_toggle).collapsed(b).count(n).roles(r)` and `DropMarker` builders in `crates/micold-client/src/ui/material/group_header.rs` (export in `ui/material/mod.rs`) and add the role `TypeRole::UserGroupHeader` (not `SidebarGroup`); a long name is truncated, empty group shows an empty state (R7, Principle VIII).

## Phase 3: User Story 1 - Create, rename and delete a group (P1) 🎯 MVP

**Goal**: the user creates, renames, deletes named groups; they persist.
**Independent test**: create, rename, delete a group; header appears, renames, disappears; worktree list unchanged.

Create (scenarios 1, 4, 5 on create):

- [ ] T009 [U13][U14] [US1] Tests in `crates/micold-core/src/sidebar_layout.rs`: `create(name)` trims; `Blank` for empty or whitespace; `Duplicate` case-insensitively on trimmed name; ids monotonic and never reused; state unchanged on error (C2). Red first.
- [ ] T010 [U13][U14] [US1] Implement `SidebarLayout::create` in `crates/micold-core/src/sidebar_layout.rs` (depends on T009).
- [ ] T011 [A1][A4][A5][U31] [US1] Tests in new `crates/micold-client/tests/sidebar_group_actions.rs`: Msg `New` opens the naming draft; confirm with a valid name adds the group and saves; `Blank`/`Duplicate` keep the draft and show the message, layout unchanged; "New group" is an item of `more_menu_items` and of the empty-area menu and works with no worktree; the chord is in the keymap and shortcut sheet; no project open offers no "New group"; reachable without a pointer (SC-006). Red first.
- [ ] T012 [A1][U31] [US1] Create `crates/micold-client/src/features/sidebar_group_actions.rs`: `Msg::New`, naming draft `{ target: New | Rename(id) | NewAndMove(item), text, error }`, apply-and-save outcome (save off the render path, failed save keeps the in-memory layout and sets the status message, R9) (depends on T010, T011).
- [ ] T013 [US1] Edit `crates/micold-client/src/ui/rename_dialog.rs` (title and error line parameters) and register the group naming dialog in `crates/micold-client/src/overlay/registry.rs` beside `WorktreeContextMenu` (R6).
- [ ] T014 [US1] Add "New group" to `more_menu_items` (feature 1125) and a new empty-area sidebar menu, the chord entry in `crates/micold-client/src/keymap.rs` and `crates/micold-client/src/ui/shortcut_sheet.rs` (C6a, FR-001).
- [ ] T015 [US1] Geometry gate test `crates/micold-client/tests/gates/sidebar_group_header_fits.rs`: `row_heights` with headers, long name truncates, header fits the row height; the full name stays reachable (tooltip on the header, and the rename field shows it). Red first.
- [ ] T016 [US1] Glue in `crates/micold-client/src/ui/sidebar.rs` (draw `GroupHeader` rows from `SidebarEntry::UserGroup`, "New group" button in the project header and empty-area menu) and `crates/micold-client/src/main.rs` (route group Msgs) (depends on T007, T008, T012, T013, T014, T015).
- [ ] T017 [US1] User guide: creating a group, naming rules, in `docs/user-guide/worktrees-and-sessions.md` (Constitution VII; CI user-guide gate).

Rename and delete (scenarios 2, 3):

- [ ] T018 [U15][U16] [US1] Tests in `crates/micold-core/src/sidebar_layout.rs`: `rename(id, name)` with `Blank`, `Duplicate` (own current name is not a duplicate), `UnknownGroup`; `delete(id)` appends the group's items to the end of `ungrouped` in their order, `UnknownGroup` otherwise (C1). Red first.
- [ ] T019 [U15][U16] [US1] Implement `rename` and `delete` in `crates/micold-core/src/sidebar_layout.rs` (depends on T018).
- [ ] T020 [A2][A3][A4][A5][U30] [US1] Tests in `crates/micold-client/tests/sidebar_group_actions.rs`: rename through the draft keeps members; blank/duplicate rename refused with message; delete returns members to ungrouped and the worktree and session lists are unchanged (SC-004, FR-003, FR-014); group menu offers Rename, Delete, opens by keyboard (803); both save. Red first.
- [ ] T021 [A2][A3][U30] [US1] Add `Msg::Rename`, `Msg::Delete` and `GroupContextMenu` (Rename, Delete; Move up/Move down are added by T038 and Collapse/Expand by T040) to `crates/micold-client/src/features/sidebar_group_actions.rs`; register `GroupContextMenu` (layer ContextMenu) in `crates/micold-client/src/overlay/registry.rs` (depends on T019, T020).
- [ ] T022 [US1] Glue in `crates/micold-client/src/ui/sidebar.rs`: open the group menu from the header by right click and by the 803 chord; `crates/micold-client/src/main.rs` routing (depends on T021).
- [ ] T023 [US1] User guide: renaming and deleting a group (sessions are never touched) in `docs/user-guide/worktrees-and-sessions.md`.

## Phase 4: User Story 2 - Assign a worktree without dragging (P1)

**Goal**: "Move to group" in the worktree menu. **Independent test**: open the menu by keyboard, pick a group, the row is under it.

- [ ] T024 [U6][U17][U18][U19] [US2] Tests in `crates/micold-core/src/sidebar_layout.rs`: `move_item(item, DropTarget::Group|Ungrouped)` removes before insert so an item is in exactly one list; run group moves as one `LayoutItem::Run`; `UnknownItem`, `IsDefault` errors; index clamped; a property-style loop over all outcomes keeps each item in exactly one list (SC-002, FR-005, FR-016). Red first.
- [ ] T025 [U6][U17][U18][U19] [US2] Implement `move_item` (including in-list moves and index adjustment) in `crates/micold-core/src/sidebar_layout.rs` (depends on T024).
- [ ] T026 [A6][A7][A8][A9][U27] [US2] Tests in `crates/micold-client/tests/sidebar_group_actions.rs`: `move_menu_items(state, key)` lists every group, "Ungrouped", "New group…"; with no group only "New group…" which creates the group and moves the worktree (`NewAndMove`); also for a run group row; Default offers none; keyboard-only move through the 803 menu (SC-006). Red first.
- [ ] T027 [A6][A7][A8][A9][U27] [US2] Implement `move_menu_items`, `Msg::Move`, `NewAndMove` in `crates/micold-client/src/features/sidebar_group_actions.rs` and the "Move to group" items in `crates/micold-client/src/features/worktree.rs` (`WorktreeContextMenu`, run group menu) (depends on T025, T026).
- [ ] T028 [US2] User guide: "Move to group" in `docs/user-guide/worktrees-and-sessions.md`.

## Phase 5: User Stories 3 and 5 - Drag between lists, cancel a drag (P1)

**Goal**: drag a row onto a group or the ungrouped list with highlight and marker; Escape or an invalid drop cancels. **Independent test**: drag ungrouped to group A, to group B, to ungrouped; then Escape, then drop outside; nothing changes on cancel.

- [ ] T029 [A27] [A10][A11][A12][A13][A14][A15][U20][U21][U22][U23] [US3] Tests in new `crates/micold-client/tests/sidebar_drag.rs`: press arms, movement past 4 px activates, release below is a click that selects as before; `hit_test(y, row_heights, entries)` returns `Some` only over a group header, a group's rows, an empty group or the ungrouped rows, otherwise `None`; release commits `move_item` and saves; drop on a collapsed group expands it; drop on own position changes nothing; Default, chip bar and Overview rows never arm; hidden (tag filtered) rows are not targets; a run group row arms, hit-tests and drops as one unit (FR-016); a group header arms for group drag (FR-019); in-list index semantics (dropping below the item's own position adjusts for the removal); sessions untouched (C3, C4, FR-006..FR-008, FR-018). Red first.
- [ ] T030 [A10][A11][A12][A13][A14][A15][U20][U21][U22][U23] [US3] Create `crates/micold-client/src/features/sidebar_drag.rs`: `DragState { armed, active, over }`, `DropTarget` mapping, `hit_test`, reducer (depends on T025, T029).
- [ ] T031 [A20][A21][A22] [US5] Tests in `crates/micold-client/tests/sidebar_drag.rs`: Escape ends the drag with no change; release with `over = None` no change; the dragged worktree vanishing from the live list ends the drag with no change and no error; Escape with no drag keeps its existing meaning; every cancel leaves entries identical (SC-002). Red first.
- [ ] T032 [A20][A21][A22] [US5] Implement cancel paths in `crates/micold-client/src/features/sidebar_drag.rs` (depends on T030, T031).
- [ ] T033 [A13] [US3] Glue: `crates/micold-client/src/ui/sidebar.rs` (`mouse_area` press arms, drop highlight, `DropMarker`; the render-free stand-in for highlight and marker is `DragState.over` plus the marker position, asserted in T029) and `crates/micold-client/src/main.rs` (window cursor-moved, button-released and Escape route to the reducer) (depends on T030, T032).
- [ ] T034 [US3] User guide: dragging and cancelling in `docs/user-guide/worktrees-and-sessions.md`.

## Phase 6: User Stories 4, 6 and 7 - Reorder, collapse and persist, no state grouping (P2)

- [ ] T035 [A16][A17][A18][U24][U25] [US4] Tests in `crates/micold-core/src/sidebar_layout.rs`: `move_item` within a list, `move_group(id, to_index)`, `step_item(item, ±1)`, `step_group(id, ±1)` (no-op or error at the ends), others keep relative order. Red first.
- [ ] T036 [A16][A17][A18][U24][U25] [US4] Implement `move_group`, `step_item`, `step_group` in `crates/micold-core/src/sidebar_layout.rs` (depends on T035).
- [ ] T037 [A27] [A16][A17][A18][U25][U26] [US4] Tests in `crates/micold-client/tests/sidebar_drag.rs` and `sidebar_group_actions.rs`: `hit_test` yields `GroupSlot` between headers only when the dragged item is a group, `Group/Ungrouped` index for in-list reorder; "Move up"/"Move down" in worktree, run group and group menus, disabled at the ends; order survives a redraw. Red first.
- [ ] T038 [A16][A17][A18][U25][U26] [US4] Implement `GroupSlot` hit-test in `crates/micold-client/src/features/sidebar_drag.rs` and `Msg::MoveUp/MoveDown` plus menu items in `crates/micold-client/src/features/sidebar_group_actions.rs` and `crates/micold-client/src/features/worktree.rs` (depends on T036, T037).
- [ ] T039 [A23][A24][A25][U28][U29] [US6] Tests: core `set_collapsed`; client Enter/Space on a focused header toggles; group menu "Collapse"/"Expand"; `reveal_ready` expands the holding group; collapsed state and layout round trip through `save_layout`/`load_layout` (restart, SC-003); stale assignment ignored on load. Files `crates/micold-core/src/sidebar_layout.rs`, `crates/micold-client/tests/sidebar_group_actions.rs`, `sidebar_layout_store.rs`. Red first.
- [ ] T040 [A23][U28][U29] [US6] Implement `set_collapsed`, `Msg::Toggle`, header tab stop with Enter/Space, menu entry in `crates/micold-core/src/sidebar_layout.rs`, `crates/micold-client/src/features/sidebar_group_actions.rs`, `crates/micold-client/src/features/navigation.rs` (depends on T039).
- [ ] T041 [A19][A26] [US7] Test in `crates/micold-client/tests/sidebar_group_actions.rs`: cycle one grouped worktree through all five card states, group and position identical at each step (SC-005, FR-013); regression guard, green on arrival; walk every `SidebarEntry::UserGroup` header and role and assert none is keyed by a card state (US7-2).
- [ ] T042 [US4] User guide: reordering, collapsing, persistence across restarts in `docs/user-guide/worktrees-and-sessions.md`.

## Phase 7: Polish and cross-cutting

- [ ] T043 [P] Showcase poses for group header, drop marker and drop highlight in `crates/micold-client/src/showcase/sections/surfaces.rs` (Principle VIII).
- [ ] T044 Re-record the layout snapshot `crates/micold-client/tests/layout_snapshot.rs` and update `docs/development/layout-snapshot.md`.
- [ ] T045 Run quickstart §B (visual-pass skill), save screenshots of layout, highlight and marker in both themes to `specs/1786-sidebar-user-groups/visual-pass/` (SC-007).

## Dependencies and order

Phase 1 → Phase 2 → US1 (create, then rename/delete) → US2 → US3/US5 → US4/US6/US7 → Polish. US2 needs `move_item` (T025) and the draft (T012). US3 needs `move_item`. US4 extends US3's hit-test. T008 is parallel to T002..T007; T004 parallel to T002.

## Parallel examples

- T002 with T004 (core unit file vs store test file); T008 with T004.
- T043 with T044 after all stories.

## Implementation Strategy

MVP = M1: core layout, store, entries and the create-group flow, so a group header appears, is named and persists. Each later milestone adds one observable behaviour. TDD order is kept inside each phase.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Layout core, store and create a group 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: a user chooses "New group", names it, sees the group header in the sidebar and finds it after a restart; blank and duplicate names are refused with a message.
- **Satisfies**: US1 acceptance scenarios 1, 4, 5 (create); FR-001, FR-004, FR-012, FR-013, FR-014, FR-018; SC-003
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test sidebar_group_actions --test sidebar_layout_store`; quickstart §B step 1
- **Depends on**: —
- **Tier**: full

### M2 — Rename and delete a group

- **Tasks**: T018–T023
- **Deliverable**: the group header menu renames and deletes a group; delete returns its worktrees to the ungrouped list and touches no session.
- **Satisfies**: US1 acceptance scenarios 2, 3 and rename refusals; FR-002, FR-003; SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test sidebar_group_actions`
- **Depends on**: M1
- **Tier**: full

### M3 — Move to group from the menu

- **Tasks**: T024–T028
- **Deliverable**: a worktree or run group menu offers "Move to group" (groups, Ungrouped, New group…) and the row moves, by keyboard too.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-005, FR-010 (Move to group), FR-016 (menu); SC-002, SC-006
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test sidebar_group_actions`
- **Depends on**: M2
- **Tier**: full

### M4 — Drag between lists and cancel

- **Tasks**: T029–T034
- **Deliverable**: dragging a row onto a group or the ungrouped list moves it with highlight and insertion marker; Escape or an invalid drop changes nothing.
- **Satisfies**: US3 acceptance scenarios 1–6; US5 acceptance scenarios 1–3; FR-006..FR-009, FR-018, FR-019 (cancel rules)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test sidebar_drag`; quickstart §B steps 2–3
- **Depends on**: M3
- **Tier**: full

### M5 — Reorder, collapse and no state grouping

- **Tasks**: T035–T042
- **Deliverable**: rows and groups reorder by drag and Move up/down; groups collapse and expand and keep their state across restarts; card state never moves a row.
- **Satisfies**: US4 acceptance scenarios 1–4; US6 acceptance scenarios 1–3; US7 acceptance scenarios 1–2; FR-011, FR-013, FR-017, FR-019 (group reorder, header arming); SC-005
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test sidebar_drag --test sidebar_group_actions`; quickstart §B steps 4–6
- **Depends on**: M4
- **Tier**: full

### M6 — Showcase, snapshot and visual pass

- **Tasks**: T043–T045
- **Deliverable**: the showcase shows the group header, marker and highlight; the layout snapshot and its doc describe user groups; the visual pass is recorded in both themes.
- **Satisfies**: SC-007; Constitution VII, VIII
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`; quickstart §B step 7
- **Depends on**: M5
- **Tier**: full
