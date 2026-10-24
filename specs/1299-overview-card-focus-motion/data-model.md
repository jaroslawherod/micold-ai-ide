# Data model: Overview Card Focus and Expansion Motion

Nothing here is persisted or sent anywhere. Both entities are widget tree state, created and
dropped with their widget.

## Focus fade (per focusable control, `keyboard_focus::Focus`, changed)

| Field | Type | Meaning |
|---|---|---|
| `fade` | `cdk::motion::Progress` (curve `STANDARD`) | how far the focus indication is drawn, `0.0` none to `1.0` full |

- Target: `1.0` while `focused && visible` (focus held and shown), else `0.0`.
- Created at `0.0`, or `1.0` when a wrapper with `on_focus_change` is built already focused and
  shown. Overview cards wire only `on_shown_change`, so theirs start at `0.0` and focus given back
  by traversal after reopening fades in (FR-001).
- Reversal (FR-004): `on_frame` retargets inside the call when the target changes (`from = value`,
  `t = 0`).
- Stepped in `update` with `on_frame` over the control's `fade` duration; with no duration it
  `restart_at(target)` (instant).
- A pointer press on the control that hides its shown focus: `restart_at(0.0)`.
- Drawn strength: `target` when instant, else `fade.value()`; layer opacity = today's top-up ×
  strength, ring alpha = strength.

## `TakesTheKeyboard` (changed)

| Field | Type | Meaning |
|---|---|---|
| `fade` | `Option<Duration>` | `None` (default): instant, as today. `Some(d)`: the focus fades over `d` |

## Lift (per lifted or settling card, `card_lift::Lifts`, new; held in the grid's `ZoomState`)

| Field | Type | Meaning |
|---|---|---|
| entries | `Vec<(SessionLocation, Progress)>` | one per card whose shadow is not at rest at none |

- Invariant: no entry rests at `0.0` (it is removed), and at most one entry targets `1.0` (the
  expanded card, feature 1121).
- Transitions per location: *absent* → `expanded` → entry at `0.0`, easing to `1.0`;
  entry → not expanded → easing to `0.0` → removed on arrival; entry → location gone → removed;
  any → `instant` → snapped to its target.
- Seeded at `1.0` for the card expanded when the grid's state is created.

## `Grid` / `CardGrid` (changed)

| Field | Type | Meaning |
|---|---|---|
| `instant` | `bool` | *Reduce motion*: transitions end on the first frame; passed to every card |
| `roles` (Grid) | `Roles` | to draw the shadow quad |

## `OverviewCard` (changed)

| Field | Type | Meaning |
|---|---|---|
| `instant` | `bool` | the card's focus does not fade |
| `own_shadow` | `bool` (default `true`) | whether the expanded card's fill casts the menu shadow; `false` inside a grid |
