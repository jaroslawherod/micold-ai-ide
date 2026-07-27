# Phase 1 Data Model: Material Component Architecture

**Feature**: `specs/017-material-component-architecture` | **Date**: 2026-07-27

This feature introduces almost no new data. What it does is **relocate ownership** of data that
already exists. The model below is therefore organised by *who owns what* rather than by entity.

---

## The ownership rule

| | Owner | Test |
|---|---|---|
| **Logical state** | the application | would still matter with animation disabled |
| **Presentation state** | the component instance | exists only to make a transition look right |

Everything below follows from that one line (FR-011, FR-012).

---

## Presentation state — moves into components

Held per widget instance in the widget tree, dropped when the instance is (research R1). No central
store, no caller-allocated identity.

| State | Currently | Moves to |
|-------|-----------|----------|
| menu fade progress | central animator, global enum variant | the menu component |
| sidebar slide progress | central animator, global enum variant | the sidebar component |
| main-view fade progress | central animator, global enum variant | the main view |
| resize-handle hover highlight | central animator, global enum variant | the resize handle |
| overlay fade progress | central animator, global enum variant | the overlay primitive |
| filter-panel fade progress | central animator, global enum variant | the filter panel |
| per-row hover fade | second animator, keyed by hashed row identity | each row instance |
| currently-hovered row | application-state field | each row instance |
| resize-handle drag flag | application-state field | the resize handle |

Afterwards the global enumeration and both animators are deleted (FR-014, FR-015).

**Invariants** (tested):
- Two instances of the same component animate independently; neither can observe the other.
- A removed instance takes its state with it — nothing continues to animate (FR-025).
- No component requires an identity from its caller.
- No public component API exposes a progress value, animation key or state type (FR-013).

---

## Logical state — stays with the application

Worktrees, active session, expanded tree nodes, sidebar visibility and width, open-menu identity,
drafts, filters, theme preference.

**Invariant** (tested): everything that survives a restart survives it identically, and no persisted
value changes shape (SC-006). Application state is not itself a serialized type — persistence goes
through the core's own store types — so none of the moved presentation state was ever persisted.

---

## New pure logic in the render-free core

Decisions extracted so component-owned state does not put logic beyond tests (FR-017). The density scale and ripple geometry were considered here and deferred to [018](../018-material3-visual-system/data-model.md) with the appearance they serve.

### Dismissal rules

| Input | Output |
|-------|--------|
| surface kind (modal / non-modal), dismissible flag, trigger (outside click, Escape, scroll) | dismiss or ignore |

**Invariants**: non-modal dismisses on all three triggers; modal dismisses on Escape and scrim click
only; a surface declared non-dismissible ignores every trigger; the rule is total — no input
combination is undefined.

---

## Stacking order

Floating surfaces render in a defined order (FR-010) owned by the single overlay primitive rather
than by whichever surface happened to be composed last.

**Invariant** (tested): given two open surfaces, the order is deterministic and independent of
composition order.

---

## What this feature deliberately does not model

- **No token values change.** Tokens are relocated, not re-valued (FR-021). New values belong to
  [`018`](../018-material3-visual-system/data-model.md).
- **No persisted data.** Nothing gains, loses or changes a stored field.
- **No session-scoped state.**
- **No appearance.** Elevation levels, type roles, state-layer opacities and motion tokens are
  *carried* by this feature's token move but not *changed* by it.
