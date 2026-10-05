# Research: Tooltip follows the cursor and waits before showing

## R1 — What `after_rest` plus `show_delay` means (the question clarify left open)

**Decision**: they are alternatives of one mode, `Wait::{Hover, Delay, Rest}`. Each setter replaces
the whole mode, so the one called last wins. Documented on both setters and tested both ways.

**Rejected**:
- *Both must elapse* (open only after the delay and a rest): a third behaviour nothing needs, with
  its own timing rule and tests, for a combination no caller has (spec Out of Scope).
- *Panic or ignore the second call*: hides a bug until runtime; call order is the one thing a
  builder chain already expresses.
- *Two `Option` fields plus a precedence rule*: keeps an unrepresentable state representable.

## R2 — Where the delay rule lives

**Decision**: `micold_core::tooltip::ShowTimer`, a sibling of `RestTimer` returning the same
`Rest { open, wake_at }`. Core is render-free and `RestTimer` already sits there with its table test.

**Rejected**: inline in the widget (untestable without a driver, against the repo's rule that
decisions are render-free); a flag on `RestTimer` (its anchor and tolerance are the rest rule; the
delay has neither, so a flag makes both harder to read).

## R3 — Pointer placement

**Decision**: `place_at_pointer(pointer, content, window, gap) -> Rectangle`. Per axis, the panel
goes after the pointer (right, below) at `gap`; when it would pass the window's edge it goes before
(left, above); when it fits neither, the side with more room. The result is kept inside the window
by the existing `inside`. A final check that the visible panel (inside `EDGE_PADDING`) does not
contain the pointer, using the same visible-area idea as `covers`. A window too small to avoid the
pointer takes the clamp: stays as far inside as it fits (spec Edge Cases).

Offset and padding reuse `gap` and `EDGE_PADDING` (spec Assumptions); both are logical units.

**Rejected**: reusing `place` with a zero-size trigger (it centres on the trigger, so the panel
would sit under the pointer on one axis); one-axis flip only (a corner pointer would run past the
other edge).

## R4 — Getting the pointer to the overlay

**Decision**: `State` gains `pointer: Option<Point>`, written in `update` from
`cursor.position_over(layout.bounds())` on mouse events and redraws, and read by `overlay()`,
which adds `translation` as it already does for the trigger. While open, a changed pointer calls
`shell.invalidate_layout()`; an unchanged one does nothing, so nothing repaints (FR-007).

**Rejected**: asking iced for the cursor inside `Overlay::layout` (the overlay trait's `layout`
receives only the bounds); a timer that polls the pointer (continuous redraw, against FR-007).

## R5 — Keeping the four fixed placements exactly as they are

**Decision**: `Wait::Hover` keeps the `cursor.is_over(bounds)` branch verbatim. Only a
`FollowCursor` tooltip with `Wait::Hover` is promoted to the delay rule with a zero delay, to get
the press-closes rule the spec's edge case names without touching the fixed placements.

**Rejected**: giving every tooltip the press rule (changes shipping behaviour, FR-006).

## R6 — Showcase and docs

**Decision**: two showcase poses (a pointer-following tooltip over a large trigger; a show-delay
tooltip) are the demonstration (spec Assumptions); no user-guide change; the PR takes
`docs-not-needed`.
