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
  a_picked_rows_marker_is_beside_the_first_line_whatever_the_height: no child 1 at depth 2 of [0, 1, 0]
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

