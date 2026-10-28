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
