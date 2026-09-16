# Data Model: The settings rail slides when it collapses and expands

**Feature**: 030-settings-rail-slide | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

Nothing here is stored, serialised or sent. Every entity is widget-tree state or a value derived
during one layout, private to `ui/material/section_list.rs`. The one piece of application state,
the rail's collapsed flag, already exists (027 FR-026d) and is unchanged.

---

## Entities

### `SectionList` (existing, unchanged API)

The builder callers use: `sections`, `selected`, `badge_accent`, `collapsed`, `toggle`. Converting
it to an `Element` now yields a `Rail` whose child is the rail's single rendering, in which every
destination row and the collapse control is a `RowSlide`.

### `Rail` (widget) and its tree state `Slide`

| Field | Type | Meaning |
|---|---|---|
| `child` | `Element` | The rail's rendering: padded column of `RowSlide`s, a fill space, the control's `RowSlide` |
| `collapsed` | `bool` | The state the rail is heading for, from `SectionList::collapsed` |
| `Slide.progress` | `Progress` | 1.0 = labelled, 0.0 = icons only; `MEDIUM_4`, `EMPHASIZED` |

Derived per layout: `width = 80 + 208 · progress.value()`.

Validation rules:

- `Slide` is created with `progress` at the target of the first `collapsed` it sees, so a rail
  mounted in either state lays out at its final width on its first frame (FR-005, SC-006).
- The target is `collapsed ? 0.0 : 1.0`, set every `update` through `on_layout_frame`, which
  retargets from the current value (FR-004, SC-004) and requests frames only while the value is
  changing (FR-011).
- `size()` is `Fixed(width_of(collapsed))`, the destination width, as today.

### `RowSlide` (widget), no tree state of its own beyond its forms' trees

It remembers nothing between layouts: the drawn form follows from the width it is given, and focus
follows from which form's tree holds it (research R5).

| Field | Type | Meaning |
|---|---|---|
| `forms` | `Forms` | The row's rest forms (below) |
| `has_badge` | `bool` | Whether the row is badged; selects `Marked` mid-slide |

`Forms` is one of:

| Variant | Holds | Used for |
|---|---|---|
| `Sliding { labelled, marked, icons_only }` | Three `Element`s of the same widget type, always in this child order | A destination with an icon, and the collapse control; unless the row is badged, `marked` is a never-drawn copy of `labelled` given a zero-size parked node (laid out only for the focus step), so its tree survives a badge toggling (research R3) |
| `Single(element)` | One `Element` | A destination with no icon (research R3a) |

Rest widths: `labelled` and `marked` at 272, `icons_only` at 64; `Single` at the width given.

Validation rules:

- Every form is a row exactly as today's `rendering` builds it for its state, so the forms at rest
  are today's rows (FR-006).
- `marked` differs from `labelled` only in the icon's tint.
- Exactly one form is drawn per layout; the others are laid out and parked (moved out of reach, as
  `NavigationDrawer::parked` does), so they cost no hit-testing and no drawing. Parked forms receive
  only redraws and the release, finger-lifted, finger-lost and cursor-left events, with the cursor
  unavailable; their messages and captures are dropped and their redraw requests and invalidations
  kept (research R5, contract §5 *Pointer*).

### `RowForm` (value)

`enum RowForm { Labelled, Marked, IconsOnly }` — which of a `Sliding` row's forms is drawn. An enum
so "labelled and icons-only at once" cannot be represented (Principle V).

### Derived values (pure functions)

| Function | Inputs | Output | Rule |
|---|---|---|---|
| `fraction` | row's max width `w` | `f ∈ [0, 1]` | `clamp((w − 64) / 208, 0, 1)` (R2) |
| `form` | `f`, `has_badge` | `RowForm` | `IconsOnly` if `f ≤ 0.001`; `Labelled` if `f ≥ 0.999`; else `Marked` if badged, else `Labelled` (R3) |
| `offset` | `f`, `x_labelled`, `x_icons`, `x_drawn` | `dx` | `x_icons + (x_labelled − x_icons) · f − x_drawn` (R4) |

`x_labelled`, `x_icons` and `x_drawn` are the x of the first leaf node, depth-first, of each form's
laid-out node, which is its glyph, measured relative to the form's node before any form is parked
or offset. A `Single` row has no icon and `dx = 0`.

---

## State transitions

### Rail

```text
            press (collapsed := true)                 value reaches 0
  Labelled ───────────────────────────▶ Collapsing ───────────────────▶ IconsOnly
  (p = 1)                               (0 < p < 1)                     (p = 0)
     ▲                                   │     ▲                            │
     │ value reaches 1                   │press│press                       │ press (collapsed := false)
     │                                   ▼     │                            ▼
     └──────────────────────────────── Expanding ◀──────────────────────────┘
                                       (0 < p < 1)
```

- Every press flips `collapsed` at once (FR-010). A press mid-slide swaps Collapsing ↔ Expanding
  from the value reached.
- Only `Labelled` and `IconsOnly` are at rest; only they request no frames.
- Leaving Settings drops the tree; reopening mounts at the target (spec Edge Cases).

### A `Sliding` row, as `f` moves

```text
 collapse:  Labelled ──(f < 0.999)──▶ Marked* ──(f ≤ 0.001)──▶ IconsOnly
 expand:    IconsOnly ──(f > 0.001)──▶ Marked* ──(f ≥ 0.999)──▶ Labelled
                                      * Labelled when the row has no badge
```

On every `layout` (so on each transition without detecting it):

1. The chosen form is placed and every other form parked.
2. If a parked form holds focus, focus moves to the same-numbered focusable in the chosen form,
   with its `visible` flag, so pointer focus stays unshown (FR-007, FR-006). When none does, nothing
   happens, which makes the step idempotent.
3. `offset` is computed for the chosen form, so the icon's drawn x is on its line whichever form
   is drawn (FR-014).

A selection change mid-slide rebuilds the row with the other variant: `x_labelled` changes, `f`
does not, and the icon moves at once by `(x_labelled′ − x_labelled) · f`, which is FR-014's
exception.
