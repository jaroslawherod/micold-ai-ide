# Data model: Review tab fits the smallest window

## Compact flag (derived, per window)

`window::State::compact()` = `window_size.0 != 0 && window_size.0 < COMPACT_WIDTH` (720). Not
stored; `(0, 0)` is "not reported" and not compact.

## Compact override (transient, per window)

`sidebar::State.compact_open: bool`, default `false`. Meaning: the user expanded the sidebar while
the window was compact. Set by a toggle at compact width. Cleared by `window::resized` when the new
width is not compact. Never serialised.

## Effective sidebar state (derived)

`app::State::sidebar_hidden()`:

| compact | result |
|---|---|
| no | `sidebar.hidden` |
| yes | `!sidebar.compact_open` |

Full screen (`overview.full_screen()`) retracts the drawer on top and is independent of it.

## Unchanged

`sidebar.hidden` (the user's choice) and `sidebar.width`: no resize writes them (SC-003).

## State transitions

| Event | compact before | after | `compact_open` | `hidden` |
|---|---|---|---|---|
| resize to < 720 | no | yes | `false` | unchanged |
| resize within < 720 | yes | yes | unchanged | unchanged |
| resize to >= 720 | any | no | cleared | unchanged |
| toggle | yes | yes | flipped | unchanged |
| toggle | no | no | unchanged | flipped |
