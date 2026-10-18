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
