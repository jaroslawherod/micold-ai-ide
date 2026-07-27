# Contract: Shared Component API Surface

**Feature**: `specs/017-material-component-architecture` | **Date**: 2026-07-27

This document specifies **how components are structured and exposed** — the boundary, the layers,
the API shape, and who owns which state. It deliberately says **nothing about appearance**: this
feature reproduces the current look exactly (FR-005, FR-023). Every colour, elevation, corner,
height and type role is specified by [feature 018's design-tokens
contract](../../018-material3-visual-system/contracts/design-tokens.md) and applied there.

If a rule here mentions a visual value, that is a defect in this document.

Constitution Principle VIII requires every shared component to present a chainable builder
terminating in `.into()`, mirroring the rendering stack's own widget idiom — not free functions with
many positional parameters.

---

## 1. The wrapping rule (FR-001 – FR-005)

**The library wraps the rendering stack. Feature modules never touch it directly.**

Today the boundary leaks: 13 feature modules construct rendering widgets and apply styling
themselves at 119 call sites, and 135 sites select a raw text size. Every one is free to render a
slightly different button, and many do.

**Wrapped** — has an appearance, so it must come from the library:

| Rendering widget | Wrapper |
|---|---|
| button | `Button` (the variants already in use) + existing `IconButton` |
| text | `Text` |
| text input | `TextField` |
| select / pick list | existing `Select` |
| checkbox | `Checkbox` |
| container used as a surface | `Surface` |
| scrollable | `Scrollable` |
| progress indicator | existing `StageProgress` |

**Not wrapped** (FR-003) — pure layout, no appearance: rows, columns, spacers, stacks and
pointer-area wrappers. These position other widgets and have nothing to style. Wrapping them would
add indirection for no gain.

**Parity is the requirement** (FR-005). Each wrapper reproduces what it replaces exactly. This
feature changes *where* an appearance is decided, never *what* it is.

**Enforcement** (FR-004): a test asserts that no module outside the two library layers imports a
wrapped rendering widget or references the styling layer. The styling layer becomes internal, so a
call site is structurally unable to reach it. That is what makes the boundary hold as the codebase
grows rather than decaying between reviews.

---

## 2. The layer split (FR-006, FR-007)

Two layers, mirroring the established separation between a component *development kit* and the
*styled component set* built on it:

```
ui/cdk/        behavior, no appearance     the overlay
ui/material/   appearance on top of it     Button, Text, TextField, Surface, …
```

- A behavior primitive MUST NOT hard-code an appearance value (FR-007).
- A styled component MUST NOT re-implement a behavior the lower layer provides (FR-007).

**Scope note.** `cdk` contains exactly one primitive in this feature: the overlay. A ripple
renderer and a density resolver were considered and deliberately deferred to
[018](../../018-material3-visual-system/spec.md) — both exist only to serve an appearance that does
not exist here yet, so building them now would add untested code with no consumer.

---

## 3. One overlay (FR-008 – FR-010)

**Window-level** floating surfaces are built on a single overlay primitive owning positioning,
backdrop, dismissal and stacking order.

Before this feature there were **five** independent implementations — the modal, the overflow menu,
the context menu, the project-switcher popover and the select dropdown — which is why their
behavior had diverged.

### What was built

Four of the five moved onto `cdk::overlay`. Hand-rolled implementations went from four to zero.

The **select dropdown did not move, and should not.** It is built on the rendering stack's
`pick_list`, which implements `Widget::overlay()` itself and is positioned from its trigger's
on-screen bounds. That is what lets it work inside a content-sized dialog, where a window-level
surface has no fill-sized window to anchor against — precisely the failure a hand-rolled version
produced, revealing the list inline. `select.rs` never had an implementation to remove.

So the contract is one primitive for window-level surfaces, plus a **closed list** of delegations
to the rendering stack's own overlay system, which is itself a single shared implementation:

| Delegation | Where | Why it cannot be window-level |
|---|---|---|
| `pick_list` | `material/select.rs` | must anchor to its trigger inside a content-sized dialog |
| `tooltip` | `material/mod.rs` | follows its trigger; has no backdrop, dismissal or stacking order to own |

The list is held closed by `tests/one_overlay_implementation.rs`, which fails both when an
unsanctioned delegation appears and when a sanctioned one disappears without being struck off.

| Concern | Owner |
|---|---|
| Position and backdrop | the overlay primitive |
| Dismissal rules | pure logic in the render-free core (FR-017) |
| Stacking order | the overlay primitive (FR-010) |
| Surface appearance | **not this feature** — see 018 |

**Unified dismissal (FR-009)** — the one sanctioned behavior change (FR-024):

| Surface kind | Dismisses on |
|---|---|
| Non-modal on the primitive (menus, context menus, popovers) | outside click, Escape, scroll beneath |
| Modal dialog | Escape, scrim click |
| Modal declared non-dismissible | nothing — reserved for dialogs where losing input destroys work |
| Widget-attached (select dropdown) | **the rendering stack's own rule** — any left click. Not Escape, not scroll beneath |

The rule must be **total** for surfaces on the primitive: no combination of surface kind and
trigger may be undefined.

> **Known shortfall against FR-009 — accepted deviation.** FR-009 requires *every* non-modal
> floating surface to dismiss on outside click, Escape and scroll beneath. The select dropdown
> meets one of the three: iced's `pick_list` closes on any left press, has no Escape handler, and
> does not close when content scrolls beneath it. Earlier revisions of this contract listed the
> select dropdown as following the unified rule, which was never true.
>
> **Decided**: accept the gap rather than fix it now. `pick_list`'s open/closed state is private to
> `iced_widget` — no `Operation`, no public hook — so closing it on Escape or scroll beneath is not
> reachable through a wrapper. The only way to add it is to vendor `pick_list.rs` (929 lines) and
> `overlay/menu.rs` (~340 lines) into this crate and add the missing event handling, which trades a
> minor interaction gap for a permanent fork: manual reconciliation against upstream on every iced
> upgrade, and a second hand-rolled overlay implementation of exactly the kind FR-008 consolidated
> away. Not worth it for closing a combo box with Escape.
>
> **Revisit if**: iced adds a public way to close a `pick_list` programmatically or exposes
> Escape/scroll hooks directly (check on the next iced upgrade), or if this gap starts costing real
> user complaints — at which point the vendoring cost above is the thing to weigh against it.

---

## 4. The builder rule (Principle VIII)

Every shared component obeys the same shape:

- A public struct constructed with **only its required inputs**.
- Optional configuration through chainable, `self`-consuming methods.
- Termination via a conversion into an element, so call sites end in `.into()`.
- The active theme supplied through the builder, preserving the light/dark guarantee.

A change that adds or edits a shared component with a free-function or many-positional-parameter
signature must be rejected in review, per the constitution's Component-reuse gate.

**Example shape** — note that nothing in the signature describes appearance or animation:

```
Button::filled(label, on_press, theme)
    .disabled(true)
    .into()
```

---

## 5. The encapsulation rule (FR-011 – FR-017)

**A component owns its own presentation state and hides how it works.**

| | Owner | Examples |
|---|---|---|
| **Logical state** | the application | sidebar collapsed, selected worktree, field value |
| **Presentation state** | the component instance | hover progress, expand progress, press feedback |

The dividing line (FR-012): *if the state would still matter with animation disabled it is the
application's; if it exists only to make a transition look right it is the component's.*

**Forbidden in a public component API** (FR-013): animation keys, animator handles, progress
values, style functions, internal state types, and rendering-stack types. A call site describes
what it wants — a label, a variant, a message to emit — never how it is rendered or animated.

**How state is held**: components are custom widgets carrying per-instance state in the widget
tree. No central store, no caller-allocated identity. Two instances cannot interfere because
neither can see the other, and a removed widget drops its state — which is what makes
"nothing animates after an element disappears" structural rather than policed.

**How Principle I stays satisfied** (FR-017): *storage* moves into the widget, *decisions* stay
pure. Dismissal rules and any state machine live in the render-free core, unit-tested with no
renderer present. The existing activity badge is the in-repo reference: its signal-to-emphasis
mapping is a pure function while its builder handles presentation.

### What moves (FR-014, FR-015)

| Currently central | Moved into | Built as |
|---|---|---|
| menu fade track | the menu component | `material::MenuOverlay` |
| sidebar slide track | the sidebar component | `material::NavigationDrawer` |
| main-view fade track | the main view | `material::ViewFade` |
| resize-handle hover track | the resize handle | `material::ResizeHandle` |
| overlay fade track | the overlay primitive | `material::Modal` |
| filter-panel fade track | the filter panel | `material::Accordion` |
| per-row hover-fade tracks, keyed by hashed row identity | each row instance | `material::HoverReveal` |
| resize-handle drag flag | the resize handle | `material::ResizeHandle` |

The global animated-element enumeration and both central animators are then deleted. Success is
measured by adding a newly animated element and finding that no enumeration variant and no
application-state field was needed (SC-005).

**One row of the original table was wrong, and is removed rather than quietly satisfied.** It read
*"currently-hovered row → each row instance"*. The hover *fade* did move, and the hashed row
identity that was its actual defect is gone with it. The hovered-row **field** did not, and should
not: it is what arms a row's delete button and attaches its tooltip. A widget owning that privately
would be a widget deciding whether a destructive action is available — which is a decision, not an
appearance, and so belongs on the far side of the FR-012 line drawn above. Recorded as a deviation
against T040.

Two components in the built table were not anticipated when it was written. `NavigationDrawer`
exists because the sidebar's slide is not a wrapper that shrinks to nothing: at zero width the
panel is *replaced* by the collapsed rail, so owning the slide means owning both elements.
`ResizeHandle` took the drag as well as the hover, which let a full-window pointer-capture layer be
deleted along with them.

### What deliberately does not move (FR-016)

Worktrees, active session, expanded tree nodes, sidebar visibility and width, open-menu identity,
drafts, filters, theme preference. Several are persisted, and persistence must be unchanged.

---

## 6. The style layer

The styling module is the only place tokens become rendering types. It is not a component and has
no builder; it exposes conversion helpers consumed by the components above.

**Contract**:
- It becomes **internal to the library** (FR-002). Feature modules cannot reach it, so a call site
  cannot render an off-spec variant of a shared component.
- It holds **no decision logic** — it converts and composes. That is what keeps it inside
  Principle I's GUI-wiring exception. Anything with branching lives in the render-free core instead.
- Its *values* are unchanged by this feature. Re-valuing them is 018's work.

---

## 7. Call-site contract

After this feature, a feature module contains only:

- layout primitives (FR-003),
- shared components from the library,
- application state and messages.

It contains no rendering-widget construction, no style application, and no component presentation
state. If a call site needs something a wrapper cannot express, **the wrapper gains the
capability** — the call site must not be able to bypass the library instead (FR-002).

---

## 8. What this contract does not cover

- **Every appearance value** — colour roles, elevation, shape, typography, state-layer opacities,
  component heights and paddings, motion timings. All specified in
  [018's design-tokens contract](../../018-material3-visual-system/contracts/design-tokens.md).
- **The press ripple** and **the density scale** — deferred to 018 with the appearance they serve.
- **Keyboard traversal** and any focus model beyond what exists today.
