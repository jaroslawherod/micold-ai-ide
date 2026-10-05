# Data model: Tooltip follows the cursor and waits before showing

No persisted data (Principle IV). In-memory only.

## `Wait` (client, `cdk::tooltip`, `pub(crate)` so `material::Tooltip` holds it too)

| Variant | Meaning | Rule |
|---|---|---|
| `Hover` | default: open while the pointer is over the trigger | `cursor.is_over(bounds)`, unchanged |
| `Delay(Duration)` | open after the pointer has been over the trigger for the delay | `ShowTimer` |
| `Rest(Duration)` | open after the pointer rests for the delay | `RestTimer`, unchanged |

Replaces `rest: Option<Duration>`; `after_rest` sets `Rest`, `show_delay` sets `Delay`, last call wins.

## `ShowTimer` (core, `micold_core::tooltip`)

States: `Away` (default), `Waiting { since }`, `Open`, `Spent`.

| From | Input | To |
|---|---|---|
| any | pointer off the trigger | `Away` |
| `Away` | pointer on, delay is zero | `Open` |
| `Away` | pointer on | `Waiting { since: now }` |
| `Waiting` | pointer on, `now - since >= delay` | `Open` |
| `Waiting` | pointer on, earlier | `Waiting` (movement never restarts it) |
| `Open`, `Spent` | pointer on | unchanged |
| any | `press()` | `Spent` |
| any | `reset()` (subject changed) | `Away` |

`observe(over: bool, now, delay) -> Rest { open, wake_at }`; `wake_at` is `since + delay` only
while `Waiting`.

## `State` additions (client)

| Field | Meaning |
|---|---|
| `show: ShowTimer` | the delay's state; unused unless `Wait::Delay` or follow-with-hover |
| `pointer: Option<Point>` | the pointer over the trigger, in the trigger's own coordinate space; read by `overlay()` |

`describe(subject)` also resets `show`.

## `Position` addition

`FollowCursor`: placed by `place_at_pointer`, not `place`.
