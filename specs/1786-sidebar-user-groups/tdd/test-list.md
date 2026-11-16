---
feature: 1786-sidebar-user-groups
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 27 # A19 merges US4-4 and US7-1; A27, A28 cover FR-016, FR-018
planned_at: HEAD
updated_at: HEAD
suite_baseline: unknown
---

# Test List: Sidebar user groups

Entry point for acceptance behaviors: the client's render-free state (`sidebar_entries`, `sidebar_group_actions::update`, `sidebar_drag` reducer) in `crates/micold-client/tests/`; the pointer glue in `ui/sidebar.rs` is covered by quickstart §B (SC-007). The profile has no GUI acceptance runner.

## Outer loop: acceptance behaviors

| id | behavior | traces | kind | state | test |
|---|---|---|---|---|---|
| A1 | New group with a valid name adds a header holding no worktree | US1-1, FR-001 | example | DONE | tests/sidebar_group_actions.rs::a_valid_name_adds_a_header_holding_no_worktree_and_saves |
| A2 | Renaming a group changes the header name and keeps its worktrees | US1-2, FR-002 | example | DONE | tests/sidebar_group_actions.rs::rename_opens_a_draft_holding_the_current_name, crates/micold-core/src/sidebar_layout.rs::tests::rename_changes_the_name_and_keeps_the_members_and_position |
| A3 | Deleting a group returns its worktrees to ungrouped and removes no worktree or session | US1-3, FR-003, SC-004 | example | DONE | tests/sidebar_group_actions.rs::deleting_a_group_returns_its_worktrees_and_touches_no_worktree_or_session |
| A4 | Blank name is refused on create and rename, state kept | US1-4, FR-004 | example | DONE | tests/sidebar_group_actions.rs::a_blank_name_is_refused_and_the_state_stays, a_blank_or_duplicate_rename_is_refused_with_a_message_and_the_state_stays |
| A5 | Duplicate name is refused with a visible message, state kept | US1-5, FR-004 | example | DONE | tests/sidebar_group_actions.rs::a_duplicate_name_is_refused_with_a_visible_message |
| A6 | Move to group via menu moves the row under that group | US2-1, FR-010 | example | DONE | tests/sidebar_group_actions.rs::moving_a_worktree_to_a_group_puts_it_under_that_group_and_saves |
| A7 | Move to group Ungrouped moves the row to the ungrouped list | US2-2, FR-010 | example | DONE | tests/sidebar_group_actions.rs::moving_to_ungrouped_takes_the_row_out_of_its_group |
| A8 | With no group, the menu offers only New group… which creates and moves | US2-3, FR-010 | example | DONE | tests/sidebar_group_actions.rs::new_group_from_the_move_menu_creates_the_group_and_moves_the_row_into_it, with_no_group_the_move_menu_offers_only_new_group |
| A9 | The whole move is possible with keyboard only (met through the Overview card menu; not through a sidebar row menu) | US2-4, SC-006 | example | PARTIAL | tests/sidebar_group_actions.rs::a_keyboard_user_moves_a_row_through_the_card_menu_with_no_pointer (Overview card menu only: the sidebar worktree-row and run-group menus have no keyboard model, so SC-006 is not met for Move on a sidebar row; follow-up) |
| A10 | Drop of a row on a group assigns it at the marked position | US3-1, FR-006 | example | DONE | tests/sidebar_drag.rs::dropping_an_ungrouped_row_on_a_group_assigns_it_at_the_marked_place_and_saves, dropping_on_a_header_puts_the_row_first_in_its_group |
| A11 | Drop on another group moves it out of the first | US3-2, FR-006 | example | DONE | tests/sidebar_drag.rs::dropping_on_another_group_moves_the_row_out_of_the_first |
| A12 | Drop on the ungrouped list removes it from its group | US3-3, FR-006 | example | DONE | tests/sidebar_drag.rs::dropping_on_the_ungrouped_rows_removes_the_row_from_its_group |
| A13 | Over a valid target highlight and marker are drawn, over an invalid place neither | US3-4, FR-008 | example | DONE | tests/sidebar_drag.rs::over_a_target_the_spot_carries_the_marker_and_over_none_nothing_is_drawn; ui::sidebar::tests::rows_and_headers_arm_a_drag_default_does_not_and_the_target_list_is_highlighted |
| A14 | A dropped worktree keeps its sessions listed with it | US3-5, FR-007 | example | DONE | tests/sidebar_drag.rs::a_dropped_worktree_keeps_its_sessions_listed_with_it |
| A15 | Drop on a collapsed group assigns the row and expands the group | US3-6, FR-018 | example | DONE | tests/sidebar_drag.rs::a_drop_on_a_collapsed_group_assigns_the_row_and_expands_the_group |
| A16 | Reordering within a list gives the dropped position, others keep relative order | US4-1, FR-006 | example | DONE | micold-core sidebar_layout::tests::step_item_moves_one_place_in_its_list_and_the_others_keep_their_order; tests/sidebar_drag.rs::a_row_dropped_in_its_own_list_takes_the_marked_place_and_keeps_it_on_redraw |
| A17 | Group header drop among groups reorders groups with their rows | US4-2, FR-019 | example | DONE | micold-core sidebar_layout::tests::move_group_takes_its_items_along_and_counts_the_index_before_the_removal; tests/sidebar_drag.rs::dropping_a_group_header_among_the_groups_reorders_them_with_their_rows, a_group_dropped_on_its_own_place_changes_nothing; tests/sidebar_group_actions.rs::a_groups_move_up_and_down_reorder_the_groups_with_their_rows |
| A18 | Move up/Move down moves a row or group one place | US4-3, FR-010 | example | DONE | tests/sidebar_group_actions.rs::move_up_and_down_move_a_row_one_place_in_its_list_and_save, a_groups_move_up_and_down_reorder_the_groups_with_their_rows |
| A19 | A card state change moves no row | US4-4, US7-1, FR-013, SC-005 | example | DONE | tests/sidebar_group_actions.rs::a_grouped_worktree_cycling_through_every_card_state_keeps_its_group_and_place (regression guard, green on arrival) |
| A20 | Escape during drag ends it with no change | US5-1, FR-009 | example | DONE | tests/sidebar_drag.rs::escape_ends_the_drag_with_no_change, losing_the_window_focus_cancels_the_drag |
| A21 | Drop outside a valid target changes nothing | US5-2, FR-009 | example | DONE | tests/sidebar_drag.rs::a_release_outside_every_target_changes_nothing |
| A22 | Removal of the dragged worktree before the drop ends the drag with no change | US5-3, FR-018 | example | DONE | tests/sidebar_drag.rs::the_dragged_worktree_vanishing_ends_the_drag_with_no_change_and_no_error, a_release_after_the_dragged_worktree_vanished_changes_nothing |
| A23 | Collapse hides rows and keeps the header; expand shows them | US6-1, FR-011 | example | DONE | tests/sidebar_group_actions.rs::toggling_a_header_hides_its_rows_keeps_the_header_and_saves, the_group_menus_collapse_entry_toggles_and_closes_the_menu |
| A24 | Layout, order and collapsed state survive a restart | US6-2, FR-012, SC-003 | example | DONE | tests/sidebar_layout_store.rs::a_saved_layout_loads_back_after_a_restart; tests/sidebar_group_actions.rs::order_and_collapsed_state_survive_a_save_and_a_reload |
| A25 | A stale assignment is ignored on load | US6-3, FR-018 | example | DONE | tests/sidebar_group_actions.rs::a_stored_assignment_to_a_worktree_that_is_gone_is_ignored_on_load |
| A26 | No header names a state | US7-2, FR-013 | example | DONE | tests/sidebar_group_actions.rs::a_grouped_worktree_cycling_through_every_card_state_keeps_its_group_and_place (the header walk, US7-2) |
| A27 | A run group row is dragged and dropped as one unit; a group header arms for group drag | FR-016, FR-019 | example | DONE | tests/sidebar_drag.rs::a_run_group_row_is_dragged_and_dropped_as_one_unit, a_group_header_arms_a_group_drag_that_lands_between_headers_only, dropping_a_group_header_among_the_groups_reorders_them_with_their_rows |
| A28 | A worktree added or removed while running appears in ungrouped or leaves its group | FR-005, FR-018 | example | DONE | tests/sidebar_tree.rs::user_groups::a_worktree_added_while_running_is_ungrouped_and_a_removed_one_leaves_its_group |

## Inner loop: unit behaviors

| id | behavior | traces | kind | state | test |
|---|---|---|---|---|---|
| U1 | to_json/from_json round trip | FR-012 | example | DONE | micold-core sidebar_layout::tests::a_layout_survives_its_own_json |
| U2 | Missing, corrupt, older or newer version loads empty without error | FR-018 | example | DONE | micold-core sidebar_layout::tests::missing_unparseable_older_or_newer_text_loads_as_no_groups |
| U3 | reconcile drops absent items | FR-018 | example | DONE | micold-core sidebar_layout::tests::reconcile_drops_items_no_longer_listed |
| U4 | reconcile keeps a doubly-named item in the first group | C1, FR-005 | example | DONE | micold-core sidebar_layout::tests::reconcile_keeps_an_item_named_twice_in_the_first_group_only |
| U5 | reconcile appends unknown live items to ungrouped in live order | C1 | example | DONE | micold-core sidebar_layout::tests::reconcile_appends_unknown_live_items_to_ungrouped_in_live_order |
| U6 | Default is never stored (IsDefault) | FR-015 | example | DONE | micold-core sidebar_layout::tests::move_item_refuses_what_it_cannot_place_and_changes_nothing (IsDefault) |
| U7 | save/load round trip, missing file empty | C7, SC-003 | example | DONE | tests/sidebar_layout_store.rs::a_saved_layout_loads_back_after_a_restart, a_missing_file_loads_as_no_groups |
| U8 | corrupt file renamed .corrupt, empty layout loads | FR-018 | example | DONE | tests/sidebar_layout_store.rs::a_corrupt_file_is_kept_aside_and_loads_as_no_groups |
| U9 | sidebar_entries with empty layout equals the stable order | FR-013 | example | DONE | tests/sidebar_tree.rs::user_groups::with_no_layout_the_drawn_list_is_the_stable_order |
| U10 | Entries order: Default, group headers with rows, ungrouped | C5 | example | DONE | tests/sidebar_tree.rs::user_groups::groups_come_after_default_with_their_rows_then_the_ungrouped_rows |
| U11 | Collapsed group hides rows, empty group keeps header, tag filter keeps stored order | C5, FR-017 | example | DONE | tests/sidebar_tree.rs::user_groups::a_collapsed_group_keeps_its_header_and_hides_its_rows_and_an_empty_group_keeps_its_header, a_tag_filter_hides_grouped_rows_and_keeps_their_stored_order |
| U12 | Run group is one unit in a group; row_heights/row_index count headers; location_nodes skip them | FR-016 | example | DONE | tests/sidebar_tree.rs::user_groups::a_run_group_sits_in_a_user_group_as_one_unit, row_metrics_count_headers_and_navigation_skips_them |
| U13 | create trims, ids monotonic and never reused | FR-004 | example | DONE | micold-core sidebar_layout::tests::create_trims_the_name_and_adds_an_empty_group_last, created_ids_grow_and_are_never_reused |
| U14 | create Blank and Duplicate (case-insensitive) leave state unchanged | FR-004, C2 | example | DONE | micold-core sidebar_layout::tests::create_refuses_a_blank_name_and_changes_nothing, create_refuses_a_duplicate_name_case_insensitively_and_changes_nothing |
| U15 | rename accepts its own current name, refuses Blank, Duplicate, UnknownGroup | FR-004 | example | DONE | crates/micold-core/src/sidebar_layout.rs::tests::rename_to_its_own_current_name_is_not_a_duplicate, a_refused_rename_leaves_the_layout_as_it_was |
| U16 | delete appends items to ungrouped in order; UnknownGroup otherwise | FR-003, C1 | example | DONE | crates/micold-core/src/sidebar_layout.rs::tests::delete_appends_the_items_to_ungrouped_in_their_order, deleting_an_unknown_group_is_refused_and_changes_nothing |
| U17 | move_item removes before insert, one list only, index clamped | FR-005 | example | DONE | micold-core sidebar_layout::tests::move_item_into_a_group_removes_it_from_where_it_was, move_item_clamps_an_index_past_the_end, move_item_within_its_list_counts_the_index_before_the_removal |
| U18 | move_item UnknownItem and IsDefault errors; run group moves as one | FR-015, FR-016 | example | DONE | micold-core sidebar_layout::tests::move_item_refuses_what_it_cannot_place_and_changes_nothing, a_run_group_moves_as_one_item |
| U19 | all outcomes keep each item in exactly one list | SC-002 | example | DONE | micold-core sidebar_layout::tests::every_outcome_keeps_each_item_in_exactly_one_list |
| U20 | press arms, movement under 4 px is a click, at 4 px activates (both sides) | C3 | example | DONE | tests/sidebar_drag.rs::a_press_arms_and_a_move_under_four_pixels_stays_a_click, a_move_of_four_pixels_activates_the_drag |
| U21 | hit_test returns Some only over valid places, otherwise None; hidden rows not targets | C3 | example | DONE | tests/sidebar_drag.rs::the_hit_test_finds_only_group_headers_group_rows_and_ungrouped_rows, rows_a_tag_filter_hides_are_no_targets_and_keep_their_stored_place |
| U22 | Default, chip bar, Overview never arm | C3, FR-015 | example | DONE | tests/sidebar_drag.rs::default_never_arms; ui::sidebar::tests::rows_and_headers_arm_a_drag_default_does_not_and_the_target_list_is_highlighted (the chip bar and the Overview row are no TreeView rows and carry no press) |
| U23 | drop on own position changes nothing | C4 | example | DONE | tests/sidebar_drag.rs::a_drop_on_the_rows_own_place_changes_nothing, a_drop_below_the_rows_own_place_in_its_list_lands_after_the_row_it_was_dropped_under |
| U24 | step_item/step_group move one place and refuse at the ends | FR-010 | example | DONE | micold-core sidebar_layout::tests::step_item_at_either_end_of_its_list_changes_nothing, step_item_refuses_what_it_cannot_place, step_group_moves_one_place_and_changes_nothing_at_the_ends |
| U25 | move_group reorders groups; hit_test GroupSlot only for a dragged group | FR-019 | example | DONE | micold-core sidebar_layout::tests::move_group_takes_its_items_along_and_counts_the_index_before_the_removal, move_group_refuses_an_unknown_group_and_changes_nothing; tests/sidebar_drag.rs::a_row_dragged_over_a_header_gets_a_list_place_never_a_group_slot, a_group_header_arms_a_group_drag_that_lands_between_headers_only |
| U26 | Move up/down menu items disabled at the ends | C6 | example | DONE | tests/sidebar_group_actions.rs::move_up_and_down_are_disabled_at_the_ends_and_change_nothing_there, move_up_reaches_rows_the_layout_never_held_in_their_drawn_order, a_worktree_and_a_run_group_menu_end_their_move_entries_with_move_up_and_down |
| U27 | move_menu_items lists each group, Ungrouped, New group…; none for Default | C6, FR-010 | example | DONE | tests/sidebar_group_actions.rs::the_move_menu_lists_every_group_then_ungrouped_then_new_group, the_default_row_is_offered_no_move, the_worktree_menu_offers_move_to_group_above_delete |
| U28 | set_collapsed; Enter/Space on a header toggles; menu Collapse/Expand | FR-011 | example | DONE | micold-core sidebar_layout::tests::set_collapsed_marks_only_that_group_and_keeps_its_items; ui::sidebar::tests::a_group_header_is_a_tab_stop_that_toggles_and_opens_its_menu_by_keyboard; tests/sidebar_group_actions.rs::toggling_a_header_hides_its_rows_keeps_the_header_and_saves |
| U29 | reveal_ready expands the group holding the target | FR-017 | example | DONE | tests/sidebar_group_actions.rs::a_reveal_of_a_row_in_a_collapsed_group_expands_that_group |
| U30 | group menu Rename/Delete opens by keyboard | FR-002, C6 | example | DONE | ui::sidebar::tests::a_group_header_is_a_tab_stop_that_toggles_and_opens_its_menu_by_keyboard; tests/sidebar_group_actions.rs::a_keyboard_opened_group_menu_highlights_its_first_entry_and_steps_over_disabled_ones, a_pointer_opened_group_menu_takes_the_keyboard_at_the_first_arrow |
| U31 | New group reachable from more menu, empty-area menu, chord; absent without a project | FR-001, C6a | example | DONE | tests/sidebar_group_actions.rs::new_group_is_on_the_more_menu_with_no_worktree_needed, no_project_open_offers_no_new_group, the_new_group_chord_is_primary_shift_g_and_on_the_shortcut_sheet (empty-area menu still open, T014) |
| U32 | A header's count is `n` while none is hidden, `n of m` while a tag filter hides some, `empty` only for a group with no member; a member whose worktree is gone is not counted as hidden (decision D7) | FR-017, C3, C5 | example | DONE | tests/sidebar_group_actions.rs::a_header_counts_the_rows_shown_and_says_how_many_a_filter_hides, a_group_holding_no_member_has_nothing_hidden, a_member_whose_worktree_is_gone_is_not_counted_as_hidden, crates/micold-client/src/ui/material/group_header.rs::tests::a_group_whose_members_a_filter_hides_is_not_called_empty |

## Out of scope

Nested groups, several groups per worktree, colours, sessions dragged between worktrees, reordering Default/Overview/chip bar, sharing groups, state-based grouping.

## Verification commands

- single: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- fast subset: `mise run test-core`
- suite: `mise run gate` before a push (never `cargo test --workspace` in the loop)
