# Quickstart: Review UI — reviewed marks and progress (#1271)

## Part A: automated

`mise run test-core` (marks, ids, whole-change read, store, protocol), then `mise run gate`. The
acceptance scenarios map to the tests in plan.md § Test strategy; none needs a window, a running
agent or the network. The geometry of the states below is pinned by the layout snapshot state
`changes-review-marks` (plan.md § Test strategy).

## Part B: visual pass (`visual-pass` skill), 1280×800, light and dark

Prerequisites: a worktree with a base and four changed files: `src/a.rs` and `src/b.rs`
(committed), `src/c.rs` (committed and uncommitted), `docs/d.md` (uncommitted); one Draft comment
on `src/b.rs`; a second window on the same project; one pull request listed for the branch (the
showcase fixture list, or `gh` signed in).

1. Open the entry's review. Every row shows ○, `src/b.rs` shows ● with its draft count; the
   header reads *0 of 4 reviewed* with an empty bar.
2. Select `src/a.rs`, press **Mark reviewed**: the row shows ✓, the chip reads **Reviewed**
   (pressed), the header *1 of 4 reviewed*. The second window shows the same within 2 seconds.
3. Mark the other three: *4 of 4 reviewed*, full bar; `src/b.rs` shows ✓ and keeps its count.
   Unmark `src/b.rs`: ● again, *3 of 4*.
4. Turn Uncommitted off and select `src/c.rs`: **Reviewed** stays pressable; unmark it, and
   **Mark reviewed** is unavailable with the tooltip about Committed and Uncommitted (D4).
   `src/a.rs` can still be marked. Turn Uncommitted on again.
5. Edit one line of `src/a.rs` on disk: within the watch re-read its row shows ○ with the changed
   dot (tooltip says it changed since marked), *n* goes down. Undo the edit: ✓ again.
6. Commit `docs/d.md` unaltered: it stays ✓.
7. Show the pull request: its own *0 of m reviewed*; mark a file there, return to the working tree:
   unaffected.
8. Stop the session service: the chip is unavailable with the "not connected" tooltip, rows show
   ○ / ● only, no progress, and the header's other controls have not moved.
9. Restart the app: the marks are back as in step 7.

Record light and dark screenshots of steps 1, 3 (partial and full bar), 4 and 5 in `visual-pass/`.
Check no header control overlaps or is truncated at 1280×800 (SC-005).
