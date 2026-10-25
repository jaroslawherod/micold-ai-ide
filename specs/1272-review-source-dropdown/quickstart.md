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
10. Resize 1280 → 1600 → 900: file column stays 260 px; at 900 the control stays whole and the
    base line truncates first (Close is the Review tab's ×, outside the header: ledger decision 6).
11. Both themes: trigger, list, chips and tooltip legible.

## Results (close, 2026-10-25)

**Part A: pass.** The full gate (`scripts/gate.sh`, CI's order; mise is not installed in the cloud
session) passed for M4 at `88213c041` and CI was green on `72c45fba` before #1418 merged; it passed
again on the close tree with the test-strength fixes of `tdd/verification.md` (T028–T033), at
`72c52894c`. All the key tests above exist and ran: `review::source_menu` (20 passed),
`features_changes` / `features_pr_changes` `source_menu` tests, `menu_select_fits`,
`session_area_fits`, `review_bar_fixed` and the re-recorded `layout_snapshot` (`review-source-*`
states). Root-only permission tests are skipped by `scripts/gate.sh` in the container; CI runs them.

**Part B: pass, with gaps.** Run as the M4 visual pass ([visual-pass/m4.md](visual-pass/m4.md)):
in-process tiny-skia renders of the real window over the `review_source` poses at 1280×800 (1600
and 900 for step 10), light and dark, not on a display and not against GitHub (no signed-in `gh`;
fixture pull requests #12, long title, and #9, draft). Steps 1, 2, 3, 4 (label, base line, header),
6, 8, 9 (unavailable), 10 and 11 pass; step 2 passed after the control was widened to 360 px.
Steps 4 (previous/next), 5 and 7 (keyboard) and the tooltip and setting-off parts of steps 2 and 9
were not drawn: they are covered by reducer tests (`features_changes`, `features_pr_changes`), the
component's gesture gate (`menu_select_fits`) and, since the close, the header gesture and hovered
tooltip tests in `session_area_fits` (T028).
