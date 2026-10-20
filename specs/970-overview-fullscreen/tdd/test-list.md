# Test list: Overview full-screen mode (#970)

No `/speckit-tdd-plan` list was derived: the tasks' own red phase (T001–T007) is the loop, as in
`specs/1024-overview-larger-cards/tdd/`. One row per behaviour, by milestone.

## M1 — full screen hides the sidebar and gives it back

| ID | Behaviour | Test | Traces |
|---|---|---|---|
| U1 | The toggle works only while open, Settings closed, no activation pending | `features_overview.rs::full_screen_toggles_only_while_open` | FR-001 |
| U2 | Opening the Overview clears the mode | `features_overview.rs::opening_the_overview_clears_full_screen` | FR-001 |
| U3 | Every exit leaves the mode | `features_overview.rs::every_exit_leaves_full_screen` | FR-010, SC-005 |
| U4 | A project switch keeps the mode | `features_overview.rs::a_project_switch_keeps_full_screen` | FR-011 |
| U5 | Settings open and closed keeps the mode | `overview.rs::settings_open_and_closed_keeps_full_screen` | FR-011 |
| U6 | The mode writes no sidebar state | `overview.rs::the_mode_writes_no_sidebar_state` | FR-009 |
| U7 | The chord is primary+Shift+F only; its label is the platform's | `keymap.rs::is_overview_full_screen_chord_is_primary_shift_f_only`, `overview_full_screen_chord_label_names_the_platform_chord` | FR-001, FR-014 |
| U8 | The app hears the chord as `FullScreenToggled` | `app_chord_tests.rs::the_full_screen_chord_toggles_full_screen` | FR-001 |
| U9 | Two pinned, distinct icons | `icons.rs` (`expected`, the Overview icons test, the set size) | FR-012 |
| U10 | Retracted settles at zero width, all parked | `navigation_drawer.rs::a_retracted_drawer_settles_at_zero_width_with_every_child_parked` | FR-002 |
| U11 | Built retracted starts at zero width | `a_drawer_built_retracted_starts_at_zero_width` | FR-011 |
| U12 | Retraction follows `emphasized`, ends by 500 ms, translates left | `a_retracting_drawer_narrows_on_the_emphasized_curve` | FR-005, SC-002 |
| U13 | Reversed midway continues from its width | `a_retraction_reversed_midway_continues_from_its_width` | FR-007 |
| U14 | Un-retracted shows the drawer as given | `leaving_retraction_shows_the_drawer_as_given` | FR-009 |
| U15 | Header label, tooltip and icon | `ui/overview.rs::the_full_screen_toggle_names_its_action_and_chord` | FR-012 |
| U16 | 20 cycles return the sidebar as it was | `overview.rs::twenty_cycles_return_the_sidebar_as_it_was` | SC-004 |
| U17 | Settings and a project switch keep mode and sidebar | `overview.rs::settings_and_a_project_switch_keep_the_mode_and_the_sidebar` | FR-011 |

## M2 — the cards zoom with the sidebar (`card_grid.rs`, unit tests in `mod tests`)

| ID | Behaviour | Test | Traces |
|---|---|---|---|
| U18 | `zoom_fraction` is the clamped position between the ends | `zoom_fraction_is_the_clamped_position_between_the_ends` | FR-004 |
| U19 | At either end of a zoom the ribbon is the rest grid | `the_ribbon_at_either_end_equals_the_rest_grid` | FR-004 |
| U20 | No overlap, in range, on every sampled frame | `ribbon_pieces_never_overlap_and_stay_in_range` | FR-006, SC-002 |
| U21 | A grid built full screen is at rest | `a_grid_built_full_screen_lays_out_at_rest` | FR-011 |
| U22 | A flip starts a zoom between the two end widths (hand-worked frame at f 0.5) | `a_flip_starts_a_zoom_between_the_two_end_widths` | FR-004, FR-006 |
| U23 | A zoom ends at an end, out of range, or after the slide | `a_zoom_ends_at_an_end_outside_the_range_or_after_the_slide` | FR-005 |
| U24 | A reversal midway keeps the ends | `a_reversal_midway_keeps_the_ends` | FR-007 |
| U25 | A card-count change mid-zoom keeps the zoom | `a_card_count_change_mid_zoom_keeps_the_zoom` | FR-007 |
| U26 | At rest in full screen a row spans the freed width | `full_screen_rows_span_the_freed_width` | FR-002, SC-001 |

## M3 — the scroll anchor

| ID | Behaviour | Test | Traces |
|---|---|---|---|
| U27 | The first visible card stays in view across a zoom | `card_grid.rs::the_first_visible_card_stays_in_view_across_a_zoom` | FR-013 |
| U28 | `Reflowed` changes no state | `features_overview.rs::reflowed_changes_no_state` | FR-013 |

## M4 — Reduce motion

| ID | Behaviour | Test | Traces |
|---|---|---|---|
| U29 | The setting defaults off and round-trips | core `settings.rs::reduce_motion_defaults_off`, `reduce_motion_round_trips` | FR-008 |
| U30 | Draft seed, toggle edits the draft, validated copy, Cancel, Save applies | `features/settings.rs::reduce_motion_tests` (`the_draft_is_seeded_from_the_stored_setting`, `toggling_edits_only_the_draft`, `the_validated_settings_carry_the_draft_value`, `cancel_discards_the_edit`); `main_tests.rs::reduce_motion_is_applied_by_save_and_discarded_by_cancel` | FR-008 |
| U31 | Saving keeps every other setting | `shell/persist.rs::saving_reduce_motion_keeps_every_other_setting` | FR-008 |
| U32 | An instant drawer snaps; a snapped width ends the zoom | `navigation_drawer.rs::an_instant_drawer_retracts_and_shows_at_once`, `turning_instant_on_mid_retraction_completes_it_in_one_step`, `an_instant_drawer_still_slides_the_panel_open`; `card_grid.rs::a_flip_laid_out_at_the_end_width_runs_no_zoom`, `leaving_full_screen_onto_the_start_width_runs_no_zoom`, `a_snapped_width_mid_zoom_ends_the_zoom_on_the_next_layout` | FR-008, SC-003 |

## App wiring (`ui/mod.rs`, tasks T044-T045)

| ID | Behaviour | Test | Traces |
|---|---|---|---|
| U33 | Full screen retracts the drawer, otherwise not | `ui/drawer_flags_tests.rs::full_screen_retracts_the_drawer`, `outside_full_screen_the_drawer_is_not_retracted` | FR-002, FR-009 |
| U34 | Reduce motion makes the drawer instant, otherwise not | `ui/drawer_flags_tests.rs::reduce_motion_makes_the_drawer_instant`, `without_reduce_motion_the_drawer_animates` | FR-008, SC-003 |
| U35 | The Changes exit (`ChangesRequested`) leaves full screen (real message; Compare and `MainAreaTaken` not app-tested) | `overview.rs::opening_changes_leaves_full_screen` | FR-010 |
