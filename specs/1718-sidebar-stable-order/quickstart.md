# Quickstart: sidebar stable order (#1718)

## Part A — automated

```
mise run test-core
scripts/build-lock.sh cargo test -p micold-client --test sidebar_state
scripts/build-lock.sh cargo test -p micold-client --test navigation_apply
scripts/build-lock.sh cargo test -p micold-client --lib state_icon
mise run gate
```

Test map: SC-001/SC-003 and FR-002/003/006 in `tests/sidebar_state.rs` (rewrite the 1125 group
tests as flat-list tests); SC-002 in `ui/material/state_icon.rs`; FR-006 in `tests/navigation_apply.rs`;
FR-009 snapshot in `tests/layout_snapshot.rs`.

## Part B — visual pass (needs eyes at a display)

1. Open the showcase sidebar sections: the five-state rows show five different glyphs; no group header pose.
2. Run the client on a project with worktrees in each state: rows keep their place while a session goes working, waiting, idle.
3. Showcase: Needs action, Needs attention and Done rows are tinted (Done quietest), Working and Idle are not; hover and select a tinted row, the state layer shows over the tint; check light and dark.
4. Down/Up arrows step over every row; reveal selects a Done row without expanding anything.
