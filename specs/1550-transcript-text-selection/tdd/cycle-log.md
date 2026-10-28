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
| Review A M3 F1 | an empty markdown span no longer panics `LinkText` (`split_at` drops empty ranges) | unfixed `split_at`: `an_empty_span_draws_without_a_piece_of_its_own`, `split_at_cuts_pieces_at_the_selection_ends` FAILED | `micold-material` lib 441 passed |

## M4a (T033–T038)

T033 was written first, against `RenderCtx` with its new `selected`/`row` fields and no leaf wired,
so it compiled and failed on behaviour. T034's budgets are new measurements of code that already
existed after T038 (structural).

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T033 (red) | parity: every fixture, folded and open, `RICH` and `PLAIN`, notice and request/resolution rows — the leaves' `(TextPath, text)` per row equal `units(row, view)`; a highlighted view reports the same | first run: all 4 FAILED, every row `drawn []`; mutant after green (header summary at `[2]`): 3 FAILED, 19 mismatches | `transcript_selection_parity` 4 passed |
| T035 | `RenderCtx { selected, row: RowCtx }`, `hit_msg`, the entry's lazy key holding `in_row(Entry(id, Body))`; `PLAIN` leaves at `[at]` / `[at, k]` (`BlockRef.at`) | T033 | same |
| T036 | `MarkdownView::selection` (R3 paths, per-block lazy key with `RowSelection::clip`), `CodeBlock`/`InlineDiff::selectable` from `RICH` | T033; `clip_keeps_the_part_over_a_blocks_units` written with `clip` | `micold-core` selection 11 passed |
| T037 | card headers (`AccordionSection`/`DisclosureCard::selectable`, `HeaderText`), `IconLabel::selectable`, captions, footer, raw rows, user text; `Units::header` drops an open card's summary | T033 (open cards drew no summary but units listed one); `a_selectable_header_reports_its_title_and_folded_summary` | `micold-material` lib 442 passed; `features_transcript_selection` 33 passed (2 M2 tests updated to the open header) |
| T038 | request blocks (`PromptCard::title_text`/`note_text`) and resolution rows at `Entry(id, Prompt(k))` / `End(k)`, the omitted notice at `Omitted`; `Selectable` passes its child's node through | `layout_snapshot` FAILED (wrapper node added a level) until the pass-through; `showcase_completeness` FAILED until `HeaderText::Summary` was named | `layout_snapshot` 169 passed, unchanged fixture; `showcase_completeness` 46 passed |
| T034 | select-all + `copy_text` < 1 s; one drag step (hit → update → view/layout) < 100 ms, release, `MAX_EVENTS` | structural | `transcript_open_budget` (release) 6 passed: copy 0.85 ms, slowest drag step 27 ms |

## M4b (T039–T043)

T039/T040's tests were written with T041/T042 in one pass (the wiring is a handful of lines per
behaviour), so their red evidence is a mutant run against the finished code: each behaviour
reverted in place (Press not refocusing, `edge_scroll` always `None`, granularity from a fixed
single click, Copy always enabled, the menu not registered as a surface, a second menu not closing
the first), one build, every new test red, then reverted.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T039 (red) | `Press` focuses `FieldId::Transcript(s)`; `Edge` → `Scroll::By(±EDGE_SCROLL_STEP)` for the dragging session only, none without a drag or after release; the area's click count decides the granularity (M3 A F4) | mutant run: `a_press_gives_the_transcript_the_keyboard`, `an_edge_scrolls_the_dragging_session_only_and_only_while_dragging`, `the_areas_click_count_decides_the_granularity` (and the M2 `double_and_triple_clicks_select_a_word_and_a_line`) FAILED | `features_transcript_selection` 40 passed; `shell::transcript` (bin) 1 passed: an edge during a drag is one task, none for another session |
| T040 (red) | menu items Copy (`None` without a selection) and Select all; empty transcript Copy disabled; Escape closes the registered menu and the menu holds the keyboard; one menu per window | mutant run: `the_menu_offers_copy_only_with_a_selection_and_select_all_always`, `an_empty_transcript_shows_copy_disabled`, `escape_closes_the_menu_and_the_menu_holds_the_keyboard`, `opening_a_menu_closes_another_sessions` FAILED (8 of 40 in all) | same |
| T041 | `SelectionArea` around the transcript's stack (`transcript_area_id`), `TrackFocusAs` for it in `ui/focus.rs`, `Scroll::By` → `scroll_by` in `shell/transcript.rs`; folded summary highlight (`Ellipsized::highlighted`, M4a follow-up) | T039; `the_shown_part_of_a_selection_is_highlighted` | `micold-material` lib passed |
| T042 | `ui::transcript::context_menu` on the window overlay, `TranscriptContextMenu` registered (`overlay/registry.rs`) | T040 | same |
| T043 | user guide *Selecting and copying transcript text* | docs | `scripts/tests/*.test.sh` in the gate |

## M5 (T044–T049)

T044's tests were written against T046's API (the `TranscriptMsg::Copy` message and the entry-based
`RichCache::copy` do not exist before it, so the first red is a compile failure); the behavioural
red is a mutant run: `copy_value` answering `None` for every tool target, one build, then reverted.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T044 (red) | `ToolValue` copies `work_value` uncut and exists iff `Some`; `ToolInput` copies `ToolCard::input` as held, the text so far while it streams; `ToolResult` copies the parts `\n`-joined with `[image not shown]`, only once a result exists; `DiffPath` copies `EditDiff::path`; an empty value copies `""`; state per `(session, key, target)` per window, expiring as a code block's | mutant run: `the_header_copies_the_uncut_work_value_and_only_when_there_is_one`, `the_input_and_result_copy_what_the_card_holds`, `a_streaming_input_copies_the_text_so_far`, `an_empty_value_copies_the_empty_string`, `a_controls_confirmation_is_its_own` FAILED (5 of 24) | `features_transcript_rich` 24 passed (#940's code-copy tests unchanged but for the message's name) |
| T045 (red) | the `DiffPath` control is one focus stop; a press, Enter and Space each send its message; it sits on the path's line, after it; at the narrowest pane it fits and the path wraps before it | compile red (no `InlineDiff::copy_path` before T047) | `ui::material::inline_diff` 9 passed; `layout_snapshot` `inline_diff_path_copy_fits` passed, fixture re-accepted (the showcase's diffs gained the control and the MultiEdit a long path) |
| T046 | `CopyTarget::{ToolValue, ToolInput, ToolResult, DiffPath}`, `copy_value`, `DerivedBody::Tool` (every tool card is held, for its controls' state), `TranscriptMsg::Copy`; a card in a subagent's thread is copied by the root from the open threads | T044 | same |
| T047 | `copy_control` (the code block's `IconButton` in `TakesTheKeyboard`) on the tool card header (`ToolValue`), above the unfolded input and result, and beside the diff's path (`InlineDiff::copy_path`) | T045 | same |
| T048 | user guide *Copy controls*; #939's out-of-scope note points to #1550 | docs | `scripts/tests/*.test.sh` in the gate |
