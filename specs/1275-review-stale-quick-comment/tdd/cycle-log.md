# TDD cycle log — #1275

One line per behaviour: the test, how red was seen, what made it green.

## M4 — drag to comment (T026–T034) and carried MINOR fixes

| Behaviour | Test | Red seen | Green by |
|---|---|---|---|
| `Drag::span` with a 0 line number does not panic (M2/M3 MINOR b) | core `review::drag::tests::d9_a_zero_line_number_spans_from_line_one` | panicked at `LineRange::new(..).expect` (drag.rs:67) | clamp start/end with `.max(1)` as `Pick::range` |
| Number cell publishes press (Shift extends) and `over` on entry only while dragging (T026) | `diff_view::tests::a_number_cell_reports_presses_and_drag_entries_only_while_dragging` | compile error: no field `over` / `dragging` on `Gutters` | `Gutters.over/dragging`, `GutterState { modifiers, inside }`, `.on_gutter_over`, `.dragging` |
| Q1 press starts a drag remembering pick + box | `drag::a_press_starts_a_drag_that_remembers_the_pick_and_box_before_it` | compile error: no `OpenView.drag` / `Msg::GutterDragOver` | `DragState`, `GutterPressed` uses `gutter_line` and starts the drag |
| Q1/FR-021 nothing where picking is not allowed | `drag::a_drag_where_picking_is_not_allowed_picks_nothing` | same compile error | drag only starts inside the `pick_allowed` branch |
| Q2 Shift-click extends, no drag | `drag::shift_click_extends_the_pick_and_starts_no_drag` | same compile error | `drag = (!extend).then(..)` |
| Q3 pick follows the pointer on the pressed side | `drag::the_pick_follows_the_pointer_on_the_pressed_side_only` | same compile error | `GutterDragOver` → `Drag::over`, `Pick::of_drag` |
| Q4 click release only picks | `drag::a_release_on_the_pressed_line_only_picks_it` | same compile error | `gutter_drag_released`: `Release::Click` → nothing more |
| Q5 range release opens + focuses the box | `drag::a_range_release_opens_the_box_under_the_released_line_and_focuses_it` | same compile error | new `Composer` + `Effect::FocusComposer` |
| Q6 moves an open new box, keeps text | `drag::a_range_release_moves_an_open_new_box_and_keeps_its_text` | same compile error | retarget `New` composer |
| Q7 keeps an edit box | `drag::a_range_release_keeps_an_edit_box_and_its_text` | same compile error | `Edit`/`Answer` arm returns early |
| Q8 Esc restores pick + box | `drag::esc_restores_the_pick_and_box_from_before_the_press` | same compile error | `GutterDragCancelled` restores `before_*` |
| Q9 release away from numbers ends at last line, no scroll | `drag::a_release_away_from_the_numbers_ends_at_the_last_line_passed` | same compile error | window-wide release listener + reducer |
| File change mid-drag ends it | `drag::selecting_another_file_mid_drag_ends_it` | same compile error | `close_new` clears `drag` |
| Release/Esc listener maps only those events (T030) | shell `subscriptions::tests::changes_drag_message_ends_on_release_and_cancels_on_esc` | written with the listener (glue; no red run) | `changes_drag_message` |
| User's read forgets "chose a gone file" (M3 MINOR a) | `hold::a_users_read_forgets_that_the_chosen_file_was_gone` | assertion: "b.rs dropped out of a background read while open: its diff stays" (got NoLongerChanged) | `forget_hold` in `request_user_read` / `list_read`; reset in `apply_tree_hold` and `SourceChosen` |
| Slots stay under the pre-press pick while dragging (visual pass finding) | `drag::the_slots_stay_put_while_the_pick_follows_the_pointer` | not run red: found on screen (drag 10→15 picked 10–13), fix written first | `slot_pick` (drag's `before_pick`) for the composer and **Add comment**; a drag retargets the box on release only |
| A click still takes an open new box to its line (C4) | `drag::a_click_takes_an_open_new_box_to_its_line`; `a_composer_whose_lines_are_gone_waits_for_a_new_pick_and_keeps_its_text` now releases | the old test failed after the slot fix ("the new pick places it": None) | `Release::Click` → `follow_pick` |
| **Add comment** ends a dangling drag | `features_pr_changes` `review_comments::*` (press then Add comment) | 3 failed: "an edit, got None" | `AddComment` clears `view.drag` |
