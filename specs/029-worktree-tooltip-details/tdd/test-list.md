# Test list — 029-worktree-tooltip-details, BUG-001 (milestone M1)

029 predates the `tdd` extension, so it has no feature-wide test list. This one covers **BUG-001's
milestone M1 only** (T031–T036), so the `tdd.run` loop that `speckit-implement`'s
`before_implement` hook drives has the list it requires. The behaviours are the ones T031 and T032
specify, traced to FR-013 and SC-006 (and SC-005 for staying inside the window).

B1–B3 are geometry-gate cases in
`crates/micold-client/tests/gates/tooltip_clears_its_row.rs`, compiled into the `layout_snapshot`
binary; B4–B5 are unit tests of `place()` in `ui/cdk/tooltip.rs`. The flip itself is layout glue inside `ui/cdk/tooltip.rs`, covered by these cases and by
`quickstart.md` §B7 under the visual-pass skill (plan.md § Bugfix BUG-001, *Test layer*).

| ID | Behaviour | Traces to | Task | State | Test |
|---|---|---|---|---|---|
| B1 | Hovering the last row of the fullest unscrolled list in the harness window (1280×800) opens a tooltip that does not intersect that row, inside the window | FR-013, SC-006, SC-005 | T031, T033 | DONE | `crates/micold-client/tests/gates/tooltip_clears_its_row.rs::the_last_row_keeps_its_tooltip_off_itself` |
| B2 | The same in the smallest window (640×480) | FR-013, SC-006 | T032, T033 | DONE | `crates/micold-client/tests/gates/tooltip_clears_its_row.rs::the_last_row_keeps_its_tooltip_off_itself_in_the_smallest_window` |
| B3 | A row with room below it still gets its tooltip below it (unchanged behaviour; characterization) | FR-010, FR-013 | T032 | DONE | `crates/micold-client/tests/gates/tooltip_clears_its_row.rs::a_row_with_room_below_gets_its_tooltip_below_it` |
| B4 | A panel that fits neither side of its trigger takes the side with more room (added mid-loop) | FR-013, plan § Bugfix BUG-001 fallback clause | T033 | DONE | `crates/micold-client/src/ui/cdk/tooltip.rs::placement_tests::a_panel_that_fits_neither_side_takes_the_side_with_more_room` |
| B5 | The flip works on the horizontal axis too: `Right` with no room opens `Left` (added mid-loop) | FR-013, plan § Bugfix BUG-001 "every tooltip" | T033 | DONE | `…::placement_tests::a_panel_with_no_room_on_the_right_opens_on_the_left` |

B4 and B5 were added while writing `cdk/tooltip.rs`: the plan's rule has two clauses the gates cannot
reach (neither side fits; the horizontal axis), since every sidebar row has room above it.
