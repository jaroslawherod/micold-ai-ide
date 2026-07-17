# Contract: Filter Panel UI

Governs the `FilterTrigger` button and the sidebar's filter accordion, and their wiring into
the sidebar and app shell. Builder-style API (Principle VIII) for the trigger; the accordion
itself is composed directly from existing/generic primitives rather than a bespoke component
(research R7 — superseded the original floating-overlay design in R2).

## `FilterTrigger<M>` (src/ui/material/filter_panel.rs)

```rust
pub struct FilterTrigger<M> { /* icon button, toggles the accordion */ }

impl<M> FilterTrigger<M> {
    /// A trigger emitting `on_toggle` when pressed, themed by `roles`.
    pub fn new(on_toggle: M, roles: Roles) -> Self;
    /// Whether any filter is currently active (drives the tint — research R4).
    pub fn active(mut self, active: bool) -> Self;
}

impl<'a, M: Clone + 'a> From<FilterTrigger<M>> for Element<'a, M>;
```

- Renders `IconButton::new(Icon::Filter, roles)` — the `filter_list` glyph (three
  descending-length lines, `U+E152`; research R5/R7) — tinted `primary` when `active`,
  `on_surface_variant` otherwise (matches `add_worktree`/`hide`'s tint convention).
- Wrapped in the shared `Tooltip` primitive with label "Filter worktrees", consistent with
  every other sidebar-header action.
- Placed at the **left** edge of the sidebar header, ahead of the "Worktrees" title (research
  R7 — moved from the right-side action cluster per direct user direction).

## `material::expand` (src/ui/material/animation.rs)

A new shared animation primitive, a vertical sibling to the existing `slide` (which does a
horizontal, edge-anchored reveal for the sidebar itself):

```rust
pub fn expand<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    progress: f32,
) -> Element<'a, Message>;
```

- `progress` 0.0 → zero height (collapsed); 1.0 → the content's full natural height (expanded).
- Unlike `slide` (which translates its child to anchor the reveal to the trailing edge — the
  right fit for a panel sliding out from behind a fixed handle), `expand` never translates its
  child: the top edge stays fixed, so it always reveals **top-down** — the natural accordion
  behavior for a panel that grows downward from a header.
- A passthrough widget: layout, events, and overlay all delegate to the child; only the
  reported/clipped height changes with `progress`. Applied unconditionally (no manual
  `progress <= 0` short-circuit needed) — mirrors how `material::slide(sidebar::view(...),
  progress)` is already applied unconditionally in `ui/mod.rs`.

## Accordion composition (`src/ui/sidebar.rs`)

```rust
let filter_accordion: Element<'_, Message> = expand(
    container(filter_bar(state, r))
        .padding(spacing::XS)
        .style(style::menu_surface(r)),
    filter_progress,
);
```

- `filter_bar(state, r)` (feature 008, unchanged logic) is the accordion's content: one chip
  per available filter, a "Clear filters" action when any is active, or (feature 009, FR-009) a
  "No tags to filter yet." message when `available_tag_filters()` is empty.
- Placed in the sidebar's own `column![header, filter_accordion, body]` — an ordinary layout
  child, not a floating/stacked overlay. At `filter_progress` 0 it occupies (approximately)
  zero vertical space; as it opens, the worktree list below is pushed down exactly as much as
  the accordion grows — no floating panel, no backdrop, no separate anchor math.
- `filter_progress` is threaded in from `ui/mod.rs::view()` via `sidebar::view`'s parameter
  list: `sidebar::view(state, scheme, row_fx, motion.get(MotionKey::SidebarFilter))`, mirroring
  exactly how `HandleHover`'s progress is passed into `sidebar::handle` as a plain `f32`.

## Wiring contract (`src/ui/sidebar.rs`, `src/ui/mod.rs`, `src/app.rs`, `src/main.rs`)

- `sidebar.rs::view()`: header row is
  `row![filter_toggle, title.width(Fill), add_worktree, hide]` (filter trigger first/leftmost —
  research R7). The accordion (`filter_accordion`) sits between `header` and `body` in the
  sidebar's content column.
- `ui/mod.rs::view()`: no longer composes any filter-specific overlay — the accordion lives
  entirely inside `sidebar::view()`'s own return value, which is what `material::slide(...)`
  wraps for the sidebar's own show/hide. (Contrast with the superseded R2 design, which
  composed a `FilterOverlay` in the same overlay-stacking chain as `MenuOverlay`/
  `ProjectSwitcherOverlay`.)
- `ui/mod.rs::subscription()`: unchanged from research R3 — when
  `state.overlay == Overlay::None && state.sidebar_filter_open`, returns an `on_key_press`
  subscription mapping `Escape` to `Message::SidebarFilterMenuToggled`. This did not need to
  change when the presentation moved from overlay to accordion.
- `app.rs::on_escape()`: unchanged from research R3 — leading check on `sidebar_filter_open`
  before the `state.overlay` match.
- `main.rs::motion_targets()`: unchanged from the original design —
  `(MotionKey::SidebarFilter, if app.core.sidebar_filter_open { 1.0 } else { 0.0 }, MENU_FADE)`
  drives `filter_progress` regardless of whether it animates an overlay or an accordion.

## Invariants

1. At `filter_progress` 0, the accordion occupies (approximately) zero layout height and
   cannot receive input — no dead-space, no invisible hit-testable area (a property `expand`
   gets for free from clipping to a zero-height box, unlike the superseded overlay's explicit
   `progress <= 0.001` early-return, which existed specifically to remove a floating backdrop
   that would otherwise capture clicks).
2. Toggling `sidebar_filter_open` never mutates `sidebar_filters` (FR-007/FR-008) — enforced by
   the reducer contract in `data-model.md`, tested in `tests/sidebar_state.rs`.
3. The panel's chip list is always built from the *current* `available_tag_filters()` /
   `sidebar_filters` on every render — since `iced` is immediate-mode, this is automatic (no
   stale snapshot to invalidate), satisfying FR-010.
4. Unlike the superseded floating-overlay design, there is no outside-click dismissal — the
   accordion is inline content, not a panel floating over other content (research R7). Escape
   and re-toggling the trigger are the only two dismissal paths (FR-006).
