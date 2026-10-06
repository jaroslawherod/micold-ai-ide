# TDD cycle log: 430 tooltip follows the cursor and waits before showing

Honest record. M1 and M2 wrote tests and source together; only T011 was seen red during the milestones.
No red run exists for the other behaviors (`TEST_AFTER`, see `verification.md`); this cannot be recovered.

## Close-unit remediation (tests added after the code, strengthened by mutation)

| Task | Behavior | Red evidence |
|---|---|---|
| T020 | A pointer move while the follow panel is open requests a relayout; a still pointer does not | Mutant `if false && state.open && rest.open` at `crates/micold-client/src/ui/cdk/tooltip.rs:284` made `a_pointer_move_while_open_moves_the_panel` fail: `tooltip_show_glue.rs:58: the move asks the runtime to lay the panel out again` (14 passed, 1 failed). Restored. |
| T022 | Glue test shares `visible_part` and `EDGE_PADDING` with the library | refactor, tests stayed green |
| T023 | Leaving cancels the delay for a follow panel; `after_rest` set after `show_delay` restarts on movement | no red recorded (test added after code) |
| T024 | `ShowTimer` with a delay too long for the clock (`Duration::MAX`) waits without a wake | no red recorded (test added after code) |
