# Cycle log: 1786-sidebar-user-groups

Baseline: suite not run at planning time (build cost; see quickstart Part A). Appended by speckit-tdd-run.

## Baseline (M1)

- suite: not run as a whole (cloud thread, no workspace test run; CI is the full gate). Targeted
  runs below; `micold-core --lib sidebar_layout` and `micold-client --test main sidebar_layout_store`
  had no tests before this milestone.
- commit: `fa0e41a7` (origin/main, branch head before any change)

## Cycle 1: U1–U5, U13, U14 `sidebar_layout` round trip, reconcile and create (T001–T003, T009, T010)

- test: `crates/micold-core/src/sidebar_layout.rs::tests` (10 new tests: round trip, missing/corrupt/older/newer
  text, unknown item kind, three reconcile rules, create trims, ids never reused, Blank, Duplicate)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib sidebar_layout`
  -> `panicked at crates/micold-core/src/sidebar_layout.rs:142:9` (`not implemented: SidebarLayout::to_json`),
  `test result: FAILED. 0 passed; 10 failed`. The T001 types and `unimplemented!` stubs were
  declared first so the tests compile.
- green: `to_json`, `try_from_json` (version check), `from_json` (never fails), `reconcile`
  (one keep-set over groups then ungrouped, then append), `create` + `valid_name`.
  Same command -> `10 passed; 0 failed`.
- refactor: none needed
- notes: several behaviours in one cycle (batched for build cost in a cloud thread; each test
  failed on its own stub). U6's M1 half (a stored `Default` item cannot load) is in
  `a_stored_item_of_no_known_kind_such_as_default_loads_as_no_groups`; U6 stays PENDING for M3's
  `move_item` `IsDefault`. `next_id` was added to the stored layout so ids are never reused across
  a restart (data-model `id`: "never reused").

## Cycle 2: U7, U8 `JsonFileStore::{save,load}_layout` (T004, T005)

- test: `crates/micold-client/tests/sidebar_layout_store.rs` (4 new tests), wired in `tests/main.rs`
  (the client's tests are one executable, #1631, so the Verify command is
  `cargo test -p micold-client --test main sidebar_layout_store`)
- red: `scripts/build-lock.sh cargo test -p micold-client --test main sidebar_layout_store`
  -> `error[E0599]: no method named save_layout found for struct JsonFileStore` (and
  `layouts_path`, `load_layout`). A compile red, not an assertion red; the test's own import was
  then corrected (`Store` -> `ProjectStore`, the trait's real name) before the implementation.
- green: `layouts_path`, `load_layout` (`read_or_quarantine` + `try_from_json`), `save_layout`
  (`write_atomic`), `ProjectStore` defaults, `JsonFileStore` and `FakeProjectStore` impls.
  Same command -> `4 passed; 0 failed`.
- refactor: none needed
- notes: deviation, the red is a missing-symbol compile error rather than an assertion failure.

## Cycle 3: U9–U12, A28 the drawn list applies the layout (T006, T007)

- test: `crates/micold-client/tests/sidebar_tree.rs::user_groups` (7 new tests)
- red: `scripts/build-lock.sh cargo test -p micold-client --test main user_groups`
  -> `assertion left == right failed: Default first, the group's rows in stored order, ...` /
  `left: ["Default", "a", "b", "c", "d"]` (`1 passed; 6 failed`; the no-layout test passes by
  design). `SidebarEntry::UserGroup(GroupHeaderNode)`, `State::sidebar_layout_loaded` and the new
  arms of every `SidebarEntry` match (src and 7 test files, made exhaustive, no assertion changed)
  were added first so the tests compile.
- green: `layout_item`, `apply_layout` (reconcile a copy against the listed rows, then headers,
  members, ungrouped) in `features/sidebar.rs`; `State::sidebar_list` applies the active project's
  layout. `--test main sidebar` -> `193 passed; 0 failed`; whole `--test main` -> one failure,
  `root_state_is_shared::component_local_paths_are_pinned_or_moved` (`state.sidebar.layouts` has
  one writer), answered by an interim `COMPONENT_LOCAL` pin that T012 removes; then
  `--test main root_state_is_shared` -> `4 passed`.
- refactor: none needed
- notes: the layout is applied in `sidebar_list` (the drawn list), not in `sidebar_entries` as
  T007 words it: the Overview's cards walk `sidebar_entries`, and a collapsed group must not hide
  cards. Reconcile runs on a copy at every draw, so a worktree added or removed on disk shows at
  once (A28) without writing the file. T007's `reveal_ready` expanding a collapsed group (U29) and
  the header as a keyboard stop (U28) are M5 behaviours on the test list; not implemented here.
