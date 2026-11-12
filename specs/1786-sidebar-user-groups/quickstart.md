# Quickstart: Sidebar user groups

## Part A: automated
```sh
mise run test-core                                  # sidebar_layout unit tests
scripts/build-lock.sh cargo test -p micold-client --test sidebar_group_actions
scripts/build-lock.sh cargo test -p micold-client --test sidebar_drag
scripts/build-lock.sh cargo test -p micold-client --test sidebar_layout_store
mise run gate                                       # before pushing
```
Expected: all pass; red before green per task order.

## Part B: visual pass (SC-007)
Launch the client on a project with 4+ worktrees, one run group, sessions in several card states.
1. New group "Work"; rename it; try a blank and a duplicate name (message shown, state kept).
2. Drag a worktree onto "Work": highlight on the target, marker at the landing place, row lands there.
3. Drag it to a second group, then to the ungrouped list; Escape mid-drag; drop outside the sidebar: nothing changes.
4. Reorder rows and group headers by drag and by Move up/down.
5. Collapse a group, restart: layout, order and collapsed state return.
6. Keyboard only: every step above through the context menu and chords.
7. Record screenshots of layout, highlight, marker in light and dark themes in `visual-pass/`.
