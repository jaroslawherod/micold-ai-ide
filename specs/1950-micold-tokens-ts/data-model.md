# Data model: the token module

| Export | Shape | Source |
|---|---|---|
| `colors` | `{ light: Record<Role, "#rrggbb">, dark: same keys }` | `roles(scheme)` via `tables::COLOR_ROLES` |
| `typeScale` | `Record<TypeRole, { font, size, lineHeight, weight }>` | `typography::ALL` |
| `shape` | `Record<Radius, number>` | `tables::CORNER_RADII` |
| `elevation` | `{ levels: { dp, shadowAlpha: { light, dark } }[], scrimAlpha }` | `elevation::LEVELS`, `SCRIM_ALPHA` |
| `motion` | `{ duration: Record<Name, ms>, easing: Record<Name, [n,n,n,n]> }` | `tables::DURATIONS`, `EASINGS` |
| `state` | `{ opacity: Record<State, number>, focusRingWidth }` | `tables::STATES`, `state::FOCUS_RING_WIDTH` |
| `spacing` | `Record<Step, number>` | `tables::SPACING` |

Every export is `as const` and always present (an empty family is `{}`).
