# Quickstart: changed-file tree, filter and sort (#1273)

## Part A: automated

```
mise run test-core     # M1, M2: review::tree
mise run gate          # M3 onward
```

Expect green. Key tests:

- `crates/micold-core/tests/review_tree.rs` and `review::tree` unit tests: contract cases L1–L12
  ([contracts/review-tree.md](contracts/review-tree.md) §1), `TreeChoices` transitions (toggle on a
  chained row, filter-time folds dropped on clear, reveal, sort refused when not offered),
  `sort_options` with and without marks, `bar` (4:1 split, never over `max_width`, `None` at zero),
  `indent_levels` cap, SC-001 fixture (30 files, `tests.r` lists exactly one), SC-002 (no file row label
  contains `/`), SC-004 (largest file first at each level under Size), and the SC-003 timing test
  (5,000 files: collapse, keystroke, sort change each < 100 ms).
- `crates/micold-client/tests/features_changes.rs`, `features_pr_changes.rs`: re-read keeps filter,
  sort, collapsed folders and the selection (SC-005); source switch keeps them; selection inside a
  collapsed folder opens it; filtered-out selection keeps its diff; Esc and the clear action empty the
  filter; existing **Files** tree tests unchanged.
- `ui/material/text_field.rs` harness test: Escape on a focused field publishes the `on_escape`
  message once; unfocused, nothing.
- `tests/layout_snapshot.rs` with the new covered states (tree with a collapsed folder, filter with
  matches, filter with no match); `tests/showcase_completeness.rs` with `ChangeBar`.

## Part B: visual pass (1280×800, light and dark; a worktree with ~30 changed files in nested folders)

Run with the `visual-pass` skill against the client, or the showcase for step 2's component.

1. Open the Review view: folders before files, chained folders on one row, file names first and
   bold, no folder prefix on file rows; hover a file: full path (and old path for a rename).
2. Bars: a large and a small text file differ visibly in length; green and red parts in proportion;
   a binary file and a Mode change show no bar.
3. Type a fragment matching three files: those three under their folders, matched characters
   highlighted; type a fragment matching nothing: the no-match line and **Clear filter**; `Esc`
   clears.
4. Collapse a folder, filter, clear: the folder is collapsed again. Collapse the folder holding the
   selected file: its diff stays.
5. Switch sort to **Size**: largest folder and file first at each level; with 5,000 files (generated
   worktree) collapsing, typing and sorting feel immediate and scrolling is smooth.
6. Long file name (24+ chars) and a deep path at the default list width: name ellipsised, bar and
   counts in place, indentation capped; no clipped name up to 24 characters (SC-006).
7. Edit a file from the agent while filtered, sorted and folded: all three kept after the re-read.

## Part B results

Recorded at close, 2026-10-26.

**Part A.** `review_tree` 42 passed and the `micold-core` lib 1763 passed (SC-001 to SC-004, the
SC-003 timing test included); the client's tree, marks and source-menu tests 38 + 29 passed; the
full gate (`fmt`, clippy, the workspace suite, `layout_snapshot`, `showcase_completeness`,
`scripts/tests`) passed for the close PR (see the ledger). Ten deliberate mutants of the layout and
the reducer each failed a test (`tdd/verification.md`).

**Part B**, at 1280×800 in light and dark, release build, a fixture worktree of 28 changed files in
nested folders ([visual-pass/m4.md](visual-pass/m4.md), [visual-pass/m5.md](visual-pass/m5.md)):

| Step | Result |
|---|---|
| 1 Tree, chains, names first, hover path | pass (M4) |
| 2 Bars in proportion; none for binary and Mode | pass (M4) |
| 3 Filter, highlights, no-match line, `Esc` | pass after a fix (M5): highlights moved to the tertiary role |
| 4 Collapse; the selected file's diff stays | pass (M4: collapse and the diff); filter-then-clear restoring the fold is `clearing_the_filter_restores_the_folds_before_it`, not checked by eye |
| 5 Size and Unreviewed first | pass (M5); the 5,000-file timing is the core test SC-003, not judged by eye |
| 6 24-character name at depth; ellipsis past it | pass after a fix (M4): indent cap sized to the row's real width |
| 7 Re-read while filtered, sorted and folded keeps all three | pass (M5) |
