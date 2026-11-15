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

## Cycle 7: move an item to a group (U6, U17–U19; T024, T025)

- test: core `sidebar_layout::tests` (`move_item_*`, `a_run_group_moves_as_one_item`,
  `every_outcome_keeps_each_item_in_exactly_one_list`, `adopt_places_an_unheld_item_*`).
- red: `cargo test -p micold-core --lib sidebar_layout` -> `error[E0599]: no method named
  move_item found for struct SidebarLayout` (10 errors).
- green: `SidebarLayout::move_item(&item, DropTarget)` removes before it inserts, adjusts the index
  for a move inside one list, clamps it, refuses `Worktree("")` (`IsDefault`), an unlisted item
  (`UnknownItem`) and a missing group or a `GroupSlot` (`UnknownGroup`), leaving the layout as it
  was; helpers `holds`, `list_of`, `adopt`. `23 passed`.
- refactor: review A r1 moved "a live row the layout does not hold is ungrouped" into
  `SidebarLayout::adopt`, so the client's two callers do not each copy it.

## Cycle 8: Move to group in the menus (A6–A9, U27; T026, T027)

- test: `tests/sidebar_group_actions.rs` (`the_move_menu_*`, `moving_*`, `new_group_from_the_move_menu_*`,
  `a_refused_name_keeps_the_move_pending_*`, `a_run_group_row_moves_as_one_unit_*`,
  `the_worktree_menu_offers_move_to_group_above_delete`, `a_keyboard_user_moves_a_row_through_the_card_menu_*`,
  `choosing_the_list_the_row_is_already_in_*`, `new_group_from_a_move_menu_leaves_an_open_dialog_*`).
- red: `cargo test -p micold-client --test main sidebar_group_actions` -> 5 errors (`E0432`
  unresolved `move_menu_items`, `MoveTarget`; `E0599` no variant `Msg::Move`, 2x `NamingTarget::NewAndMove`).
  One test went red after the first green for the right reason: moving a row the layout never held
  to *Ungrouped* pinned it to the front of the saved order (`left: ungrouped: [b]`, `right: []`); it
  now does nothing, the row being ungrouped already.
- green: `features/sidebar_group_actions.rs` (`MoveTarget`, `MoveEntry`, `move_menu_items`,
  `Msg::Move`, `NamingTarget::NewAndMove`), `ui::sidebar::move_items`, entries in
  `ui::location_menu_items` (worktree menu and Overview card menu, above the bind and setup entries
  that sit above Delete) and in the run group menu. Keyboard: the Overview card menu (Menu or
  Shift+F10, Up/Down, Enter) carries the same entries. `--test main` -> `3465 passed; 0 failed`.
  Two existing tests went red and pointed at the placement: `setup_output_geometry` (setup entries
  stay directly above Delete) and `feature_write_isolation` / `surface_registration_cost` (the run
  group menu is closed through `runs::close_menu`, not by writing `state.runs.menu`).
- refactor: none beyond the `adopt` extraction above.

## Cycle 9: the header's count under a filter (decision D7)

- test: `tests/sidebar_group_actions.rs::a_header_counts_the_rows_shown_and_says_how_many_a_filter_hides`,
  `a_group_holding_no_member_has_nothing_hidden`, `a_member_whose_worktree_is_gone_is_not_counted_as_hidden`;
  `group_header.rs` unit test `a_group_whose_members_a_filter_hides_is_not_called_empty`.
- red: written beside the code, so its red is shown by mutation: setting `hidden: 0` in
  `apply_layout` fails the first test (`left: (0, 0)`, `right: (0, 2)`), and `count_words` without
  the `hidden` arm shows `empty` for `(0, 2)`.
- green: `GroupHeaderNode.hidden`, `GroupHeader::hidden`, `count_words(count, hidden)`: `n`, `n of m`
  or `empty`.
- refactor: none.

## Cycle 10: the drag reducer and hit test (A10–A15, A27 in part, U20–U23; T029, T030)

- test: `tests/sidebar_drag.rs` (`a_press_arms_*`, `a_move_of_four_pixels_*`,
  `the_list_pointer_is_ignored_until_*`, `the_hit_test_finds_only_*`, `the_spans_follow_*`,
  `over_a_target_the_spot_carries_the_marker_*`, `dropping_*`, `a_dropped_worktree_keeps_its_sessions_*`,
  `a_drop_on_a_collapsed_group_*`, `a_drop_on_the_rows_own_place_*`, `a_drop_below_the_rows_own_place_*`,
  `default_never_arms`, `rows_a_tag_filter_hides_*`, `a_run_group_row_is_dragged_*`,
  `a_group_header_arms_a_group_drag_*`); `tests/features_sidebar_drag.rs` (isolation, SC-004).
- red: `cargo check -p micold-client --test main` -> 31 errors: `E0432` unresolved import
  `micold_client::features::sidebar_drag` (2), `E0599` no variant `Message::SidebarDrag` (2),
  `E0609` no field `sidebar_drag` on `State` (27).
- green: `features/sidebar_drag.rs` (`State`, `Dragged`, `DropSpot`, `Msg`, `entry_spans`,
  `entry_lists`, `hit_test`, `update`); `Message::SidebarDrag` and the `sidebar_drag`
  field in `app.rs`; `SidebarLayout::locate` made public in the core for the drop's index and the
  no-op check. `cargo test -p micold-client --test main sidebar_drag` -> `25 passed; 0 failed`
  on the first run after the module compiled.
- refactor: `sidebar::ROW_GAP` made public rather than repeated, so the hit test and the scroll
  arithmetic measure the same gap.

## Cycle 11: cancelling the drag (A20–A22; T031, T032)

- test: `tests/sidebar_drag.rs` (`escape_ends_the_drag_with_no_change`,
  `a_release_outside_every_target_changes_nothing`, `the_dragged_worktree_vanishing_*`,
  `a_release_after_the_dragged_worktree_vanished_*`, `escape_with_no_drag_keeps_its_meaning`,
  `losing_the_window_focus_cancels_the_drag`). Each compares `sidebar_list()` or the layout before
  and after (SC-002).
- red: written in the same file as cycle 10, so the same 31 compile errors; the Escape test then
  needed `on_escape` to answer `Message::SidebarDrag(Msg::Cancelled)` while a drag is active.
- green: the cancel paths in `sidebar_drag::update` (`Cancelled`, a release with `over = None`,
  `still_there` on every pointer message and on the release) and one `on_escape` arm in `app.rs`
  after the registry, guarded by `sidebar_drag.active`.
- refactor: none.

## Cycle 12: the glue (T033)

- test: `ui::sidebar::tests::rows_and_headers_arm_a_drag_default_does_not_and_the_target_list_is_highlighted`,
  `shell::subscriptions::tests::sidebar_drag_message_follows_the_pointer_and_ends_on_release_or_blur`,
  `micold-material cdk::context_area::tests::a_primary_release_over_the_area_reports`.
- red: written beside the glue (Principle I exception for view glue); each fails without its line:
  no `on_primary_press` on a row leaves `arming` at 0, no `drop_highlight` leaves `lit` at 0.
- green: `TreeItem::on_primary_press` and `GroupHeader::on_primary_press` (press point and release
  through `cdk::ContextArea`, whose new `on_primary_release` hears a release that comes in the
  press's own batch); `ui::sidebar` arms top-level worktree rows, run group rows and headers, tints
  the target list, puts `DropMarker` on the edge of the row `over.marker` names
  (`TreeItem::drop_marker`), and wraps the list in a `mouse_area` that reports the pointer's
  height while the drag is active; `shell/subscriptions.rs` follows the pointer, the release and
  the focus loss while a press is armed.
- refactor: the first glue drew the marker in a `stack` over the whole list. The stack added a
  layout level above every row and broke the layout snapshot and the gates that address rows by
  path (`context_menu_anchor`, `tooltip_clears_its_row`: "no node at 0/0/0/1/0/0/0/2/0/0/8").
  The marker moved onto the row it marks, laid over the row's edge, and the `mouse_area`, which
  adds no layout node, stayed; the paths are as before.

## Cycle 13: review A fixes (rounds 1 to 3)

- test: `tests/sidebar_drag.rs::the_hit_test_finds_only_*` (the marker of a header and of a lower
  half sits on the target's own row, `below`), `escape_cancels_the_drag_before_a_surface_open_under_it`;
  `ui::sidebar::tests::rows_and_headers_arm_*` (the whole target list is lit, marker edges, the
  dragged row is `inert`); `cdk::context_area::tests::a_held_release_reaches_the_content_with_no_cursor`.
- red: the header assertion failed against the first `hit_test` (`marker` was the next list's first
  row, `below` absent: E0609 until the field existed); the Escape test failed while
  `dismiss_topmost` closed the registry's surface first (`armed` still `Some`); the inert count
  failed (3 against 2) when it picked a row with no press, which showed the count was the thing
  measured.
- green: `DropSpot.below`; `drop_highlight` tints each target entry's whole row range;
  `drag_escape` read by `on_escape` and `dismiss_topmost`; `TreeItem::inert` and
  `ContextArea::hold_release`, which hand the dragged row's button its release with no cursor, so
  it lets go without pressing and keeps its look (round 2's disabled button kept a stale pressed
  state, found in round 3).
- refactor: `TreeItem`'s press point and release are one `Option<(OnPress, M)>`.
