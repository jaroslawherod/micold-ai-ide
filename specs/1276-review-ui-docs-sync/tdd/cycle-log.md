# Cycle Log: Review UI docs sync — #1276

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: M1's full gate (`scripts/gate.sh`) green at `ea01a8b9`; CI's merge gate green on
  `origin/main`.

## Cycle 1: T003–T005 `review_keys_doc` holds the guide's shortcut table to `REVIEW_KEYS`

- test: `crates/micold-core/tests/review_keys_doc.rs` (3 new: `the_markers_are_there`,
  `the_guides_table_is_the_review_key_table`, `each_primary_chord_names_its_macos_form`), on the
  shared `tests/support/marked_table.rs`
- red: recorded after the fact (review B F1), without editing tracked files: a detached scratch
  worktree at `origin/main` (`02421815`, guide before M1) with only the four test files copied in
  (`review_keys_doc.rs`, `support/marked_table.rs`, `support/mod.rs`, `terminal_only_doc.rs`), then
  `cargo test -p micold-core --test review_keys_doc` -> `0 passed; 3 failed`:
  `the_markers_are_there` panicked at `review_keys_doc.rs:46`:
  ``docs/user-guide/reviewing-changes.md` has the marker `<!-- review-shortcuts:start -->``; the
  other two panicked in `marked_section` (`marked_table.rs:33`) on the same missing marker
- green: the branch's guide (T006–T008 shortcut table between the markers):
  `cargo test -p micold-core --test review_keys_doc --test terminal_only_doc --test
  documentation_is_not_read` -> 3 + 2 + 3 passed, 0 failed; `cargo clippy -p micold-core
  --all-targets -D warnings` clean
- refactor: `terminal_only_doc.rs` moved onto `marked_table.rs`; it passes both against
  `origin/main`'s guide in the scratch worktree (2 passed) and on the branch (2 passed), so the
  refactor kept its behaviour. Review A F1: `key_cell` now builds from `ReviewKey::labels()` (the
  text the `?` sheet renders) instead of re-deriving the key; F3: row count asserted before the zip loop.

## M2 — the layout fixture and `review_tab_fits` (T014–T018, T022–T024)

- T014: `cargo test -p micold-client --test layout_snapshot --test layout_text_overflow --test
  session_area_stability` on `origin/main` 1669ef5d -> 150 + 3 + 16 passed, 0 failed: the merged
  fixture holds.
- T015 red: the contract's anchors added to the slices' states and four new states registered
  (`review-tab-beside-sessions`, `review-drag-composer`, `review-empty`, `review-two-worktrees`);
  `layout_snapshot` -> 149 passed, 1 failed: `the_layout_matches_the_committed_fixture`, differing
  from `session-area-review-hidden` on (the positional diff names every state after the first
  insertion). Green waits for T019's re-record (stop b).
- T016: nothing to change; #1269 already replaced `session-area-changes-open` with
  `session-area-review-shown` and moved `session_area_stability.rs` onto the hidden-review presses.
- T018 red: `review_tab_fits` -> 1 failed: at 640×480 `drawer-drafts` and 19 more state/theme pairs
  lay nodes outside their parents (progress slot, sort chips, diff header, narrow drawer, and
  scrolled viewports). Filed as #1474; the gate declares them in `SMALLEST_WINDOW`, each required to
  keep firing -> 2 passed. Wrong-anchor check: `REVIEW_DRAWER` pointed at the review body ->
  `every_review_state_fits_at_both_windows_in_both_themes` failed; reverted -> passed.
- T022: `layout_snapshot` (`--skip review_tab_fits`) warm, three runs each: 11.74 / 10.82 / 11.14 s
  with the four new states, 10.29 / 10.33 / 10.98 s without.
- Scoped gate: fmt and clippy clean; `cargo test -p micold-client --no-fail-fast` -> only
  `the_layout_matches_the_committed_fixture` fails (the re-record waits for stop b).
  `layout_coverage_registry` first read `review_tab_fits.rs`'s `-> &'static CoveredState {` as a
  registration; `constructs` now treats a `&…` word before the name as a type, with that line added
  to `defining_or_naming_the_type_is_not_registering_a_state`.

## M3 — the visual pass's styling FAILs (T029)

- red: four tests added to the shared components, run on the unfixed components
  (`cargo test -p micold-client --lib` filtered to them) -> 0 passed, 4 failed:
  `pull_request_row::a_long_title_is_one_ellipsized_line_inside_the_row` (check 02: the title
  wrapped, its second line below the row), `review_bar::an_ellipsized_error_reads_whole_in_its_tooltip`
  (check 07: no tooltip on the cut error), `review_panel_row::a_draft_rows_actions_fit_the_drawer_width_and_the_row_height`
  (check 08: Edit/Delete past 360 px), `changes::a_long_entry_name_is_one_ellipsized_line` (check 11:
  the header's name wrapped to two lines).
- green: `PullRequestRow` and `changes::heading` use `Ellipsized`; `ReviewBar` wraps a cut error in a
  `Tooltip`; `ReviewPanelRow` puts a draft's actions on their own line -> the four tests pass.
- re-shot in both themes from a release build of the fixed tree (visual-pass/results.md).
- not fixed here (behaviour or review-layout geometry, out of scope): #1519 (card width, checks 04
  and 12), #1539 (source-control tooltip over its list, check 02), #1540 (rows under the list's
  scrollbar, check 03).

## M3 — review A F1: a cut name or title reads whole on hover

- red: `pull_request_row::a_cut_title_reads_whole_in_its_tooltip` added and
  `changes::a_long_entry_name_is_one_ellipsized_line` extended to hover the cut name -> 0 passed,
  2 failed (no tooltip: `left: []`).
- green: `Tooltip::whole_label` (micold-material) wraps an `Ellipsized` in a tooltip that opens only
  while `Ellipsized::is_cut` holds of its tree (`cdk::tooltip::Tooltip::open_if`); `PullRequestRow`
  and `changes::heading` use it -> both tests pass, and a label shown whole has no tooltip.
