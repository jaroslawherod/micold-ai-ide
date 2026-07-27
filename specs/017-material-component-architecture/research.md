# Phase 0 Research: Material Component Architecture

**Feature**: `specs/017-material-component-architecture` | **Date**: 2026-07-27

Findings verified against the vendored sources actually in `Cargo.lock` — `iced 0.13.1`,
`iced_core 0.13.2`, `iced_widget 0.13.4` — not from documentation or memory. Each names the file
read. Findings that concern *visual* capability rather than architecture live in the visual
feature's [research](../018-material3-visual-system/research.md).

---

## R1 — Can a component own its own state, or must the application hold it?

**Decision**: Components own it. They are custom widgets carrying per-instance state in the widget
tree; no central store and no caller-allocated identity.

**Evidence**:
- `iced_core-0.13.2/src/widget.rs:83-100` — the `Widget` trait exposes `fn tag()`, `fn state()`,
  `fn children()` and `fn diff()`: the per-instance state hooks.
- `iced_core-0.13.2/src/widget/tree.rs:212` — `tree::State::{None, Some(Box<dyn Any>)}` carries
  arbitrary state across frames.
- The `advanced` feature, required to implement `Widget` directly, is already enabled in the
  workspace manifest.

**Consequence**: FR-011 and FR-013 are implementable as written. A component holds its hover
progress and press state itself; a call site never learns they exist. Because a removed widget
drops its state, "nothing animates after an element disappears" is structural rather than policed.

**How Principle I stays satisfied**: *storage* moves into the widget, *decisions* stay pure. The
codebase already demonstrates the split — the per-session activity badge documents that "the
signal→emphasis decision is a pure function so it is unit-testable independent of theming; the
builder maps emphasis to a glyph + role colour." The overlay dismissal rules follow the same shape (FR-017).

**Alternatives considered**: keeping a central animator read by call sites (rejected — it is the
coupling FR-011 removes, and forces every call site to allocate a key); component state in a
side-table outside the widget tree (rejected — it would not be dropped with the widget, so a
removed element could leak an animation).

---

## R2 — What exactly leaks across the boundary today?

**Decision**: Measured, not estimated, so SC-001 has a real baseline.

**Evidence** (current tip):
- **13** feature modules under `crates/micold-client/src/ui/` import rendering widgets directly.
- **119** direct style applications outside the component library.
- **135** raw text-size references across the client.
- Only **one** existing test touches the styling layer (`style::chip`), so making that layer
  internal breaks a single test rather than requiring a test migration.

**Consequence**: the migration is bounded and countable, and the boundary test has a concrete
target — zero.

---

## R3 — How many overlay implementations exist?

**Decision**: Five, which is why their behavior has diverged.

**Evidence**: `Modal`, `MenuOverlay`, `ContextMenu` (`ui/material/menu.rs`),
`ProjectSwitcherOverlay` (`ui/material/project_switcher.rs`) and the select dropdown
(`ui/material/select.rs`) each implement their own positioning, backdrop and dismissal.

**Consequence**: FR-008's consolidation is the highest-leverage change in the feature — it removes
four implementations and makes every later change to floating surfaces a single edit.

**Note on scope**: unifying dismissal changes behavior for surfaces that differ today. That is
sanctioned and scoped by FR-022 to dismissal only.

---

## R4 — Where do the tokens live, and can they move cleanly?

**Decision**: Move them to the render-free core. Friction-free.

**Evidence**: the current token file sits in the client crate but imports only
`micold_core::naming` and `micold_core::theme` — both already in the core. The core's manifest
declares no rendering dependency; the three occurrences of the renderer's name in it are comments
asserting exactly that.

**Consequence**: FR-020 becomes a compile-time guarantee rather than a convention — the core
*cannot* name a rendering type. The move needs no new dependency edge.

**Scope discipline**: FR-021 makes the move mechanical. Introducing new token values is the visual
feature's work. Moving and re-valuing in one step would forfeit this feature's zero-visual-change
property, which is what makes it reviewable.

---

## R5 — Does the stack expose what the overlay needs?

**Decision**: Yes.

**Evidence**: the scrollable widget reports viewport offset on scroll, which is what dismiss-on-scroll
requires; the codebase already composes floating content in five places, so positioning and backdrop
are proven patterns rather than new ground.

**Deferred**: a ripple renderer and a density resolver were considered for this layer and moved to
[018](../018-material3-visual-system/research.md). Both exist only to serve an appearance this
feature does not introduce, so building them here would add untested code with no consumer — and
the ripple carries an unresolved question (whether the pointer area reports element-relative or
window-absolute coordinates) that belongs with the work that actually needs the answer.

---

## R6 — What is the test command, and is the old one still valid?

**Decision**: `cargo test --workspace` (`mise run test`). Baseline **781 passing, 0 failed** on the
current tip.

**Evidence**: `mise.toml`'s test task runs `cargo test --workspace`; the client crate has no
`[features]` section, so the `--no-default-features` invocation some documentation still references
no longer exists.

**Note — pre-existing drift**: `CLAUDE.md` still documents the old invocation. Corrected as a task
here since this feature depends on the right command.

---

## Summary

| # | Finding | Effect on the spec |
|---|---------|--------------------|
| R1 | Widgets carry per-instance state; `advanced` already enabled | FR-011/FR-013 implementable; state does **not** go in the core |
| R2 | 13 modules, 119 style applications, 135 raw sizes, 1 coupled test | SC-001 has a measured baseline |
| R3 | Five overlay implementations | FR-008 confirmed as the highest-leverage change |
| R4 | Token move is friction-free; core cannot name a renderer | FR-020 is a compile error, not a convention |
| R5 | The overlay has everything it needs | No new dependency; ripple and density deferred to 018 with their appearance |
| R6 | `cargo test --workspace`; 781 passing | Parity has a concrete baseline |
