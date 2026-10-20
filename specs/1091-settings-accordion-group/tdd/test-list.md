# Test list: Settings accordions form a group (1091)

Paths under `crates/micold-client/`. Status: `retro` = written test-after, red is the retroactive mutant red only; `mutant-red` = written after the audit, fails with the named mutant applied.

| # | Behavior | Criterion | Test | Status |
| --- | --- | --- | --- | --- |
| 1 | Default key open until another opens; opening replaces | FR-001 | `accordion_group::tests::the_default_is_open_until_another_opens` | retro |
| 2 | Pressing the open key closes to none; a closed key reopens; no default starts closed | FR-002 | `pressing_the_open_key_closes_it`, `a_closed_key_can_reopen`, `a_group_without_a_default_starts_closed` | retro (split, finding 10) |
| 3 | Reset returns to the default (after opening, after closing) | FR-007 | `reset_returns_to_the_default_after_*` | retro |
| 4 | Two groups share nothing | FR-003 | `two_groups_are_independent` | retro |
| 5 | `in_group` sets `open` from the group | FR-001 | `in_group_sets_open_from_the_group` | retro |
| 6 | Every kind: open replaces, press closes, draft unchanged | FR-001, FR-002, FR-004, FR-005 | `daemons_feature::opening_a_section_replaces_the_open_one_and_pressing_it_closes_it` | retro (split, finding 7) |
| 7 | Rapid presses leave the last pressed open, never two | SC-001 | `a_rapid_sequence_of_presses_leaves_the_last_pressed_open` | retro (rewritten, finding 6) |
| 8 | Kind change resets to the first section | FR-007 | `a_kind_change_resets_to_the_first_section` | retro |
| 9 | New form does not inherit the open section | FR-007 | `a_new_form_does_not_inherit_the_previous_open_section` | retro |
| 10 | Editing a second daemon resets to the first section | FR-007 | `editing_a_second_daemon_resets_the_open_section` | mutant-red (A) |
| 11 | Stale press for a section outside the kind changes nothing | none (guard from `bd1e87877`) | `a_stale_press_for_a_section_outside_the_kind_changes_nothing` | mutant-red (F) |
| 12 | Error-held section stays open beside the user's; press on it not recorded | FR-006 | `an_error_held_section_stays_open_beside_the_group` | retro |
| 13 | Several held sections stay open together | FR-006 | `several_held_sections_stay_open_together` (asserts literal sections) | retro (strengthened, finding 9) |
| 14 | Showcase group exclusive; independent set untouched | FR-001, FR-003, FR-008 | `showcase_state::grouped_sections_are_exclusive_and_leave_the_independent_set_alone` | retro |
| 15 | Independent sections close and reopen on their own | FR-003 | `showcase_state::independent_sections_close_and_reopen_on_their_own` | mutant-red (K) |
| 16 | Grouped pose paints the group's state | FR-008 | `showcase_accordion_group::the_grouped_pose_paints_one_open_body_by_default`, `the_grouped_pose_follows_a_press` | mutant-red (L) |
| 17 | Keyboard activation of a header | FR-005 | none; owned by the `Button` widget (finding 11, accepted) | not covered |
