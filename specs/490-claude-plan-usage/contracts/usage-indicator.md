# Contract: `UsageIndicator` and its place in the app bar

Shared component, `crates/micold-client/src/ui/material/usage_indicator.rs` (Principle VIII).

## U1. API

```rust
UsageIndicator::new(label: impl Into<String>, roles: &Roles)
    .warning(bool)          // default false
    .details(Vec<String>)   // tooltip lines; empty = no tooltip
    .into()                 // Element<'a, M>
```

Takes strings, not `UsageReading`: `ui::material` does not depend on feature modules (the
`PullRequestIndicator` rule). The view maps `CurrentUsage` (data-model) to these arguments.

## U2. Look

| | Normal | Warning |
|---|---|---|
| Glyph | `Icon::PlanUsage` (`data_usage`, U+E1AF), 18 px | `Icon::UsageWarning` (`warning`, U+E002), 18 px |
| Glyph role | `on_surface_variant` | `error` |
| Label | `label_large`, `on_surface_variant` | `label_large`, `error` |

- U2.1 The glyphs are different shapes, so the warning is not colour alone (FR-010).
- U2.2 Height 40 px (the app-bar action height), 8 px glyph–label gap, 12 px ends; width follows
  the label. Light and dark come from `roles`.
- U2.3 Not pressable. Hover shows the shared `Tooltip` with the details lines (FR-006), with the
  tooltip's existing rest delay.

## U3. Placement

`ui/toolbar.rs::view` adds the indicator as the first `Toolbar::action`, before the project
switcher, only when the switch is on and `CurrentUsage` exists (FR-004, FR-015). Absent, the bar
is exactly as today.

## U4. Gates and showcase

- Showcase: a `UsageIndicator` entry in `showcase/sections/atoms.rs` with a normal, a warning and a
  long-label sample (`showcase/samples.rs`).
- Geometry: `tests/gates/bar_controls_hold_their_size.rs` covers the indicator in the bar (height
  matches the switcher; the bar's other actions keep their positions with it present).
- `tests/icons.rs` locks U+E1AF and U+E002 to the two new variants; `tests/icon_roles.rs` covers
  their roles.
