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
