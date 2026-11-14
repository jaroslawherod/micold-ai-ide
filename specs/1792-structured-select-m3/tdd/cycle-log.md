# TDD cycle log: 1792

No `tdd/test-list.md` was derived for this feature; the M3 tasks name the behaviours, so each red
below is a task's test, run before its implementation.

## M3 (T013–T021)

Command: `scripts/build-lock.sh cargo test -p micold-client --lib -- expand_tests collapsing_the_options_row a_picker_opened_by_command`

### Red (row still built only when open, no picker closing, no D1)

| Test | Task | Red for |
|---|---|---|
| `the_row_takes_the_standard_duration_and_none_under_reduce_motion` | T013 | stub duration `0ns`, not `MEDIUM_2` |
| `opening_and_closing_take_the_standard_duration_without_a_jump` | T013 | "took 0 ms": full height on the first frame |
| `under_reduce_motion_the_row_switches_in_one_step` | T013 | passed red: the instant switch was already the behaviour; kept as the FR-013 pin |
| `a_toggle_mid_way_reverses_from_the_current_height` | T014 | "mid-way: 160": no intermediate height |
| `two_sessions_keep_separate_tracks` | T015 | "still on its way: 160 vs 160": no track |
| `a_collapsed_rows_controls_are_not_reachable` | T016 | red only on the probe's own "+2 stops" threshold (corrected while green); the old code did not build a collapsed row, so this pins `expand`'s hidden guard rather than driving code |
| `collapsing_the_options_row_closes_its_pickers` | T017 | "model picker closed" |
| `at_a_narrow_width_the_row_ends_at_its_wrapped_height` | T018 | grew by 90.2, not row 82.2 plus gap (first assertion form) |
| `a_picker_opened_by_command_opens_the_options_row` | D1 | "/model opens the row" |

### Green

T019 (`around_options`, `options_motion` in `ui/composer.rs`), T020 (`State::options_toggled` in
`features/composer.rs`, close-all on the three picker stores), D1 (`open_model_picker`,
`open_effort_picker` expand the row). Two test corrections while green, both measurement, not
behaviour: arrival is read at the end height exactly (the decelerating tail spends its last
frames within half a pixel), and the open row adds at least one keyboard stop (the pose's model
trigger), not two. All 9 pass.
