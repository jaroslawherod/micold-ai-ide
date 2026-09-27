# Cycle log — 029-worktree-tooltip-details, BUG-001 (milestone M1)

Append only. One entry per red-green-refactor cycle, with the red evidence verbatim.

Commands are the `.specify/memory/tdd-profile.md` rust stack's, through `scripts/build-lock.sh`.
The full-workspace suite runs once at the end as `mise run gate` (T035). **Baseline**: CI's `CI` run
on `origin/main` 23a0e0ee is `success`; this branch's HEAD 4afd673d adds only spec documents on top.

## B1, B2, B3 — the gate, written before the component changed

- **Tests**: `crates/micold-client/tests/gates/tooltip_clears_its_row.rs`, registered in
  `tests/layout_snapshot.rs` beside `context_menu_anchor`, whose fixture helpers (`worktree`,
  `record_every_worktree`, `sidebar_row`, `PROJECT`) were made `pub(crate)` for reuse. The gate
  searches for the longest worktree list whose last row is still fully inside the window, dispatches
  a real `CursorMoved` over the row into a retained tree, and reads the tooltip's overlay record.
- **Red** (the tree's source was `origin/main`'s: HEAD differs from it only in `specs/`):
  `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot tooltip_clears_its_row`

  ```
  test tooltip_clears_its_row::the_last_row_keeps_its_tooltip_off_itself_in_the_smallest_window ... FAILED
  test tooltip_clears_its_row::a_row_with_room_below_gets_its_tooltip_below_it ... ok
  test tooltip_clears_its_row::the_last_row_keeps_its_tooltip_off_itself ... FAILED
  in a 640×480 window, the last worktree row's tooltip covers the row it describes: row 4,419 244×60 (y 419–479), tooltip 21,398 210×82 (y 398–480). …
  in a 1280×800 window, the last worktree row's tooltip covers the row it describes: row 4,739 244×60 (y 739–799), tooltip 21,718 210×82 (y 718–800). …
  test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 39 filtered out; finished in 0.19s
  ```

  B1 and B2 fail for BUG-001's reason: the panel is clamped to end at the window's bottom edge
  (y …–800 / …–480), which is on top of the row.
- **B3 passed first run** (it pins unchanged behaviour), so the deliberate-mutant check was applied:
  `material::Tooltip::new`'s default `Position::Bottom` → `Top`, then
  `cargo test … tooltip_clears_its_row::a_row_with_room_below_gets_its_tooltip_below_it`:

  ```
  the first worktree row has the whole list below it, so its tooltip opens below it: row 4,163 244×60 (y 163–223), tooltip 21,77 210×82 (y 77–159) (FR-010, FR-013)
  test result: FAILED. 0 passed; 1 failed; …
  ```

  Restored with `git checkout` on that file.
- **Green**: new `crates/micold-client/src/ui/cdk/tooltip.rs` — its own `Widget::overlay()` with a
  `place()` that keeps the stack's geometry (5px edge padding, centred along the trigger, `gap`
  away) and, when the asked-for side kept inside the window overlaps the trigger, uses the opposite
  side. `material::Tooltip` builds on it with its public API unchanged (`TooltipPosition` is the same
  re-exported type). `one_overlay_implementation.rs`: the stale `SANCTIONED` `ui/material/mod.rs`
  `tooltip` entry struck, an argued `CDK_OVERLAY_IMPLEMENTORS` entry for `ui/cdk/tooltip.rs` added.
  `cargo test -p micold-client --test layout_snapshot --test one_overlay_implementation --test cdk_no_appearance`:
  42 + 8 + 3 passed, 0 failed (the layout snapshot fixture unchanged).
- **Refactor**: none needed; the module doc of `one_overlay_implementation.rs` was corrected to say
  no stack delegations remain.
- **Notes**: B1–B3 were written together in one file before any implementation (T031 and T032 name
  the same file); B2 is a second window size of B1, and both were seen failing.

## B4 — a panel that fits neither side takes the side with more room

- **Test**: `crates/micold-client/src/ui/cdk/tooltip.rs::placement_tests::a_panel_that_fits_neither_side_takes_the_side_with_more_room`
- **Red**: `scripts/build-lock.sh cargo test -p micold-client --lib placement_tests`

  ```
  assertion `left == right` failed: asked for the top, which has 70px of room to the bottom's 20, so the panel stays at the top of the window rather than flipping to the side where it would cover the whole trigger: got Rectangle { x: 105.0, y: 60.0, width: 90.0, height: 90.0 }
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 470 filtered out
  ```

- **Green**: `place()` keeps the flipped side only when it clears the trigger or has more `room()`.
  1 passed.
- **Refactor**: none needed.

## B5 — the flip on the horizontal axis

- **Test**: `…::placement_tests::a_panel_with_no_room_on_the_right_opens_on_the_left`
- **Passed first run** (B1's `opposite()` already covers both axes), so the deliberate-mutant check:
  `opposite(Right)` → `Some(Right)`:

  ```
  no room to the right of a trigger at the window's right edge, so the panel opens to its left, clear of it: trigger Rectangle { x: 340.0, y: 100.0, width: 60.0, height: 40.0 }, panel Rectangle { x: 270.0, y: 100.0, width: 130.0, height: 40.0 }
  test result: FAILED. 0 passed; 1 failed; …
  ```

  Restored; 2 passed.
- **Notes**: B1–B5 go into one commit. The loop was not committed between cycles, since B4's red was
  written before B1–B3's green was committed; the evidence above is per cycle.

## Review round 1 — refactors and fixes on green

Not new behaviours; each change was re-run green, and the gate was re-proved against a mutant.

- **`mise run gate` failure** (`idle_requests_no_frames`): the first cut called `shell.request_redraw()`
  directly, which the rendering layer reserves to `cdk::motion::Progress`. The panel's open/close now
  drives a zero-duration `Progress`, which asks for exactly the one frame that paints the change.
- **Review A**: the flip decision now tests the panel's *visible* surface (inside its 5px transparent
  margin), not its padded box, so a panel that visibly fits is not flipped; `cdk::tooltip::Position`
  is its own four-sided enum (no caller used the stack's follow-the-cursor variant, which has no side
  to flip to); the gate reuses `context_menu_anchor::with_worktrees` instead of a copy of its
  fixture, measures the visible surface, fails if one hover opens two panels, requires the hover
  point to be inside the list's clip, and gives B3 a 3-row list instead of the full search.
- **Mutant re-check of the refactored gate**: `place()` forced to return the asked-for side:

  ```
  in a 640×480 window, the last worktree row's tooltip covers the row it describes: row 4,419 244×60 (y 419–479), tooltip 26,403 200×72 (y 403–475). …
  in a 1280×800 window, the last worktree row's tooltip covers the row it describes: row 4,739 244×60 (y 739–799), tooltip 26,723 200×72 (y 723–795). …
  test result: FAILED. 1 passed; 2 failed; …
  ```

  Restored; `layout_snapshot` 42 passed, `placement_tests` 2 passed, `idle_requests_no_frames` 8,
  `one_overlay_implementation` 8, `cdk_no_appearance` 3 passed.
