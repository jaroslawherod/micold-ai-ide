# Plan drift, M5

| Path | Reason |
|---|---|
| `crates/micold-client/src/features/sidebar.rs` | `State::sidebar_layout_placed`: the held layout with every drawn row adopted where the sidebar draws it, which the drop already built inline and **Move up** / **Move down** need too (one copy, called as a `State` method, so no feature names another). `reveal_ready` expands the collapsed user group holding the reveal row (T039/T040, U29: `reveal_ready` lives here, not in `navigation.rs`). |
| `crates/micold-client/src/ui/material/group_header.rs` | Doc only: the twisty is no longer "arriving with US6". |
| `crates/micold-client/src/ui/material/tree_view.rs` | `TreeItem::on_menu_key`: a Tab-stop row answers the Menu key and Shift+F10 (the card menu's keys, feature 972) with its own menu, which is how the group header's menu opens by keyboard (T040, D6). `tab_stop` made `pub(crate)` so the sidebar's in-crate test can read it. |
| `crates/micold-client/src/ui/mod.rs` | The group menu's key listener (Up, Down, Enter, Space while it is open, mapped from the card menu's own key function) beside the card menu's, and the Overview's arrow listener left out while a group menu has the keys. |
| `crates/micold-client/src/ui/sidebar.rs` | The header is a Tab stop with a twisty whose press toggles (or runs the open group menu's highlighted entry), and its Menu key message carries an anchor beside the header (`header_menu_anchor`); the group menu is drawn from `group_menu_items` with the keyboard's highlight; `move_items` appends **Move up** / **Move down**, so the worktree, run group and Overview card menus all get them from the one place the **Move to group** entries come from (T038 names `worktree.rs`, which builds no menu items: they are built here and in `ui/mod.rs`, unchanged); a dragged group highlights no list. |
| `crates/micold-client/tests/overlay_registration.rs` | The `GroupMenu` literal gains `highlighted: None`. |
| `crates/micold-client/tests/sidebar_state.rs` | Its card-state walk moved to the shared fixture (`card_state_steps`), which T041's test reuses rather than copying it. |
| `crates/micold-client/tests/support/sidebar_fixture.rs` | `card_state_steps`: the walk of one worktree through the five card states (T041, SC-005). |

`MISSING crates/micold-client/src/features/navigation.rs`: T040 lists it for the header as a navigation stop and the reveal; the header is a Tab stop through `TreeItem::tab_stop` in `ui/sidebar.rs`, and the reveal is `State::reveal_ready` in `features/sidebar.rs`. The location chords step over locations only (contract U2), and a header is no location, so navigation needed no change.

`MISSING crates/micold-client/src/features/worktree.rs`: the worktree menu's items are built in `ui/mod.rs` from `ui::sidebar::move_items`; see `ui/sidebar.rs` above.

`MISSING sidebar_layout_store.rs`: T039's save/reload test is `tests/sidebar_group_actions.rs::order_and_collapsed_state_survive_a_save_and_a_reload`, which drives the client's operations and then the store; `tests/sidebar_layout_store.rs::a_saved_layout_loads_back_after_a_restart` already round-trips the collapsed state.

The Verify commands `--test sidebar_drag --test sidebar_group_actions` are `--test main sidebar_drag:: sidebar_group_actions::` (both files are modules of `tests/main.rs`, as in M4).
