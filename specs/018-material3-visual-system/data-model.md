# Phase 1 Data Model: Material 3 Visual System

**Feature**: `specs/018-material3-visual-system` | **Date**: 2026-07-26

Every entity here is **pure data in `micold-core`**, a crate that declares no rendering dependency
(feature 017). None of them can name a rendering type — that conversion lives in
`crates/micold-client/src/ui/style.rs`. Values are normative in
[`contracts/design-tokens.md`](./contracts/design-tokens.md); this document defines their
*structure* and *invariants*.

---

## Tonal palette

The raw material every color role is drawn from.

| Field | Type | Notes |
|-------|------|-------|
| `tones` | fixed map from tone → `Rgb` | Tone stops: 0, 4, 6, 10, 12, 17, 20, 22, 24, 30, 40, 50, 60, 70, 80, 87, 90, 92, 94, 95, 96, 98, 100 |

Six instances: `primary`, `secondary`, `tertiary`, `error`, `neutral`, `neutral_variant`. All are
baked from Material's baseline seed `#6750A4` (FR-005b) and are `const`.

**Invariants** (tested):
- Tone 0 is black, tone 100 is white, in every palette.
- Relative luminance is **monotonically non-decreasing** as tone increases. This is the check that
  catches a transcription error without asserting individual digits (plan risk 1).

---

## Color role

One semantic role, defined as a palette-and-tone pair rather than a literal color (FR-005a).

| Field | Type | Notes |
|-------|------|-------|
| `palette` | enum of the six palettes | which ramp to read |
| `light_tone` | tone | tone used in the light scheme |
| `dark_tone` | tone | tone used in the dark scheme |

Roughly 36 roles, enumerated in contract §1.2. Resolving a role for a scheme yields an `Rgb`.

**Invariants** (tested):
- Every `on_*` role clears **WCAG AA ≥ 4.5:1** against each background role it is paired with, in
  **both** schemes — the pairs are enumerated in contract §1.3. Failing this fails the build
  (FR-004, SC-001).
- `outline` clears the non-text 3:1 threshold against the surfaces it divides.
- Disabled states are **excluded** from the AA assertion: dimming to 0.38 opacity necessarily
  drops below AA, and WCAG exempts inactive controls (FR-024).

### Tag role

The eleven worktree/issue tags are ordinary color roles, not a special case (FR-006a).

| Field | Type | Notes |
|-------|------|-------|
| `hue` | one fixed hue per conventional-commit type, plus neutral for issue | contract §1.4 |
| fill / text tones | fixed recipe | light 40/100, dark 80/20 — the same recipe the accent roles use |

**Invariant**: because the tone delta matches `primary`/`on_primary`, AA holds by construction —
including under the hover, pressed and selected state layers (FR-024). Distinguishability across
the ten types is asserted separately (FR-006).

---

## Type role

| Field | Type | Notes |
|-------|------|-------|
| `size` | dp | |
| `line_height` | dp | absolute, not a ratio |
| `weight` | 400 or 500 | the only weights the scale uses (FR-008a) |
| `tracking` | dp | **recorded, never applied** — accepted fidelity gap #1 (FR-042) |

Fifteen roles: {display, headline, title, body, label} × {large, medium, small}. Plus three
sidebar-scoped roles (`sidebar_name`, `sidebar_session`, `sidebar_tag`) that *alias* existing roles
rather than introducing new sizes, so the 80% density decision stays one auditable table (FR-011).

**Invariants** (tested):
- Every role's `line_height` ≥ its `size`.
- Sizes decrease monotonically large → medium → small within each family.
- `weight` is exactly 400 or 500 — no role may request a weight the shipped fonts cannot render.
- Each sidebar role resolves to a role already in the scale; none defines its own size.

---

## Elevation level

| Field | Type | Notes |
|-------|------|-------|
| `surface_role` | color role | the tonal shift — what makes depth read in dark (FR-016) |
| `shadow_offset_y` | dp | |
| `shadow_blur` | dp | |
| `shadow_alpha_light` / `shadow_alpha_dark` | 0.0–1.0 | |

Six levels, 0–5. **One shadow per level**, not Material's separate key and ambient — the renderer
exposes a single shadow per widget (research R1).

**Invariants** (tested):
- Level 0 has no shadow.
- Offset and blur increase monotonically with level.
- Every level's `surface_role` is a real role in the role set.

---

## Shape size

A named corner radius. Seven: `none` (0), `extra_small` (4), `small` (8), `medium` (12),
`large` (16), `extra_large` (28), `full` (pill).

**Invariant** (tested): radii increase monotonically none → extra_large; `full` is the sentinel
pill value and is excluded from that ordering.

---

## State layer

| Field | Type | Notes |
|-------|------|-------|
| `opacity` | 0.0–1.0 | composited over the container |

Seven: hover 0.08, focus 0.10, pressed 0.10, dragged 0.16, selected 0.12, disabled_content 0.38,
disabled_container 0.12.

**Invariants** (tested): every opacity in (0.0, 1.0]; `pressed` ≥ `hover` (press must read
stronger than hover — spec US3 scenario 2).

**Applicability note**: `focus` applies only where focus is reachable — text fields and the select
control. Accepted fidelity gap #2 (FR-043).

---

## Motion token

Two kinds, both pure data:

| Kind | Fields | Values |
|------|--------|--------|
| Duration | `ms` | short 1–4 (50/100/150/200), medium 1–4 (250/300/350/400), long 1–4 (450/500/550/600) |
| Easing | four control points | standard and emphasized sets, contract §6.2 |

**Invariants** (tested): durations strictly increase within each band; every easing's control
points are in [0, 1] on the x axis (a valid cubic-bézier timing function).

---

## Notification queue

A pure, independently testable structure (feature 017) **owned by the snackbar component**. The
application pushes notifications; it does not sequence or time them, and does not hold the queue
(FR-032a, feature 017). The structure itself lives in `micold-core` so its discipline is unit-tested
without a renderer.

| Field | Type | Notes |
|-------|------|-------|
| `visible` | optional notification | at most one (FR-032a) |
| `pending` | ordered queue | shown in turn as each clears |
| `level` | info or error | selects the duration |
| `duration` | derived | info → 4 s, error → 10 s (FR-032b) |

**State transitions**:

```
(empty) --push--> visible
visible --push--> visible, other queued
visible --timeout or manual dismiss--> next pending becomes visible, or empty
```

**Invariants** (tested):
- Never more than one visible.
- Pushing a duplicate of the currently visible notification does not enqueue a second copy
  (dedup preserved from today's behavior, FR-032b).
- The queue respects the existing retention cap; overflow drops oldest pending, never the visible.
- An error's duration is strictly longer than an info's.
- Manual dismissal is always available and promotes the next pending immediately.

---

## Ripple state — owned by the component, not the application

**State ownership is [feature 017](../017-material-component-architecture/data-model.md)'s concern** — it established that the ripple renderer holds its own per-instance state. What this feature adds is the ripple's *appearance*: which color role it draws in, at what opacity, and on what timing.

Per-instance state, held inside the widget:

| Field | Type | Notes |
|-------|------|-------|
| `origin` | point within the element | press position; element center when unknown |
| `progress` | 0.0–1.0 | expand then fade |
| `phase` | expanding or fading | selects which duration/easing applies |

Pure functions in core, unit-tested without a renderer:

| Function | Contract |
|----------|----------|
| end radius | distance from origin to the element's furthest corner |
| clamp origin | an origin outside the element's bounds is pulled inside |
| default origin | with no known pointer position, the element's center |
| phase advance | expanding → fading → complete, given elapsed time and the duration tokens |

Because each widget owns its own state, per-element independence (FR-024d) is structural: two
ripples cannot interfere because neither can see the other. A removed element drops its state with
the widget, so a menu item pressed as its menu closes cannot leak a running animation.

**State transitions** (within one widget instance):

```
(idle) --press(origin)--> expanding --complete--> fading --complete--> (idle, state cleared)
```

**Invariants** (tested against the pure functions):
- An origin outside the element's bounds is clamped into them.
- With no known pointer position the origin is the element's center, not the point (0, 0) — the
  keyboard/synthetic-activation edge case.
- The end radius covers the element from its origin — it reaches the furthest corner.
- Phase advance reaches "complete" in exactly the summed expand + fade duration, so a ripple cannot
  run forever and the animation clock returns to idle (FR-024d).

## Relationships

```
Tonal palette --(read at a tone)--> Color role --+--> Tag role (fixed tone recipe)
                                                 |
                                                 +--> Elevation level (surface_role)
                                                 |
                                                 +--> State layer (composited over)

Type role ----+
Shape size ---+--> Component anatomy spec (contract §7)
Elevation ----+
Color role ---+

Motion token --> existing animations + app-bar elevation + snackbar enter/exit
                 + indeterminate progress + ripple expand/fade
Notification queue --> Snackbar component
Ripple state --------> Ripple wrapper, composed inside every interactive component
```

**Component anatomy spec** is not a runtime entity — it is the contract §7 tables. Each shared
component reads the tokens it needs; nothing serialises an "anatomy" object.

---

## What is deliberately absent

- **No persisted state.** This feature stores nothing (Principle IV unaffected). The
  light/dark/follow-system preference already lives in `micold-core::settings` and is untouched.
- **No per-session state.** The notification queue is global, exactly as the stack it replaces is.
- **No runtime color computation.** Ramps are `const` tables; there is no HCT solver and no seed
  input at runtime (research R7).
