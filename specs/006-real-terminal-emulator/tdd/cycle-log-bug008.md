# Cycle Log: BUG-007 — a single click in the terminal selects nothing

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: CI (`ci complete`, full workspace on Linux plus the three-platform client run) on
  `origin/main` `226d3a8b` -> `success`. The local full suite was not run before the first test;
  `mise run gate` runs it on the finished change.
- commit: `226d3a8b`
- recorded: cycle 0, before any change

## Red: T074, the regression tests (one Red commit)

Deviation, recorded up front: this repository commits a task's failing tests as one Red commit and
the fix as a separate Green commit (tasks.md Phase 14 purpose; feature 029's `test(029): … (Red)`
commits). The five behaviours that fail on `origin/main` were therefore written and observed red
together, before any implementation existed, and share one Green change (T075). The one declaration
the pane tests need to compile, `SessionMsg::TerminalSelectionReleased` (a variant with a no-op arm
and no sender), was added with the tests.

- U1 `crates/micold-client/src/selection.rs::tests::a_click_without_a_drag_selects_nothing` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib selection::tests`
    -> `panicked at crates/micold-client/src/selection.rs:427:9: a click is not a drag: no cell may be highlighted (FR-013e)` (11 passed; 1 failed)
- U2 `selection.rs::tests::an_update_onto_the_start_anchor_selects_that_cell` (new) — passes on
  `origin/main`, as planned (`example (mutant)`); its red is taken against a mutant after T075.
- U3 `selection.rs::tests::a_drag_out_and_back_selects_the_pressed_cell` (new) — passes on
  `origin/main`, as planned; red against a mutant after T075.
- U4 covered by existing tests `selection.rs::tests::word_expansion_selects_whole_word` and
  `line_granularity_selects_whole_line` (both assert a `Word`/`Line` selection started with no
  update selects the word and the line, and both pass). No new test.
- U5 `crates/micold-client/tests/clipboard_request.rs::a_click_without_a_drag_asks_for_nothing` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --test clipboard_request`
    -> `left: Some(ClipboardWrite("w"))  right: None` (14 passed; 1 failed)
- U6 `crates/micold-client/src/ui/material/terminal_pane.rs::tests::clipboard_gestures::pointer_jitter_inside_the_pressed_cell_is_not_a_drag` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures`
    -> `left: [(6, 0), (6, 0)]  right: []` (7 passed; 3 failed)
- U7 `clipboard_gestures::motion_into_another_cell_and_back_extends_the_selection_each_time` (new)
  — passes on `origin/main`, as planned; red against a mutant after T075.
- U8 `clipboard_gestures::a_tap_over_a_held_selection_writes_nothing_to_the_clipboard` (new)
  - red: same run -> `a press and release delivered together copied ["hello world"] — the selection from before the press — over the user's clipboard (FR-013e, BUG-007)`
- U9 `clipboard_gestures::a_release_asks_for_the_copy_after_its_press_starts_the_selection` (new)
  - red: same run -> `… start at Some(0), release request at None`

## Green: T075, the fix (one Green commit)

- change: `Selection` gains `extended`, false from `start` and set by any `update`; a `Char`
  selection that was never extended contains no cell and yields empty text. The pane records the
  pressed cell and publishes no `TerminalSelectUpdate` while the pointer stays in it; once it has
  left, every cell publishes. The left release no longer writes `self.selection` to the clipboard: it
  publishes the new `SessionMsg::TerminalSelectionReleased`, and `main.rs` hands that to
  `shell::clipboard::on_selection_released`, which copies the shell's current selection through
  `selection_copy_request` and leaves the context menu alone.
- green: `scripts/build-lock.sh cargo test -p micold-client` -> 1696 passed; 0 failed (every
  target of the client crate, the five reds above included). `mise run gate` runs the workspace
  before the push.
- U1, U5, U6, U8, U9: red above, green in this run.
- U4: already covered, unchanged and green.
- Deliberate mutants for the three behaviours that passed on `origin/main`, each applied alone to
  the fixed code, run, and reverted by copying the file back (byte-compared):
  - U2, mutant `self.extended = anchor != self.start;` in `Selection::update`:
    `scripts/build-lock.sh cargo test -p micold-client --lib selection::tests::an_update_onto_the_start_anchor_selects_that_cell`
    -> `panicked at crates/micold-client/src/selection.rs:462:9` (the "a drag that ends on its start
    cell selects that cell" assertion) (0 passed; 1 failed)
  - U3, mutant `is_empty` also true while `self.current == self.start`:
    `… --lib selection::tests::a_drag_out_and_back_selects_the_pressed_cell`
    -> `panicked at crates/micold-client/src/selection.rs:475:9` (the "returning to the pressed cell
    must not turn a drag back into a click" assertion) (0 passed; 1 failed)
  - U7, mutant `if true {` in place of the pressed-cell check, so the pane never publishes an update:
    `… --lib clipboard_gestures::motion_into_another_cell_and_back_extends_the_selection_each_time`
    -> `left: []  right: [(7, 0), (6, 0)]` (0 passed; 1 failed)
- refactor: none. The copy chord still uses `selectable_content`, so nothing became dead. The
  `SessionMsg` doc comments' arm and transition counts, already stale on `origin/main`, were
  corrected alongside the new variant's handling.
- commit: the `fix(006)` Green commit that carries this entry (the SHA is in `git log`; a commit
  cannot name its own hash).

## Acceptance: A1, the visual pass (T076)

- A1 [evidence/bugfix-007-pass-2026-09-14.md](../evidence/bugfix-007-pass-2026-09-14.md). This is not
  an automated test: the profile's acceptance runner has no GUI. On `origin/main` `226d3a8b`, a
  synthetic click and a click at human pace each left the `w` highlighted, and the slow one pasted
  `w` (red). On this branch's build, neither click leaves a highlight, the dragged `hello` is pasted,
  and a synthetic click after a drag leaves another client's `OTHER-TEXT` on the clipboard (green).
- commit: the `docs(006)` T076 commit that carries this entry.

## Red: U10 and U11, found by code review A (one Red commit)

Review A (the `code-review` skill at `high`) reported two ways the pane's pressed-cell check (U6)
treats a drag as a click. First, a wheel scroll while the button is held puts other text under an
unmoved pointer. Second, a position outside the pane clamps back onto a pressed edge cell. Both
were added to the list as behaviours and written as tests before any change.

- U10 `terminal_pane.rs::tests::clipboard_gestures::motion_after_a_scroll_while_held_is_a_drag_even_in_the_pressed_screen_cell` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures`
    -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1751:13` `left: []  right: [(6, 0)]` (10 passed; 2 failed)
- U11 `clipboard_gestures::motion_past_the_panes_edge_is_a_drag_even_where_it_clamps_to_the_pressed_cell` (new)
  - red: same run -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1775:13` `left: []  right: [(0, 0)]`

## Green: U10 and U11

- change: the pane's wheel `ScrollLocally` arm clears the pressed cell. The `CursorMoved` arm treats
  a position outside the content area as having left the pressed cell.
- green: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures` -> 12 passed;
  0 failed. `mise run gate` runs the full suite before the push.
- refactor: none. With the same commit, and at Review B's request, one comment in the copy-chord arm
  now points at `selection::copy_request` instead of the pane's old release write.
- commit: the `fix(006)` commit that carries this entry.

## Red: U12 and U13, found by code reviews A and B, round 2 (one Red commit)

Round 2 of both reviews found that the U11 fix treats every position outside the character area
as having left the pressed cell. A press in the pane's focus gutter, which the pane still accepts
and `grid_at` clamps onto the edge cell, therefore became a drag on the first jitter. Review A also
found that the U10 fix clears the pressed cell on a wheel turn that cannot move the view. Both were
added to the list and written as tests before any change.

- test fixture (a stated test change, taken before the implementation): U10 now runs over
  `grid_with_history(0, 10)`, because its premise is a wheel turn that moves the view. On a grid with
  no scrollback that turn moves nothing, which is U13. `grid(mode)` delegates to the new helper
  unchanged. U10 still passes on the unchanged code.
- U12 `terminal_pane.rs::tests::clipboard_gestures::jitter_in_the_focus_gutter_beside_the_pressed_edge_cell_is_not_a_drag` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures`
    -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1812:13` `left: [(0, 0), (0, 0)]  right: []` (12 passed; 2 failed)
- U13 `clipboard_gestures::a_wheel_turn_that_cannot_scroll_leaves_jitter_a_click` (new)
  - red: same run -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1839:13` `left: [(6, 0)]  right: []`

