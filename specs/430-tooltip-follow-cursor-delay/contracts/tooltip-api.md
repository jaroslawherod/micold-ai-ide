# Contract: the shared tooltip's new surface

## `cdk::tooltip::Tooltip` / `material::Tooltip`

- `show_delay(self, delay: Duration) -> Self`: the panel opens only once the pointer has been over
  the trigger for `delay`, counted from entering; movement does not restart it; leaving cancels;
  a press on the trigger closes it until the pointer has left. Replaces any earlier `after_rest`.
- `after_rest(self, delay) -> Self`: unchanged; replaces any earlier `show_delay`.
- `Position::FollowCursor` (re-exported as `material::TooltipPosition::FollowCursor`): the panel sits
  beside the pointer and follows it while the pointer is over the trigger.
- No call: opens at once on hover, as today.

## Behaviour

1. Opening: no panel before the delay; the panel at the first observation at or after the deadline.
2. A `FollowCursor` panel opens at the pointer's current position, then moves with it.
3. `FollowCursor` panel: per axis after the pointer at `gap`, else before it, else the side with
   more room, then inside the window; it never sits under the pointer while either side has room.
4. The four fixed placements: unchanged, including the flip (029 FR-013).
5. Idle: while no tooltip is waiting or open, no frame is requested. While waiting, one timed wake
   at the deadline. While open and the pointer is still, nothing.
6. No pointer, a zero-size trigger, or an unknown window size: nothing opens.

## `micold_core::tooltip::ShowTimer`

`observe(over: bool, now: Instant, delay: Duration) -> Rest`, `press()`, `reset()`; transitions in
[data-model.md](../data-model.md).
