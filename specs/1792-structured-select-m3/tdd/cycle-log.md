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

### Moved to the shell (feature write isolation)

`tests/main.rs` `feature_write_isolation` refused the composer feature writing the effort and
permission features' state, so the collapse's close-all and the reopen-on-press moved into the
shell (`shell/composer.rs` `options_toggled`, the `EffortPickerToggled` arm,
`shell/permission_prompts.rs` `ModeMenuToggled`). T017's test moved with them, driven through
`update` now: `shell::composer::tests::collapsing_the_options_row_closes_its_pickers` and
`a_list_opened_with_the_row_collapsed_opens_the_row` (D1 plus review A F1, row state written each
time). Red evidence for both is the features-level red above; the moved tests ran green at once.

## Review B round 1 F2: mutation checks for the two pins that passed red-less

- `under_reduce_motion_the_row_switches_in_one_step`: mutated `options_motion` (`ui/composer.rs`) to ignore `reduce_motion` (`if false && state.settings.reduce_motion`). `cargo test -p micold-client --lib under_reduce_motion_the_row_switches_in_one_step` FAILED (panic at `composer_expand_tests.rs:124`). Mutation reverted with `git checkout`; the pin is real.
- `a_collapsed_rows_controls_are_not_reachable`: accepted pin. The hidden guard it pins lives in `expand` in the shared `micold-material` crate (`animation.rs`), outside this feature's files; mutating it would rebuild another crate for a pre-existing widget behaviour. Its open-row half proves the probes reach the controls (the test fails if they stop doing so).

## M1 (T002–T010), after the fact (close unit)

No red was recorded for M1: its tests were written after the implementation (review B round 1
said so). Each was re-proved by a deliberate mutant instead, run by the close unit. Command:
`scripts/build-lock.sh cargo test -p micold-client --lib -- menu_select_anatomy`.

| Mutant | Result |
|---|---|
| `menu_select.rs` trigger press guard `(!s.disabled).then_some(..)` to `Some(..)` | red: `a_disabled_trigger_looks_disabled_and_ignores_the_pointer` |
| `menu_select.rs` `list_open = s.open && !s.rows.is_empty()` to `s.open` | red: `an_empty_option_list_opens_no_menu` |
| `select_trigger.rs` hover layer: `.disabled(t.disabled)` to `.disabled(true)` | red: `hovering_the_trigger_shades_its_container`, `an_open_trigger_thickens_its_indicator_as_a_select_does`, `a_disabled_trigger_looks_disabled_and_ignores_the_pointer` |
| `select_trigger.rs` disabled chevron colour branch removed | SURVIVES: no test reads the chevron's colour when disabled (follow-up) |

Not re-proved: trigger geometry and long-value ellipsis (covered by their tests, mutants not run).

## Close unit additions (T024, T025, T028)

| Test | Proof |
|---|---|
| `the_collapsed_row_adds_no_gap_whichever_parts_surround_it` (`composer_expand_tests.rs`) | red under `let between = false;` and under `gap(under)` to `gap(false)` in `around_options` (the two survivors of the tdd-verify), green restored |
| `pressing_the_trigger_paints_a_layer_other_than_hover`, `keyboard_focus_on_the_trigger_draws_a_ring` (`menu_select_anatomy.rs`) | red when the press, resp. the focus step, is removed from the test (shows the test can fail; no product-code mutant was run for these two), green restored. They check that the paint changes, not that it equals `Select`'s: the trigger shows the button's ring, not the field's focus layer (research.md, M1) |
| `the_model_and_permission_selectors_share_one_trigger_on_one_line` (`composer_fits.rs`) | now asserts a chevron beside each named label; effort stays unnamed (it is offered only for some models) |
