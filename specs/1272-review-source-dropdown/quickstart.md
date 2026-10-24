# Quickstart: review source control and two-column layout (#1272)

## Part A: automated

```
mise run test-core     # M1: source_menu unit tests only, fast
mise run gate          # everything CI runs (or scripts/gate.sh where mise is not installed)
```

Expect green. Key tests:

- `micold_core::review::source_menu` (M1): rows and order for each `PrRows` state, with the setting
  off and on the Default entry (no heading, no rows); which row is chosen; Files marked
  independently; stops in order with Committed disabled on the Default entry; every label of
  data-model §6, including `#n` for an unlisted number; tooltip with and without a base line.
  One test per FR-005 state (SC-005).
- `tests/features_changes.rs`, `tests/features_pr_changes.rs` (M2): open, dismiss (`Esc` path) and
  outside click leave the source unchanged; picking Working tree / a PR / Files closes the list and
  sends the same effects as `SourceChosen` / `FilesPaneToggled` did; chips keep the list open;
  a chip with a PR shown returns to the working tree with the new kinds (and re-reads the tree with
  Files on); Files stays on across a source pick with Back cleared; a PR list refresh keeps the list
  open and re-seats the highlight; the PR-disappears fallback changes the label; previous/next
  changes the label; last pick wins; existing 1003/1150 tests pass unchanged (SC-004).
- Geometry gate for `MenuSelect` (M3): the list floats below the trigger and does not move the
  body; a long title ellipsizes while `#n · ` stays whole; the list caps at its max height and
  scrolls.
- View tests (M4): the left column holds no PR section and no toggle row in any source state
  (SC-003); at 1280×800 the file column is 260 px and the diff ≥ 70% of the body (SC-002); layout
  snapshot `session-area-changes-open` re-recorded, plus new covered states for the open control.

## Part B: visual pass (real client, 1280×800, light and dark; M4)

A worktree with two pull requests, one with a long title.

1. Open the review: header shows **Working tree ▾**; left column starts with the file list.
2. Open the control: Working tree (with Committed / Uncommitted chips), Pull requests heading, two
   rows newest first with state and check marks, Files. Hover: tooltip with the base line.
3. Turn Committed off: list stays open, file list narrows, label `Working tree · uncommitted`.
4. Pick the long-title PR: list closes, label `#n · <title…>` with the number whole; file list,
   diff, PR header and base line follow. Previous/next in the PR header changes the label.
5. With the PR shown, toggle a chip: the working tree returns with the new kinds.
6. Pick Files: tree and path picker, label `Files · Working tree`; pick the PR: still Files, label
   `Files · #n · …`, tree of the PR head; Open diff and Back as in 1150; pick Files again: off.
7. Keyboard: focus the control, Enter opens, arrows move (chips included), Enter picks, Esc closes.
8. Default entry: no PR part; Committed chip disabled with the note.
9. PR status setting off: no PR part. Offline (no `gh`): "Pull requests can't be shown…".
10. Resize 1280 → 1600 → 900: file column stays 260 px; at 900 the control and Close stay visible,
    the subtitle truncates first.
11. Both themes: trigger, list, chips and tooltip legible.
