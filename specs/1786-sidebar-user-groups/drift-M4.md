# Plan drift, M4

| Path | Reason |
|---|---|
| `crates/micold-client/src/app.rs` | Registration of the new feature: `Message::SidebarDrag`, the `sidebar_drag` field, the root's dispatch arm, and one `on_escape` arm so Escape cancels an active drag first and alone (US5-1). |
| `crates/micold-client/src/features/mod.rs` | The one registration line for `sidebar_drag` (feature 021, SC-002). |
| `crates/micold-client/src/features/sidebar.rs` | `ROW_GAP` made public, so the hit test measures rows with the gap the scroll arithmetic already uses rather than a second copy. |
| `crates/micold-client/src/shell/subscriptions.rs` | The window listener that follows an armed press (pointer moves, the left release, focus loss) lives with the other `listen_with` callbacks, not in `main.rs`; see MISSING below. |
| `crates/micold-client/src/ui/material/group_header.rs` | `GroupHeader::on_primary_press`: the header arms a group drag (FR-019). |
| `crates/micold-client/src/ui/material/tree_view.rs` | `TreeItem::on_primary_press`: a row reports its left press point and release beside its own click, which is how a row arms a drag without stealing the click. |
| `crates/micold-client/tests/feature_registration_cost.rs` | One `ALLOWED_CROSS_FEATURE_NAMES` entry: `sidebar_drag` reads `SidebarEntry`, `row_heights`, `layout_item` and `ROW_GAP` from `sidebar` (pure reads, no write). |
| `crates/micold-client/tests/features_sidebar_drag.rs` | The isolation test every feature module must have (SC-004, `every_feature_module_has_an_isolation_test`). |
| `crates/micold-client/tests/main.rs` | The `mod` lines for `sidebar_drag` and `features_sidebar_drag`. |
| `crates/micold-core/src/sidebar_layout.rs` | `SidebarLayout::locate` made public: the drop maps its neighbour row to a stored index and checks whether the row moved at all. |
| `crates/micold-material/src/cdk/context_area.rs` | `ContextArea::on_primary_release`: a release that arrives in the press's own event batch, before the window listener exists, still ends the press (otherwise the next pointer move would start a drag with no button held). |

`MISSING crates/micold-client/src/main.rs`: the window's pointer and release events reach the reducer through `shell/subscriptions.rs` (the binary's subscription module, where the Review tab's drag listener already is), and Escape through `app::on_escape`, which the binary's keyboard subscription already asks. `main.rs` needed no change.

Task T029 names the reducer's state `DragState`; it is `sidebar_drag::State`, because `tests/root_state_is_shared.rs` accepts a root field only when its type is `crate::features::<n>::State`. The **Verify** command `cargo test -p micold-client --test sidebar_drag` is `--test main sidebar_drag`: the file is a module of the one `tests/main.rs` binary, as every sidebar test is.

Group drags arm and activate (FR-019) but drop nowhere in M4: the hit test for a dragged group, `GroupSlot` between headers, is T037/T038 (M5).
