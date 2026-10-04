# Cycle Log: Reporter, Labels and a Description Tooltip in the Issue List

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 2629 passed, 0 failed, 2 ignored (profile, observed at `cdc473ab`; not re-run for this design change)
- commit: `d9deabff` (test list planned against it)
- recorded: cycle 0, before any change. The first M1 cycle re-measures on its own base.

## Baseline, re-measured for M1

- suite (fast subset): `scripts/build-lock.sh cargo test -p micold-core --all-targets` on the M1 base
  (`3d52e83e` plus this branch's ledger commit): every test that existed before M1 passed in the
  run that produced cycle 1's red (1513 passed, 7 ignored; the 14 failures are the new tests below).
- suite (workspace): not re-run before the first cycle (343 s, shared build lock). The base is CI's
  green `main` at `3d52e83e`. The workspace suite runs once, in the milestone's gate (T017).
- recorded: before any source change of M1.

## Batching note (M1)

As in `specs/034-daemon-mcp-server/tdd/cycle-log.md`: cycles are grouped per task pair (the test
task and its implementation task). A group's tests are written together and observed failing
together against stubs that only make the symbols resolve, then made green together. One entry per
group, every behavior id listed with its test. The inner loop runs the crate's suite; the workspace
suite runs in the gate.

## Cycle 1 — U1, U2, U3, U4, U5, U6 (A15's request half) — T001, T007

- tests: `crates/micold-core/tests/github_parse.rs::{a_node_with_an_author_parses_to_that_reporter (U1),
  a_node_without_an_author_is_reported_by_ghost (U2), a_bots_login_is_kept_as_reported (U3),
  every_query_holds_the_shared_node_selection (U4), a_node_parses_alike_from_every_source (U5)}` (new);
  `list_args_send_only_the_repository` and `search_args_send_only_the_query` extended (U6, A15);
  fixture `tests/fixtures/gh/issue_node_reporter.json` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --all-targets --no-fail-fast`, against stubs
  (`reporter()` returning `""`, `reported_by` a no-op, `ISSUE_NODE_SELECTION = "<unset>"`)
  ```
  thread 'a_node_with_an_author_parses_to_that_reporter' panicked at crates/micold-core/tests/github_parse.rs:377:5:
  assertion `left == right` failed: the reporter is the author's login
    left: ""
   right: "octocat"
  thread 'every_query_holds_the_shared_node_selection' panicked at crates/micold-core/tests/github_parse.rs:431:5:
  the shared selection asks for the author's login: <unset>
  test result: FAILED. 7 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- U6 passed on first run (the arguments are meant to be unchanged). Deliberate mutant: `search_args`
  sending `q=repo:… is:issue is:open author:{text}` ->
  `search_args_send_only_the_query ... FAILED` (`left: [… "q=repo:o/r is:issue is:open author:crash on open"]`);
  restored exactly.
- green: `github.rs`: `GHOST_LOGIN`, `Issue::reporter`, `.reported_by`, `reporter()`; the node
  fields in one `issue_node_selection!` text shared by the three queries through `concat!`, with
  `author { login }`; `issue_from_node` reads `author.login`. `github_parse`: 12 passed.
- refactor: none. The search queries now write `state` after the shared selection instead of
  between `updatedAt` and `labels`; the fields asked for are the same.
- notes: A15 stays PENDING; its other half (the reducer shows the returned matches) is T025 in M3.

## Cycle 2 — U7, U8, U9, U10 (baseline), U11, U12, U13, U14, U15 — T002, T008

- tests: `crates/micold-core/tests/github_issue_lines.rs` (new, 9 tests):
  `the_title_line_is_the_number_and_the_title` (U7), `the_details_line_is_the_reporter_then_the_labels` (U8),
  `the_details_line_without_labels_is_the_reporter_alone` (U9), `the_match_text_omits_the_reporter` (U10),
  `a_span_in_the_title_maps_to_the_title_line` (U11), `a_span_in_the_labels_maps_to_the_details_line` (U12),
  `a_span_crossing_the_separator_is_split` (U13), `a_span_outside_every_part_is_dropped` (U14),
  `ranges_are_sorted_disjoint_and_on_character_boundaries` (U15)
- red: same run as cycle 1, against stubs (`title_line`/`details_line` returning `""`, `emphasis`
  returning the default)
  ```
  thread 'the_details_line_is_the_reporter_then_the_labels' panicked at crates/micold-core/tests/github_issue_lines.rs:51:5:
  assertion `left == right` failed: the reporter first, then the separator and the comma-joined labels
    left: ""
   right: "ana  ·  bug, ui"
  thread 'a_span_in_the_title_maps_to_the_title_line' panicked at crates/micold-core/tests/github_issue_lines.rs:93:5:
  assertion `left == right` failed: the title part of the match text is the title line
    left: RowEmphasis { title: [], details: [] }
   right: RowEmphasis { title: [3..6], details: [] }
  test result: FAILED. 1 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  The one pass is U10, a characterization of today's `row_text()`; it stays the baseline until T024.
- green: `github.rs`: `title_line()`, `details_line()`, `RowEmphasis`, `Issue::emphasis` over a
  list of parts (title part -> title line, labels part -> details line after the reporter and the
  separator), spans widened to character boundaries and merged per line. 9 passed.
- refactor: `PART_SEPARATOR` names the separator `Issue::new` wrote inline.

## Cycle 3 — U28 (reporter), U29 — T003, T009

- tests: `crates/micold-core/tests/github_privacy.rs` (new): `debug_output_redacts_the_reporter` (U28),
  `no_logging_call_names_issue_data` (U29), `the_scan_finds_a_logging_call_that_names_issue_data`
  (the scanner's own positive control)
- red (U28): `scripts/build-lock.sh cargo test -p micold-core --all-targets --no-fail-fast`, with the
  reporter implemented and `Debug` still printing every field
  ```
  thread 'debug_output_redacts_the_reporter' panicked at crates/micold-core/tests/github_privacy.rs:41:5:
  the reporter's login is not printed: Issue { number: 518, title: "Show the reporter", labels: ["enhancement"], updated_at: "t", reporter: "a-private-login", row_text: "#518 Show the reporter  ·  enhancement" }
  test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- U29 passed on first run (no file it scans logs anything today). Deliberate mutant: the line
  `tracing::warn!("could not show {:?}", issue);` planted in `github.rs` ->
  `no_logging_call_names_issue_data ... FAILED`
  (``github.rs: names `issue` in `warn!("could not show {:?}", issue)` ``); removed again.
- green: hand-written `impl Debug for Issue` printing `reporter: "<redacted>"` and leaving the match
  text out. Suite `cargo test -p micold-core --all-targets`: 1527 passed, 0 failed, 7 ignored.
- refactor: none.
- notes: U28's description half is T045/T049 (M5).

## Cycle 4 — U31, U32, U33, U34, U35 (baseline), A7 — T004, T010, T011

- tests: unit tests in `crates/micold-client/src/ui/material/picker.rs`:
  `the_wrapping_label_takes_more_lines_inside_its_bound` (U31),
  `a_word_wider_than_the_label_breaks_inside_the_word` (U32),
  `the_wrapping_labels_runs_concatenate_to_the_input` (U33),
  `a_row_with_details_is_at_least_a_menu_item_high_and_grows_when_it_wraps` (U34),
  `a_row_without_details_keeps_its_fixed_height_and_single_line` (U35),
  `a_picked_rows_marker_is_beside_the_first_line_whatever_the_height` (A7)
- red: `scripts/build-lock.sh cargo test -p micold-client --lib material::picker::tests`, against
  stubs (`Row::details` storing the text, `EmphasisedLabel::wrapping` setting a flag, `row_element`
  and the label's layout ignoring both)
  ```
  a_row_with_details_is_at_least_a_menu_item_high_and_grows_when_it_wraps: a row whose title wraps is 48dp high: it did not grow
  the_wrapping_label_takes_more_lines_inside_its_bound: a label of 77 characters is 18.199999dp high in 160dp: one line is 20dp, so it did not wrap
  a_word_wider_than_the_label_breaks_inside_the_word: 256 characters without a space are 18.199999dp high in 200dp: they did not break
  a_picked_rows_marker_is_beside_the_first_line_whatever_the_height: no child 0 at depth 2 of [0, 1, 0]: the row's tree changed shape
  test result: FAILED. 6 passed; 4 failed; 0 ignored; 0 measured; 472 filtered out
  ```
  U33 and U35 passed on first run. U35 is the characterization of today's row (BASELINE). U33 is
  over `segments`, the split both modes shape from, which already kept every character; the new
  case adds multi-byte text and two spans.
- green: `EmphasisedLabel::wrapping()`: one paragraph through `Paragraph::with_spans`, bounds
  `(available, INFINITY)`, `Wrapping::WordOrGlyph`, the role's line height, height from
  `min_bounds()`; the accent is part of the cache key because the spans carry their colour.
  `row_element` branches on `Row::details`: a column of the two labels (`Body`/`on_surface`,
  `Caption`/`on_surface_variant`) beside a marker box one first line high, `spacing::XS` vertical
  padding, height `Shrink`, and a strut of `MENU_ITEM_BASE` less the padding for the minimum.
  `cargo test -p micold-client --lib material::`: 364 passed, 0 failed.
- refactor: none.
- notes: a first green attempt gave the strut `Length::Fixed(0.0)` width; iced's `Row::push` drops
  a child whose size hint is void, so the row came out 44dp. The strut's width is `Shrink` now.
  **Deviation from T014**: `material/menu_anatomy.rs` is edited. Its two
  `TypeaheadRow { label, spans, enabled }` literals stop compiling when `Row` gains a field; each
  gained `..Default::default()`. No assertion changed and both tests pass.

## Cycle 5 — U36, A1, A2, A3, A8 — T005, T012

- tests: `crates/micold-client/tests/issue_picker_rows.rs` (new):
  `a_listed_issues_row_carries_the_title_line_and_the_details_line`,
  `listed_searched_and_typed_number_issues_get_the_same_two_lines`,
  `the_picked_issues_row_is_the_selected_one`
- red: `scripts/build-lock.sh cargo test -p micold-client --test issue_picker_rows`, with
  `issue_rows` extracted from `issue_picker` and still building today's single-line rows
  ```
  a_listed_issues_row_carries_the_title_line_and_the_details_line: assertion `left == right` failed
    left: "#42 Crash when opening empty project  ·  bug, ui"
   right: "#42 Crash when opening empty project"
  listed_searched_and_typed_number_issues_get_the_same_two_lines: assertion `left == right` failed: #1100
    left: "#1100 Titles are cut off  ·  ui, 1100-series"
   right: "#1100 Titles are cut off"
  test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  The one pass is the picked row's position, which the extraction kept.
- green: `issue_rows` builds `TypeaheadRow::new(issue.title_line(), e.title).details(issue.details_line(), e.details)`
  with `e = issue.emphasis(&matched.spans)`. 3 passed.
- refactor: none. `issue_rows` is exported as `micold_client::ui::issue_rows` for the test; the
  `worktree_form` view module stays `pub(crate)`.

## Cycle 6 — U44, A1, A2, A4, A5, A9 — T006, T014

- tests: `crates/micold-client/tests/gates/issue_rows_show_all_text.rs` (new, four tests, a
  `#[path]` module of `tests/layout_snapshot.rs`) over three new covered states with the list open:
  `add-worktree-dialog-issue-list-longest-titles` (two 256-character titles, one without a space),
  `add-worktree-dialog-issue-list-labels` (20 labels, one label, none) and
  `add-worktree-dialog-issue-list-mixed-heights` (titles of 1, 2, 3 and 5 lines). Each state is read
  at `WINDOW` and at `NARROW` = 440x800 through `support::layout::painted_text_settled_at`.
- red: the gate was written after T010–T012, so it passed on first run. Deliberate mutant:
  `Wrapping::WordOrGlyph` -> `Wrapping::None` in `material/picker.rs`; 3 of the 4 tests failed
  ```
  6 finding(s): an issue row has to show its whole title, its reporter and every label, inside the row and inside the list (038 SC-001, FR-004). ...
  add-worktree-dialog-issue-list-longest-titles at 1280x800: row 0 (#2101) paints text 1479.0px wide in a row 464.0px wide, starting at 438.0, 340.0: "#2101 A long issue title is cut off ..."
  add-worktree-dialog-issue-list-longest-titles at 1280x800: row 1 (#2102) paints text 1586.1px wide in a row 464.0px wide, starting at 438.0, 388.0: "#2102 crates/micold-client/src/ui/material/picker.rs::..."
  ```
  restored exactly.
- green: `layout_snapshot` 47 passed; `gates/containment.rs` passes over the new states unedited.
  Fixture regenerated: 837 lines added in one hunk, 0 removed; no record of the branch picker,
  `Select` or 034's `add-worktree-dialog-issue-*` states changed (FR-027, FR-029).
- notes: one state cannot hold rows of one to five lines: five such rows are 424dp, over the
  384dp viewport, so the four-line title is in `longest-titles`. `Overflow` gained `layer`, so the
  gate can tell the floated rows' text from the dialog's under it.

## Cycle 6, second red — U44 — T006 (review A, round 1, F2)

- finding: the gate read each paragraph's source text and its width, so a row that stopped growing,
  or a line cut vertically, passed. `support::layout::Overflow` gained `natural_height` and `clip`;
  the gate now fails for a paragraph that ends below its row or outside its clip (commit 86ea073c).
- red, by mutant: the two-line row's button at `height(Length::Fixed(48.0))` instead of `Shrink` in
  `material/picker.rs`; 3 of the 4 tests failed, 10 findings of the new kind
  ```
  ... paints text down to 418.0, below the row's end at 382.0: the row did not grow for every line of ...
  ```
- green: mutant reverted, `cargo test -p micold-client --test layout_snapshot issue_rows`: 4 passed.

## T013 — the showcase pose (no cycle)

- `Showcase::new` starts with sample row 0 chosen and row 1 highlighted (ledger D9), and
  `typeahead_rows` hands each sample row its second line. `tests/showcase_state.rs` asserts the
  seeded pose (`the_typeahead_highlight_moves_and_stops_at_the_ends`); the assertion and the seed
  went in together in e8428c23, so no red was recorded. That the rows are two lines of differing
  height is held by quickstart §B2 only (`evidence/b2-showcase-*.png`); review B, F2, names the
  missing assertion and it is listed under the ledger's follow-ups.


## Cycle 7 — U39 — T018 (M2)

- test: `crates/micold-client/tests/picker_highlight_into_view.rs`
  `each_press_moves_the_highlight_by_one_issue_and_enter_picks_it`. It drives the real add-worktree
  form (`support::layout::view_of`): keys go to the floated list and then the window with one
  shell, and what is published is applied to the real reducer.
- red: the behaviour exists since 034, so it passed on first run. Deliberate mutant in
  `features/worktree_form.rs` `issue_highlight_moved`: `Some(next)` ->
  `Some((next + 1).min(len - 1))`;
  `scripts/build-lock.sh cargo test -p micold-client --test picker_highlight_into_view`
  ```
  assertion `left == right` failed: after Down 0
    left: Some(1)
   right: Some(0)
  ```
  restored with `git checkout`.
- green: 1 passed. No refactor. Commit 069165fa (with cycle 8: the two were not committed apart).

## Cycle 8 — U40 — T018, T019, T020 (M2)

- test: same file, `the_highlighted_row_is_in_view_after_every_down_and_up`: 12 issues with titles
  of 1 to 5 lines; after each of 11 Down and 11 Up presses the operation is run to the end of its
  chain over the base tree and the overlay, and the row with `PICKER_HIGHLIGHT` must lie inside the
  viewport of the scrollable it is in. Asserts the list is 384 high and that it scrolled at all.
- red, against stubs (`PICKER_HIGHLIGHT` declared and unused, a no-op operation):
  `... --test picker_highlight_into_view the_highlighted_row_is_in_view_after_every_down_and_up -- --exact`
  ```
  assertion `left == right` failed: exactly one row carries the highlight's Id
    left: 0
   right: 1
  ```
- green: `menu_element` wraps the highlighted row in a container with the `Id` (T019);
  `ui/picker_scroll.rs` finds it and scrolls the innermost scrollable around it by
  `focus::delta_into_view`, now `pub(super)` (T020). File: 2 passed. Commit 069165fa.
- notes: pass two moves only the panel the row was found in, by its ordinal in the traversal. The
  focus operation's rule (every panel whose content overlaps the control) would also move a
  scrolling form under the floated list, which shares window coordinates with it.
  The full suite was not run per cycle: the build lock is shared with other worktrees and each run
  queued for many minutes. Per cycle: this file, `focus_scroll` and the `focus` unit tests; the
  full suite is T023's gate.

## Cycle 9 — U41 — T018, T020 (M2)

- test: `a_row_already_wholly_visible_causes_no_scroll`: eight one-line issues fill the list
  exactly, so the eighth row is flush with the viewport's bottom; a ninth lets the list scroll.
- red (the operation still passed `focus::MARGIN`):
  ```
  assertion `left == right` failed: row 7 spans 670..718 of a list showing (350.0, 734.0), and the list moved
    left: 16.0
   right: 0.0
  ```
- green: `delta_into_view` takes `margin`; the focus caller and its unit tests pass `MARGIN`
  unchanged, the picker passes 0. `picker_highlight_into_view` 3 passed, `focus_scroll` 2 passed.
  Commit f824aa41.

## Cycle 10 — U42 — T018 (M2)

- test: `a_row_taller_than_the_list_is_aligned_to_its_top` (a 40-line title among one-line rows;
  reached from above and from below).
- red: passed on first run, since margin 0 and `delta_into_view`'s too-tall branch already give
  it. Deliberate mutant in `ui/focus.rs`: the condition
  `wanted_top < top || wanted_bottom - wanted_top > viewport_height` -> `wanted_top < top`
  ```
  the row starts at 430 and the list shows from 630
  ```
  restored with `git checkout`.
- green: 1 passed unmutated. No refactor.

## Cycle 11 — U43 — T018, T021 (M2)

- test: `only_the_issue_highlight_move_chains_the_operation` (source check over `src/`).
- red, before T021:
  ```
  assertion `left == right` failed: the operation has exactly one caller, the tail of the issue shell's handler
    left: []
   right: [("src/shell/issues.rs", "micold_client::ui::picker_highlight_into_view()")]
  ```
- green attempt: `main.rs` routes `FormMsg::IssueHighlightMoved` to the new
  `shell::issues::on_issue_highlight_moved`, which applies the move and returns the task. The test
  still failed, on its own second assertion: it looked for
  `FormMsg::IssueHighlightMoved(direction)` in the handler's text and rustfmt had wrapped that
  call over three lines. The assertion now compares without whitespace (a defect of the test, not
  a loosened check). Run again, it still failed: rustfmt's wrap also adds a trailing comma,
  `IssueHighlightMoved(direction,)` (review A, F1). The check now folds `,)` to `)` as well.
- green, in `mise run gate` (GATE_EXIT=0):
  ```
  test only_the_issue_highlight_move_chains_the_operation ... ok
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
  ```
- refactor: none.

## Cycle 12 — U16, U17, U18, U19, U13 (M3 part), U20 (M3 part), U21 — T024, T027, T028 (M3)

- tests: `crates/micold-core/tests/github_issue_lines.rs::{the_match_text_holds_the_reporter_between_title_and_labels
  (U16, U20; replaces U10's `the_match_text_omits_the_reporter`, as T024 says),
  a_span_in_the_reporter_maps_to_the_start_of_the_details_line (U17),
  rank_matches_part_of_a_reporter_login_in_another_letter_case (U18),
  one_match_emphasises_the_title_and_the_reporter (U19)}` (new);
  `a_span_crossing_the_separator_is_split` (U13) now holds the contract's M3 spans `7..16` and
  `10..13`; `a_span_outside_every_part_is_dropped` (U14): an unlabelled issue's whole span now
  emphasises its reporter (contract §4, M3 match text). `typeahead_budget.rs`: the corpus is built
  from `Issue`s with reporters; `the_issue_corpus_carries_reporters` (U21, new).
- red: `scripts/build-lock.sh bash -c 'cargo test -p micold-core --test github_issue_lines --test typeahead_budget; …'`
  ```
  thread 'the_match_text_holds_the_reporter_between_title_and_labels' panicked at crates/micold-core/tests/github_issue_lines.rs:82:5:
    left: "#7 Fix it  ·  bug, ui"
   right: "#7 Fix it  ·  ana  ·  bug, ui"
  thread 'a_span_crossing_the_separator_is_split' panicked at crates/micold-core/tests/github_issue_lines.rs:214:5:
    left: RowEmphasis { title: [7..9], details: [9..10] }
   right: RowEmphasis { title: [7..9], details: [0..1] }
  thread 'rank_matches_part_of_a_reporter_login_in_another_letter_case' panicked at crates/micold-core/tests/github_issue_lines.rs:126:5:
    left: []
   right: [1]
  thread 'one_match_emphasises_the_title_and_the_reporter' panicked …:149:5:
    left: RowEmphasis { title: [3..6], details: [] }
   right: RowEmphasis { title: [3..6], details: [0..3] }
  test result: FAILED. 6 passed; 6 failed; 0 ignored
  ```
  and, run alone because cargo stopped at the first failing target:
  ```
  thread 'the_issue_corpus_carries_reporters' panicked at crates/micold-core/tests/typeahead_budget.rs:338:9:
  row 0 "#12000 Crash when opening an empty project (0)  ·  bug" holds its reporter "octocat"
  ```
  U17's red is `range_of` finding no `ana` in the match text (`github_issue_lines.rs:38:28`).
- green: `Issue::match_text()` (title line, separator, details line) is the row text, rebuilt by
  `reported_by`; `parts()` maps the reporter to `0..len` of the details line and the labels after
  the second separator. Existing expectations of the M1 match text updated to the M3 one, as the
  contract's §4 table says: `github_parse.rs::row_text_shows_labels_only_when_present` and
  `main_tests.rs::issue_choosing_the_source_lists_open_issues` (the fixtures carry no author, so
  `ghost`). Release budget: `ranking_1000_issue_rows_for_a_short_query_fits_the_budget ... ok`.
- refactor: none beyond `match_text()`, which replaces the hand-built row text in `Issue::new`.
- notes: one grouped cycle, as M1's cycle 1: one change (the reporter in the match text) turns
  every one of these tests green.
- deviation (U19): the red above was for `ana` on "Ana's crash" by `ana`, expecting both the title's
  `Ana` and the login emphasised. With the reporter in the match text it stayed red
  (`details: []`): the literal tier marks only the leftmost occurrence, and the contract forbids a
  change to `typeahead`. The test now types `fixana` on issue #7, one subsequence match whose
  characters fall in `Fix` and in `ana` (D11); it passes with `title: [3..6], details: [0..3]`.
  Cycle 13's U37 "both" case was written the same way (`blurryocto`).

## Cycle 13 — U45, U46, U47, U37, U38 (A10–A15) — T025, T026, T029 (M3)

- tests: `crates/micold-client/tests/issue_source_state.rs::{typing_a_login_narrows_to_the_reporters_issues (U45),
  a_searched_issue_matching_only_by_its_reporter_is_kept (U46), typing_a_login_runs_the_one_search_for_the_text (U47)}`;
  `issue_picker_rows.rs::{the_issue_search_hint_names_the_reporter (U38), a_row_matched_by_its_reporter_emphasises_the_login (U37)}`.
  Stub for U38: `ui::ISSUE_SEARCH_PLACEHOLDER` holding the old hint, used by the view.
- red: `cargo test -p micold-client --test issue_source_state --test issue_picker_rows` (under the build lock)
  ```
  thread 'the_issue_search_hint_names_the_reporter' panicked at crates/micold-client/tests/issue_picker_rows.rs:164:5:
    left: "Search by number, title or label"
   right: "Search by number, title, label or reporter"
  thread 'a_row_matched_by_its_reporter_emphasises_the_login' panicked at crates/micold-client/tests/issue_picker_rows.rs:192:5:
    left: [9]
   right: [9, 42]
  thread 'a_searched_issue_matching_only_by_its_reporter_is_kept' panicked at crates/micold-client/tests/issue_source_state.rs:1232:5:
    left: []
   right: [1300]
  thread 'typing_a_login_narrows_to_the_reporters_issues' panicked at crates/micold-client/tests/issue_source_state.rs:1204:5:
    left: [55, 108]
   right: [7, 108]
  ```
- deviation: U45's red was for the query `OctoC`, which the fuzzy tier also matches in two titles;
  after the red the query became the whole login `OctoCat` (exact `[7, 108]`) and `octo` asserts
  inclusion, since `octo` fuzzily reaches `hubot` across the row. The red line still shows octocat's
  #7 missing for want of the reporter in the match text.
- U47 passed before the change: FR-013 asks that nothing change in the request, and the request
  half was driven red in cycle 1 (T001). U48 is the 034 suite passing unedited.
- green: core's match text (cycle 12) and the placeholder's new text (T029).
- refactor: none.

## Cycle 14 — U49–U58 — T032, T036 (M4)

- tests: `crates/micold-core/tests/tooltip_rest.rs` (new, 11 tests): `a_cursor_still_for_the_delay_opens_it`
  (U49), `a_cursor_still_for_a_millisecond_less_leaves_it_closed` (U50),
  `a_move_beyond_the_tolerance_restarts_the_wait` (U51),
  `a_move_of_exactly_the_tolerance_is_at_rest_and_the_anchor_does_not_drift` (U52),
  `a_cursor_that_keeps_moving_never_opens_it` (U53), `once_open_movement_over_the_trigger_keeps_it_open`
  (U54), `leaving_closes_it_and_the_next_entry_waits_the_full_delay` (U55),
  `a_press_closes_it_until_the_cursor_has_left` and `a_press_while_waiting_also_spends_it` (U56),
  `a_reset_closes_it_and_waits_the_full_delay_again` (U57), `it_asks_to_be_woken_only_while_waiting` (U58).
  Stub: `micold_core::tooltip` with the contract's types, `observe` answering closed and no wake.
- red: `scripts/build-lock.sh cargo test -p micold-core --no-fail-fast --test tooltip_rest --test tooltip_clamp`
  ```
  thread 'a_cursor_still_for_the_delay_opens_it' panicked at crates/micold-core/tests/tooltip_rest.rs:38:5:
  still for the whole delay: Away
  test result: FAILED. 2 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- passed against the stub: U53 and `a_press_while_waiting_also_spends_it` (a stub that never opens
  cannot fail "never opens"). Deliberate mutants after green: the tolerance test replaced by
  `> f32::MAX` failed U53, U51 and U52; `press()` made a no-op failed both press tests. Restored.
- deviation: U52's first form stepped to `(+2.4, +3.2)`, which `f32` holds as a distance a hair over
  4.0; it failed after green with `anchor: (102.4, 53.2)`. The test was wrong, not the rule: it now
  steps 4.0 along each axis, figures `f32` holds exactly.
- green: `RestTimer::observe`, `press`, `reset` per data-model §5. `cargo test -p micold-core
  --all-targets`: 1699 passed, 0 failed; clippy `-D warnings` clean.
- refactor: none.

## Cycle 15 — U59–U62 — T033, T037 (M4)

- tests: `crates/micold-core/tests/tooltip_clamp.rs` (new, 5 tests):
  `a_text_that_fits_is_returned_borrowed_and_unchanged` (U59),
  `overflowing_words_are_cut_after_a_whole_word_and_end_in_one_ellipsis` and
  `a_text_already_ending_in_an_ellipsis_ends_in_one` (U60),
  `a_text_without_spaces_is_cut_at_a_character_and_ends_in_an_ellipsis` (U61),
  `no_length_yields_more_than_the_limit` (U62). Stub: `clamp_to_lines` returning its text borrowed.
- red: the same run as cycle 14
  ```
  thread 'a_text_already_ending_in_an_ellipsis_ends_in_one' panicked at crates/micold-core/tests/tooltip_clamp.rs:84:5:
  test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- passed against the stub: U59, which is the stub's own behaviour (a text that fits comes back
  borrowed); the four cut cases hold the other branch.
- green: `clamp_to_lines` per data-model §6 (binary search over character boundaries, back-up to a
  space within 24 characters, one `…`). 5 passed.
- refactor: none.

## Cycle 16 — U63, U64 — T034, T038, T039 (M4)

- tests: `crates/micold-client/tests/idle_requests_no_frames.rs`:
  `a_waiting_rest_tooltip_asks_for_one_timed_wake_and_none_once_open`,
  `a_rest_tooltip_that_is_away_or_spent_asks_for_no_timed_wake`,
  `a_tooltip_without_a_rest_delay_asks_for_no_timed_wake` (U64),
  `the_frame_requests_are_the_guarded_one_and_the_timed_one` (replacing the one-door test) and
  `wake_at_has_one_caller` (U63); harness `tests/support/tooltip.rs` (`Driven`).
  Stub: `cdk::Tooltip::after_rest` and `subject` that do nothing.
- red: `scripts/build-lock.sh cargo test -p micold-client --no-fail-fast --test tooltip_rest_glue --test idle_requests_no_frames --test material_builder_api`
  ```
  thread 'a_waiting_rest_tooltip_asks_for_one_timed_wake_and_none_once_open' panicked at crates/micold-client/tests/idle_requests_no_frames.rs:153:5:
  assertion `left == right` failed: the cursor came to rest: one wake, when the delay has run
  expected exactly one next-frame request and one timed request in the motion primitive, found 1 and 0
  `ui/cdk/tooltip.rs` is listed as the caller of `wake_at` and calls it 0 times: one call, on the one path that waits
  test result: FAILED. 8 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- passed against the stub: `a_tooltip_without_a_rest_delay_asks_for_no_timed_wake` (the baseline it
  guards).
- deviation: after `wake_at` was added, `the_scan_actually_finds_the_rendering_layer` failed
  ("expected exactly one frame request across the whole rendering layer, found 2"): its count is now
  two, the guarded request and the timed one.
- green: `motion::wake_at` (T038) and the rest mode in `cdk/tooltip.rs` (T039): 12 passed, 0 failed.
- refactor: none.

## Cycle 17 — U65, U66, U67, U69, U70, U78, U79, U80 — T035, T039, T040 (M4)

- tests: `crates/micold-client/tests/tooltip_rest_glue.rs` (new, 11 tests):
  `the_panel_shows_only_after_the_delay_at_rest` and `the_panel_does_not_show_while_the_cursor_moves`
  (U65), `a_cursor_move_onto_the_trigger_starts_the_wait` (U65 setup),
  `a_press_closes_the_panel_and_still_reaches_the_trigger` (U66),
  `a_changed_subject_closes_the_panel_and_starts_the_wait_again` and
  `an_unchanged_subject_keeps_the_panel_open_across_a_rebuild` (U67),
  `a_tooltip_without_a_rest_delay_opens_at_once` (U69),
  `a_trigger_that_moves_from_under_a_still_cursor_closes_the_panel` and
  `another_subject_arriving_under_a_still_cursor_waits_the_full_delay` (U78),
  `with_no_cursor_no_panel_opens_and_the_trigger_still_takes_keys` (U79),
  `two_trees_from_the_same_view_keep_separate_rest_state` (U80); and
  `the_tooltips_rest_mode_is_three_chainable_steps` in `tests/material_builder_api.rs` (U70).
- red: the same command as cycle 16
  ```
  thread 'the_panel_shows_only_after_the_delay_at_rest' panicked at crates/micold-client/tests/tooltip_rest_glue.rs:51:5:
  test result: FAILED. 4 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
  thread 'the_tooltips_rest_mode_is_three_chainable_steps' panicked at crates/micold-client/tests/material_builder_api.rs:138:13:
  test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- passed against the stub: `an_unchanged_subject_keeps_the_panel_open_across_a_rebuild`,
  `a_tooltip_without_a_rest_delay_opens_at_once` (U69, a characterization baseline),
  `a_trigger_that_moves_from_under_a_still_cursor_closes_the_panel` (a hover tooltip also closes when
  the trigger moves away) and `with_no_cursor_no_panel_opens_and_the_trigger_still_takes_keys`.
- green: `Tooltip` holds `rest` and `subject`; `State` holds a `RestTimer` and the subject; a changed
  subject resets in `diff` and at the top of `update`; `material::Tooltip::after_rest`, `max_lines`,
  `subject`. `tooltip_rest_glue` 11 passed, `material_builder_api` 11 passed.
- refactor: none.

## Cycle 18 — U68 — T040 (M4)

- tests: in-crate, `crates/micold-client/src/ui/material/line_clamp.rs` `mod tests` (5 tests):
  `a_long_text_is_cut_to_three_lines_and_ends_in_an_ellipsis`, `a_text_that_fits_is_shown_whole`,
  `a_text_without_spaces_is_cut_as_well`, `the_opened_panel_is_at_most_three_lines_and_its_padding_tall`,
  `the_line_limit_leaves_a_short_tooltip_as_it_was`. In-crate per decision D12 (`ui::material` is
  `pub(crate)`). Stub: a `LineClamped` that shapes the whole text without cutting.
- red: `scripts/build-lock.sh cargo test -p micold-client --lib line_clamp`
  ```
  thread 'ui::material::line_clamp::tests::a_long_text_is_cut_to_three_lines_and_ends_in_an_ellipsis' panicked at crates/micold-client/src/ui/material/line_clamp.rs:271:9:
  cut, so marked: "The list cuts long titles off and gives no way to tell two issues apart. …"
  test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 508 filtered out
  ```
- deviation: the panel test's first form measured the overlay group's node (as wide and tall as the
  window) instead of the panel in it, so it compared 0.0 with 32.0 for the wrong reason; it now reads
  the group's first child.
- green: `clamp_to_lines` with the paragraph's measured line count. 5 passed.
- refactor: none.

## Cycle 19 — U6, U20, U22–U28, U30, U81 — T044, T045, T049 (M5)

- tests: `crates/micold-core/tests/github_description.rs` (new, 13 tests) and three added to
  `crates/micold-core/tests/github_privacy.rs` (`debug_output_redacts_the_description` among them,
  U28); fixtures `issue_node_description.json` and `issue_node_comment_only.json`. Stubs:
  `description_from`, `DESCRIPTION_MAX_CHARS`, `Issue::described` and `Issue::description` with the
  contract's signatures, doing nothing.
- red: `scripts/build-lock.sh bash -c 'cargo test -p micold-core --test github_description --test github_privacy'`
  ```
  thread 'a_long_text_is_cut_to_the_limit_and_marked' panicked at crates/micold-core/tests/github_description.rs:88:5:
  a cut text ends in the mark
  test result: FAILED. 3 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out
  thread 'debug_output_redacts_the_description' panicked at crates/micold-core/tests/github_privacy.rs:58:5:
    left: ""
  test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- passed against the stub: U30 (`a_malformed_page_with_a_body_gives_an_error_without_it`) and U81
  (`an_issue_has_no_serialize`), which guard what the stub does not yet change. Mutant check after
  green: with the page error formatting the node and `serde::Serialize` derived on `Issue`, they
  fail at `github_privacy.rs:125:17` and `:159:5`. Restored.
- deviation: the two fixtures are written from contracts/issue-fields.md §2, not captured from
  GitHub as T044's word "captured" says. T055's §B run confirms GitHub's `bodyText` for scenario
  12's body.
- green: `description_from`, `DESCRIPTION_MAX_CHARS`, `Issue::described`, `Issue::description`,
  `bodyText` in the shared node selection, `Debug` printing the description as `<redacted>`.
  `cargo test -p micold-core --all-targets --no-fail-fast`: exit 0, 148 binaries ok. Commits
  ddb85c37 (red), a34b07bc (green).
- refactor: none.

## Cycle 20 — U71, U72, U73, U74, U82 (loading half), U83, U86 — T046, T051, T052 (M5)

- tests: `crates/micold-client/tests/issue_picker_rows.rs` (3 new):
  `a_described_issues_row_carries_the_description_and_the_number` (U71),
  `an_issue_without_a_description_gets_no_tooltip` (U72),
  `a_list_that_is_loading_again_builds_no_row` (U82, loading half); and nine in-crate in
  `crates/micold-client/src/ui/material/picker.rs` `mod tests`:
  `a_row_holds_exactly_the_tooltip_text_it_was_given` (U73),
  `a_rows_tooltip_waits_three_seconds_and_shows_three_lines` and
  `a_row_with_a_tooltip_opens_its_panel_after_the_rest_delay` (U74),
  `a_row_without_a_tooltip_floats_nothing`, `a_rows_panel_is_at_most_three_lines_tall`,
  `another_key_at_the_same_place_closes_the_panel_and_waits_again`,
  `the_same_key_keeps_its_panel_across_a_rebuild`,
  `a_highlighted_row_without_the_cursor_floats_nothing` (U83) and
  `debug_output_of_a_row_redacts_its_details_and_its_tooltip` (U86, new; FR-025; it also closes M1
  review A F3). Stubs: `Row::tooltip` and `Row::key` that hold nothing, `menu_element` not wrapping.
- red: `scripts/build-lock.sh bash -c 'cargo fmt --all; cargo test -p micold-client --test issue_picker_rows; cargo test -p micold-client --lib picker'`
  ```
  thread 'a_described_issues_row_carries_the_description_and_the_number' panicked at crates/micold-client/tests/issue_picker_rows.rs:256:9:
  thread 'an_issue_without_a_description_gets_no_tooltip' panicked at crates/micold-client/tests/issue_picker_rows.rs:309:9:
  test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
  ui::material::picker::tests::a_row_holds_exactly_the_tooltip_text_it_was_given panicked at crates/micold-client/src/ui/material/picker.rs:1095:9:
    left: None
   right: Some("  Two  spaces, kept. ")
  test result: FAILED. 24 passed; 7 failed; 0 ignored; 0 measured; 491 filtered out
  ```
- passed against the stubs: `a_list_that_is_loading_again_builds_no_row`,
  `a_rows_tooltip_waits_three_seconds_and_shows_three_lines` and `a_row_without_a_tooltip_floats_nothing`
  (guards for what the stubs leave as it was).
- deviation: `a_rows_panel_is_at_most_three_lines_tall` first measured the panel's outer node, which
  includes the 5 px margin `cdk::tooltip` keeps from the window's edge on each side (66 against 56);
  `panels_of` now reads the panel's first child, the visible surface, as the gate does.
- deviation: `tests/one_overlay_implementation.rs` read the builder method `Row::tooltip(` and its call
  `.tooltip(` as the rendering stack's `tooltip(` widget (`every_widget_attached_overlay_is_on_the_list`
  failed at `one_overlay_implementation.rs:191:5`: "ui/material/picker.rs constructs `tooltip`",
  "ui/worktree_form.rs constructs `tooltip`"). Its `calls` now excludes a method call and a `fn`
  definition, with four assertions added to `a_helper_ending_in_the_widget_name_is_not_a_use_of_it`,
  two of them that a free call (`iced::widget::tooltip(`, and one after a method on the same line) is
  still found.
- green: `Row::tooltip` (None when empty), `Row::key`, a hand-written `Debug` for `Row`; `menu_element`
  wraps a row with a text in `material::Tooltip` (`after_rest(ROW_TOOLTIP_REST)`,
  `max_lines(ROW_TOOLTIP_LINES)`, `Bottom`, `subject(key)`), the tooltip outermost so its state stays
  at the row's place when the highlight moves; `issue_rows` passes `.key(issue.number())` and
  `.tooltip(issue.description())`. `issue_picker_rows` 8 passed, `--lib picker` 31 passed,
  `picker_highlight_into_view` 5 passed. Commit b9ef217d.
- refactor: none.

## Cycle 21 — U75, U76, U84, U85, U82 (source half), U77 — T047, T048, T050, T053 (M5)

- tests: `crates/micold-client/tests/gates/picker_row_tooltip_clears_its_row.rs` (new, 9 tests),
  registered in `tests/layout_snapshot.rs`:
  `the_first_row_shows_one_panel_clear_of_itself`,
  `a_row_at_the_lists_lower_edge_shows_one_panel_clear_of_itself`,
  `the_last_row_of_the_scrolled_list_shows_one_panel_clear_of_itself` (U75),
  `a_click_on_the_row_under_an_open_panel_picks_its_issue` (U76),
  `moving_onto_the_adjacent_row_by_less_than_the_tolerance_closes_the_panel` and
  `the_adjacent_row_opens_its_panel_only_after_its_own_full_delay` (U84),
  `the_search_field_keeps_keyboard_focus_while_a_panel_is_open` and
  `up_down_and_enter_act_under_an_open_panel_as_without_one` (U85),
  `a_form_on_another_source_builds_no_row_and_floats_no_panel` (U82, source half); and U77,
  `no_view_code_fetches_issues` in `tests/issues_are_requested_only_on_named_events.rs`.
  Stub: no `Menu::overlay`.
- red: `scripts/build-lock.sh bash -c 'cargo fmt --all; cargo test -p micold-client --test layout_snapshot picker_row_tooltip'`
  ```
  thread 'picker_row_tooltip_clears_its_row::the_first_row_shows_one_panel_clear_of_itself' panicked at crates/micold-client/tests/gates/picker_row_tooltip_clears_its_row.rs:563:9:
  exactly one tooltip panel has to be open above the list, and 0 are
  test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 48 filtered out
  ```
- passed against the stub: U77 (`no_view_code_fetches_issues`), a characterization that passed at once
  (3 passed). Mutant check: a line `const _MUTANT: () = start_issue_load(app);` under `#[cfg(any())]`
  in `ui/confirm_link_open.rs` made it panic at `issues_are_requested_only_on_named_events.rs:178:5`
  (and the named-events test at `:137:5`). Restored.
- deviation: U82's source half was written after the green, so it had no red against the stub; it
  passes at once (9 passed). Mutant check: with the source switch commented out it fails at
  `picker_row_tooltip_clears_its_row.rs:944:5` (the panel assertion). Restored.
- green: `Overlay::overlay` for `cdk::picker`'s `Menu` forwards to its content's `Widget::overlay` with
  the list's bounds as viewport, none while leaving (research R10). `layout_snapshot` 57 passed,
  `one_overlay_implementation` 8 passed, `overlay_stacking` 5 passed. T053: the fixture
  `tests/fixtures/layout_snapshot.txt` did not change (the tooltip adds no layout node), so nothing
  was regenerated.
- refactor: none.

## Cycle 22 — U87 — T044, T049 (M5, review A round 1 F3)

- tests: `crates/micold-core/tests/github_description.rs` (2 new):
  `a_text_of_invisible_characters_gives_no_description` and
  `invisible_characters_are_dropped_from_a_description`.
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_description`
  ```
  thread 'a_text_of_invisible_characters_gives_no_description' panicked at crates/micold-core/tests/github_description.rs:374:9:
  thread 'invisible_characters_are_dropped_from_a_description' panicked at crates/micold-core/tests/github_description.rs:387:5:
  test result: FAILED. 13 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `is_invisible` in `crates/micold-core/src/github.rs` (control characters and the named
  format ranges); `description_from` skips a word with no visible character and drops the marks
  inside one before counting. `github_description` 15 passed, `github_privacy` 6 passed. Commit
  e841c7dd.
- refactor: none.

## Cycle 23 — U88 — T047, T050 (M5, review A round 1 F1)

- tests: `a_reopened_list_waits_the_whole_delay_again` in
  `crates/micold-client/tests/gates/picker_row_tooltip_clears_its_row.rs`.
- red: `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot picker_row_tooltip`
  ```
  thread 'picker_row_tooltip_clears_its_row::a_reopened_list_waits_the_whole_delay_again' panicked at crates/micold-client/tests/gates/picker_row_tooltip_clears_its_row.rs:1046:5:
  assertion `left == right` failed: the list was just opened again under a still cursor: its row has not rested for the delay yet, so no panel (038 FR-015, FR-017)
    left: 1
   right: 0
  test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 48 filtered out
  ```
- green: `Menu::update` in `crates/micold-client/src/ui/cdk/picker.rs` hands a leaving list
  `mouse::Cursor::Unavailable`, so every row sees the cursor leave on the first leaving frame and
  its rest state is `Away` when the list comes back. 10 passed. Commit e841c7dd.
- deviation: two more source scans read this milestone's names as violations, as
  `one_overlay_implementation.rs` did in cycle 20. `tests/material_boundary.rs` counted the builder
  call `.tooltip(` as the rendering stack's `tooltip(` widget (`the_boundary_is_closed` at
  `material_boundary.rs:362:5`, `no_feature_module_builds_a_styled_widget` at `:284:5`); its
  `names_widget` now excludes a method call, held by the new self-test
  `a_method_with_a_widgets_name_is_not_a_widget_call` (a free call is still counted, also after a
  method call on the same line). `tests/motion_tokens.rs` (`every_duration_is_a_named_token` at
  `motion_tokens.rs:158:5`) flagged the in-crate test constant `MS = Duration::from_millis(1)` in
  `material/picker.rs`; it is now `ROW_TOOLTIP_REST.checked_div(1000)`, a part of the delay.
- refactor: none.

## Cycle 24 — U89 — T044, T049 (M5, review A round 2 F1)

- tests: `crates/micold-core/tests/github_description.rs` (2 new):
  `joiners_between_visible_characters_are_kept` and `a_cut_leaves_no_joiner_before_the_mark`.
  Cycle 22's `is_invisible` dropped U+200C and U+200D everywhere, which changes what is read: a
  Persian word lost its required non-joiner and an emoji sequence fell apart.
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_description`
  ```
  thread 'a_cut_leaves_no_joiner_before_the_mark' panicked at crates/micold-core/tests/github_description.rs:415:5:
  thread 'joiners_between_visible_characters_are_kept' panicked at crates/micold-core/tests/github_description.rs:402:5:
    left: "میخواهم"
   right: "می\u{200c}خواهم"
  test result: FAILED. 15 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `is_joiner` in `crates/micold-core/src/github.rs`; `is_invisible` no longer names the two
  joiners; `description_from` holds a word's joiners back until a visible character follows one,
  drops them otherwise, and `cut` trims a joiner as it trims a space. `github_description` 17
  passed, `github_privacy` 6 passed. Cycle 22's cases still hold (a body of only invisible
  characters, joiners among them, gives the empty string).
- refactor: the two cut sites share the inner `cut`.

## Cycle 25 — U27, U90, U91, U92, U93, U94 — T058, T061 (M5, second pass, ledger D13)

- tests: `crates/micold-core/tests/github_description_pass.rs` (NEW, 10 tests) and
  `github_description.rs` (`only_the_searches_ask_for_the_body_text` replaces
  `every_query_asks_for_the_body_text_in_the_shared_selection`). The pass's API was first added as
  stubs (an empty query, no arguments, an empty page, a `Debug` that printed the descriptions, a
  cap of 1), so the tests fail on what they assert and not on a missing name.
- red: `scripts/build-lock.sh cargo test --no-fail-fast -p micold-core --test github_description_pass --test github_description`
  ```
  thread 'only_the_searches_ask_for_the_body_text' panicked at crates/micold-core/tests/github_description.rs:246:5:
  test result: FAILED. 16 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
  thread 'a_descriptions_error_is_classified_like_the_lists' panicked at crates/micold-core/tests/github_description_pass.rs:197:9:
  thread 'a_descriptions_page_parses_to_numbers_and_descriptions' panicked at crates/micold-core/tests/github_description_pass.rs:149:5:
  thread 'neither_an_error_nor_a_page_shows_a_body' panicked at crates/micold-core/tests/github_description_pass.rs:220:67:
  thread 'the_descriptions_arguments_are_the_lists' panicked at crates/micold-core/tests/github_description_pass.rs:119:9:
  thread 'describe_listed_matches_by_number' panicked at crates/micold-core/tests/github_description_pass.rs:243:5:
  thread 'the_descriptions_query_reads_the_lists_connection' panicked at crates/micold-core/tests/github_description_pass.rs:92:9:
  thread 'the_fake_source_scripts_and_records_description_pages' panicked at crates/micold-core/tests/github_description_pass.rs:296:5:
  thread 'the_list_query_asks_for_no_body' panicked at crates/micold-core/tests/github_description_pass.rs:62:5:
  thread 'the_pass_ends_where_the_list_would' panicked at crates/micold-core/tests/github_description_pass.rs:258:5:
  thread 'the_two_passes_give_the_scenarios_description' panicked at crates/micold-core/tests/github_description_pass.rs:182:5:
  test result: FAILED. 0 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: in `crates/micold-core/src/github.rs`, `issue_node_selection!` loses `bodyText` and the two
  search queries name it after the selection; `open_issues_connection!` is shared by `LIST_QUERY`
  and the new `DESCRIPTIONS_QUERY`; `descriptions_args` and `list_args` go through `page_args`;
  `DescriptionPage` (its `Debug` prints the count), `parse_descriptions_page`, `describe_listed`,
  `DESCRIPTION_PAGE_CAP`, `next_description_cursor`; `IssueSource::describe_open` for `GhCli` and
  `FakeIssueSource` (`with_descriptions`, `description_calls`).
  `scripts/build-lock.sh cargo test --no-fail-fast -p micold-core`: exit 0, every target `ok`.
- refactor: `next_cursor_of` reads `pageInfo` for both page parsers.
- not yet built: `micold-client` does not compile against this commit's trait until T062 (no
  client type implements `IssueSource`, so only the new state and message are missing there; it
  was not built after this change).
