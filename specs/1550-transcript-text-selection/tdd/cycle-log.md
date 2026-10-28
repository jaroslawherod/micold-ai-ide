# TDD cycle log: 1550

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers. Append only, newest last.

## M1 (T001–T012)

T007/T008 and T011/T012 ran red before their code: the tests were written first and failed to
compile. The selection tests (T002–T004, T009) were written in the same step as `selection.rs`, so
their red runs were recovered after the fact (review B M1 F1), two ways, on `75cbb5030` plus the
`bounds` simplification:

- **structural**: without `selection.rs` the test file does not compile
  (`unresolved import micold_core::structured::selection`);
- **mutant**: five mutants in `selection.rs`, applied together, then
  `cargo test -p micold-core --no-fail-fast --lib` and `--test structured_selection`: 4 of 10 unit
  tests and 6 of 13 integration tests FAILED, each mutant failing its own task's tests. The file
  was restored by copy and both suites ran green.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | module, `pub mod selection;`, test file, `built`/`replayed` moved into `tests/support/stream_json.rs` | structural | `structured_transcript` 72 passed unchanged |
| T002 (red), T005 | `TextPath`/`RowPos`/`TextPos` order, `snap` | mutant (`snap` returns the offset unsnapped): `snap_returns_the_nearest_grapheme_boundary`, `a_position_built_in_a_text_is_snapped` FAILED; mutant (`in_row` first row `To`): `a_range_from_the_omitted_notice_into_an_entry_covers_both_rows`, `a_range_across_request_rows_lists_the_rows_in_drawing_order` FAILED | `structured_selection` 13 passed |
| T003 (red), T006 | `Selection` `start`/`extend`/`is_empty`/`bounds`/`in_row`, `range_in` | mutant (`bounds` keeps the anchor first): `a_selection_extended_backwards_has_normalised_bounds` FAILED; mutant (`in_row` first row `To`): `in_row_gives_each_rows_part` FAILED; mutant (`range_in` ignores a start offset): `range_in_slices_each_unit_of_a_row` FAILED | `selection::tests` 4 passed |
| T004 | stability while recorded fixtures stream | structural only: it asserts transcript properties (entry ids by key kept, the grown entry's selected prefix unchanged) that hold without a mutant in `selection.rs` | `structured_selection` 13 passed |
| T007 (red), T008 | `markdown::shown_text`, `list_marker`; `markdown_view` draws `list_marker` | `error[E0432]: unresolved imports super::list_marker, super::shown_text` | `markdown::tests` passed |
| T009 (red), T010 | `word_at`, `source_line_at`, granularity in `start`/`extend`/`bounds` | mutant (runs never joined): `word_at_selects_a_word_a_space_run_or_a_punctuation_run` FAILED; mutant (granularity ignores `expand`): `a_selection_by_word_or_line_extends_by_whole_units`, `a_selection_is_empty_iff_its_expanded_start_is_its_end` FAILED | same |
| T011 (red), T012 | `summary::work_value` | `error[E0425]: cannot find function work_value in this scope` | `summary::tests` passed |

## M2 (T013–T023)

Every test task was written before its code and ran red as a compile failure
(`unresolved imports micold_client::features::transcript_selection::{copy_text, rows, units, …}`,
`no variant … CopySelection found for enum KeyIntent`, `this function takes 2 arguments but 5
arguments were supplied`, `no method named copy_selection`, `no field transcript_selection`).
After green, four mutants applied together (`applied` never compares a changed entry's print,
every unit joined by a line break, `transcript_key` ignores `focused`, `toggled` clears outside the
range) failed six tests, each its own task's; the files were restored by copy.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T013 (red), T014 | `rows`, `units`, `entry_units`, `copy_text`; shown text moved to `features/transcript_selection/shown.rs` | structural; mutant (joints all `\n`): `markdown_copies_its_shown_text_markers_cells_and_code` FAILED | `features_transcript_selection` 28 passed |
| T015 (red), T016 | `State`, `SelectMsg`, gesture, granularity, select-all, menu | structural | same |
| T015 (red), T017 | clearing: rebuild, eviction, notice, changed text, prompts, toggle, forget | structural; mutants (no print compare, toggle inverted): `growth_past_the_focus_keeps_it_and_replaced_text_in_range_clears_it`, `a_toggle_in_range_clears_it_and_one_outside_keeps_it` FAILED | same |
| T018 (red), T019 | `retain_shown` from `shell/panes.rs::sync` through `transcript_selection_sync` | structural | same |
| T020 (red), T021 | `transcript_key(press, phase, selecting, focused, mac)`, `KeyIntent::{CopySelection, SelectAll}`, `FieldId::Transcript`, `transcript_keys` | E0061 on the old signature; mutant (`focused` ignored): `k11_…`, `k5_k6_…`, `k7_…` FAILED | `features_composer` 40 passed |
| T022 (red), T023 | `CopyTarget`, `Checked`, `copy_selection`, the copy outcome and "Copy failed" notice | E0432 `Checked`, `CopyTarget`; E0599 `copy_selection` | `features_transcript_rich` 18 passed |
| Review A M2 fixes | empty copy text writes nothing; menu Copy/Select all focus the transcript without a selection; Show in full in range clears; `transcript_focused` gates the unfocused transcript listener | stash of the `src` fix: `a_selection_that_covers_no_text_copies_nothing`, `the_menus_copy_and_select_all_focus_the_transcript_without_a_selection`, `show_in_full_in_range_clears_it_and_one_outside_keeps_it` FAILED; `the_transcript_holds_the_keyboard_only_while_nothing_replaces_it` E0599 (`transcript_focused`) | `features_transcript_selection` 32 passed |

## M3 (T024–T032)

The component tests (T024, T025, the leaf tests of T026 and the gate of T027) were written in the
same step as their widgets, so their red runs were recovered by mutants after green, as M1's were.
Each set of mutants was applied together, the suites run with `--no-fail-fast`, and the files
restored by copy.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T024 (red), T028 | `Selectable` hit-testing: one paragraph per hard line, `Hit { offset, click, line }`, drag, links first, the `Shown` operation | mutants (line start not added; every move republished): `a_press_publishes_the_nearest_boundary_in_the_whole_text`, `a_drag_publishes_each_new_offset_once` FAILED | `selectable_text::tests` 9 passed |
| T025 (red), T030 | `SelectionArea`: controls first, focus, `on_press` before the hit, clamped cursor and `on_edge`, `on_context`, `on_release` | mutants (no clamp; click count always 1): `a_drag_past_an_edge_clamps_the_pointer_and_scrolls_each_frame`, `a_press_on_text_takes_focus_and_comes_before_the_hit` FAILED | `selection_area::tests` 8 passed |
| T026 (red), T029, T031 | `.selectable` on `Text`, `LinkText`, `CodeBlock`, `InlineDiff`: pixel-equal with nothing selected, selected bytes on the fill, code/diff lines in bytes of their unit | mutants (code line base 0; diff line starts not advanced; highlight `surface_container_high`/`on_surface`): `a_press_on_a_line_answers_in_bytes_of_the_whole_code`, `each_line_reports_its_full_text_while_wider_than_the_block`, `selected_lines_are_drawn_on_the_selection_fill`, `a_press_on_a_line_answers_in_bytes_of_the_joined_lines`, `the_path_captions_and_line_text_are_reported_without_the_gutter` FAILED | `micold-material` lib 440 passed; `ui::material` 648 passed |
| T027 (red) | contrast gate: `on_primary_container` on the fill ≥ 4.5:1 over six transcript surfaces, both schemes; the fill ΔE ≥ 10 from each | first run red on the design: a luminance margin (1.1:1) failed light code surface (1.06), diff added (1.10), diff removed (1.04) — a tint of another hue; the margin became CIE76 ΔE ≥ 10. Mutant (highlight = code fill): `selected_transcript_text_is_legible_on_the_highlight_over_every_surface` FAILED | same |
| T032 | showcase entries `SelectableText`, `SelectionArea`; `Selectable` exempted like `TakesTheKeyboard` | `every_named_variant_has_an_instance` FAILED until the `Click`, `Edge` and `DiffPart` variants were named | `showcase_completeness` 46, `material_builder_api` 13 passed |
