# Quickstart: Overview Card Focus and Expansion Motion

## Part A: automated

Run the gate: `scripts/gate.sh` in a cloud session, `mise run gate` locally. Expect green:

- the new fade tests in `ui/material/keyboard_focus.rs`, lift tests in `ui/material/card_lift.rs`,
  draw-order, shadow and **Reduce motion** tests in `ui/material/card_grid.rs`, and
  `eased_shadow` tests in `ui/material/style.rs` (plan, Test strategy);
- unchanged: the ripple (971), context menu (972), link focus (1122, 1123) and keyboard navigation
  (813, 1121) tests (SC-004), `tests/idle_requests_no_frames.rs` and `tests/motion_tokens.rs`.

## Part B: visual pass (`visual-pass` skill)

Reduce motion **off** unless stated. Capture frames at 60 fps where timing is checked.

1. **Showcase, Overview card section, live pose.** Tab onto the first card: the outline and tint
   fade in (part-way at ~75 ms, full by ~150 ms). Tab on: one fades out while the next fades in
   (US1-1, US1-2, [contract](contracts/ui-contract.md) C1).
2. Same pose: press *Expand*: the shadow grows from none over ~150 ms while the card is already at
   full height (FR-006 decision); *Collapse*: the shadow fades and is not cut by the cards to its
   right or below (US2-1, US2-2, contract C3).
3. Showcase static expanded poses open already lifted, with no fade on opening (research R8).
4. **Client**, a project with several locations, Overview open: rest the pointer on a card, then
   move straight to the next: one shadow fades while the other grows; no other card moves (US2-3,
   SC-005). Arrow keys across cards: each card fades on its own (Edge Cases, *Rapid moves*).
5. Focused, expanded card: Escape collapses it, the shadow fades and the focus outline stays
   (US1-3). Tab onto its issue link: the card's outline fades, the card stays expanded, the link's
   ring appears at once (US1-4, US4-3).
6. Menu key on a focused card: the card's outline fades out, the target mark appears at once; Escape
   in the menu: the outline fades back in (Clarifications).
7. Click a focused card: the ripple plays and no focus layer fades under it (US4-1, research R7).
8. Settings → Appearance → Reduce motion **on**, Save; repeat 4 to 6: every change is complete on the
   first frame (US3, SC-002).
9. Both schemes, all five card states at rest: focused, expanded and collapsed cards look as before
   (FR-010).
