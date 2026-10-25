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
| Release listener maps only the left release (T030; Esc moved to `on_escape`, review A F2) | shell `subscriptions::tests::changes_drag_message_ends_on_release_only` (was `..._and_cancels_on_esc`) | review B B1: run red with the Esc arm put back: `left: Some(Changes(GutterDragCancelled)) right: None` | `changes_drag_message` maps `ButtonReleased(Left)` only |
| User's read forgets "chose a gone file" (M3 MINOR a) | `hold::a_users_read_forgets_that_the_chosen_file_was_gone` | assertion: "b.rs dropped out of a background read while open: its diff stays" (got NoLongerChanged) | `forget_hold` in `request_user_read` / `list_read`; reset in `apply_tree_hold` and `SourceChosen` |
| Slots stay under the pre-press pick while dragging (visual pass finding) | `drag::the_slots_stay_put_while_the_pick_follows_the_pointer` | found on screen (drag 10→15 picked 10–13), fix written first; review B B1: run red afterwards with `slot_pick` returning `view.pick`: `left: Some(Pick { anchor: 2, head: 4 }) right: Some(Pick { anchor: 1, head: 1 })` | `slot_pick` (drag's `before_pick`) for the composer and **Add comment**; a drag retargets the box on release only |
| A click still takes an open new box to its line (C4) | `drag::a_click_takes_an_open_new_box_to_its_line`; `a_composer_whose_lines_are_gone_waits_for_a_new_pick_and_keeps_its_text` now releases | the old test failed after the slot fix ("the new pick places it": None) | `Release::Click` → `follow_pick` |
| **Add comment** ends a dangling drag | `features_pr_changes` `review_comments::*` (press then Add comment) | 3 failed: "an edit, got None" | `AddComment` clears `view.drag` |

## M4 review round 1 fixes (review A F1–F3)

| Behaviour | Test | Red seen | Green by |
|---|---|---|---|
| F1: a press and its release in one batch of window events end the drag (fast click, tap) | `diff_view::tests::a_number_cell_publishes_the_release_of_its_own_press` | assertion at diff_view.rs:1545: only the press published (`.on_gutter_release` wired, cell not yet publishing) | `GutterState.pressed`; the cell publishes `on_release` on `ButtonReleased(Left)` after its own press; `.on_gutter_release(GutterDragReleased)`; idempotent via `drag.take()` |
| F2: Esc during a drag cancels it and leaves the drawer open | `overlay_dispatch_ordering::escape_during_a_drag_cancels_it_before_closing_the_drawer` | `left: Some(Changes(Drawer(Escape))) right: Some(Changes(GutterDragCancelled))` | `on_escape` arm before the drawer's; Esc arm dropped from `changes_drag_message` |
| F3: leaving the tab mid-drag cancels it | `drag::leaving_the_tab_mid_drag_cancels_it` | assertion features_changes.rs:6853 (`drag.is_none()`) | `hide_shown` calls `cancel_drag` |
| F3: window focus loss mid-drag cancels it | `drag::the_window_losing_focus_mid_drag_cancels_it` | assertion features_changes.rs:6866 (stub `cancel_drag`) | `changes::cancel_drag` (shared with `GutterDragCancelled`) |
| F3: the window losing focus mid-drag cancels it in the app (the gate refused a main.rs call, `feature_registration_cost`, and a deciding root arm, `root_is_routing_only`) | shell `subscriptions::tests::changes_drag_message_ends_on_release_and_cancels_on_blur` | run with the `Unfocused` arm removed: `left: None right: Some(Changes(GutterDragCancelled))` | the drag listener maps `Window(Unfocused)` to `GutterDragCancelled`, as the Overview's blur does |

## M5 — suggestion chips (T035–T043)

| Behaviour | Test | Red seen | Green by |
|---|---|---|---|
| A chip that takes the keyboard is one Tab stop answering Enter/Space, at the plain chip's size; plain or disabled is no stop (T037) | material `toggle_chip::tests::a_keyboard_chip_is_one_stop_answering_enter_and_space_at_the_plain_size` | compile error: no method `takes_the_keyboard` on `ToggleChip` | opt-in `.takes_the_keyboard()` wraps the rippled chip in `TakesTheKeyboard` (Enter, Space, ring) |
| The switch on the draft: off by default, seeded, toggled draft-only, validated into `Settings`, Cancel discards, another window's save refreshes it (T035) | client lib `features::settings::suggestion_chips_tests::*` (5) | compile error: no field `suggestion_chips` on `AppearanceDraft` / `settings::State` | `AppearanceDraft`/`ValidSettings`/`State.suggestion_chips`, `SuggestionChipsToggled`, `refresh!` entry |
| A form save writes the switch and keeps every other stored setting (T038) | bin `shell::persist::…::saving_suggestion_chips_keeps_every_other_setting` | mutation (no `set!(suggestion_chips)`): "the choice is written" | `set!(suggestion_chips)` in `set_changed`; T014's stopgaps removed |
| Save applies the switch to the window and the Changes view; Cancel discards (T038, T039) | bin `tests::suggestion_chips_are_applied_by_save_and_discarded_by_cancel` | compile error (no `SuggestionChipsToggled`); then mutation (no `suggestion_chips_in_force` in the save): "Save applies it" | `shell::changes::suggestion_chips_in_force` from `save_and_prepare_check` and at start |
| Q10 chips under New/Edit/Answer boxes only with the setting on | `chips::q10_chips_show_under_every_box_only_with_the_setting_on` | compile error: no `chips_shown` / `Msg::SuggestionChipsInForce` / `State.suggestion_chips` | `chips_shown` |
| Q11 a chip fills an empty box, Save sends it, the box is focused | `chips::q11_a_chip_fills_an_empty_box_and_counts_as_typed_text` | same compile error | `Msg::ChipPressed` → `chips::insert` + `Effect::FocusComposer` (the shell's `sync_composer` puts the cursor at the end) |
| Q11 a chip adds a line after written text | `chips::q11_a_chip_adds_a_line_after_written_text` | same compile error | same |
| Q11 nothing off, without a box, or out of range | `chips::q11_a_chip_does_nothing_off_without_a_box_or_out_of_range` | same compile error | `chip_pressed` guards |
| Q12 the setting in force follows each message | `chips::q12_the_setting_in_force_follows_each_message` | same compile error | `SuggestionChipsInForce` arm |
| H14 chip while held; Save quotes the lines shown | `hold::while_held_a_chip_fills_the_box_and_save_quotes_the_lines_shown` | same compile error | no hold check in `chip_pressed` |
| Chips row + switch in the layout snapshot (T042) | `layout_snapshot` new state `changes-review-held-chips` | fixture mismatch until re-recorded (new state; the switch's row in `settings-view-rail-collapsed`, `status-bar-settings`, `title-bar-settings`) | `UPDATE_LAYOUT_SNAPSHOT=1` (523 lines added, none removed) |
