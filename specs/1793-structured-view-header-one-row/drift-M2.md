# Plan drift, M2

- crates/micold-material/src/menu.rs: `MenuOverlay::controls` hosts the segmented control under the items (T015 needs it; no other shared slot exists).
- crates/micold-client/src/overlay/registry.rs: the worktree menu is a registered overlay surface (architecture guard overlay_registry).
- tests/support/covered_states.rs, tests/layout_snapshot.rs, tests/fixtures/layout_snapshot.txt: the always-present worktree menu surface adds two layers, so the modal's and switcher's layer paths re-point and the snapshot is regenerated.
- tests/gates/{containment,sibling_parity,title_bar_fits,top_bar_items}.rs: same layer shift; the new top-bar trigger adds one lane child.
- `MISSING settings.sh`: a stray token in a task line, not a path.
