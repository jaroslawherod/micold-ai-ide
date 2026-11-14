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

## Cycle 4: A1, A4/A5 (create half), U31 creating a group (T011, T012, T013, part of T014)

- test: `crates/micold-client/tests/sidebar_group_actions.rs` (8 tests, module of `tests/main.rs`)
- red: `cargo check -p micold-client --tests` -> `E0432 unresolved import
  micold_client::features::sidebar_group_actions`, `is_new_group_chord`; `E0599 no variant
  NewGroup`, `Message::SidebarGroups`; `E0609 no field layouts_unsaved`, `sidebar_groups`
  (17 errors). A compile red: the feature did not exist.
- green: `features/sidebar_group_actions.rs` (`Msg::{New,TextChanged,Confirmed,Cancelled}`,
  `NamingDraft{target: New, text, error}`, `refusal`, `GroupNamingDialog` surface), root
  `Message::SidebarGroups`, `State.sidebar_groups`, `Outcome::SidebarLayoutChanged` applied by
  `State::sidebar_layout_changed` (keeps it, marks `sidebar.layouts_unsaved`), shell
  `shell/sidebar_layout.rs::after_update` (loads the active project's layout once, saves the
  unsaved ones, `notify_error` on a failed save), `ui/group_naming.rs` over `rename_dialog`
  (new `confirm` and `error` fields), `MoreAction::NewGroup`, `keymap::is_new_group_chord`
  (Primary+Shift+G), the composer sheet's "Sidebar" group. `cargo test -p micold-client --test
  main sidebar_group_actions` -> `8 passed; 0 failed`.
- refactor: none
- notes: whole `cargo test -p micold-client --tests --no-fail-fast` -> only
  `layout_snapshot::the_layout_matches_the_committed_fixture` fails (T044 re-records it). Guard
  tests updated for the new feature: features_sidebar/sidebar_state more-menu arrays, root
  vocabulary 33, popover DIALOGS 31, keyboard-subscription EXEMPT, the interim
  `sidebar.layouts` COMPONENT_LOCAL pin removed, new `tests/features_sidebar_group_actions.rs`.
  The "Sidebar" sheet group is shown only on the sheet opened outside a Review tab:
  with it, the Review sheet overflowed `review_tab_fits` at 640x480 by 8 px.

## Cycle 5: the header row, its gate, the empty-area menu (T008, rest of T014, T015, T016, T044)

- test: `group_header.rs` unit tests (count words, one-line heading row with its full name on the
  tooltip); `tests/gates/sidebar_group_header_fits.rs` (module of `tests/layout_snapshot.rs`);
  two empty-area menu tests in `tests/sidebar_group_actions.rs`; the `sidebar_empty_menu` row of
  `tests/popover_displacement.rs` and `tests/overlay_registration.rs`.
- red: the gate and the menu tests were written beside the code, not before it, so their red is
  shown by mutation instead: giving the header a second line (`supporting_text`) fails the gate at
  `sidebar_group_header_fits.rs:55` ("no 36 dp header row"); the guard tests went red on the new
  popover until it was listed (`overlay_registration` "a new popover-shaped field ...
  sidebar_groups.empty_menu"; showcase completeness "variant `UserGroupHeader` has no instance").
- green: `ui/material/group_header.rs` (`GroupHeader::new(name).on_toggle(m).collapsed(b)
  .count(n).roles(r).key(k)` into a `TreeItem`, `DropMarker` into an `Element`),
  `TreeItem::heading(role)`, `TypeRole::UserGroupHeader` (`label_medium`), showcase poses
  `group_header_row`, `group_header`, `drop_marker`; `ui/sidebar.rs` draws `SidebarEntry::UserGroup`
  headers and wraps the list in a `ContextArea` that opens `SidebarEmptyAreaMenu` (New group…).
  `cargo test -p micold-client --test layout_snapshot` -> `177 passed` after
  `UPDATE_LAYOUT_SNAPSHOT=1` (the more menu's fourth item, all under `0/8`).
- refactor: none

## Cycle 6: rename and delete a group (T018 to T023)

- test: core `sidebar_layout::tests` (`rename_*`, `a_refused_rename_*`, `delete_*`,
  `deleting_an_unknown_group_*`); `tests/sidebar_group_actions.rs` (rename draft, rename keeps
  members and saves, blank/duplicate/vanished rename refused, delete returns members and touches no
  worktree or session, the group menu); `popover_displacement` and `overlay_registration` rows for
  `user_group_menu`.
- red: core tests: `cargo test -p micold-core --lib sidebar_layout` -> `error[E0599]: no method
  named rename/delete found for struct SidebarLayout` (5 errors). Client tests:
  `cargo test -p micold-client --test main sidebar_group_actions` -> 20 errors (`Msg::Rename`,
  `Msg::Delete`, `Msg::MenuOpened`, `Msg::MenuDismissed`, `NamingTarget::Rename`,
  `State.menu`, `GROUP_MENU_ITEMS`, `ui::sidebar::group_menu` not found).
- green: `SidebarLayout::rename`/`delete`; `Msg::{Rename, Delete, MenuOpened, MenuDismissed}`,
  `NamingTarget::Rename`, `GroupContextMenu` (id `user_group_menu`, registered as a context menu
  that replaces the other row menus); header `on_right_press`; `ui::sidebar::group_menu`.
  Core `15 passed`; `cargo test -p micold-client --test main` -> `3419 passed; 0 failed`.
- refactor: none
- notes: the state field is `group_menu`, not `menu`: `overlay_registration` finds popover flags by
  a `_menu`/`_open` name. No chord opens a row menu from the keyboard for worktree rows either, so
  the group menu matches them: its items are keyboard-operable menu items (U30); a keyboard opener
  arrives with the header's focus handling in M5 (T040).
