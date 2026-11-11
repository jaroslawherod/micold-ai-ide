# Research: 1792

## R1. How the structured selects get the Material 3 trigger
- **Decision**: extract the trigger `Select` draws (filled field, label, value, chevron, state layer, ripple) into one shared piece in `ui/material`, used by `Select` and by `MenuSelect`'s trigger. `MenuSelect` keeps its caller-owned openness, filter, chips and keyboard highlight.
- **Rationale**: `menu_select.rs` documents why `Select` cannot hold chips, a filter or caller-owned openness, so the model picker (filter field) cannot become a `Select`. Sharing the trigger gives FR-009 by construction: one definition. Principle VIII forbids a second look.
- **Rejected**: replacing the three with `Select` (loses filter, chips, caller-owned openness the `/effort` and model flows need); restyling `MenuSelect`'s `Button` trigger independently (two definitions drift, breaks FR-009).
- **To confirm in the tasks unit**: `Select` assembles its trigger inside `Widget::layout` (select.rs docs, BUG-001) because open state is in tree state; `MenuSelect` is given `open`, so its use is the simpler composition. The extraction takes `open` as an input.

## R2. The option menu
- **Decision**: no change. `MenuSelect` and `Select` both use `material::picker` (`animated_menu`, `row_surface`); a parity test asserts the same surface, item layout, selected emphasis and state layers.
- **Rejected**: new menu widget (duplicate); `MenuOverlay` (no keyboard highlight over disabled entries, per menu_select.rs R6).

## R3. Expand and collapse animation
- **Decision**: wrap the options row in `micold_material::animation::expand(row, options_open, over)`, `over` = a `tokens::motion::duration` standard-set constant (MEDIUM_1 or the value contract §6.3 assigns to expand; the tasks unit picks from §6.3, not a literal). Easing: `expand`'s default standard pair. The row is always built so the track persists; closed state is zero height. Reversal from the current height is the wrapper's track behaviour (`Progress`); a test pins it.
- **Per-session independence (FR-008)**: `options_open` is one app-wide flag today (`composers.options_open`), and the composer view is built per session, so each session's wrapper holds its own track in its own widget tree; nothing is shared. Tested with two sessions.
- **Rejected**: a per-session animation state in `features::composer` ticked by a subscription (duplicates `Progress`, adds persistence-adjacent state); `Fade` (spec asks for height growth); max-height tween in core (re-implements the wrapper).
- **Risk**: a zero-height `expand` must not keep the controls focusable or in the tab order, and must not keep the model picker's text-input focus target alive. Confirm in the first test; if it does, build the row only when open or animating, using `on_hidden` to drop it.

## R4. Reduce motion
- **Decision**: pass `settings.reduce_motion` into the composer view as `DrawerFlags.instant` is, and use `Duration::ZERO` for `over` (`expand_clipping.rs` already uses `expand(.., Duration::ZERO)`). OS preference not read (spec Out of Scope).
- **Rejected**: reading the OS preference (spec); skipping the wrapper (changes tree shape, state loss on toggling the setting).

## R5. Small window (FR-011)
- **Decision**: the row already wraps (`Row::wrap`); `expand` animates its own laid-out height, which is the wrapped height, so the end height is the reachable one. A geometry test at a narrow width checks the end height equals the row's wrapped height.
