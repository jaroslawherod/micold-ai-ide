# Plan drift, M1

| Path | Reason |
|---|---|
| `crates/micold-client/src/app.rs` | The `Message::SidebarGroups` arm, the `sidebar_groups` state field and the `SidebarLayoutChanged` outcome interpretation that route group actions through the root. |
| `crates/micold-client/src/features/changes/keys.rs` | The New group chord mapping (C6a), which is also where the shortcut sheet's chord row lives. |
| `crates/micold-client/src/features/mod.rs` | Declares `sidebar_group_actions` and adds the `Outcome::SidebarLayoutChanged` variant. |
| `crates/micold-client/src/features/window.rs` | Adds the `GroupName` field id for the group-naming dialog's text field. |
| `crates/micold-client/src/shell/clipboard.rs` | One match arm: `SidebarLayoutChanged` needs no clipboard work. |
| `crates/micold-client/src/shell/mod.rs` | Declares the `sidebar_layout` shell module. |
| `crates/micold-client/src/shell/sidebar_layout.rs` | Shell loading of the active project's layout and saving of changed layouts after each update (R9). |
| `crates/micold-client/src/showcase/catalogue.rs` | Catalogue entry for the new `GroupHeader` component and its poses, plus the `UserGroupHeader` type role. |
| `crates/micold-client/src/ui/group_naming.rs` | New group-naming dialog, built on the shared naming dialog. |
| `crates/micold-client/src/ui/material/tree_view.rs` | `heading(role)` so a group header row draws its label in a fixed type role (R7). |
| `crates/micold-client/src/ui/mod.rs` | Declares `group_naming`, wires the empty-area menu and the New group chord into the root. |
| `crates/micold-client/src/ui/overview.rs` | `SidebarEntry::UserGroup` match arms (no action rows, no default). |
| `crates/micold-client/src/ui/rename.rs` | Rename dialog struct gains `confirm` and `error` fields shared with the naming dialog. |
| `crates/micold-client/src/ui/worktree_rename.rs` | Same `confirm` and `error` fields for the worktree rename dialog. |
| `crates/micold-client/tests/features_sidebar.rs` | Sidebar tests updated for header rows and the New group button. |
| `crates/micold-client/tests/features_sidebar_group_actions.rs` | Test module for the group actions (T-tasks of M1), wired into `tests/main.rs`. |
| `crates/micold-client/tests/fixtures/layout_snapshot.txt` | Re-recorded layout snapshot for the M1 header and New group button (T044). |
| `crates/micold-client/tests/main.rs` | Wires the new test modules. |
| `crates/micold-client/tests/navigation_apply.rs` | Test fixtures updated for the new `App` field and outcome. |
| `crates/micold-client/tests/overlay_registration.rs` | Overlay list gains the group-naming dialog. |
| `crates/micold-client/tests/overlay_registry.rs` | Same: registry expectations include the group-naming dialog. |
| `crates/micold-client/tests/popover_displacement.rs` | Fixture update for the new `App` field. |
| `crates/micold-client/tests/root_vocabulary_is_cross_cutting.rs` | Vocabulary allow-list gains the new `Message::SidebarGroups` and outcome. |
| `crates/micold-client/tests/session_archive.rs` | Fixture update for the new `App` field. |
| `crates/micold-client/tests/sidebar_attention.rs` | Fixture update for the `UserGroup` entry kind. |
| `crates/micold-client/tests/worktree_visibility.rs` | Fixture update for the `UserGroup` entry kind. |
| `crates/micold-daemon/src/catalog.rs` | Forgetting a project also removes its stored sidebar layout (review A fix). |
| `crates/micold-material/src/type_role_mapping.rs` | Maps the new `TypeRole::UserGroupHeader` to `label_medium` (R7). |

`MISSING crates/micold-client/src/ui/shortcut_sheet.rs`: the chord row lives in `features/changes/keys.rs` and the shortcut sheet test passes. `MISSING data-model.md` is a spec file, not a code path. `MISSING sidebar_groups.rs` is deliberately unchanged in M1.
