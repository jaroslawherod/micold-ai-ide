# Contract: New Component API Surface

**Feature**: [018 — Material 3 Visual System](../spec.md)

**Companion to**: [`contracts/design-tokens.md`](./design-tokens.md), which owns every *value*.
This document owns every *shape* — what a component is constructed from, what it exposes, and what
it refuses to do. A dp figure or an opacity in this file would be a duplicate with an independent
life, so there are none: anatomy lives in design-tokens §7 and is referenced, never restated.

**Extends**: [017's `component-api.md`](../../017-material-component-architecture/contracts/component-api.md),
which governs the wrapping rule, the `cdk`/`material` layer split, the single overlay, the builder
rule and the encapsulation rule. Everything there still binds. This contract adds only the three
components 018 introduces, and does not restate 017's rules.

---

## 1. What is new here

Feature 017 built the component library. This feature restyles it and adds exactly three
components:

| Component | Layer | Why it did not exist before |
|-----------|-------|------------------------------|
| `FormField` | `ui/material/form_field.rs` | Nothing needed the shared field chrome until fields gained a label, supporting text and an error state (FR-031a–c) |
| `Ripple` | appearance in `ui/material/ripple.rs` over the renderer in `ui/cdk/ripple.rs` | Deliberately deferred out of 017: it exists only to serve an appearance that did not exist yet (017 contract §2, scope note) |
| `Snackbar` | `ui/material/snackbar.rs` | Replaces an inline layout node; needs a floating surface and queue presentation (FR-032) |

`Button`, `Text`, `TextField`, `Checkbox`, `Scrollable` and `Surface` are **not** new. This feature
changes what they look like, not whether they exist.

---

## 2. The new components

### 2.1 `FormField` (FR-031a, FR-031b, FR-031c)

A **wrapper**, on the model of Angular Material's form field — the precedent this library already
mimics. It wraps whichever control it is given rather than replacing it.

```
FormField::new(control, roles)
    .label("Branch name")          // in-container; rests over the value, floats when populated
    .supporting("Lowercase only")  // beneath the container
    .error(Some("Already exists")) // switches indicator + supporting text to the error role
    .leading(icon)                 // optional adornments
    .trailing(icon)
    .into()
```

**The wrapper owns**: the filled container, the bottom active indicator, the in-container label, the
supporting-text slot, the error presentation, and the optional leading/trailing adornment slots.
Anatomy per design-tokens §7.7.

**The wrapped control owns**: its own input behavior, and nothing else. `TextField` and the select
MUST NOT draw a container, an indicator, a label or supporting text of their own — that is the
duplication FR-031c removes, and it is what the seven input call sites do today.

**The active state** that thickens the indicator and takes the accent colour differs by control,
and the wrapper MUST accept it rather than assume it: **focus** for a text input, **open** for the
select, which cannot report focus at all (FR-043a). The rendered result is identical; only the
trigger differs.

**The populated state** decides where the label sits, and the wrapper MUST accept it for the same
reason it accepts the active state: it is handed an opaque control and cannot see whether an input
holds text or a select holds a selection. A resting label occupies the value's line, so a control
whose label is resting MUST NOT also draw a placeholder — the two would overprint.

**Fidelity gap.** The label takes both of Material's positions but **snaps** between them rather
than animating — the rendering stack's text input has no label concept to transition. Both
endpoints are correct; only the transition is absent. Accepted gap #4 (FR-044), design-tokens
§7.7.

---

### 2.1a `Ripple` (FR-024a – FR-024g)

The appearance half of the press indication. The renderer — press capture, geometry, phase
progression and lifetime — is a **behavior primitive** in `ui/cdk/ripple.rs` and carries no colour
or opacity of its own (FR-024f). `material::Ripple` supplies the Material appearance over it.

```
Ripple::new(content, roles).into()   // composed inside a component, never at a call site
```

**Composition, not opt-in.** `Ripple` is composed *inside* `Button`, `TreeView`, `MenuItem`, `Tag`,
`ToggleChip` and `IconButton`. A call site presses a button; it never learns a ripple exists
(FR-024e). No call site may construct a `Ripple` directly.

**Geometry** (FR-024b):
- Origin is the **actual press point** within the element, never its centre — except when no pointer
  position is known (keyboard or synthetic activation), where it falls back to the centre.
- An origin outside the element's bounds is clamped into them.
- End radius reaches the element's **furthest corner**, so an off-centre press still covers it.
- Clipped to the element's own shape, so it never spills past a pill or rounded corner.

**Layering**: beneath content, above container. It composes *with* the state layers rather than
replacing them, drawn in the element's state-layer colour at the pressed opacity (FR-024c,
design-tokens §5).

**State lives in the component instance** (FR-024e). No central registry, no animation key, no
progress threaded in from outside. Two elements rippling at once animate independently (FR-024d).

**Frames** come from the motion primitive, never from a direct request — 017 holds the rendering
layer to exactly one frame-request site and this feature does not add a second (FR-039e). A faded
ripple releases its state, so nothing animates at rest (FR-039a).

**Coordinate space** MUST be confirmed against a real widget before the renderer is finalised: an
origin expressed in the wrong frame places every ripple incorrectly, and the terminal canvas works
in absolute window coordinates (FR-024g).

---

### 2.2 `Snackbar` (FR-032, FR-032a, FR-032b, FR-032c)

A floating, elevated surface in the inverse colour roles, replacing the inline notification strip.
Anatomy per design-tokens §7.8.

```
Snackbar::new(visible_notification, roles).on_dismiss(msg).into()
```

**The component owns presentation. The core owns the queue.** Queue discipline — which notification
is visible, what order the rest follow in, when each expires, how dedup and the retention cap
interact — is pure decision logic and lives in `micold-core::notify`, tested with no renderer. The
component renders whatever is currently visible and reports dismissal. The application supplies
notifications and is responsible for neither sequencing nor timing (FR-032a).

**One at a time** (FR-032a). Further notifications queue behind the visible one and are shown in
turn. Each auto-dismisses after its duration; manual dismissal remains available and promotes the
next immediately.

**Duration by severity** (FR-032b): informational notices take the short duration, errors the long
one, so an error is not lost before it can be read.

**Stacking**: renders above a dialog and its scrim, and must not permanently obstruct the dialog's
action row (spec Edge Cases).

**Not the connection banner** (FR-032c). The persistent connection-status strip is a *different
component* and MUST NOT be folded in: it is full-width, deliberately non-dismissible and not
queued. Material treats banners and snackbars as separate components, and only the dismissible
notification stack becomes a snackbar.

---

## 3. What this contract does not cover

- **Every value** — heights, paddings, corner sizes, opacities, durations, colour roles. Those are
  [`design-tokens.md`](./design-tokens.md), which supersedes feature 003's contract in full.
- **The rules all components follow** — wrapping, the layer split, the builder form, encapsulation,
  the single overlay. Those are 017's contract and are unchanged by this feature.
- **Restyled components.** `Button`, `TextField`, `Toolbar`, `TreeView`, `Modal`, `Menu`, `Tag` and
  the rest change appearance only; their API surface is 017's and is not restated here.
