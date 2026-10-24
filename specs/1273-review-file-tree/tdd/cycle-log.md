# TDD cycle log: 1273 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `review::tree` module and `tests/review_tree.rs` | structural | compiles |
| T002, T003, T005 (red only) | layout L1–L4, L12, chaining, ordering, SC-002; `TreeChoices` default, toggle, reveal, no pruning, options | `cargo test -p micold-core --test review_tree`: did not compile, `unresolved imports micold_core::review::tree::layout, FolderRow, LayoutOptions, Leaf, Openness, Row, RowKind, TreeChoices, TreeSort` | — |
| T004, T006 | `layout` (one `BTreeMap` build, folders first, byte order, chaining, both openness modes); `TreeChoices::{toggle, reveal, options}` | — | `review_tree` 17 passed |
