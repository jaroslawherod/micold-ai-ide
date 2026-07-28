# The component library

How the rendering layer is organised, and the one rule that keeps it that way.

## Two layers

Everything that decides an appearance lives in `crates/micold-client/src/ui/`, split in two:

**`ui/cdk/`** — behavior primitives. Things that *do* something without deciding how it looks: the
overlay that positions and stacks floating surfaces, the `Progress` track that advances an
animation and knows when to stop. A primitive here must not hard-code a colour, a corner radius or
a duration.

**`ui/material/`** — styled components. `Button`, `TextField`, `Modal`, `NavigationDrawer`,
`TreeView`, and the rest. These decide appearance, and they are the only place that may. The style
layer (`ui/material/style.rs`) is *internal to this directory* — nothing outside it can reach the
functions that turn a design token into a widget style.

Everything else under `ui/` — `sidebar.rs`, `toolbar.rs`, `shell.rs`, the dialogs — is a **feature
module**. Feature modules compose components. They do not style widgets.

## The rule

> A feature module composes components. It never reaches for the rendering stack's widgets to style
> them itself.

Concretely, a feature module may use layout primitives (`row!`, `column!`, `Space`, `stack!`) and
components from `material::`. It may not call `.style(...)` on an iced widget, construct a
`container::Style`, or import `ui::material::style`.

The reason is not tidiness. When a call site can style a widget directly, the design system becomes
advisory: two places that should look the same drift, and changing the look means finding every
place that decided it. Making the styling layer unreachable turns "use the component" from a
convention into the only thing that compiles.

### Why this is enforced by tests rather than review

Three gates run in CI. Each exists because the corresponding mistake is easy to make and invisible
in a diff:

| Gate | File | What it catches |
|---|---|---|
| Boundary | `tests/material_boundary.rs` | a feature module importing the style layer or styling a raw widget |
| Builder API | `tests/material_builder_api.rs` | a component that is not built the same way as every other one |
| Opacity ratchet | `tests/component_api_opacity.rs` | a public component signature exposing *how* it works |

The ratchet is worth understanding before you add a component. It scans every public signature in
the library for a short list of forbidden things — progress values, style closures, animator
handles, rendering-stack types — and holds a `REMAINING` list of known exceptions. It fails **both**
when a new leak appears *and* when the list names something already fixed. It is currently empty,
which is the state it is meant to stay in.

## Components own how they look; the application owns what is true

The dividing line:

> If the state would still matter with animation disabled, it is the application's. If it exists
> only to make a transition look right, it is the component's.

So `sidebar_hidden` is application state — it is written to disk and restored next run. How far the
drawer has slid is not; the drawer owns that, in the widget tree, where the renderer already keeps
per-instance state.

This has a consequence worth stating plainly: **there is no central animator**, no enumeration of
animated elements, and no animation clock. A component that animates holds a `cdk::motion::Progress`
in its widget-tree state and asks the runtime for the next frame only while it is moving. Two
instances cannot interfere, because neither can see the other. A removed component drops its state
with it — so "nothing animates after an element disappears" is structural rather than policed.

The corollary for callers: a component takes a **destination**, never a position.

```rust
// The caller says where it should be:
Modal::new(dialog, roles).shown(state.overlay != Overlay::None)

// Not how far along it currently is:
fade(dialog, progress)   // ← what this used to look like
```

## Adding a component

1. Put it in `ui/material/`, one file per component, and register it in `mod.rs`.
2. Give it a builder: `Thing::new(required, args, roles)` plus chainable `.option(x)` steps, and a
   `From<Thing> for Element` at the end. The builder gate checks this.
3. Take design tokens (`Roles`, `spacing::*`, `type_scale::*`), never literal colours or sizes.
4. If it animates, own a `Progress` and take a destination. Do not add a parameter that carries
   progress in — the ratchet will fail, and it is right to.
5. If it needs to tell the application something, emit a message describing a **decision** ("the
   edge was dragged to x"), not a mechanism ("a drag started").
6. Add it to [the component showcase](component-showcase.md) — an `Entry` in
   `src/showcase/catalogue.rs` plus a render function in `src/showcase/sections/`. This is not
   optional and not a courtesy: `tests/showcase_completeness.rs` fails the build naming your component
   until you do, because a catalogue that silently omits things is worse than no catalogue at all.

Point 5 is the one most often got wrong. The resize handle used to emit `SidebarDragStarted` and
`SidebarDragEnded` so the application could mount a pointer-capture layer on its behalf. Owning the
drag itself let all of that be deleted; what remained was the single message that carried a
decision.

## Where the boundary genuinely bends

Two things sit outside the library on purpose, and both are documented at their call sites:

- **The terminal grid.** It draws cells from a VT model, not components, and is a feature module for
  that reason.
- **Widget-attached dropdowns.** `Select` is built on iced's `pick_list`, whose overlay is
  positioned by the rendering stack from the trigger's on-screen bounds. That is what makes it work
  inside a content-sized dialog, where a window-level floating surface has nothing to anchor
  against.

If you find yourself wanting a third exception, the gates will tell you — and the honest move is to
extend a component rather than add an entry to a `REMAINING` list.
