# Plan drift, M2

| Path | Reason |
|---|---|
| `crates/micold-client/src/ui/group_naming.rs` | One match arm: `NamingTarget::Rename(_)` titles the shared group-naming dialog "Rename group" with a Rename button (T021 reuses the M1 dialog instead of adding a second one). |
| `crates/micold-client/src/ui/material/group_header.rs` | `GroupHeader::on_right_press`, which forwards the press point to the tree item so a right click on a header opens the group menu (T022). |
| `crates/micold-client/src/ui/mod.rs` | One line in `view_with`: pushes `sidebar::group_menu` into the overlay stack so the menu draws (T022). |
| `crates/micold-client/tests/overlay_registration.rs` | The popover list gains `user_group_menu` and its count moves from 11 to 12, as the guard requires when a popover is added (T021). |
| `crates/micold-client/tests/popover_displacement.rs` | `user_group_menu` joins the displacement table, its opener and a fixture group, so it and the other context menus replace each other (C6). |

`MISSING crates/micold-client/src/main.rs`: the routing T022 planned there already lives in the `Message::SidebarGroups` arm in `app.rs` (recorded in drift-M1.md), and M2 only adds `Msg` variants in `features/sidebar_group_actions.rs`, so `main.rs` needed no change.
