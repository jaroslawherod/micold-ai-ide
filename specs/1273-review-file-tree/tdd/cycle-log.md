# TDD cycle log: 1273 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `review::tree` module and `tests/review_tree.rs` | structural | compiles |
| T002, T003, T005 (red only) | layout L1–L4, L12, chaining, ordering, SC-002; `TreeChoices` default, toggle, reveal, no pruning, options | `cargo test -p micold-core --test review_tree`: did not compile, `unresolved imports micold_core::review::tree::layout, FolderRow, LayoutOptions, Leaf, Openness, Row, RowKind, TreeChoices, TreeSort` | — |
| T004, T006 | `layout` (one `BTreeMap` build, folders first, byte order, chaining, both openness modes); `TreeChoices::{toggle, reveal, options}` | — | `review_tree` 17 passed; 18 after review A (same-name files in two folders, default-closed chains) |

## M2

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T007 | folder totals (L5, chained, collapsed, `u64` sums) | `review_tree`: 4 failed (`a_folder_row_sums_its_files_counts` etc.: totals were 0) | T010 |
| T008, T009 | `bar_scale`, `bar`, `MIN_BAR`, `max_indent_levels`, `indent_levels`, `hover_text` | `cargo test -p micold-core --lib review::tree`: did not compile, `cannot find function bar_scale, bar, max_indent_levels, indent_levels, hover_text`, `cannot find value MIN_BAR` | T010: lib `review::tree` 8 passed, `review_tree` 22 passed |
| T011, T013, T015, T016 | filter (L8–L10, spans, union, whitespace, no reorder, SC-001); `TreeChoices` while filtering; sorts (L6, L7, L11, SC-004); `Offer`, `sort_options`, `set_sort` | did not compile: `no method set_filter, clear_filter` on `TreeChoices`; lib: `cannot find type Offer`, `sort_options`, `UNREVIEWED_UNTIL_MARKS`, `set_sort`. With T014/T016's API in: `review_tree` 12 failed (filter and sort tests) | T014: `TreeChoices` tests pass; T012, T017: `review_tree` 42 passed |
| T018 | SC-003: 5,000 files in 200 folders, slowest of collapse / keystroke / sort | written after T012/T017 (the timing it guards needs `layout` complete) | passes in debug (500 ms) and release (100 ms) |
