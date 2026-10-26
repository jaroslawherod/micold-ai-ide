# Research: Compact session header

## R1. Where the switch lives
- **Decision**: inside `status_cluster`, between the status words and restart.
- **Why**: restart is already the cluster's last child (1126 R9) and the fixed controls follow; adding the slot inside the cluster keeps FR-005 without touching add/split controls.
- **Rejected**: a new child of the strip row between cluster and add control (restart would no longer be last of the cluster); a header-row placement (issue asks for the strip); putting it into `StripModel` (view-only derivation from `State` suffices, no data added).

## R2. Compact Changes button
- **Decision** (amended in M1): `material::Button::padding` cannot shrink a button, whose height is fixed at 40 dp; a new `Button::density(step)` (default standard) lets `changes_button` use `density::DENSE` (28 dp).
- **Why**: Principle VIII, one button, no fork.
- **Rejected**: a bespoke container-styled button (fork); shrinking the font (changes look).

## R3. Header as one row, title Fill
- **Decision**: `row![title Fill, chips, changes]`.
- **Why**: Fill takes what remains, so the title is the only element that shortens first (FR-002); the 1126 M3 finding (button beside two lines starved the chips) no longer applies with one line, but the gate still checks 640x480.
- **Rejected**: wrapping chips below at narrow widths (varying child list, breaks FR-001 and 1126 FR-011).

## R4. Single-pane versus tiled
- **Decision**: single pane draws no row from `terminal::pane`; tiled panes keep `terminal_switch_strip`.
- **Why**: FR-007/SC-004; the tiled call sits in `panes.rs` and needs no change.
- **Rejected**: moving the tiled rows too (out of scope).

## R5. Narrow strip and the caption
- **Decision**: the caption is an `Ellipsized` with a `max_width`; its exact cap is set by the 640x480 gate with the longest held-queue caption (`held_queue_line`).
- **Why**: FR-012/SC-006; tabs scroll in a Fill region and may shrink to nothing, so the caption must give way before the buttons.
- **Rejected**: letting the caption Fill (starves the tabs); dropping the caption below a width (varying child list).

## R6. Absent switch keeps the child list
- **Decision**: absence is an empty `container(Space)` in a fixed slot.
- **Why**: a bare `Space` is dropped from the tree (the existing comment in `terminal_switch_strip`), moving later siblings; FR-008.
- **Rejected**: omitting the child.

## R7. Tab width
- **Decision**: unchanged (spec Clarifications, FR-011).
