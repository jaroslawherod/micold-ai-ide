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

## M3

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T019 | #1272's list pane: 260 px file column, row 36 px; no renames (ledger decision 7) | structural | — |
| T020, T021 | reducer: tree default, `TreeFolderToggled`, reveal by `FileSelected` / `OpenFileDiff` / `ReferenceActivated`, folds kept by re-read, reading/failed, source switch, `tree_pane`, leaves' counts and bar scale | `cargo check -p micold-client --test features_changes --test features_pr_changes`: 15 errors, `no field change_rows / tree / change_scale on OpenView`, `no variant TreeFolderToggled`, `unresolved imports changes::tree_pane, changes::TreePane` | T022 |
| T022, T023 | `OpenView.tree`, `change_rows`, `change_scale`, `refresh_change_rows` in `update`, reveal; Files `tree_rows` over `review::tree::layout` (client `Folder` walk deleted) | — (T022 turns T020/T021's red green; T023 is a behaviour-preserving refactor guarded by the unchanged Files tree tests) | `features_changes` 173 passed (Files tree tests unchanged), `features_pr_changes` 92 passed |

## M4

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T024, T025 | `ChangeBar`: two adjacent segments at the `BarParts` lengths, never past `.max_width`, nothing for a zero part (pixel and geometry checks) | not observed separately: the geometry test and the component were written in one step | `cargo test -p micold-client --lib change_bar`: 3 passed |
| T026 | `ChangeBar` in the showcase | `showcase_completeness` names a component with no entry until the entry exists (rule of the gate) | `showcase_completeness` 13 passed, `material_builder_api` 46 passed |
| T027 | 24-character name keeps its room at every depth (SC-006); tree rows drawn | not observed separately (written with the row layout; ledger decision 11 records why the rows went to two lines) | `ui::changes::tests` 1 passed |
| T028 | `changes-tree-collapsed` recorded; the changed list states re-recorded (`changes-review-marks*`, `review-source-open*`) | containment failed on a first 12-row state (the list's scroll content past its viewport); cut to 8 rows | `layout_snapshot` 140 passed |
