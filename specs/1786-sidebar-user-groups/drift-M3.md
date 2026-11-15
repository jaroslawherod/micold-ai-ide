# Plan drift, M3

| Path | Reason |
|---|---|
| `crates/micold-client/src/features/runs.rs` | `close_menu`, so a pick from a user-group entry closes the run group row's menu without the new feature writing `state.runs.menu` (feature_write_isolation, FR-021). |
| `crates/micold-client/src/features/sidebar.rs` | `GroupHeaderNode.hidden` and `State::layout_item_exists`: the header's count decision D7 (rows shown, `n of m` while a filter hides members), and the check that a row named by a message is still there. |
| `crates/micold-client/src/ui/group_naming.rs` | One match arm: `NamingTarget::NewAndMove` shares the "New group" dialog title. |
| `crates/micold-client/src/ui/material/group_header.rs` | `GroupHeader::hidden` and the `n of m` words (D7). |
| `crates/micold-client/src/ui/mod.rs` | The move entries are spliced into `location_menu_items` (worktree menu and Overview card menu, the keyboard path) and the run group menu; both are built here, not in `worktree.rs`. |
| `crates/micold-client/src/ui/sidebar.rs` | `move_items`, the menu items drawn from `move_menu_items`. |
| `crates/micold-client/tests/gates/sidebar_group_header_fits.rs` | One field for the new `GroupHeaderNode.hidden`. |
| `crates/micold-client/tests/fixtures/layout_snapshot.txt` | Regenerated (`UPDATE_LAYOUT_SNAPSHOT=1`): the `worktree-menu-open` state has one more entry, "Move to new group…", so its panel is 48 dp taller (256 to 304). |

`MISSING crates/micold-client/src/features/worktree.rs`: the move entries are built where the worktree and run group menus are built, in `ui/mod.rs` (`location_menu_items` and the run group menu); `worktree.rs` keeps the menu state and needed no change.
