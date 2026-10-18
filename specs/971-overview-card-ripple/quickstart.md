# Quickstart: Overview Card Ripple

## Part A: automated
`mise run test-core` is not enough (client crate): run `mise run gate` (cloud: `scripts/gate.sh`).
Expect green: new ripple, overview_card and overview-state tests, plus the unchanged feature 813 tests (SC-006).

## Part B: visual pass (use the `visual-pass` skill)
1. Component showcase, Overview card section: press near each corner of a card and hold → circle centred at the press, clipped to the rounded corners, fades after release (US1).
2. Focus a card (Tab), press Enter then Space → centred ripple each time; a card that is not focused shows none (US2).
3. Check each of the five states for visibility (US1-4).
4. Client with a project: open the Overview (Ctrl+Shift+O), click a card quickly → the ripple is seen, then the main session shows ~300 ms later; repeat with Enter. A second click or Escape during the delay changes nothing (US3).
