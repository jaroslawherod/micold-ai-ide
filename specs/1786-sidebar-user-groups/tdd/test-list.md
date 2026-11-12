---
feature: 1786-sidebar-user-groups
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 26
planned_at: HEAD
updated_at: HEAD
suite_baseline: unknown
---

# Test List: Sidebar user groups

Entry point for acceptance behaviors: the client's render-free state (`sidebar_entries`, `sidebar_group_actions::update`, `sidebar_drag` reducer) in `crates/micold-client/tests/`; the pointer glue in `ui/sidebar.rs` is covered by quickstart §B (SC-007). The profile has no GUI acceptance runner.

## Outer loop: acceptance behaviors

| id | behavior | traces | kind | state | test |
|---|---|---|---|---|---|
| A1 | New group with a valid name adds a header holding no worktree | US1-1, FR-001 | example | PENDING | tasks T011, T012 |
| A2 | Renaming a group changes the header name and keeps its worktrees | US1-2, FR-002 | example | PENDING | tasks T020, T021 |
| A3 | Deleting a group returns its worktrees to ungrouped and removes no worktree or session | US1-3, FR-003, SC-004 | example | PENDING | tasks T020, T021 |
| A4 | Blank name is refused on create and rename, state kept | US1-4, FR-004 | example | PENDING | tasks T011, T020 |
| A5 | Duplicate name is refused with a visible message, state kept | US1-5, FR-004 | example | PENDING | tasks T011, T020 |
| A6 | Move to group via menu moves the row under that group | US2-1, FR-010 | example | PENDING | tasks T026, T027 |
| A7 | Move to group Ungrouped moves the row to the ungrouped list | US2-2, FR-010 | example | PENDING | tasks T026, T027 |
| A8 | With no group, the menu offers only New group… which creates and moves | US2-3, FR-010 | example | PENDING | tasks T026, T027 |
| A9 | The whole move is possible with keyboard only | US2-4, SC-006 | example | PENDING | tasks T026, T027 |
| A10 | Drop of a row on a group assigns it at the marked position | US3-1, FR-006 | example | PENDING | tasks T029, T030 |
| A11 | Drop on another group moves it out of the first | US3-2, FR-006 | example | PENDING | tasks T029, T030 |
| A12 | Drop on the ungrouped list removes it from its group | US3-3, FR-006 | example | PENDING | tasks T029, T030 |
| A13 | Over a valid target highlight and marker are drawn, over an invalid place neither | US3-4, FR-008 | example | PENDING | tasks T029, T030, T033 |
| A14 | A dropped worktree keeps its sessions listed with it | US3-5, FR-007 | example | PENDING | tasks T029, T030 |
| A15 | Drop on a collapsed group assigns the row and expands the group | US3-6, FR-018 | example | PENDING | tasks T029, T030 |
| A16 | Reordering within a list gives the dropped position, others keep relative order | US4-1, FR-006 | example | PENDING | tasks T035, T036, T037, T038 |
| A17 | Group header drop among groups reorders groups with their rows | US4-2, FR-019 | example | PENDING | tasks T035, T036, T037, T038 |
| A18 | Move up/Move down moves a row or group one place | US4-3, FR-010 | example | PENDING | tasks T035, T036, T037, T038 |
| A19 | A card state change moves no row | US4-4, US7-1, FR-013, SC-005 | example | PENDING | tasks T041 |
| A20 | Escape during drag ends it with no change | US5-1, FR-009 | example | PENDING | tasks T031, T032 |
| A21 | Drop outside a valid target changes nothing | US5-2, FR-009 | example | PENDING | tasks T031, T032 |
| A22 | Removal of the dragged worktree before the drop ends the drag with no change | US5-3, FR-018 | example | PENDING | tasks T031, T032 |
| A23 | Collapse hides rows and keeps the header; expand shows them | US6-1, FR-011 | example | PENDING | tasks T039, T040 |
| A24 | Layout, order and collapsed state survive a restart | US6-2, FR-012, SC-003 | example | PENDING | tasks T002, T003, T004, T005, T039 |
| A25 | A stale assignment is ignored on load | US6-3, FR-018 | example | PENDING | tasks T002, T003, T039 |
| A26 | No header names a state | US7-2, FR-013 | example | PENDING | tasks T041 |

## Inner loop: unit behaviors

| id | behavior | traces | kind | state | test |
|---|---|---|---|---|---|
| U1 | to_json/from_json round trip | FR-012 | example | PENDING | tasks T002, T003 |
| U2 | Missing, corrupt, older or newer version loads empty without error | FR-018 | example | PENDING | tasks T002, T003 |
| U3 | reconcile drops absent items | FR-018 | example | PENDING | tasks T002, T003 |
| U4 | reconcile keeps a doubly-named item in the first group | C1, FR-005 | example | PENDING | tasks T002, T003 |
| U5 | reconcile appends unknown live items to ungrouped in live order | C1 | example | PENDING | tasks T002, T003 |
| U6 | Default is never stored (IsDefault) | FR-015 | example | PENDING | tasks T002, T003, T024, T025 |
| U7 | save/load round trip, missing file empty | C7, SC-003 | example | PENDING | tasks T004, T005 |
| U8 | corrupt file renamed .corrupt, empty layout loads | FR-018 | example | PENDING | tasks T004, T005 |
| U9 | sidebar_entries with empty layout equals the stable order | FR-013 | example | PENDING | tasks T006, T007 |
| U10 | Entries order: Default, group headers with rows, ungrouped | C5 | example | PENDING | tasks T006, T007 |
| U11 | Collapsed group hides rows, empty group keeps header, tag filter keeps stored order | C5, FR-017 | example | PENDING | tasks T006, T007 |
| U12 | Run group is one unit in a group; row_heights/row_index count headers; location_nodes skip them | FR-016 | example | PENDING | tasks T006, T007 |
| U13 | create trims, ids monotonic and never reused | FR-004 | example | PENDING | tasks T009, T010 |
| U14 | create Blank and Duplicate (case-insensitive) leave state unchanged | FR-004, C2 | example | PENDING | tasks T009, T010 |
| U15 | rename accepts its own current name, refuses Blank, Duplicate, UnknownGroup | FR-004 | example | PENDING | tasks T018, T019 |
| U16 | delete appends items to ungrouped in order; UnknownGroup otherwise | FR-003, C1 | example | PENDING | tasks T018, T019 |
| U17 | move_item removes before insert, one list only, index clamped | FR-005 | example | PENDING | tasks T024, T025 |
| U18 | move_item UnknownItem and IsDefault errors; run group moves as one | FR-015, FR-016 | example | PENDING | tasks T024, T025 |
| U19 | all outcomes keep each item in exactly one list | SC-002 | example | PENDING | tasks T024, T025 |
| U20 | press arms, movement under 4 px is a click, at 4 px activates (both sides) | C3 | example | PENDING | tasks T029, T030 |
| U21 | hit_test returns Some only over valid places, otherwise None; hidden rows not targets | C3 | example | PENDING | tasks T029, T030 |
| U22 | Default, chip bar, Overview never arm | C3, FR-015 | example | PENDING | tasks T029, T030 |
| U23 | drop on own position changes nothing | C4 | example | PENDING | tasks T029, T030 |
| U24 | step_item/step_group move one place and refuse at the ends | FR-010 | example | PENDING | tasks T035, T036 |
| U25 | move_group reorders groups; hit_test GroupSlot only for a dragged group | FR-019 | example | PENDING | tasks T035, T036, T037, T038 |
| U26 | Move up/down menu items disabled at the ends | C6 | example | PENDING | tasks T037, T038 |
| U27 | move_menu_items lists each group, Ungrouped, New group…; none for Default | C6, FR-010 | example | PENDING | tasks T026, T027 |
| U28 | set_collapsed; Enter/Space on a header toggles; menu Collapse/Expand | FR-011 | example | PENDING | tasks T039, T040 |
| U29 | reveal_ready expands the group holding the target | FR-017 | example | PENDING | tasks T039, T040 |
| U30 | group menu Rename/Delete opens by keyboard | FR-002, C6 | example | PENDING | tasks T020, T021 |
| U31 | New group reachable from more menu, empty-area menu, chord; absent without a project | FR-001, C6a | example | PENDING | tasks T011, T012 |

## Out of scope

Nested groups, several groups per worktree, colours, sessions dragged between worktrees, reordering Default/Overview/chip bar, sharing groups, state-based grouping.

## Verification commands

- single: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- fast subset: `mise run test-core`
- suite: `mise run gate` before a push (never `cargo test --workspace` in the loop)
