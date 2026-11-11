# Research: 1792

## R1. How the structured selects get the Material 3 trigger
- **Decision**: extract the trigger `Select` draws (filled field, label, value, chevron, state layer, ripple) into one shared piece in `ui/material`, used by `Select` and by `MenuSelect`'s trigger. `MenuSelect` keeps its caller-owned openness, filter, chips and keyboard highlight.
- **Rationale**: `menu_select.rs` documents why `Select` cannot hold chips, a filter or caller-owned openness, so the model picker (filter field) cannot become a `Select`. Sharing the trigger gives FR-009 by construction: one definition. Principle VIII forbids a second look.
- **Rejected**: replacing the three with `Select` (loses filter, chips, caller-owned openness the `/effort` and model flows need); restyling `MenuSelect`'s `Button` trigger independently (two definitions drift, breaks FR-009).
- **To confirm in the tasks unit**: `Select` assembles its trigger inside `Widget::layout` (select.rs docs, BUG-001) because open state is in tree state; `MenuSelect` is given `open`, so its use is the simpler composition. The extraction takes `open` as an input.
- **Confirmed (M1, T001)**: `Select` now draws its trigger through `SelectTrigger` (`select_trigger.rs`), `open` and `focused` being builder inputs; `MenuSelect` passes the caller's `open` and wraps it in `Button::bare` (press, keyboard, ripple). Pieces extracted: container (`FormField`), value line, chevron, layers, indicator, ripple (turned off inside the bare button). Consequences recorded: the open chevron no longer flips (FR-009, as `Select`), and `MenuSelect`'s focus shows the button's keyboard ring, not the filled field's focus layer (no focus signal reaches the trigger).

## R2. The option menu
- **Decision**: no change. `MenuSelect` and `Select` both use `material::picker` (`animated_menu`, `row_surface`); a parity test asserts the same surface, item layout, selected emphasis and state layers.
- **Rejected**: new menu widget (duplicate); `MenuOverlay` (no keyboard highlight over disabled entries, per menu_select.rs R6).

## R3. Expand and collapse animation
- **Decision**: wrap the options row in `micold_material::animation::expand(row, options_open, over)`, `over` = `tokens::motion::duration::MEDIUM_2` (300 ms, the standard-set duration §6.3 gives the sidebar-sized expand and the ripple's expand), easing `expand`'s default `STANDARD_DECELERATE` in and `STANDARD_ACCELERATE` out. §6.3 has no accordion row, and its "no fifth animation" cap is feature 018's own count of animations it introduced (FR-035a/SC-010); this feature adds one use of the existing `Expand` wrapper with existing tokens and no new token, so no §6.3 amendment is made. Sidebar filter accordion is the precedent. The row is always built so the track persists; closed state is zero height. Reversal from the current height is the wrapper's track behaviour (`Progress`); a test pins it.
- **Per-session independence (FR-008)**: `options_open` is one app-wide flag today (`composers.options_open`), and the composer view is built per session, so each session's wrapper holds its own track in its own widget tree; nothing is shared. Tested with two sessions.
- **Rejected**: a per-session animation state in `features::composer` ticked by a subscription (duplicates `Progress`, adds persistence-adjacent state); `Fade` (spec asks for height growth); max-height tween in core (re-implements the wrapper).
- **Hidden controls**: the wrapper already skips `operate`, `update`, `mouse_interaction` and `draw` for its child at zero height (animation.rs), so a collapsed row is not tabbable or clickable; one test pins it. No `on_hidden` fallback needed.
- **Open picker on collapse**: `Expand::overlay` has no hidden guard, so an open model picker would outlive a collapse. Decision: toggling the row closed also closes any open model/effort/permission picker (existing `*Closed` messages); a test pins it.

## R4. Reduce motion
- **Decision**: read `state.settings.reduce_motion` inside `composer::view` (it takes the app `State`, as `drawer_flags` does) and use `Duration::ZERO` for `over` (`expand_clipping.rs` already uses `expand(.., Duration::ZERO)`). OS preference not read (spec Out of Scope).
- **Rejected**: reading the OS preference (spec); skipping the wrapper (changes tree shape, state loss on toggling the setting).

## R5. Small window (FR-011)
- **Decision**: the row already wraps (`Row::wrap`); `expand` animates its own laid-out height, which is the wrapped height, so the end height is the reachable one. A geometry test at a narrow width checks the end height equals the row's wrapped height.
